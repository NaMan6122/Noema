use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use noema_core::db::Database;
use noema_core::error::Result;
use rusqlite::params;
use serde::Serialize;

use crate::query::{Filter, ParsedQuery};

pub struct SearchEngine {
    db: Arc<Database>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub file_path: String,
    pub filename: String,
    pub score: f32,
    pub snippet: Option<HighlightedSnippet>,
    pub match_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HighlightedSnippet {
    pub text: String,
    pub highlights: Vec<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResults {
    pub results: Vec<SearchResult>,
    pub total_estimate: u64,
    pub took_ms: u64,
}

impl SearchEngine {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn search(&self, query: &ParsedQuery, limit: usize, offset: usize) -> Result<SearchResults> {
        let start = Instant::now();

        // If no text and no filters, return recent files
        if query.keyword_text.is_none() && query.exact_phrases.is_empty() && query.filters.is_empty() {
            return self.recent_files(limit);
        }

        // If only filters, no text — metadata-only search
        if query.keyword_text.is_none() && query.exact_phrases.is_empty() {
            return self.filter_only_search(&query.filters, limit, offset, start);
        }

        // FTS5 search
        self.fts_search(query, limit, offset, start)
    }

    fn fts_search(
        &self,
        query: &ParsedQuery,
        limit: usize,
        offset: usize,
        start: Instant,
    ) -> Result<SearchResults> {
        let conn = self.db.connection()?;

        // Build FTS5 match expression
        let mut match_parts: Vec<String> = Vec::new();

        if let Some(ref kw) = query.keyword_text {
            let escaped = escape_fts5(kw);
            match_parts.push(escaped);
        }

        for phrase in &query.exact_phrases {
            match_parts.push(format!("\"{}\"", escape_fts5(phrase)));
        }

        let match_expr = match_parts.join(" ");
        if match_expr.trim().is_empty() {
            return Ok(SearchResults { results: vec![], total_estimate: 0, took_ms: start.elapsed().as_millis() as u64 });
        }

        // Build filter conditions for the join
        let mut where_clauses: Vec<String> = Vec::new();
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        params_vec.push(Box::new(match_expr.clone()));

        for filter in &query.filters {
            match filter {
                Filter::FileType(types) => {
                    let placeholders: Vec<String> = types.iter().map(|_| "?".to_string()).collect();
                    where_clauses.push(format!("f.extension IN ({})", placeholders.join(",")));
                    for t in types {
                        params_vec.push(Box::new(t.clone()));
                    }
                }
                Filter::InPath(path) => {
                    where_clauses.push("f.path LIKE ?".to_string());
                    params_vec.push(Box::new(format!("{}%", path.to_string_lossy())));
                }
                Filter::SizeRange { min, max } => {
                    if let Some(min_val) = min {
                        where_clauses.push("f.size_bytes >= ?".to_string());
                        params_vec.push(Box::new(*min_val as i64));
                    }
                    if let Some(max_val) = max {
                        where_clauses.push("f.size_bytes <= ?".to_string());
                        params_vec.push(Box::new(*max_val as i64));
                    }
                }
                Filter::DateRange { after, before } => {
                    if let Some(a) = after {
                        where_clauses.push("f.modified_at >= ?".to_string());
                        params_vec.push(Box::new(a.to_rfc3339()));
                    }
                    if let Some(b) = before {
                        where_clauses.push("f.modified_at <= ?".to_string());
                        params_vec.push(Box::new(b.to_rfc3339()));
                    }
                }
                Filter::HasTag(tag) => {
                    where_clauses.push(
                        "f.id IN (SELECT ft.file_id FROM file_tags ft JOIN tags t ON ft.tag_id = t.id WHERE t.name = ?)".to_string()
                    );
                    params_vec.push(Box::new(tag.clone()));
                }
            }
        }

        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("AND {}", where_clauses.join(" AND "))
        };

        let sql = format!(
            "SELECT f.path, f.filename, c.content, c.heading, bm25(chunks_fts) as rank
             FROM chunks_fts
             JOIN chunks c ON c.id = chunks_fts.rowid
             JOIN files f ON f.id = c.file_id
             WHERE chunks_fts MATCH ?1
             {where_sql}
             ORDER BY rank
             LIMIT ?{} OFFSET ?{}",
            params_vec.len() + 1,
            params_vec.len() + 2,
        );

        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let path: String = row.get(0)?;
            let filename: String = row.get(1)?;
            let content: String = row.get(2)?;
            let heading: Option<String> = row.get(3)?;
            let rank: f64 = row.get(4)?;
            Ok((path, filename, content, heading, rank))
        })?;

        let mut results: Vec<SearchResult> = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        for row in rows {
            let (path, filename, content, heading, rank) = row?;
            if seen_paths.contains(&path) {
                continue;
            }
            seen_paths.insert(path.clone());

            let snippet = extract_snippet(&content, query.keyword_text.as_deref(), heading.as_deref());

            results.push(SearchResult {
                file_path: path,
                filename,
                score: (-rank as f32).max(0.0),
                snippet: Some(snippet),
                match_type: "Keyword".to_string(),
            });
        }

        let total = results.len() as u64;

        Ok(SearchResults {
            results,
            total_estimate: total,
            took_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn filter_only_search(
        &self,
        filters: &[Filter],
        limit: usize,
        offset: usize,
        start: Instant,
    ) -> Result<SearchResults> {
        let conn = self.db.connection()?;

        let mut where_clauses: Vec<String> = vec!["f.is_indexed = 1".to_string()];
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        for filter in filters {
            match filter {
                Filter::FileType(types) => {
                    let placeholders: Vec<String> = types.iter().map(|_| "?".to_string()).collect();
                    where_clauses.push(format!("f.extension IN ({})", placeholders.join(",")));
                    for t in types {
                        params_vec.push(Box::new(t.clone()));
                    }
                }
                Filter::InPath(path) => {
                    where_clauses.push("f.path LIKE ?".to_string());
                    params_vec.push(Box::new(format!("{}%", path.to_string_lossy())));
                }
                Filter::SizeRange { min, max } => {
                    if let Some(min_val) = min {
                        where_clauses.push("f.size_bytes >= ?".to_string());
                        params_vec.push(Box::new(*min_val as i64));
                    }
                    if let Some(max_val) = max {
                        where_clauses.push("f.size_bytes <= ?".to_string());
                        params_vec.push(Box::new(*max_val as i64));
                    }
                }
                Filter::DateRange { after, before } => {
                    if let Some(a) = after {
                        where_clauses.push("f.modified_at >= ?".to_string());
                        params_vec.push(Box::new(a.to_rfc3339()));
                    }
                    if let Some(b) = before {
                        where_clauses.push("f.modified_at <= ?".to_string());
                        params_vec.push(Box::new(b.to_rfc3339()));
                    }
                }
                Filter::HasTag(tag) => {
                    where_clauses.push(
                        "f.id IN (SELECT ft.file_id FROM file_tags ft JOIN tags t ON ft.tag_id = t.id WHERE t.name = ?)".to_string()
                    );
                    params_vec.push(Box::new(tag.clone()));
                }
            }
        }

        let sql = format!(
            "SELECT f.path, f.filename FROM files f WHERE {} ORDER BY f.modified_at DESC LIMIT ? OFFSET ?",
            where_clauses.join(" AND ")
        );

        params_vec.push(Box::new(limit as i64));
        params_vec.push(Box::new(offset as i64));

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_refs.as_slice(), |row| {
            let path: String = row.get(0)?;
            let filename: String = row.get(1)?;
            Ok((path, filename))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (path, filename) = row?;
            results.push(SearchResult {
                file_path: path,
                filename,
                score: 1.0,
                snippet: None,
                match_type: "Metadata".to_string(),
            });
        }

        Ok(SearchResults {
            total_estimate: results.len() as u64,
            results,
            took_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn recent_files(&self, limit: usize) -> Result<SearchResults> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT path, filename FROM files WHERE is_indexed = 1 ORDER BY modified_at DESC LIMIT ?1"
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            let path: String = row.get(0)?;
            let filename: String = row.get(1)?;
            Ok((path, filename))
        })?;

        let mut results = Vec::new();
        for row in rows {
            let (path, filename) = row?;
            results.push(SearchResult {
                file_path: path,
                filename,
                score: 1.0,
                snippet: None,
                match_type: "Metadata".to_string(),
            });
        }

        Ok(SearchResults {
            total_estimate: results.len() as u64,
            results,
            took_ms: 0,
        })
    }

    pub fn find_duplicates(&self) -> Result<Vec<DuplicateGroup>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT content_hash, GROUP_CONCAT(path, '|') as paths
             FROM files
             WHERE content_hash IS NOT NULL
             GROUP BY content_hash
             HAVING COUNT(*) > 1"
        )?;

        let rows = stmt.query_map([], |row| {
            let hash: String = row.get(0)?;
            let paths_str: String = row.get(1)?;
            Ok((hash, paths_str))
        })?;

        let mut groups = Vec::new();
        for row in rows {
            let (hash, paths_str) = row?;
            let paths: Vec<String> = paths_str.split('|').map(|s| s.to_string()).collect();
            groups.push(DuplicateGroup { hash, paths });
        }

        Ok(groups)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub paths: Vec<String>,
}

fn escape_fts5(input: &str) -> String {
    // Remove FTS5 special characters that could cause syntax errors
    input
        .chars()
        .filter(|c| !matches!(c, '"' | '*' | '(' | ')' | ':' | '^' | '{' | '}'))
        .collect()
}

fn extract_snippet(content: &str, query_text: Option<&str>, heading: Option<&str>) -> HighlightedSnippet {
    let max_len = 200;
    let text = if content.len() > max_len {
        // Try to find the first occurrence of a query term
        if let Some(qt) = query_text {
            let lower_content = content.to_lowercase();
            let first_term = qt.split_whitespace().next().unwrap_or("");
            if let Some(pos) = lower_content.find(&first_term.to_lowercase()) {
                let start = pos.saturating_sub(40);
                let end = (start + max_len).min(content.len());
                let s = &content[start..end];
                if start > 0 { format!("…{}", s) } else { s.to_string() }
            } else {
                format!("{}…", &content[..max_len])
            }
        } else {
            format!("{}…", &content[..max_len])
        }
    } else {
        content.to_string()
    };

    // Find highlight positions
    let mut highlights: Vec<(usize, usize)> = Vec::new();
    if let Some(qt) = query_text {
        let lower_text = text.to_lowercase();
        for term in qt.split_whitespace() {
            let lower_term = term.to_lowercase();
            let mut search_from = 0;
            while let Some(pos) = lower_text[search_from..].find(&lower_term) {
                let abs_pos = search_from + pos;
                highlights.push((abs_pos, abs_pos + term.len()));
                search_from = abs_pos + term.len();
            }
        }
    }

    highlights.sort_by_key(|h| h.0);

    HighlightedSnippet { text, highlights }
}

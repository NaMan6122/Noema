use std::path::PathBuf;

use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub semantic_text: Option<String>,
    pub keyword_text: Option<String>,
    pub exact_phrases: Vec<String>,
    pub filters: Vec<Filter>,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub enum Filter {
    FileType(Vec<String>),
    DateRange {
        after: Option<DateTime<Utc>>,
        before: Option<DateTime<Utc>>,
    },
    SizeRange {
        min: Option<u64>,
        max: Option<u64>,
    },
    HasTag(String),
    InPath(PathBuf),
}

pub struct QueryParser;

impl QueryParser {
    pub fn parse(&self, input: &str) -> ParsedQuery {
        let mut filters = Vec::new();
        let mut exact_phrases = Vec::new();
        let mut remaining = Vec::new();

        // Extract quoted phrases
        let mut chars = input.chars().peekable();
        let mut tokens: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut in_quote = false;

        while let Some(ch) = chars.next() {
            if ch == '"' {
                if in_quote {
                    exact_phrases.push(current.clone());
                    current.clear();
                    in_quote = false;
                } else {
                    if !current.is_empty() {
                        tokens.push(current.clone());
                        current.clear();
                    }
                    in_quote = true;
                }
            } else if ch == ' ' && !in_quote {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            if in_quote {
                exact_phrases.push(current);
            } else {
                tokens.push(current);
            }
        }

        for token in tokens {
            if let Some(val) = token.strip_prefix("type:") {
                let types: Vec<String> = val.split(',').map(|s| s.to_string()).collect();
                filters.push(Filter::FileType(types));
            } else if let Some(val) = token.strip_prefix("after:") {
                if let Some(dt) = parse_date(val) {
                    filters.push(Filter::DateRange { after: Some(dt), before: None });
                }
            } else if let Some(val) = token.strip_prefix("before:") {
                if let Some(dt) = parse_date(val) {
                    filters.push(Filter::DateRange { after: None, before: Some(dt) });
                }
            } else if let Some(val) = token.strip_prefix("size:>") {
                if let Some(bytes) = parse_size(val) {
                    filters.push(Filter::SizeRange { min: Some(bytes), max: None });
                }
            } else if let Some(val) = token.strip_prefix("size:<") {
                if let Some(bytes) = parse_size(val) {
                    filters.push(Filter::SizeRange { min: None, max: Some(bytes) });
                }
            } else if let Some(val) = token.strip_prefix("has:tag:") {
                filters.push(Filter::HasTag(val.to_string()));
            } else if let Some(val) = token.strip_prefix("in:") {
                filters.push(Filter::InPath(PathBuf::from(val)));
            } else {
                remaining.push(token);
            }
        }

        let text = if remaining.is_empty() { None } else { Some(remaining.join(" ")) };

        ParsedQuery {
            semantic_text: text.clone(),
            keyword_text: text,
            exact_phrases,
            filters,
            raw: input.to_string(),
        }
    }
}

fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    // Try YYYY-MM-DD
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
    }
    // Try YYYY-MM
    if s.len() == 7 {
        let with_day = format!("{}-01", s);
        if let Ok(d) = NaiveDate::parse_from_str(&with_day, "%Y-%m-%d") {
            return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
        }
    }
    None
}

fn parse_size(s: &str) -> Option<u64> {
    let s = s.to_lowercase();
    if let Some(val) = s.strip_suffix("gb") {
        return val.parse::<u64>().ok().map(|v| v * 1024 * 1024 * 1024);
    }
    if let Some(val) = s.strip_suffix("mb") {
        return val.parse::<u64>().ok().map(|v| v * 1024 * 1024);
    }
    if let Some(val) = s.strip_suffix("kb") {
        return val.parse::<u64>().ok().map(|v| v * 1024);
    }
    s.strip_suffix("b").unwrap_or(&s).parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_query() {
        let q = QueryParser.parse("hello world");
        assert_eq!(q.keyword_text, Some("hello world".to_string()));
        assert!(q.filters.is_empty());
        assert!(q.exact_phrases.is_empty());
    }

    #[test]
    fn test_exact_phrase() {
        let q = QueryParser.parse("find \"budget report\" 2024");
        assert_eq!(q.exact_phrases, vec!["budget report"]);
        assert_eq!(q.keyword_text, Some("find 2024".to_string()));
    }

    #[test]
    fn test_filters() {
        let q = QueryParser.parse("type:pdf,docx after:2024-01-01 revenue");
        assert_eq!(q.keyword_text, Some("revenue".to_string()));
        assert!(matches!(&q.filters[0], Filter::FileType(t) if t == &["pdf", "docx"]));
        assert!(matches!(&q.filters[1], Filter::DateRange { after: Some(_), .. }));
    }

    #[test]
    fn test_size_filter() {
        let q = QueryParser.parse("size:>10mb");
        assert!(matches!(&q.filters[0], Filter::SizeRange { min: Some(10485760), .. }));
    }

    #[test]
    fn test_in_path() {
        let q = QueryParser.parse("in:/Users/me/docs notes");
        assert!(matches!(&q.filters[0], Filter::InPath(p) if p == &PathBuf::from("/Users/me/docs")));
        assert_eq!(q.keyword_text, Some("notes".to_string()));
    }
}

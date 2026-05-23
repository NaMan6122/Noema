use std::path::Path;

use noema_core::db::Database;
use noema_core::error::Result;
use rusqlite::params;

use crate::chunker::Chunk;

#[derive(Debug, Clone)]
pub struct FileRecord {
    pub id: i64,
    pub path: String,
    pub content_hash: Option<String>,
    pub is_indexed: bool,
}

pub fn upsert_file_record(
    db: &Database,
    path: &Path,
    size: u64,
    content_hash: &str,
    mime_type: Option<&str>,
) -> Result<i64> {
    let conn = db.connection()?;
    let path_str = path.to_string_lossy().to_string();
    let filename = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let extension = path.extension().map(|e| e.to_string_lossy().to_string());
    let parent = path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO files (path, filename, extension, size_bytes, created_at, modified_at, content_hash, parent_dir, mime_type, is_indexed, index_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7, ?8, 1, 1)
         ON CONFLICT(path) DO UPDATE SET
            size_bytes = excluded.size_bytes,
            modified_at = excluded.modified_at,
            content_hash = excluded.content_hash,
            mime_type = excluded.mime_type,
            is_indexed = 1,
            index_version = index_version + 1",
        params![path_str, filename, extension, size as i64, now, content_hash, parent, mime_type],
    )?;

    let id = conn.query_row(
        "SELECT id FROM files WHERE path = ?1",
        params![path_str],
        |row| row.get(0),
    )?;

    Ok(id)
}

pub fn get_file_by_path(db: &Database, path: &Path) -> Result<Option<FileRecord>> {
    let conn = db.connection()?;
    let path_str = path.to_string_lossy().to_string();

    let result = conn.query_row(
        "SELECT id, path, content_hash, is_indexed FROM files WHERE path = ?1",
        params![path_str],
        |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                content_hash: row.get(2)?,
                is_indexed: row.get::<_, i64>(3)? != 0,
            })
        },
    );

    match result {
        Ok(record) => Ok(Some(record)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_file_by_hash(db: &Database, hash: &str) -> Result<Option<FileRecord>> {
    let conn = db.connection()?;

    let result = conn.query_row(
        "SELECT id, path, content_hash, is_indexed FROM files WHERE content_hash = ?1",
        params![hash],
        |row| {
            Ok(FileRecord {
                id: row.get(0)?,
                path: row.get(1)?,
                content_hash: row.get(2)?,
                is_indexed: row.get::<_, i64>(3)? != 0,
            })
        },
    );

    match result {
        Ok(record) => Ok(Some(record)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn delete_chunks_for_file(db: &Database, file_id: i64) -> Result<()> {
    let conn = db.connection()?;
    conn.execute("DELETE FROM chunks WHERE file_id = ?1", params![file_id])?;
    Ok(())
}

pub fn insert_chunks(db: &Database, file_id: i64, chunks: &[Chunk]) -> Result<()> {
    let conn = db.connection()?;
    let mut stmt = conn.prepare(
        "INSERT INTO chunks (file_id, chunk_index, content, token_count, heading, chunk_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )?;

    for chunk in chunks {
        let chunk_type_str = match &chunk.chunk_type {
            crate::parser::SectionType::Paragraph => "text",
            crate::parser::SectionType::Heading => "heading",
            crate::parser::SectionType::Code { .. } => "code",
            crate::parser::SectionType::Table => "table",
            crate::parser::SectionType::List => "list",
        };

        stmt.execute(params![
            file_id,
            chunk.index as i64,
            chunk.content,
            chunk.token_count as i64,
            chunk.heading_context,
            chunk_type_str,
        ])?;
    }

    Ok(())
}

pub fn remove_file_record(db: &Database, path: &Path) -> Result<()> {
    let conn = db.connection()?;
    let path_str = path.to_string_lossy().to_string();
    conn.execute("DELETE FROM files WHERE path = ?1", params![path_str])?;
    Ok(())
}

pub fn update_file_path(db: &Database, old_path: &Path, new_path: &Path) -> Result<()> {
    let conn = db.connection()?;
    let old = old_path.to_string_lossy().to_string();
    let new = new_path.to_string_lossy().to_string();
    let new_filename = new_path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    let new_parent = new_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

    conn.execute(
        "UPDATE files SET path = ?1, filename = ?2, parent_dir = ?3 WHERE path = ?4",
        params![new, new_filename, new_parent, old],
    )?;
    Ok(())
}

use std::path::Path;
use std::sync::Arc;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

use crate::error::Result;

pub type DbPool = Pool<SqliteConnectionManager>;

pub struct Database {
    pool: DbPool,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::builder()
            .max_size(5)
            .build(manager)
            .map_err(|e| crate::error::NoemaError::Database(
                rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(1),
                    Some(e.to_string()),
                )
            ))?;

        let conn = pool.get().map_err(|e| crate::error::NoemaError::Database(
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some(e.to_string()),
            )
        ))?;

        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
        ")?;

        Ok(Self { pool })
    }

    pub fn run_migrations(&self) -> Result<()> {
        let conn = self.connection()?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS files (
                id          INTEGER PRIMARY KEY,
                path        TEXT NOT NULL UNIQUE,
                filename    TEXT NOT NULL,
                extension   TEXT,
                size_bytes  INTEGER NOT NULL,
                created_at  TEXT NOT NULL,
                modified_at TEXT NOT NULL,
                accessed_at TEXT,
                content_hash TEXT,
                parent_dir  TEXT NOT NULL,
                mime_type   TEXT,
                is_indexed  INTEGER DEFAULT 0,
                index_version INTEGER DEFAULT 0,
                deleted_at  TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
            CREATE INDEX IF NOT EXISTS idx_files_parent ON files(parent_dir);
            CREATE INDEX IF NOT EXISTS idx_files_hash ON files(content_hash);

            CREATE TABLE IF NOT EXISTS chunks (
                id          INTEGER PRIMARY KEY,
                file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                chunk_index INTEGER NOT NULL,
                content     TEXT NOT NULL,
                token_count INTEGER NOT NULL,
                heading     TEXT,
                chunk_type  TEXT DEFAULT 'text',
                UNIQUE(file_id, chunk_index)
            );
            CREATE INDEX IF NOT EXISTS idx_chunks_file ON chunks(file_id);

            CREATE TABLE IF NOT EXISTS annotations (
                id          INTEGER PRIMARY KEY,
                file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                note        TEXT,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tags (
                id   INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS file_tags (
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                tag_id  INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
                source  TEXT DEFAULT 'user',
                PRIMARY KEY (file_id, tag_id)
            );

            CREATE TABLE IF NOT EXISTS ai_context (
                id          INTEGER PRIMARY KEY,
                file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                version     INTEGER NOT NULL DEFAULT 1,
                summary     TEXT,
                entities    TEXT,
                suggested_tags TEXT,
                generated_at TEXT NOT NULL,
                model_id    TEXT,
                user_edited INTEGER DEFAULT 0,
                UNIQUE(file_id, version)
            );

            CREATE TABLE IF NOT EXISTS relationships (
                id          INTEGER PRIMARY KEY,
                source_id   INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                target_id   INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                rel_type    TEXT NOT NULL,
                strength    REAL DEFAULT 0.0,
                created_at  TEXT NOT NULL,
                UNIQUE(source_id, target_id, rel_type)
            );
            CREATE INDEX IF NOT EXISTS idx_rel_source ON relationships(source_id);
            CREATE INDEX IF NOT EXISTS idx_rel_target ON relationships(target_id);

            CREATE TABLE IF NOT EXISTS virtual_folders (
                id          INTEGER PRIMARY KEY,
                name        TEXT NOT NULL,
                query_json  TEXT NOT NULL,
                icon        TEXT,
                position    INTEGER,
                created_at  TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS access_log (
                id          INTEGER PRIMARY KEY,
                file_id     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                action      TEXT NOT NULL,
                timestamp   TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_access_time ON access_log(timestamp);

            CREATE TABLE IF NOT EXISTS undo_stack (
                id          INTEGER PRIMARY KEY,
                operation   TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                expired     INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS workspaces (
                id          INTEGER PRIMARY KEY,
                name        TEXT NOT NULL UNIQUE,
                state_json  TEXT NOT NULL,
                created_at  TEXT NOT NULL,
                is_active   INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS recent_paths (
                id          INTEGER PRIMARY KEY,
                path        TEXT NOT NULL UNIQUE,
                accessed_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_recent_accessed ON recent_paths(accessed_at);
        ")?;
        Ok(())
    }

    pub fn connection(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>> {
        self.pool.get().map_err(|e| crate::error::NoemaError::Database(
            rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(1),
                Some(e.to_string()),
            )
        ))
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }
}

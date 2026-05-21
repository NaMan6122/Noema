use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NoemaError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Parser error ({format}): {detail}")]
    Parser { format: String, detail: String },

    #[error("Config error ({key}): {detail}")]
    Config { key: String, detail: String },

    #[error("Search error: {detail}")]
    Search { detail: String },

    #[error("AI error: {detail}")]
    Ai { detail: String },

    #[error("Plugin error ({plugin}): {detail}")]
    Plugin { plugin: String, detail: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Not found: {path}")]
    NotFound { path: PathBuf },

    #[error("Operation cancelled")]
    Cancelled,
}

pub type Result<T> = std::result::Result<T, NoemaError>;

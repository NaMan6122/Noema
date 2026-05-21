use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperationId(pub Uuid);

impl OperationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryId(pub Uuid);

impl QueryId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub id: Option<FileId>,
    pub path: PathBuf,
    pub filename: String,
    pub extension: Option<String>,
    pub size: u64,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortField {
    Name,
    Size,
    Modified,
    Created,
    Extension,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SortOrder {
    pub field: SortField,
    pub direction: SortDirection,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self {
            field: SortField::Name,
            direction: SortDirection::Asc,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ThumbSize {
    Small,
    Medium,
    Large,
    Preview,
}

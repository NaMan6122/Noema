use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use noema_core::error::Result;
use noema_core::events::EventBus;
use noema_core::types::{FileEntry, OperationId, SortOrder};
use tokio::sync::Mutex;

pub struct FileOpsEngine {
    event_bus: Arc<EventBus>,
    undo_stack: Arc<Mutex<VecDeque<UndoableOp>>>,
    redo_stack: Arc<Mutex<VecDeque<UndoableOp>>>,
}

#[derive(Debug, Clone)]
enum UndoableOp {
    Copy { sources: Vec<PathBuf>, created: Vec<PathBuf> },
    Move { moves: Vec<(PathBuf, PathBuf)> },
    Delete { trashed: Vec<PathBuf> },
    Rename { path: PathBuf, old_name: String, new_name: String },
}

impl FileOpsEngine {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            undo_stack: Arc::new(Mutex::new(VecDeque::with_capacity(50))),
            redo_stack: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn list_directory(
        &self,
        path: &Path,
        sort: &SortOrder,
        show_hidden: bool,
    ) -> Result<Vec<FileEntry>> {
        todo!("Implement directory listing")
    }

    pub async fn copy_files(
        &self,
        sources: Vec<PathBuf>,
        dest: PathBuf,
    ) -> Result<OperationId> {
        todo!("Implement copy")
    }

    pub async fn move_files(
        &self,
        sources: Vec<PathBuf>,
        dest: PathBuf,
    ) -> Result<OperationId> {
        todo!("Implement move")
    }

    pub async fn delete_files(
        &self,
        paths: Vec<PathBuf>,
        use_trash: bool,
    ) -> Result<OperationId> {
        todo!("Implement delete")
    }

    pub async fn rename_file(
        &self,
        path: PathBuf,
        new_name: String,
    ) -> Result<()> {
        todo!("Implement rename")
    }

    pub async fn undo(&self) -> Result<()> {
        todo!("Implement undo")
    }

    pub async fn redo(&self) -> Result<()> {
        todo!("Implement redo")
    }
}

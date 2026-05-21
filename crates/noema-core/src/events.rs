use std::path::PathBuf;

use tokio::sync::broadcast;

use crate::types::{FileId, OperationId, QueryId};

#[derive(Debug, Clone)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf },
}

#[derive(Debug, Clone)]
pub enum OpType {
    Copy,
    Move,
    Delete,
    Rename,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    FileChanged {
        path: PathBuf,
        change: ChangeType,
    },
    OperationStarted {
        id: OperationId,
        op_type: OpType,
        total_items: u64,
    },
    OperationProgress {
        id: OperationId,
        processed: u64,
        current: PathBuf,
    },
    OperationComplete {
        id: OperationId,
        success: bool,
        error: Option<String>,
    },
    IndexProgress {
        total: u64,
        processed: u64,
        current_file: String,
    },
    IndexComplete,
    ContextGenerated {
        file_id: FileId,
    },
    SearchResultsReady {
        query_id: QueryId,
    },
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }

    pub fn emit(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }
}

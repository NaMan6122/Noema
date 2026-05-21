use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use noema_core::error::{NoemaError, Result};
use noema_core::events::{AppEvent, ChangeType, EventBus, OpType};
use noema_core::types::{FileEntry, FileId, OperationId, SortDirection, SortField, SortOrder};
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
        let path = path.to_path_buf();
        let sort = *sort;
        let show_hidden = show_hidden;

        tokio::task::spawn_blocking(move || {
            list_directory_sync(&path, &sort, show_hidden)
        })
        .await
        .map_err(|e| NoemaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
    }

    pub async fn copy_files(
        &self,
        sources: Vec<PathBuf>,
        dest: PathBuf,
    ) -> Result<OperationId> {
        let op_id = OperationId::new();
        let total = sources.len() as u64;

        self.event_bus.emit(AppEvent::OperationStarted {
            id: op_id,
            op_type: OpType::Copy,
            total_items: total,
        });

        let event_bus = self.event_bus.clone();
        let undo_stack = self.undo_stack.clone();

        tokio::task::spawn_blocking(move || {
            let mut created = Vec::new();
            for (i, source) in sources.iter().enumerate() {
                let filename = source.file_name().unwrap_or_default();
                let dest_path = dest.join(filename);

                event_bus.emit(AppEvent::OperationProgress {
                    id: op_id,
                    processed: i as u64,
                    current: source.clone(),
                });

                if source.is_dir() {
                    copy_dir_recursive(source, &dest_path)?;
                } else {
                    std::fs::copy(source, &dest_path)?;
                }
                created.push(dest_path.clone());
                event_bus.emit(AppEvent::FileChanged {
                    path: dest_path,
                    change: ChangeType::Created,
                });
            }

            let mut stack = undo_stack.blocking_lock();
            if stack.len() >= 50 {
                stack.pop_front();
            }
            stack.push_back(UndoableOp::Copy { sources, created });

            event_bus.emit(AppEvent::OperationComplete {
                id: op_id,
                success: true,
                error: None,
            });

            Ok::<_, NoemaError>(())
        })
        .await
        .map_err(|e| NoemaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

        Ok(op_id)
    }

    pub async fn move_files(
        &self,
        sources: Vec<PathBuf>,
        dest: PathBuf,
    ) -> Result<OperationId> {
        let op_id = OperationId::new();
        let total = sources.len() as u64;

        self.event_bus.emit(AppEvent::OperationStarted {
            id: op_id,
            op_type: OpType::Move,
            total_items: total,
        });

        let event_bus = self.event_bus.clone();
        let undo_stack = self.undo_stack.clone();

        tokio::task::spawn_blocking(move || {
            let mut moves = Vec::new();
            for (i, source) in sources.iter().enumerate() {
                let filename = source.file_name().unwrap_or_default();
                let dest_path = dest.join(filename);

                event_bus.emit(AppEvent::OperationProgress {
                    id: op_id,
                    processed: i as u64,
                    current: source.clone(),
                });

                if let Err(_) = std::fs::rename(source, &dest_path) {
                    // Cross-volume: copy then delete
                    if source.is_dir() {
                        copy_dir_recursive(source, &dest_path)?;
                        std::fs::remove_dir_all(source)?;
                    } else {
                        std::fs::copy(source, &dest_path)?;
                        std::fs::remove_file(source)?;
                    }
                }

                moves.push((source.clone(), dest_path.clone()));
                event_bus.emit(AppEvent::FileChanged {
                    path: dest_path,
                    change: ChangeType::Renamed { from: source.clone() },
                });
            }

            let mut stack = undo_stack.blocking_lock();
            if stack.len() >= 50 {
                stack.pop_front();
            }
            stack.push_back(UndoableOp::Move { moves });

            event_bus.emit(AppEvent::OperationComplete {
                id: op_id,
                success: true,
                error: None,
            });

            Ok::<_, NoemaError>(())
        })
        .await
        .map_err(|e| NoemaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

        Ok(op_id)
    }

    pub async fn delete_files(
        &self,
        paths: Vec<PathBuf>,
        use_trash: bool,
    ) -> Result<OperationId> {
        let op_id = OperationId::new();
        let total = paths.len() as u64;

        self.event_bus.emit(AppEvent::OperationStarted {
            id: op_id,
            op_type: OpType::Delete,
            total_items: total,
        });

        let event_bus = self.event_bus.clone();
        let undo_stack = self.undo_stack.clone();

        tokio::task::spawn_blocking(move || {
            let mut trashed = Vec::new();
            for (i, path) in paths.iter().enumerate() {
                event_bus.emit(AppEvent::OperationProgress {
                    id: op_id,
                    processed: i as u64,
                    current: path.clone(),
                });

                if use_trash {
                    trash::delete(path).map_err(|e| NoemaError::Io(
                        std::io::Error::new(std::io::ErrorKind::Other, e)
                    ))?;
                } else if path.is_dir() {
                    std::fs::remove_dir_all(path)?;
                } else {
                    std::fs::remove_file(path)?;
                }

                trashed.push(path.clone());
                event_bus.emit(AppEvent::FileChanged {
                    path: path.clone(),
                    change: ChangeType::Deleted,
                });
            }

            if use_trash {
                let mut stack = undo_stack.blocking_lock();
                if stack.len() >= 50 {
                    stack.pop_front();
                }
                stack.push_back(UndoableOp::Delete { trashed });
            }

            event_bus.emit(AppEvent::OperationComplete {
                id: op_id,
                success: true,
                error: None,
            });

            Ok::<_, NoemaError>(())
        })
        .await
        .map_err(|e| NoemaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))??;

        Ok(op_id)
    }

    pub async fn rename_file(
        &self,
        path: PathBuf,
        new_name: String,
    ) -> Result<()> {
        if new_name.is_empty() || new_name.contains('/') || new_name.contains('\\') {
            return Err(NoemaError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid filename",
            )));
        }

        let parent = path.parent().ok_or_else(|| NoemaError::Io(
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "No parent directory")
        ))?;
        let new_path = parent.join(&new_name);

        if new_path.exists() {
            return Err(NoemaError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("File already exists: {}", new_path.display()),
            )));
        }

        let old_name = path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        std::fs::rename(&path, &new_path)?;

        let mut stack = self.undo_stack.lock().await;
        if stack.len() >= 50 {
            stack.pop_front();
        }
        stack.push_back(UndoableOp::Rename {
            path: new_path.clone(),
            old_name,
            new_name,
        });

        self.event_bus.emit(AppEvent::FileChanged {
            path: new_path,
            change: ChangeType::Renamed { from: path },
        });

        Ok(())
    }

    pub async fn create_directory(&self, path: PathBuf) -> Result<()> {
        std::fs::create_dir_all(&path)?;
        self.event_bus.emit(AppEvent::FileChanged {
            path,
            change: ChangeType::Created,
        });
        Ok(())
    }

    pub async fn create_file(&self, path: PathBuf) -> Result<()> {
        std::fs::File::create(&path)?;
        self.event_bus.emit(AppEvent::FileChanged {
            path,
            change: ChangeType::Created,
        });
        Ok(())
    }

    pub async fn undo(&self) -> Result<()> {
        let op = {
            let mut stack = self.undo_stack.lock().await;
            stack.pop_back().ok_or(NoemaError::Cancelled)?
        };

        match &op {
            UndoableOp::Copy { created, .. } => {
                for path in created {
                    if path.is_dir() {
                        std::fs::remove_dir_all(path)?;
                    } else {
                        std::fs::remove_file(path)?;
                    }
                    self.event_bus.emit(AppEvent::FileChanged {
                        path: path.clone(),
                        change: ChangeType::Deleted,
                    });
                }
            }
            UndoableOp::Move { moves } => {
                for (original, current) in moves {
                    std::fs::rename(current, original)?;
                    self.event_bus.emit(AppEvent::FileChanged {
                        path: original.clone(),
                        change: ChangeType::Renamed { from: current.clone() },
                    });
                }
            }
            UndoableOp::Delete { .. } => {
                return Err(NoemaError::Ai {
                    detail: "Cannot undo trash deletion from code — use OS trash to restore".into(),
                });
            }
            UndoableOp::Rename { path, old_name, .. } => {
                let parent = path.parent().unwrap();
                let old_path = parent.join(old_name);
                std::fs::rename(path, &old_path)?;
                self.event_bus.emit(AppEvent::FileChanged {
                    path: old_path,
                    change: ChangeType::Renamed { from: path.clone() },
                });
            }
        }

        let mut redo = self.redo_stack.lock().await;
        redo.push_back(op);
        Ok(())
    }

    pub async fn redo(&self) -> Result<()> {
        let _op = {
            let mut stack = self.redo_stack.lock().await;
            stack.pop_back().ok_or(NoemaError::Cancelled)?
        };
        // Re-execute the operation
        todo!("Implement redo execution")
    }

    pub fn can_undo(&self) -> bool {
        // Non-async check — use try_lock
        self.undo_stack.try_lock().map(|s| !s.is_empty()).unwrap_or(false)
    }

    pub fn can_redo(&self) -> bool {
        self.redo_stack.try_lock().map(|s| !s.is_empty()).unwrap_or(false)
    }
}

fn list_directory_sync(path: &Path, sort: &SortOrder, show_hidden: bool) -> Result<Vec<FileEntry>> {
    if !path.exists() {
        return Err(NoemaError::NotFound { path: path.to_path_buf() });
    }
    if !path.is_dir() {
        return Err(NoemaError::Io(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            format!("{} is not a directory", path.display()),
        )));
    }

    let mut entries = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        if !show_hidden && file_name.starts_with('.') {
            continue;
        }

        let created = metadata.created()
            .map(DateTime::<Utc>::from)
            .unwrap_or_default();
        let modified = metadata.modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_default();

        let extension = entry.path()
            .extension()
            .map(|e| e.to_string_lossy().to_string());

        let is_symlink = entry.file_type()?.is_symlink();

        entries.push(FileEntry {
            id: None,
            path: entry.path(),
            filename: file_name,
            extension,
            size: metadata.len(),
            created,
            modified,
            is_dir: metadata.is_dir(),
            is_hidden: entry.file_name().to_string_lossy().starts_with('.'),
            is_symlink,
            mime_type: None,
        });
    }

    // Sort: directories first, then by sort order
    entries.sort_by(|a, b| {
        // Directories always first
        match (a.is_dir, b.is_dir) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }

        let ord = match sort.field {
            SortField::Name => a.filename.to_lowercase().cmp(&b.filename.to_lowercase()),
            SortField::Size => a.size.cmp(&b.size),
            SortField::Modified => a.modified.cmp(&b.modified),
            SortField::Created => a.created.cmp(&b.created),
            SortField::Extension => {
                let a_ext = a.extension.as_deref().unwrap_or("");
                let b_ext = b.extension.as_deref().unwrap_or("");
                a_ext.cmp(b_ext)
            }
        };

        match sort.direction {
            SortDirection::Asc => ord,
            SortDirection::Desc => ord.reverse(),
        }
    });

    Ok(entries)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest_path = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("alpha.txt"), "hello").unwrap();
        fs::write(dir.path().join("beta.pdf"), "world").unwrap();
        fs::write(dir.path().join(".hidden"), "secret").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_list_directory_basic() {
        let dir = make_test_dir();
        let bus = Arc::new(EventBus::new(16));
        let engine = FileOpsEngine::new(bus);

        let entries = engine
            .list_directory(dir.path(), &SortOrder::default(), false)
            .await
            .unwrap();

        // Should have: subdir, alpha.txt, beta.pdf (hidden excluded)
        assert_eq!(entries.len(), 3);
        // Directories first
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].filename, "subdir");
    }

    #[tokio::test]
    async fn test_list_directory_show_hidden() {
        let dir = make_test_dir();
        let bus = Arc::new(EventBus::new(16));
        let engine = FileOpsEngine::new(bus);

        let entries = engine
            .list_directory(dir.path(), &SortOrder::default(), true)
            .await
            .unwrap();

        assert_eq!(entries.len(), 4); // includes .hidden
    }

    #[tokio::test]
    async fn test_list_directory_sort_by_name() {
        let dir = make_test_dir();
        let bus = Arc::new(EventBus::new(16));
        let engine = FileOpsEngine::new(bus);

        let entries = engine
            .list_directory(dir.path(), &SortOrder::default(), false)
            .await
            .unwrap();

        // After subdir (dir first), files sorted alphabetically
        let files: Vec<&str> = entries.iter().skip(1).map(|e| e.filename.as_str()).collect();
        assert_eq!(files, vec!["alpha.txt", "beta.pdf"]);
    }

    #[tokio::test]
    async fn test_list_directory_not_found() {
        let bus = Arc::new(EventBus::new(16));
        let engine = FileOpsEngine::new(bus);

        let result = engine
            .list_directory(Path::new("/nonexistent/path"), &SortOrder::default(), false)
            .await;

        assert!(result.is_err());
    }
}

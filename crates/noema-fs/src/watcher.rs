use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use globset::{Glob, GlobSet, GlobSetBuilder};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use noema_core::config::IndexingConfig;
use noema_core::error::{NoemaError, Result};
use noema_core::events::{AppEvent, ChangeType, EventBus};

pub struct WatcherService {
    event_bus: Arc<EventBus>,
    watched_paths: Vec<WatchedPath>,
    ignore_set: GlobSet,
    debouncer: Option<Debouncer<RecommendedWatcher, FileIdMap>>,
    running: bool,
}

#[derive(Debug, Clone)]
pub struct WatchedPath {
    pub path: PathBuf,
    pub recursive: bool,
}

impl WatcherService {
    pub fn new(event_bus: Arc<EventBus>, config: &IndexingConfig) -> Result<Self> {
        let watched_paths = config
            .watch_paths
            .iter()
            .map(|p| WatchedPath {
                path: p.clone(),
                recursive: true,
            })
            .collect();

        let ignore_set = build_ignore_set(&config.exclude_patterns)?;

        Ok(Self {
            event_bus,
            watched_paths,
            ignore_set,
            debouncer: None,
            running: false,
        })
    }

    pub fn start(&mut self) -> Result<()> {
        if self.running {
            return Ok(());
        }

        let event_bus = self.event_bus.clone();
        let ignore_set = self.ignore_set.clone();

        let (tx, mut rx) = mpsc::channel::<Vec<DebouncedEvent>>(256);

        let debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |events: std::result::Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
                match events {
                    Ok(evts) => {
                        let _ = tx.blocking_send(evts);
                    }
                    Err(errors) => {
                        for e in errors {
                            warn!("Watcher error: {}", e);
                        }
                    }
                }
            },
        )
        .map_err(|e| NoemaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        self.debouncer = Some(debouncer);

        // Watch all configured paths
        if let Some(ref mut d) = self.debouncer {
            for wp in &self.watched_paths {
                if wp.path.exists() {
                    let mode = if wp.recursive {
                        RecursiveMode::Recursive
                    } else {
                        RecursiveMode::NonRecursive
                    };
                    if let Err(e) = d.watch(&wp.path, mode) {
                        warn!("Failed to watch {}: {}", wp.path.display(), e);
                    } else {
                        info!("Watching: {}", wp.path.display());
                    }
                } else {
                    warn!("Watch path does not exist: {}", wp.path.display());
                }
            }
        }

        // Spawn event processing task
        let event_bus_clone = event_bus.clone();
        let ignore_clone = ignore_set.clone();
        tokio::spawn(async move {
            while let Some(events) = rx.recv().await {
                for debounced in events {
                    process_event(&event_bus_clone, &ignore_clone, debounced.event);
                }
            }
        });

        self.running = true;
        info!("File watcher started ({} paths)", self.watched_paths.len());
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.debouncer = None;
        self.running = false;
        info!("File watcher stopped");
        Ok(())
    }

    pub fn add_path(&mut self, path: PathBuf, recursive: bool) -> Result<()> {
        if let Some(ref mut d) = self.debouncer {
            let mode = if recursive {
                RecursiveMode::Recursive
            } else {
                RecursiveMode::NonRecursive
            };
            d.watch(&path, mode)
                .map_err(|e| NoemaError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        }
        self.watched_paths.push(WatchedPath { path, recursive });
        Ok(())
    }

    pub fn remove_path(&mut self, path: &Path) -> Result<()> {
        if let Some(ref mut d) = self.debouncer {
            let _ = d.unwatch(path);
        }
        self.watched_paths.retain(|wp| wp.path != path);
        Ok(())
    }

    pub fn watched_paths(&self) -> &[WatchedPath] {
        &self.watched_paths
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        self.ignore_set.is_match(path)
    }
}

fn process_event(event_bus: &EventBus, ignore_set: &GlobSet, event: Event) {
    for path in &event.paths {
        if ignore_set.is_match(path) {
            debug!("Ignored: {}", path.display());
            continue;
        }

        let change = match event.kind {
            EventKind::Create(_) => ChangeType::Created,
            EventKind::Modify(_) => ChangeType::Modified,
            EventKind::Remove(_) => ChangeType::Deleted,
            _ => continue,
        };

        event_bus.emit(AppEvent::FileChanged {
            path: path.clone(),
            change,
        });
    }
}

fn build_ignore_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob_pattern = if pattern.contains('*') || pattern.contains('/') {
            format!("**/{}", pattern)
        } else {
            format!("**/{}", pattern)
        };
        let glob = Glob::new(&glob_pattern).map_err(|e| NoemaError::Config {
            key: "exclude_patterns".into(),
            detail: format!("Invalid glob pattern '{}': {}", pattern, e),
        })?;
        builder.add(glob);

        // For directory-style patterns, also match contents within
        if !pattern.contains('*') {
            let contents_pattern = format!("**/{}/**", pattern);
            let contents_glob = Glob::new(&contents_pattern).map_err(|e| NoemaError::Config {
                key: "exclude_patterns".into(),
                detail: format!("Invalid glob pattern '{}': {}", pattern, e),
            })?;
            builder.add(contents_glob);
        }
    }
    builder.build().map_err(|e| NoemaError::Config {
        key: "exclude_patterns".into(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_set() {
        let patterns = vec![
            ".git".to_string(),
            "node_modules".to_string(),
            "*.tmp".to_string(),
        ];
        let set = build_ignore_set(&patterns).unwrap();

        assert!(set.is_match(Path::new("/Users/me/project/.git/HEAD")));
        assert!(set.is_match(Path::new("/foo/node_modules/bar.js")));
        assert!(set.is_match(Path::new("/foo/bar.tmp")));
        assert!(!set.is_match(Path::new("/foo/bar.rs")));
        assert!(!set.is_match(Path::new("/foo/document.pdf")));
    }
}

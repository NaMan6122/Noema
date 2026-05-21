use std::path::PathBuf;
use std::sync::Arc;

use noema_core::config::IndexingConfig;
use noema_core::error::Result;
use noema_core::events::EventBus;

pub struct WatcherService {
    event_bus: Arc<EventBus>,
    watched_paths: Vec<WatchedPath>,
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

        Ok(Self {
            event_bus,
            watched_paths,
            running: false,
        })
    }

    pub async fn start(&mut self) -> Result<()> {
        self.running = true;
        tracing::info!("File watcher started for {} paths", self.watched_paths.len());
        todo!("Implement notify watcher loop")
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.running = false;
        Ok(())
    }

    pub fn add_path(&mut self, path: PathBuf, recursive: bool) -> Result<()> {
        self.watched_paths.push(WatchedPath { path, recursive });
        Ok(())
    }

    pub fn watched_paths(&self) -> &[WatchedPath] {
        &self.watched_paths
    }
}

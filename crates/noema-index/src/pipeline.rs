use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_queue::SegQueue;
use tokio::sync::Notify;
use tracing::{debug, error, info, warn};

use noema_core::db::Database;
use noema_core::events::{AppEvent, EventBus};

use crate::chunker::SemanticChunker;
use crate::db as index_db;
use crate::parser::ParserRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    High = 0,
    Normal = 1,
    Low = 2,
}

#[derive(Debug, Clone)]
pub enum IndexReason {
    NewFile,
    Modified,
    Reindex,
    Manual,
}

#[derive(Debug)]
pub struct IndexJob {
    pub path: PathBuf,
    pub priority: Priority,
    pub reason: IndexReason,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IndexState {
    Idle,
    Running,
    Paused,
}

#[derive(Debug, Clone)]
pub struct IndexStatus {
    pub state: IndexState,
    pub total_files: u64,
    pub indexed_files: u64,
    pub pending_jobs: u64,
    pub current_file: Option<String>,
    pub files_per_second: f32,
}

pub struct IndexerPipeline {
    high_queue: Arc<SegQueue<IndexJob>>,
    normal_queue: Arc<SegQueue<IndexJob>>,
    low_queue: Arc<SegQueue<IndexJob>>,
    parser_registry: Arc<ParserRegistry>,
    chunker: SemanticChunker,
    db: Arc<Database>,
    event_bus: Arc<EventBus>,
    paused: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    notify: Arc<Notify>,
    indexed_count: Arc<AtomicU64>,
    total_count: Arc<AtomicU64>,
    pending_count: Arc<AtomicUsize>,
    current_file: Arc<tokio::sync::Mutex<Option<String>>>,
    last_user_input: Arc<AtomicU64>,
    max_file_size: u64,
}

impl IndexerPipeline {
    pub fn new(
        parser_registry: Arc<ParserRegistry>,
        db: Arc<Database>,
        event_bus: Arc<EventBus>,
        max_file_size_mb: u64,
    ) -> Self {
        Self {
            high_queue: Arc::new(SegQueue::new()),
            normal_queue: Arc::new(SegQueue::new()),
            low_queue: Arc::new(SegQueue::new()),
            parser_registry,
            chunker: SemanticChunker::default_config(),
            db,
            event_bus,
            paused: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
            indexed_count: Arc::new(AtomicU64::new(0)),
            total_count: Arc::new(AtomicU64::new(0)),
            pending_count: Arc::new(AtomicUsize::new(0)),
            current_file: Arc::new(tokio::sync::Mutex::new(None)),
            last_user_input: Arc::new(AtomicU64::new(0)),
            max_file_size: max_file_size_mb * 1024 * 1024,
        }
    }

    pub fn enqueue(&self, job: IndexJob) {
        self.total_count.fetch_add(1, Ordering::Relaxed);
        self.pending_count.fetch_add(1, Ordering::Relaxed);
        match job.priority {
            Priority::High => self.high_queue.push(job),
            Priority::Normal => self.normal_queue.push(job),
            Priority::Low => self.low_queue.push(job),
        }
        self.notify.notify_one();
    }

    pub fn enqueue_batch(&self, jobs: Vec<IndexJob>) {
        for job in jobs {
            self.enqueue(job);
        }
    }

    fn dequeue(&self) -> Option<IndexJob> {
        self.high_queue
            .pop()
            .or_else(|| self.normal_queue.pop())
            .or_else(|| self.low_queue.pop())
    }

    pub async fn start(self: &Arc<Self>) {
        if self.running.swap(true, Ordering::SeqCst) {
            return; // already running
        }

        let pipeline = Arc::clone(self);
        tokio::spawn(async move {
            info!("Indexer pipeline started");
            pipeline.run_loop().await;
            info!("Indexer pipeline stopped");
        });
    }

    async fn run_loop(&self) {
        while self.running.load(Ordering::Relaxed) {
            if self.paused.load(Ordering::Relaxed) {
                self.notify.notified().await;
                continue;
            }

            let batch_size = self.current_batch_size();

            let mut processed = 0;
            for _ in 0..batch_size {
                if let Some(job) = self.dequeue() {
                    self.process_job(job).await;
                    processed += 1;
                } else {
                    break;
                }
            }

            if processed == 0 {
                self.notify.notified().await;
            } else if self.is_user_active() {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    }

    async fn process_job(&self, job: IndexJob) {
        let path = &job.path;

        {
            let mut cf = self.current_file.lock().await;
            *cf = Some(path.to_string_lossy().to_string());
        }

        // Check file still exists
        if !path.exists() {
            warn!(path = %path.display(), "File no longer exists, removing from index");
            let _ = index_db::remove_file_record(&self.db, path);
            self.pending_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        // Check file size
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(e) => {
                error!(path = %path.display(), error = %e, "Failed to read metadata");
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };

        if metadata.len() > self.max_file_size {
            debug!(path = %path.display(), size = metadata.len(), "File exceeds max size, skipping");
            self.pending_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        // Read file content
        let content = match std::fs::read(path) {
            Ok(c) => c,
            Err(e) => {
                error!(path = %path.display(), error = %e, "Failed to read file");
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };

        // Compute hash
        let hash = blake3::hash(&content).to_hex().to_string();

        // Check if already indexed with same hash
        if let Ok(Some(existing)) = index_db::get_file_by_path(&self.db, path) {
            if existing.content_hash.as_deref() == Some(&hash) && existing.is_indexed {
                debug!(path = %path.display(), "File unchanged, skipping");
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                self.indexed_count.fetch_add(1, Ordering::Relaxed);
                return;
            }
        } else if let Ok(Some(_existing)) = index_db::get_file_by_hash(&self.db, &hash) {
            // Same content at different path — file was moved
            debug!(path = %path.display(), "File moved (same hash), updating path");
        }

        // Detect MIME
        let mime = infer::get(&content).map(|t| t.mime_type().to_string());

        // Skip binary files without a parser
        if mime.as_deref().map_or(false, |m| m.starts_with("image/") || m.starts_with("video/") || m.starts_with("audio/")) {
            debug!(path = %path.display(), "Binary file without text parser, skipping");
            self.pending_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        // Parse
        let doc = match self.parser_registry.parse_file(path, &content) {
            Ok(d) => d,
            Err(e) => {
                error!(path = %path.display(), error = %e, "Parse failed");
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };

        // Chunk
        let chunks = self.chunker.chunk(&doc);

        // Store
        let file_id = match index_db::upsert_file_record(
            &self.db,
            path,
            metadata.len(),
            &hash,
            mime.as_deref(),
        ) {
            Ok(id) => id,
            Err(e) => {
                error!(path = %path.display(), error = %e, "DB upsert failed");
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                return;
            }
        };

        if let Err(e) = index_db::delete_chunks_for_file(&self.db, file_id) {
            error!(error = %e, "Failed to delete old chunks");
        }

        if let Err(e) = index_db::insert_chunks(&self.db, file_id, &chunks) {
            error!(path = %path.display(), error = %e, "Failed to insert chunks");
            self.pending_count.fetch_sub(1, Ordering::Relaxed);
            return;
        }

        self.indexed_count.fetch_add(1, Ordering::Relaxed);
        self.pending_count.fetch_sub(1, Ordering::Relaxed);

        let indexed = self.indexed_count.load(Ordering::Relaxed);
        let total = self.total_count.load(Ordering::Relaxed);

        self.event_bus.emit(AppEvent::IndexProgress {
            total,
            processed: indexed,
            current_file: path.to_string_lossy().to_string(),
        });

        debug!(path = %path.display(), chunks = chunks.len(), "Indexed successfully");
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        self.notify.notify_one();
    }

    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
        self.notify.notify_one();
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn notify_user_active(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_user_input.store(now, Ordering::Relaxed);
    }

    fn is_user_active(&self) -> bool {
        let last = self.last_user_input.load(Ordering::Relaxed);
        if last == 0 {
            return false;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now - last < 5 // 5 second idle threshold
    }

    fn current_batch_size(&self) -> usize {
        if self.is_user_active() { 1 } else { 8 }
    }

    pub async fn status(&self) -> IndexStatus {
        let state = if !self.running.load(Ordering::Relaxed) {
            IndexState::Idle
        } else if self.paused.load(Ordering::Relaxed) {
            IndexState::Paused
        } else {
            IndexState::Running
        };

        let current_file = self.current_file.lock().await.clone();

        IndexStatus {
            state,
            total_files: self.total_count.load(Ordering::Relaxed),
            indexed_files: self.indexed_count.load(Ordering::Relaxed),
            pending_jobs: self.pending_count.load(Ordering::Relaxed) as u64,
            current_file,
            files_per_second: 0.0, // TODO: track rate
        }
    }
}

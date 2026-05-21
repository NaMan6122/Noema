# Module: noema-core

## Purpose

Shared foundation for all Noema crates. Provides types, error handling, the event bus, and configuration loading. No business logic lives here — only contracts and plumbing.

---

## Public Interface

### Types

```rust
// Identity types
pub struct FileId(pub i64);
pub struct ChunkId(pub i64);
pub struct OperationId(pub Uuid);
pub struct QueryId(pub Uuid);

// File entry returned to frontend
pub struct FileEntry {
    pub id: Option<FileId>,        // None if not yet indexed
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

// Sort
pub enum SortField { Name, Size, Modified, Created, Extension }
pub enum SortDirection { Asc, Desc }
pub struct SortOrder { pub field: SortField, pub direction: SortDirection }

// Thumbnail reference
pub struct ThumbnailRef {
    pub path: PathBuf,         // path to cached WebP
    pub size: ThumbSize,
}
pub enum ThumbSize { Small, Medium, Large, Preview }
```

### Error Type

```rust
pub enum NoemaError {
    Io(std::io::Error),
    Database(rusqlite::Error),
    Parser { format: String, detail: String },
    Config { key: String, detail: String },
    Search { detail: String },
    Ai { detail: String },
    Plugin { plugin: String, detail: String },
    PermissionDenied { path: PathBuf },
    NotFound { path: PathBuf },
    Cancelled,
}

impl From<std::io::Error> for NoemaError { ... }
impl From<rusqlite::Error> for NoemaError { ... }
```

### Event Bus

```rust
pub enum AppEvent {
    // File system events
    FileChanged { path: PathBuf, change: ChangeType },
    
    // Operation progress
    OperationStarted { id: OperationId, op_type: OpType, total_items: u64 },
    OperationProgress { id: OperationId, processed: u64, current: PathBuf },
    OperationComplete { id: OperationId, result: Result<(), NoemaError> },
    
    // Index events
    IndexProgress { total: u64, processed: u64, current_file: String },
    IndexComplete,
    
    // AI events
    ContextGenerated { file_id: FileId },
    
    // Search events
    SearchResultsReady { query_id: QueryId },
}

pub enum ChangeType { Created, Modified, Deleted, Renamed { from: PathBuf } }
pub enum OpType { Copy, Move, Delete, Rename }

pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self;
    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent>;
    pub fn emit(&self, event: AppEvent);
}
```

### Configuration

```rust
pub struct AppConfig {
    pub general: GeneralConfig,
    pub indexing: IndexingConfig,
    pub search: SearchConfig,
    pub ai: AiConfig,
    pub thumbnails: ThumbnailConfig,
    pub keybindings: HashMap<String, String>,
}

pub struct GeneralConfig {
    pub theme: Theme,                        // System, Light, Dark
    pub default_view: ViewMode,              // List, Grid, Column
    pub show_hidden: bool,
    pub confirm_delete: bool,
    pub use_trash: bool,
}

pub struct IndexingConfig {
    pub enabled: bool,
    pub watch_paths: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,       // gitignore syntax
    pub max_file_size_mb: u64,
    pub idle_cpu_percent: f32,
    pub active_cpu_percent: f32,
    pub idle_threshold_seconds: u64,
}

pub struct SearchConfig {
    pub default_mode: SearchMode,            // Hybrid, Semantic, Keyword
    pub max_results: usize,
    pub snippet_length: usize,
}

pub struct AiConfig {
    pub enabled: bool,
    pub embedding_model: EmbeddingSource,    // Builtin, Ollama(String), Path(PathBuf)
    pub llm_model_path: Option<PathBuf>,
    pub llm_threads: usize,
    pub auto_generate_context: bool,
}

pub struct ThumbnailConfig {
    pub enabled: bool,
    pub max_cache_mb: u64,
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, NoemaError>;
    pub fn load_or_default() -> Self;
    pub fn save(&self, path: &Path) -> Result<(), NoemaError>;
    pub fn config_dir() -> PathBuf;          // platform-specific
    pub fn data_dir() -> PathBuf;            // platform-specific
    pub fn cache_dir() -> PathBuf;           // platform-specific
}
```

### Database

```rust
pub struct Database {
    pool: Pool<SqliteConnectionManager>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, NoemaError>;
    pub fn run_migrations(&self) -> Result<(), NoemaError>;
    pub fn connection(&self) -> Result<PooledConnection, NoemaError>;
}
```

---

## Behavior

### Event Bus
- Capacity: 1024 events (configurable)
- If a subscriber lags behind, it receives `RecvError::Lagged(n)` — must handle gracefully
- Events are cheap to clone (Arc internals where needed)
- Multiple subscribers allowed (UI, indexer, watcher all subscribe)

### Config Loading
- Search order: CLI flags → env vars → config file → defaults
- Config file path: `{config_dir}/config.toml`
- If config file doesn't exist, create with defaults on first launch
- Hot-reload: watch config file for changes, emit event on change (future)

### Platform Paths
- macOS: `~/Library/Application Support/Noema/`, `~/Library/Caches/Noema/`
- Linux: `$XDG_CONFIG_HOME/noema/`, `$XDG_DATA_HOME/noema/`, `$XDG_CACHE_HOME/noema/`
- Windows: `%APPDATA%\Noema\`, `%LOCALAPPDATA%\Noema\cache\`

### Database
- WAL mode enabled (concurrent reads)
- Connection pool size: 4 (read) + 1 (write)
- Migrations run on startup, versioned sequentially
- Foreign keys enabled

---

## Edge Cases & Error Handling

- Config file corrupted → log warning, use defaults, write fresh config
- Database file corrupted → attempt WAL recovery, if fails → backup corrupt file, create fresh
- Event bus full → lagged subscribers miss events (acceptable, they'll catch up on next poll)
- Platform dir creation fails → fall back to current directory with warning

---

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4", "serde"] }
rusqlite = { version = "0.31", features = ["bundled", "backup"] }
r2d2 = "0.8"
r2d2_sqlite = "0.24"
thiserror = "1"
tracing = "0.1"
directories = "5"           # platform dirs
```

---

## Performance Constraints

- Config load: <10ms
- Database open + migrations: <100ms
- Event emit: <1μs (non-blocking broadcast)
- FileEntry construction from fs metadata: <1ms per file

---

## Example Usage

```rust
// App startup
let config = AppConfig::load_or_default();
let db = Database::open(&config.data_dir().join("index.db"))?;
db.run_migrations()?;
let event_bus = EventBus::new(1024);

// Subscribe to events (e.g., from UI bridge)
let mut rx = event_bus.subscribe();
tokio::spawn(async move {
    while let Ok(event) = rx.recv().await {
        // forward to frontend via Tauri event
    }
});

// Emit an event
event_bus.emit(AppEvent::FileChanged {
    path: "/Users/me/doc.pdf".into(),
    change: ChangeType::Modified,
});
```

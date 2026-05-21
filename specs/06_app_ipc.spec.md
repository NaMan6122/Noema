# Module: noema-app (IPC & Application Layer)

## Purpose

The Tauri application shell. Defines all IPC commands exposed to the Svelte frontend, manages application lifecycle, and bridges backend services to the UI via events.

---

## Public Interface

### Tauri Commands (IPC)

All commands are async and return `Result<T, String>` (Tauri serializes errors as strings).

#### File System Commands

```rust
#[tauri::command]
async fn list_directory(
    path: String,
    sort_field: String,      // "name" | "size" | "modified" | "created" | "extension"
    sort_direction: String,  // "asc" | "desc"
    show_hidden: bool,
    state: State<'_, AppState>,
) -> Result<Vec<FileEntryDto>, String>;

#[tauri::command]
async fn copy_files(
    sources: Vec<String>,
    dest: String,
    on_conflict: String,     // "ask" | "skip" | "overwrite" | "rename"
    state: State<'_, AppState>,
) -> Result<String, String>;  // returns OperationId

#[tauri::command]
async fn move_files(
    sources: Vec<String>,
    dest: String,
    on_conflict: String,
    state: State<'_, AppState>,
) -> Result<String, String>;

#[tauri::command]
async fn delete_files(
    paths: Vec<String>,
    use_trash: bool,
    state: State<'_, AppState>,
) -> Result<String, String>;

#[tauri::command]
async fn rename_file(
    path: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String>;

#[tauri::command]
async fn create_directory(path: String, state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn create_file(path: String, state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn get_metadata(path: String, state: State<'_, AppState>) -> Result<FileMetadataDto, String>;

#[tauri::command]
async fn undo(state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn redo(state: State<'_, AppState>) -> Result<(), String>;
```

#### Search Commands

```rust
#[tauri::command]
async fn search(
    query: String,
    limit: Option<usize>,
    offset: Option<usize>,
    state: State<'_, AppState>,
) -> Result<SearchResultsDto, String>;

#[tauri::command]
async fn search_suggest(
    partial: String,
    state: State<'_, AppState>,
) -> Result<Vec<SuggestionDto>, String>;

#[tauri::command]
async fn find_similar(
    file_id: i64,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<SimilarFileDto>, String>;

#[tauri::command]
async fn find_duplicates(
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<DuplicateGroupDto>, String>;
```

#### AI & Context Commands

```rust
#[tauri::command]
async fn get_context(
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<Option<FileContextDto>, String>;

#[tauri::command]
async fn generate_context(
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String>;  // async — emits ContextGenerated event when done

#[tauri::command]
async fn update_context(
    file_id: i64,
    summary: Option<String>,
    add_tags: Vec<String>,
    remove_tags: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), String>;

#[tauri::command]
async fn suggest_filename(
    file_id: i64,
    state: State<'_, AppState>,
) -> Result<String, String>;
```

#### Index Commands

```rust
#[tauri::command]
async fn get_index_status(state: State<'_, AppState>) -> Result<IndexStatusDto, String>;

#[tauri::command]
async fn add_watch_path(
    path: String,
    recursive: bool,
    state: State<'_, AppState>,
) -> Result<(), String>;

#[tauri::command]
async fn remove_watch_path(path: String, state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn pause_indexing(state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn resume_indexing(state: State<'_, AppState>) -> Result<(), String>;

#[tauri::command]
async fn reindex_file(path: String, state: State<'_, AppState>) -> Result<(), String>;
```

#### Workspace & Config Commands

```rust
#[tauri::command]
async fn get_config(state: State<'_, AppState>) -> Result<AppConfigDto, String>;

#[tauri::command]
async fn update_config(
    changes: HashMap<String, serde_json::Value>,
    state: State<'_, AppState>,
) -> Result<(), String>;

#[tauri::command]
async fn save_workspace(
    name: String,
    state_json: String,
    state: State<'_, AppState>,
) -> Result<(), String>;

#[tauri::command]
async fn load_workspace(
    name: String,
    state: State<'_, AppState>,
) -> Result<String, String>;  // returns state_json

#[tauri::command]
async fn list_workspaces(state: State<'_, AppState>) -> Result<Vec<WorkspaceDto>, String>;
```

#### Thumbnail Commands

```rust
#[tauri::command]
async fn get_thumbnail(
    path: String,
    size: String,        // "small" | "medium" | "large" | "preview"
    state: State<'_, AppState>,
) -> Result<Option<String>, String>;  // returns path to cached thumbnail or None
```

### Frontend Events (Backend → UI)

Emitted via `app_handle.emit_all(event_name, payload)`:

```rust
// Event names and payloads:
"file-changed"        → { path: String, change_type: String }
"op-started"          → { id: String, op_type: String, total: u64 }
"op-progress"         → { id: String, processed: u64, current: String }
"op-complete"         → { id: String, success: bool, error: Option<String> }
"index-progress"      → { total: u64, processed: u64, current_file: String }
"index-complete"      → {}
"context-generated"   → { file_id: i64 }
"thumbnail-ready"     → { path: String, thumb_path: String }
"conflict-ask"        → { id: String, path: String, options: Vec<String> }
```

### Application State

```rust
pub struct AppState {
    pub config: Arc<RwLock<AppConfig>>,
    pub db: Arc<Database>,
    pub event_bus: Arc<EventBus>,
    pub fs_engine: Arc<FileOpsEngine>,
    pub watcher: Arc<Mutex<WatcherService>>,
    pub indexer: Arc<IndexerPipeline>,
    pub search: Arc<SearchEngine>,
    pub ai: Arc<Mutex<LlmEngine>>,
    pub context_store: Arc<ContextStore>,
    pub thumbnail_service: Arc<ThumbnailService>,
}
```

---

## Behavior

### Application Lifecycle

1. **Startup:**
   - Load config
   - Open database, run migrations
   - Initialize event bus
   - Create all services (FileOpsEngine, WatcherService, IndexerPipeline, SearchEngine, etc.)
   - Start file watcher
   - Start indexer (background)
   - Launch Tauri window
   - Restore last workspace state

2. **Running:**
   - IPC commands handled via Tauri command router
   - Events forwarded from event bus to frontend
   - Background indexing continues
   - Thumbnail generation on demand

3. **Shutdown:**
   - Save current workspace state
   - Pause indexer, flush pending writes
   - Close database cleanly (checkpoint WAL)
   - Stop file watcher
   - Exit

### Event Bridge

A dedicated task subscribes to `EventBus` and forwards relevant events to the Tauri frontend:

```rust
async fn event_bridge(app_handle: AppHandle, event_bus: Arc<EventBus>) {
    let mut rx = event_bus.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                let (name, payload) = event.to_frontend_event();
                app_handle.emit_all(&name, payload).ok();
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Event bridge lagged by {n} events");
            }
            Err(_) => break,
        }
    }
}
```

### DTO Pattern

Backend types (with PathBuf, DateTime, etc.) are converted to DTOs (with String, i64) for serialization across the IPC boundary. Each DTO implements `Serialize`.

```rust
// Example
pub struct FileEntryDto {
    pub id: Option<i64>,
    pub path: String,
    pub filename: String,
    pub extension: Option<String>,
    pub size: u64,
    pub created: String,        // ISO 8601
    pub modified: String,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_symlink: bool,
    pub mime_type: Option<String>,
    pub thumbnail: Option<String>,
    pub tags: Vec<String>,
    pub has_context: bool,
}
```

---

## Edge Cases & Error Handling

- **Command fails:** Return serialized error string; frontend shows toast/modal
- **Event bridge lags:** Log warning, continue (frontend will request fresh data on next action)
- **Multiple windows:** Each window gets events; state is shared
- **App crash recovery:** On next launch, check WAL integrity, rebuild if needed
- **Permission denied for watched path:** Remove from watch list, notify user

---

## Dependencies

```toml
[dependencies]
noema-core = { path = "../noema-core" }
noema-fs = { path = "../noema-fs" }
noema-index = { path = "../noema-index" }
noema-search = { path = "../noema-search" }
noema-ai = { path = "../noema-ai" }
tauri = { version = "2", features = ["shell-open", "dialog", "clipboard"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## Performance Constraints

- IPC round-trip (command → response): <10ms overhead (excluding actual work)
- Event emission: non-blocking, <1ms
- App startup to interactive window: <2s cold, <500ms warm

---

## Example: App Initialization

```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let config = AppConfig::load_or_default();
            let db = Database::open(&config.data_dir().join("index.db"))?;
            db.run_migrations()?;
            let event_bus = Arc::new(EventBus::new(1024));

            let fs_engine = Arc::new(FileOpsEngine::new(event_bus.clone(), db.clone()));
            let watcher = Arc::new(Mutex::new(
                WatcherService::new(event_bus.clone(), config.indexing.clone())?
            ));
            // ... initialize other services ...

            let state = AppState { config, db, event_bus, fs_engine, watcher, /* ... */ };
            app.manage(state);

            // Start event bridge
            let handle = app.handle().clone();
            let bus = state.event_bus.clone();
            tokio::spawn(event_bridge(handle, bus));

            // Start background services
            tokio::spawn(async move { watcher.lock().await.start().await });
            tokio::spawn(async move { indexer.start().await });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_directory, copy_files, move_files, delete_files,
            rename_file, create_directory, create_file, get_metadata,
            undo, redo, search, search_suggest, find_similar,
            find_duplicates, get_context, generate_context,
            update_context, suggest_filename, get_index_status,
            add_watch_path, remove_watch_path, pause_indexing,
            resume_indexing, reindex_file, get_config, update_config,
            save_workspace, load_workspace, list_workspaces,
            get_thumbnail,
        ])
        .run(tauri::generate_context!())
        .expect("error running Noema");
}
```

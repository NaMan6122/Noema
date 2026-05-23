#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use noema_core::config::AppConfig;
use noema_core::db::Database;
use noema_core::events::{AppEvent, EventBus};
use noema_core::types::{FileEntry, SortDirection, SortField, SortOrder};
use noema_fs::highlight::Highlighter;
use noema_fs::ops::FileOpsEngine;
use noema_fs::thumbs::ThumbnailService;
use noema_ai::{ContextStore, LlmEngine, StubBackend, UserContextEdit};
use noema_index::embeddings::EmbeddingEngine;
use noema_index::parser::ParserRegistry;
use noema_index::pipeline::{IndexJob, IndexReason, IndexState, IndexerPipeline, Priority};
use noema_search::engine::SearchEngine;
use noema_search::query::QueryParser;
use serde::Serialize;
use tauri::{Emitter, State};

struct AppState {
    fs_engine: Arc<FileOpsEngine>,
    thumb_service: Arc<ThumbnailService>,
    highlighter: Arc<Highlighter>,
    event_bus: Arc<EventBus>,
    db: Arc<Database>,
    indexer: Arc<IndexerPipeline>,
    search_engine: Arc<SearchEngine>,
    ai_engine: Arc<LlmEngine>,
    context_store: Arc<ContextStore>,
}

#[derive(Serialize)]
struct FavoriteEntry {
    name: String,
    path: String,
    kind: String, // "favorite" or "volume"
}

#[derive(Serialize)]
struct FileEntryDto {
    path: String,
    filename: String,
    extension: Option<String>,
    size: u64,
    created: String,
    modified: String,
    is_dir: bool,
    is_hidden: bool,
    is_symlink: bool,
}

impl From<FileEntry> for FileEntryDto {
    fn from(e: FileEntry) -> Self {
        Self {
            path: e.path.to_string_lossy().to_string(),
            filename: e.filename,
            extension: e.extension,
            size: e.size,
            created: e.created.to_rfc3339(),
            modified: e.modified.to_rfc3339(),
            is_dir: e.is_dir,
            is_hidden: e.is_hidden,
            is_symlink: e.is_symlink,
        }
    }
}

#[tauri::command]
async fn list_directory(
    path: String,
    sort_field: Option<String>,
    sort_direction: Option<String>,
    show_hidden: Option<bool>,
    state: State<'_, AppState>,
) -> Result<Vec<FileEntryDto>, String> {
    let field = match sort_field.as_deref() {
        Some("size") => SortField::Size,
        Some("modified") => SortField::Modified,
        Some("created") => SortField::Created,
        Some("extension") => SortField::Extension,
        _ => SortField::Name,
    };
    let direction = match sort_direction.as_deref() {
        Some("desc") => SortDirection::Desc,
        _ => SortDirection::Asc,
    };
    let sort = SortOrder { field, direction };

    state
        .fs_engine
        .list_directory(&PathBuf::from(&path), &sort, show_hidden.unwrap_or(false))
        .await
        .map(|entries| entries.into_iter().map(FileEntryDto::from).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_home_dir() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Cannot determine home directory".to_string())
}

#[tauri::command]
async fn copy_files(
    sources: Vec<String>,
    dest: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let sources: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    let op_id = state
        .fs_engine
        .copy_files(sources, PathBuf::from(dest))
        .await
        .map_err(|e| e.to_string())?;
    Ok(op_id.0.to_string())
}

#[tauri::command]
async fn move_files(
    sources: Vec<String>,
    dest: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let sources: Vec<PathBuf> = sources.into_iter().map(PathBuf::from).collect();
    let op_id = state
        .fs_engine
        .move_files(sources, PathBuf::from(dest))
        .await
        .map_err(|e| e.to_string())?;
    Ok(op_id.0.to_string())
}

#[tauri::command]
async fn delete_files(
    paths: Vec<String>,
    use_trash: Option<bool>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let paths: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let op_id = state
        .fs_engine
        .delete_files(paths, use_trash.unwrap_or(true))
        .await
        .map_err(|e| e.to_string())?;
    Ok(op_id.0.to_string())
}

#[tauri::command]
async fn rename_file(
    path: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .fs_engine
        .rename_file(PathBuf::from(path), new_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_directory(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .fs_engine
        .create_directory(PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_file(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .fs_engine
        .create_file(PathBuf::from(path))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn undo(state: State<'_, AppState>) -> Result<(), String> {
    state.fs_engine.undo().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn redo(state: State<'_, AppState>) -> Result<(), String> {
    state.fs_engine.redo().await.map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct ConflictInfo {
    source: String,
    dest: String,
    filename: String,
}

#[tauri::command]
async fn check_conflicts(
    sources: Vec<String>,
    dest: String,
) -> Result<Vec<ConflictInfo>, String> {
    let dest = PathBuf::from(&dest);
    let mut conflicts = Vec::new();
    for source in &sources {
        let source_path = PathBuf::from(source);
        if let Some(filename) = source_path.file_name() {
            let dest_path = dest.join(filename);
            if dest_path.exists() {
                conflicts.push(ConflictInfo {
                    source: source.clone(),
                    dest: dest_path.to_string_lossy().to_string(),
                    filename: filename.to_string_lossy().to_string(),
                });
            }
        }
    }
    Ok(conflicts)
}

#[derive(Serialize)]
struct FileInfoDto {
    path: String,
    filename: String,
    size: u64,
    created: String,
    modified: String,
    is_dir: bool,
    is_symlink: bool,
    permissions: String,
    extension: Option<String>,
}

#[tauri::command]
async fn log_file_open(
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state.db.connection().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO recent_paths (path, accessed_at) VALUES (?1, ?2)
         ON CONFLICT(path) DO UPDATE SET accessed_at = ?2",
        rusqlite::params![path, now],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_recent_files(
    state: State<'_, AppState>,
) -> Result<Vec<FavoriteEntry>, String> {
    let conn = state.db.connection().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT path FROM recent_paths ORDER BY accessed_at DESC LIMIT 20"
    ).map_err(|e| e.to_string())?;
    let paths: Vec<String> = stmt.query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(paths.into_iter().filter_map(|p| {
        let path = std::path::Path::new(&p);
        let name = path.file_name()?.to_string_lossy().to_string();
        Some(FavoriteEntry { name, path: p, kind: "recent".to_string() })
    }).collect())
}

#[tauri::command]
async fn open_in_terminal(path: String) -> Result<(), String> {
    let dir = if std::path::Path::new(&path).is_dir() {
        path
    } else {
        std::path::Path::new(&path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    };

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-a", "Terminal", &dir])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        let terminals = ["x-terminal-emulator", "gnome-terminal", "konsole", "xterm"];
        let mut launched = false;
        for term in &terminals {
            if std::process::Command::new(term)
                .current_dir(&dir)
                .spawn()
                .is_ok()
            {
                launched = true;
                break;
            }
        }
        if !launched {
            return Err("No terminal emulator found".to_string());
        }
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "cmd", "/k", &format!("cd /d {}", dir)])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Serialize)]
struct DiskSpaceInfo {
    total: u64,
    available: u64,
    used: u64,
}

#[tauri::command]
async fn get_disk_space(path: String) -> Result<DiskSpaceInfo, String> {
    let path = PathBuf::from(&path);
    let meta = fs2::statvfs(&path).map_err(|e| e.to_string())?;
    let total = meta.total_space();
    let available = meta.available_space();
    Ok(DiskSpaceInfo {
        total,
        available,
        used: total.saturating_sub(available),
    })
}

#[tauri::command]
async fn search_files(
    root: String,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<FileEntryDto>, String> {
    let root = PathBuf::from(&root);
    let limit = limit.unwrap_or(100);
    let query_lower = query.to_lowercase();

    tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();
        for entry in walkdir::WalkDir::new(&root)
            .max_depth(8)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let name = entry.file_name().to_string_lossy();
            if name.to_lowercase().contains(&query_lower) {
                if let Ok(meta) = entry.metadata() {
                    let created = meta.created()
                        .map(chrono::DateTime::<chrono::Utc>::from)
                        .unwrap_or_default();
                    let modified = meta.modified()
                        .map(chrono::DateTime::<chrono::Utc>::from)
                        .unwrap_or_default();
                    results.push(FileEntryDto {
                        path: entry.path().to_string_lossy().to_string(),
                        filename: name.to_string(),
                        extension: entry.path().extension().map(|e| e.to_string_lossy().to_string()),
                        size: meta.len(),
                        created: created.to_rfc3339(),
                        modified: modified.to_rfc3339(),
                        is_dir: meta.is_dir(),
                        is_hidden: name.starts_with('.'),
                        is_symlink: entry.file_type().is_symlink(),
                    });
                    if results.len() >= limit { break; }
                }
            }
        }
        Ok::<_, String>(results)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_file_info(path: String) -> Result<FileInfoDto, String> {
    let path = PathBuf::from(&path);
    let meta = std::fs::metadata(&path).map_err(|e| e.to_string())?;
    let symlink_meta = std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?;

    #[cfg(unix)]
    let permissions = {
        use std::os::unix::fs::PermissionsExt;
        format!("{:o}", meta.permissions().mode() & 0o777)
    };
    #[cfg(not(unix))]
    let permissions = if meta.permissions().readonly() { "readonly".to_string() } else { "read-write".to_string() };

    Ok(FileInfoDto {
        path: path.to_string_lossy().to_string(),
        filename: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        size: meta.len(),
        created: meta.created().map(chrono::DateTime::<chrono::Utc>::from).unwrap_or_default().to_rfc3339(),
        modified: meta.modified().map(chrono::DateTime::<chrono::Utc>::from).unwrap_or_default().to_rfc3339(),
        is_dir: meta.is_dir(),
        is_symlink: symlink_meta.file_type().is_symlink(),
        permissions,
        extension: path.extension().map(|e| e.to_string_lossy().to_string()),
    })
}

#[tauri::command]
async fn highlight_code(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = PathBuf::from(&path);
    let highlighter = state.highlighter.clone();
    tokio::task::spawn_blocking(move || {
        let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let truncated = if content.len() > 10240 { &content[..10240] } else { &content };
        highlighter.highlight(&path, truncated).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn read_file_preview(
    path: String,
    max_bytes: Option<usize>,
) -> Result<String, String> {
    let max = max_bytes.unwrap_or(10240);
    let path = PathBuf::from(&path);
    tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        let truncated = &bytes[..bytes.len().min(max)];
        match String::from_utf8(truncated.to_vec()) {
            Ok(s) => Ok(s),
            Err(_) => Ok(String::from_utf8_lossy(truncated).to_string()),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn get_thumbnail(
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = PathBuf::from(&path);
    if !ThumbnailService::is_supported(&path) {
        return Err("Unsupported file type".to_string());
    }
    state.thumb_service.get_thumbnail(path).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_workspace(
    name: String,
    state_json: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state.db.connection().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO workspaces (name, state_json, created_at, is_active) VALUES (?1, ?2, ?3, 1)
         ON CONFLICT(name) DO UPDATE SET state_json = ?2, created_at = ?3",
        rusqlite::params![name, state_json, now],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn load_workspace(
    name: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let conn = state.db.connection().map_err(|e| e.to_string())?;
    let result: Option<String> = conn.query_row(
        "SELECT state_json FROM workspaces WHERE name = ?1",
        rusqlite::params![name],
        |row| row.get(0),
    ).ok();
    Ok(result)
}

#[tauri::command]
async fn get_theme() -> Result<String, String> {
    let config = AppConfig::load_or_default();
    let theme_str = match config.general.theme {
        noema_core::config::Theme::System => "system",
        noema_core::config::Theme::Light => "light",
        noema_core::config::Theme::Dark => "dark",
    };
    Ok(theme_str.to_string())
}

#[tauri::command]
async fn set_theme(theme: String) -> Result<(), String> {
    let mut config = AppConfig::load_or_default();
    config.general.theme = match theme.as_str() {
        "light" => noema_core::config::Theme::Light,
        "dark" => noema_core::config::Theme::Dark,
        _ => noema_core::config::Theme::System,
    };
    let path = AppConfig::config_dir().join("config.toml");
    std::fs::create_dir_all(AppConfig::config_dir()).map_err(|e| e.to_string())?;
    config.save(&path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_favorites() -> Result<Vec<FavoriteEntry>, String> {
    let mut favorites = Vec::new();

    let dirs_map: Vec<(&str, Option<PathBuf>)> = vec![
        ("Home", dirs::home_dir()),
        ("Desktop", dirs::desktop_dir()),
        ("Documents", dirs::document_dir()),
        ("Downloads", dirs::download_dir()),
        ("Pictures", dirs::picture_dir()),
        ("Music", dirs::audio_dir()),
        ("Videos", dirs::video_dir()),
    ];

    for (name, path) in dirs_map {
        if let Some(p) = path {
            if p.exists() {
                favorites.push(FavoriteEntry {
                    name: name.to_string(),
                    path: p.to_string_lossy().to_string(),
                    kind: "favorite".to_string(),
                });
            }
        }
    }

    // Volumes (macOS: /Volumes/*, Linux: /mnt/* and /media/*, Windows: drive letters)
    #[cfg(target_os = "macos")]
    {
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "Macintosh HD" {
                    continue;
                }
                favorites.push(FavoriteEntry {
                    name,
                    path: entry.path().to_string_lossy().to_string(),
                    kind: "volume".to_string(),
                });
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        for mount_dir in &["/mnt", "/media"] {
            if let Ok(entries) = std::fs::read_dir(mount_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        favorites.push(FavoriteEntry {
                            name: entry.file_name().to_string_lossy().to_string(),
                            path: entry.path().to_string_lossy().to_string(),
                            kind: "volume".to_string(),
                        });
                    }
                }
            }
        }
    }

    Ok(favorites)
}

#[tauri::command]
async fn index_directory(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let dir_path = PathBuf::from(&path);
    if !dir_path.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    let mut jobs = Vec::new();
    for entry in walkdir::WalkDir::new(&dir_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            jobs.push(IndexJob {
                path: entry.into_path(),
                priority: Priority::Normal,
                reason: IndexReason::Manual,
            });
        }
    }

    let count = jobs.len();
    state.indexer.enqueue_batch(jobs);
    state.indexer.start().await;

    Ok(format!("Enqueued {} files for indexing", count))
}

#[tauri::command]
async fn get_index_status(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let status = state.indexer.status().await;
    Ok(serde_json::json!({
        "state": format!("{:?}", status.state),
        "totalFiles": status.total_files,
        "indexedFiles": status.indexed_files,
        "pendingJobs": status.pending_jobs,
        "currentFile": status.current_file,
    }))
}

#[tauri::command]
async fn pause_indexing(state: State<'_, AppState>) -> Result<(), String> {
    state.indexer.pause();
    Ok(())
}

#[tauri::command]
async fn resume_indexing(state: State<'_, AppState>) -> Result<(), String> {
    state.indexer.resume();
    Ok(())
}

#[tauri::command]
async fn content_search(
    query: String,
    limit: Option<usize>,
    offset: Option<usize>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let parsed = QueryParser.parse(&query);
    let results = state.search_engine.search(&parsed, limit.unwrap_or(20), offset.unwrap_or(0)).await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "results": results.results,
        "totalEstimate": results.total_estimate,
        "tookMs": results.took_ms,
    }))
}

#[tauri::command]
async fn find_duplicates(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let groups = state.search_engine.find_duplicates().map_err(|e| e.to_string())?;
    Ok(serde_json::json!(groups))
}

#[tauri::command]
async fn generate_file_context(path: String, state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let content = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
    let ctx = state.ai_engine.generate_context(&content, 512).await.map_err(|e| e.to_string())?;

    let conn = state.db.connection().map_err(|e| e.to_string())?;
    let file_id: Option<i64> = conn.query_row(
        "SELECT id FROM files WHERE path = ?1", rusqlite::params![path], |r| r.get(0),
    ).ok();

    if let Some(fid) = file_id {
        state.context_store.save_context(fid, &ctx, "stub-v1").map_err(|e| e.to_string())?;
    }

    Ok(serde_json::json!(ctx))
}

#[tauri::command]
async fn get_file_context(path: String, state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let conn = state.db.connection().map_err(|e| e.to_string())?;
    let file_id: i64 = conn.query_row(
        "SELECT id FROM files WHERE path = ?1", rusqlite::params![path], |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    let ctx = state.context_store.get_context(file_id).map_err(|e| e.to_string())?;
    Ok(serde_json::json!(ctx))
}

#[tauri::command]
async fn suggest_tags(path: String, state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let content = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
    let tags = state.ai_engine.suggest_tags(&content, &[]).await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!(tags))
}

#[tauri::command]
async fn suggest_filename(path: String, state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let content = tokio::fs::read_to_string(&path).await.map_err(|e| e.to_string())?;
    let name = state.ai_engine.suggest_filename(&content).await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!(name))
}

#[tauri::command]
async fn apply_ai_tags(path: String, tags: Vec<String>, state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.connection().map_err(|e| e.to_string())?;
    let file_id: i64 = conn.query_row(
        "SELECT id FROM files WHERE path = ?1", rusqlite::params![path], |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    state.context_store.apply_suggested_tags(file_id, &tags).map_err(|e| e.to_string())
}

#[tauri::command]
async fn edit_context(path: String, edit: UserContextEdit, state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.connection().map_err(|e| e.to_string())?;
    let file_id: i64 = conn.query_row(
        "SELECT id FROM files WHERE path = ?1", rusqlite::params![path], |r| r.get(0),
    ).map_err(|e| e.to_string())?;

    state.context_store.update_user_edit(file_id, &edit).map_err(|e| e.to_string())
}

fn main() {
    tracing_subscriber::fmt::init();

    let _config = AppConfig::load_or_default();
    let data_dir = AppConfig::data_dir();
    let db_path = data_dir.join("index.db");

    let db = Arc::new(Database::open(&db_path).expect("Failed to open database"));
    db.run_migrations().expect("Failed to run migrations");

    let event_bus = Arc::new(EventBus::new(1024));
    let fs_engine = Arc::new(FileOpsEngine::new(event_bus.clone()));
    let thumb_service = Arc::new(
        ThumbnailService::new(data_dir.join("thumbs"), 128)
            .expect("Failed to create thumbnail service")
    );
    let highlighter = Arc::new(Highlighter::new());

    let parser_registry = Arc::new(ParserRegistry::with_defaults());

    // Try to load embedding model — gracefully degrade to FTS-only if unavailable
    let model_dir = data_dir.join("models").join("bge-small");
    let embedder = match EmbeddingEngine::load(&model_dir) {
        Ok(engine) => {
            tracing::info!("Embedding engine loaded from {:?}", model_dir);
            Some(Arc::new(tokio::sync::Mutex::new(engine)))
        }
        Err(e) => {
            tracing::warn!("Embedding model not available ({e}), running in FTS-only mode");
            None
        }
    };

    let mut indexer = IndexerPipeline::new(
        parser_registry,
        db.clone(),
        event_bus.clone(),
        100, // max_file_size_mb
    );
    if let Some(ref emb) = embedder {
        indexer = indexer.with_embedder(emb.clone());
    }
    let indexer = Arc::new(indexer);

    let mut search_engine = SearchEngine::new(db.clone());
    if let Some(ref emb) = embedder {
        search_engine = search_engine.with_embedder(emb.clone());
    }
    let search_engine = Arc::new(search_engine);

    let indexer_for_watcher = indexer.clone();
    let mut watcher_rx = event_bus.subscribe();
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = watcher_rx.recv().await {
            match event {
                AppEvent::FileChanged { path, change } => {
                    use noema_core::events::ChangeType;
                    match change {
                        ChangeType::Created | ChangeType::Modified => {
                            if path.is_file() {
                                indexer_for_watcher.enqueue(IndexJob {
                                    path,
                                    priority: Priority::Normal,
                                    reason: IndexReason::NewFile,
                                });
                                indexer_for_watcher.start().await;
                            }
                        }
                        ChangeType::Deleted => {
                            let _ = noema_index::db::remove_file_record(
                                &indexer_for_watcher.db(),
                                &path,
                            );
                        }
                        ChangeType::Renamed { from } => {
                            let _ = noema_index::db::update_file_path(
                                &indexer_for_watcher.db(),
                                &from,
                                &path,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    });

    let ai_engine = Arc::new(LlmEngine::new(120).with_backend(Arc::new(StubBackend)));
    let context_store = Arc::new(ContextStore::new(db.clone()));

    let app_state = AppState {
        fs_engine,
        thumb_service,
        highlighter,
        event_bus: event_bus.clone(),
        db,
        indexer,
        search_engine,
        ai_engine,
        context_store,
    };

    tauri::Builder::default()
        .manage(app_state)
        .setup(move |app| {
            let handle = app.handle().clone();
            let mut rx = event_bus.subscribe();
            tauri::async_runtime::spawn(async move {
                while let Ok(event) = rx.recv().await {
                    match event {
                        AppEvent::OperationStarted { id, op_type, total_items } => {
                            let _ = handle.emit("op:started", serde_json::json!({
                                "id": id.0.to_string(),
                                "opType": format!("{:?}", op_type),
                                "totalItems": total_items,
                            }));
                        }
                        AppEvent::OperationProgress { id, processed, current } => {
                            let _ = handle.emit("op:progress", serde_json::json!({
                                "id": id.0.to_string(),
                                "processed": processed,
                                "current": current.to_string_lossy(),
                            }));
                        }
                        AppEvent::OperationComplete { id, success, error } => {
                            let _ = handle.emit("op:complete", serde_json::json!({
                                "id": id.0.to_string(),
                                "success": success,
                                "error": error,
                            }));
                        }
                        AppEvent::FileChanged { .. } => {
                            let _ = handle.emit("fs:changed", ());
                        }
                        AppEvent::IndexProgress { total, processed, current_file } => {
                            let _ = handle.emit("index:progress", serde_json::json!({
                                "total": total,
                                "processed": processed,
                                "currentFile": current_file,
                            }));
                        }
                        AppEvent::IndexComplete => {
                            let _ = handle.emit("index:complete", ());
                        }
                        _ => {}
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_directory,
            get_home_dir,
            get_favorites,
            copy_files,
            move_files,
            delete_files,
            rename_file,
            create_directory,
            create_file,
            undo,
            redo,
            check_conflicts,
            save_workspace,
            load_workspace,
            get_thumbnail,
            read_file_preview,
            highlight_code,
            get_file_info,
            search_files,
            log_file_open,
            get_recent_files,
            open_in_terminal,
            get_disk_space,
            get_theme,
            set_theme,
            index_directory,
            get_index_status,
            pause_indexing,
            resume_indexing,
            content_search,
            find_duplicates,
            generate_file_context,
            get_file_context,
            suggest_tags,
            suggest_filename,
            apply_ai_tags,
            edit_context,
        ])
        .run(tauri::generate_context!())
        .expect("error running Noema");
}

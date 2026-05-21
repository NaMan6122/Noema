#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::Arc;

use noema_core::config::AppConfig;
use noema_core::db::Database;
use noema_core::events::EventBus;
use noema_core::types::{FileEntry, SortDirection, SortField, SortOrder};
use noema_fs::ops::FileOpsEngine;
use serde::Serialize;
use tauri::State;

struct AppState {
    fs_engine: Arc<FileOpsEngine>,
    event_bus: Arc<EventBus>,
    db: Arc<Database>,
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

fn main() {
    tracing_subscriber::fmt::init();

    let config = AppConfig::load_or_default();
    let data_dir = AppConfig::data_dir();
    let db_path = data_dir.join("index.db");

    let db = Arc::new(Database::open(&db_path).expect("Failed to open database"));
    db.run_migrations().expect("Failed to run migrations");

    let event_bus = Arc::new(EventBus::new(1024));
    let fs_engine = Arc::new(FileOpsEngine::new(event_bus.clone()));

    let app_state = AppState {
        fs_engine,
        event_bus,
        db,
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            list_directory,
            get_home_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error running Noema");
}

use std::sync::Arc;

use noema_core::config::AppConfig;
use noema_core::db::Database;
use noema_core::events::EventBus;

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Noema starting...");

    let config = AppConfig::load_or_default();
    let data_dir = AppConfig::data_dir();
    let db_path = data_dir.join("index.db");

    let db = Database::open(&db_path).expect("Failed to open database");
    db.run_migrations().expect("Failed to run migrations");

    let event_bus = Arc::new(EventBus::new(1024));

    tracing::info!("Noema initialized successfully");
    tracing::info!("Database: {}", db_path.display());
    tracing::info!("Config dir: {}", AppConfig::config_dir().display());

    // TODO: Initialize Tauri app with frontend
    println!("Noema v0.1.0 — Semantic File Explorer");
    println!("Database ready at: {}", db_path.display());
}

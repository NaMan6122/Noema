use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{NoemaError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewMode {
    List,
    Grid,
    Column,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchMode {
    Hybrid,
    Semantic,
    Keyword,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingSource {
    Builtin,
    Ollama(String),
    Path(PathBuf),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub theme: Theme,
    pub default_view: ViewMode,
    pub show_hidden: bool,
    pub confirm_delete: bool,
    pub use_trash: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            default_view: ViewMode::List,
            show_hidden: false,
            confirm_delete: true,
            use_trash: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    pub enabled: bool,
    pub watch_paths: Vec<PathBuf>,
    pub exclude_patterns: Vec<String>,
    pub max_file_size_mb: u64,
    pub idle_cpu_percent: f32,
    pub active_cpu_percent: f32,
    pub idle_threshold_seconds: u64,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            watch_paths: vec![],
            exclude_patterns: vec![
                ".git".into(),
                "node_modules".into(),
                "__pycache__".into(),
                ".DS_Store".into(),
                "*.tmp".into(),
            ],
            max_file_size_mb: 100,
            idle_cpu_percent: 15.0,
            active_cpu_percent: 80.0,
            idle_threshold_seconds: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub default_mode: SearchMode,
    pub max_results: usize,
    pub snippet_length: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_mode: SearchMode::Hybrid,
            max_results: 50,
            snippet_length: 200,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub enabled: bool,
    pub embedding_model: EmbeddingSource,
    pub llm_model_path: Option<PathBuf>,
    pub llm_threads: usize,
    pub auto_generate_context: bool,
    pub api_base_url: Option<String>,
    pub api_key: Option<String>,
    pub api_model: Option<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            embedding_model: EmbeddingSource::Builtin,
            llm_model_path: None,
            llm_threads: 4,
            auto_generate_context: false,
            api_base_url: None,
            api_key: None,
            api_model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbnailConfig {
    pub enabled: bool,
    pub max_cache_mb: u64,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_cache_mb: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub indexing: IndexingConfig,
    pub search: SearchConfig,
    pub ai: AiConfig,
    pub thumbnails: ThumbnailConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            indexing: IndexingConfig::default(),
            search: SearchConfig::default(),
            ai: AiConfig::default(),
            thumbnails: ThumbnailConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "noema", "Noema")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".noema"))
    }

    pub fn data_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "noema", "Noema")
            .map(|d| d.data_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".noema"))
    }

    pub fn cache_dir() -> PathBuf {
        directories::ProjectDirs::from("com", "noema", "Noema")
            .map(|d| d.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".noema/cache"))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                NoemaError::Config {
                    key: "file".into(),
                    detail: format!("Config file not found: {}", path.display()),
                }
            } else {
                NoemaError::Io(e)
            }
        })?;
        toml::from_str(&content).map_err(|e| NoemaError::Config {
            key: "parse".into(),
            detail: e.to_string(),
        })
    }

    pub fn load_or_default() -> Self {
        let config_path = Self::config_dir().join("config.toml");
        Self::load(&config_path).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| NoemaError::Config {
            key: "serialize".into(),
            detail: e.to_string(),
        })?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

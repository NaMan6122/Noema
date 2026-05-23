use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use noema_core::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedContext {
    pub summary: String,
    pub entities: Vec<Entity>,
    pub suggested_tags: Vec<String>,
    pub key_phrases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub text: String,
    pub entity_type: EntityType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Date,
    Concept,
    Technology,
    Other(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct FileContext {
    pub version: u32,
    pub summary: String,
    pub entities: Vec<Entity>,
    pub tags: Vec<String>,
    pub user_edited: bool,
    pub generated_at: DateTime<Utc>,
    pub model_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserContextEdit {
    pub summary: Option<String>,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContextVersion {
    pub version: u32,
    pub summary: String,
    pub generated_at: DateTime<Utc>,
    pub user_edited: bool,
}

#[async_trait::async_trait]
pub trait InferenceBackend: Send + Sync {
    async fn generate_context(&self, content: &str, max_tokens: usize) -> Result<GeneratedContext>;
    async fn suggest_filename(&self, content: &str) -> Result<String>;
    async fn suggest_tags(&self, content: &str, existing_tags: &[String]) -> Result<Vec<String>>;
}

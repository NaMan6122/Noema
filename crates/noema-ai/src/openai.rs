use noema_core::error::{NoemaError, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::types::{Entity, EntityType, GeneratedContext, InferenceBackend};

pub struct OpenAiBackend {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiBackend {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    async fn chat(&self, system: &str, user: &str, max_tokens: usize) -> Result<String> {
        let body = serde_json::json!({
            "model": &self.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "max_tokens": max_tokens,
            "temperature": 0.3
        });

        let resp = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| NoemaError::Ai { detail: format!("HTTP error: {}", e) })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(NoemaError::Ai { detail: format!("API error {}: {}", status, text) });
        }

        let json: ChatResponse = resp.json().await
            .map_err(|e| NoemaError::Ai { detail: format!("Parse error: {}", e) })?;

        json.choices.into_iter().next()
            .and_then(|c| c.message.content)
            .ok_or(NoemaError::Ai { detail: "Empty response".into() })
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ContextJson {
    summary: Option<String>,
    entities: Option<Vec<EntityJson>>,
    tags: Option<Vec<String>>,
    key_phrases: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct EntityJson {
    text: String,
    #[serde(rename = "type")]
    entity_type: Option<String>,
}

const CONTEXT_SYSTEM: &str = r#"You are a file analysis assistant. Given a document's content, produce:
1. A 2-3 sentence summary of what this document is about
2. Key entities (people, organizations, dates, concepts, technologies)
3. 3-5 suggested tags for categorization
4. Key phrases that capture the main topics

Respond in JSON format:
{"summary": "...", "entities": [{"text": "...", "type": "person|org|location|date|concept|tech"}], "tags": ["..."], "key_phrases": ["..."]}"#;

const RENAME_SYSTEM: &str = r#"Given this document content, suggest a descriptive filename (no extension).
Use format: YYYY-MM-DD_descriptive_name (if date is relevant) or just descriptive_name.
Use lowercase, underscores, no special characters. Respond with only the filename, nothing else."#;

const TAGS_SYSTEM: &str = r#"Suggest 3-5 descriptive tags for categorizing this document. Consider the existing tags to avoid duplicates. Respond with a JSON array of strings only, e.g. ["tag1", "tag2", "tag3"]."#;

fn parse_entity_type(s: &str) -> EntityType {
    match s.to_lowercase().as_str() {
        "person" => EntityType::Person,
        "org" | "organization" => EntityType::Organization,
        "location" => EntityType::Location,
        "date" => EntityType::Date,
        "concept" => EntityType::Concept,
        "tech" | "technology" => EntityType::Technology,
        other => EntityType::Other(other.to_string()),
    }
}

fn truncate_content(content: &str, max_chars: usize) -> &str {
    if content.len() <= max_chars { content } else { &content[..max_chars] }
}

#[async_trait::async_trait]
impl InferenceBackend for OpenAiBackend {
    async fn generate_context(&self, content: &str, max_tokens: usize) -> Result<GeneratedContext> {
        let truncated = truncate_content(content, 12000);
        let response = self.chat(CONTEXT_SYSTEM, truncated, max_tokens).await?;

        let parsed: ContextJson = serde_json::from_str(&response).unwrap_or_else(|_| {
            warn!("Failed to parse context JSON, using fallback");
            ContextJson {
                summary: Some(response.lines().next().unwrap_or("").to_string()),
                entities: None,
                tags: None,
                key_phrases: None,
            }
        });

        Ok(GeneratedContext {
            summary: parsed.summary.unwrap_or_default(),
            entities: parsed.entities.unwrap_or_default().into_iter().map(|e| Entity {
                text: e.text,
                entity_type: parse_entity_type(e.entity_type.as_deref().unwrap_or("concept")),
            }).collect(),
            suggested_tags: parsed.tags.unwrap_or_default(),
            key_phrases: parsed.key_phrases.unwrap_or_default(),
        })
    }

    async fn suggest_filename(&self, content: &str) -> Result<String> {
        let truncated = truncate_content(content, 6000);
        let name = self.chat(RENAME_SYSTEM, truncated, 100).await?;
        Ok(name.trim().to_string())
    }

    async fn suggest_tags(&self, content: &str, existing_tags: &[String]) -> Result<Vec<String>> {
        let truncated = truncate_content(content, 8000);
        let user_msg = if existing_tags.is_empty() {
            truncated.to_string()
        } else {
            format!("Existing tags: {}\n\nContent:\n{}", existing_tags.join(", "), truncated)
        };

        let response = self.chat(TAGS_SYSTEM, &user_msg, 200).await?;

        let tags: Vec<String> = serde_json::from_str(&response).unwrap_or_else(|_| {
            response.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect()
        });

        Ok(tags)
    }
}

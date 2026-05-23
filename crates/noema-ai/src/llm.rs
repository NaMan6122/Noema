use std::sync::Arc;
use std::time::{Duration, Instant};

use noema_core::error::{NoemaError, Result};
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::types::{GeneratedContext, InferenceBackend};

pub struct LlmEngine {
    backend: Option<Arc<dyn InferenceBackend>>,
    inference_lock: Mutex<()>,
    last_used: Mutex<Option<Instant>>,
    inactivity_timeout: Duration,
}

impl LlmEngine {
    pub fn new(inactivity_timeout_secs: u64) -> Self {
        Self {
            backend: None,
            inference_lock: Mutex::new(()),
            last_used: Mutex::new(None),
            inactivity_timeout: Duration::from_secs(inactivity_timeout_secs),
        }
    }

    pub fn with_backend(mut self, backend: Arc<dyn InferenceBackend>) -> Self {
        self.backend = Some(backend);
        self
    }

    pub fn is_loaded(&self) -> bool {
        self.backend.is_some()
    }

    pub fn backend_name(&self) -> &str {
        if self.backend.is_some() { "active" } else { "none" }
    }

    pub async fn generate_context(&self, content: &str, max_tokens: usize) -> Result<GeneratedContext> {
        let backend = self.backend.as_ref()
            .ok_or(NoemaError::Ai { detail: "No inference backend loaded. Configure a model or API key.".into() })?;

        let _guard = self.inference_lock.lock().await;
        *self.last_used.lock().await = Some(Instant::now());

        debug!("Generating context ({} chars)", content.len());
        backend.generate_context(content, max_tokens).await
    }

    pub async fn suggest_filename(&self, content: &str) -> Result<String> {
        let backend = self.backend.as_ref()
            .ok_or(NoemaError::Ai { detail: "No inference backend loaded.".into() })?;

        let _guard = self.inference_lock.lock().await;
        *self.last_used.lock().await = Some(Instant::now());

        backend.suggest_filename(content).await
    }

    pub async fn suggest_tags(&self, content: &str, existing_tags: &[String]) -> Result<Vec<String>> {
        let backend = self.backend.as_ref()
            .ok_or(NoemaError::Ai { detail: "No inference backend loaded.".into() })?;

        let _guard = self.inference_lock.lock().await;
        *self.last_used.lock().await = Some(Instant::now());

        backend.suggest_tags(content, existing_tags).await
    }
}

pub struct StubBackend;

#[async_trait::async_trait]
impl InferenceBackend for StubBackend {
    async fn generate_context(&self, content: &str, _max_tokens: usize) -> Result<GeneratedContext> {
        let word_count = content.split_whitespace().count();
        Ok(GeneratedContext {
            summary: format!("Document with {} words.", word_count),
            entities: vec![],
            suggested_tags: vec!["untagged".into()],
            key_phrases: content.split_whitespace().take(5).map(|s| s.to_string()).collect(),
        })
    }

    async fn suggest_filename(&self, content: &str) -> Result<String> {
        let first_line = content.lines().next().unwrap_or("untitled");
        let clean: String = first_line.chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ')
            .collect::<String>()
            .to_lowercase()
            .replace(' ', "_");
        Ok(clean.chars().take(50).collect())
    }

    async fn suggest_tags(&self, _content: &str, existing_tags: &[String]) -> Result<Vec<String>> {
        let mut tags = existing_tags.to_vec();
        if tags.is_empty() {
            tags.push("untagged".into());
        }
        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_backend() {
        let engine = LlmEngine::new(120).with_backend(Arc::new(StubBackend));
        assert!(engine.is_loaded());

        let ctx = engine.generate_context("Hello world this is a test", 512).await.unwrap();
        assert!(ctx.summary.contains("6 words"));
        assert_eq!(ctx.suggested_tags, vec!["untagged"]);
    }

    #[tokio::test]
    async fn test_no_backend_error() {
        let engine = LlmEngine::new(120);
        assert!(!engine.is_loaded());

        let result = engine.generate_context("test", 512).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_suggest_filename() {
        let engine = LlmEngine::new(120).with_backend(Arc::new(StubBackend));
        let name = engine.suggest_filename("Meeting Notes for Q4 Planning").await.unwrap();
        assert_eq!(name, "meeting_notes_for_q4_planning");
    }
}

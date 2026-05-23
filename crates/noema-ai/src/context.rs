use std::sync::Arc;

use chrono::Utc;
use noema_core::db::Database;
use noema_core::error::{NoemaError, Result};
use rusqlite::params;
use tracing::debug;

use crate::types::{
    ContextVersion, Entity, EntityType, FileContext, GeneratedContext, UserContextEdit,
};

pub struct ContextStore {
    db: Arc<Database>,
}

impl ContextStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn save_context(&self, file_id: i64, ctx: &GeneratedContext, model_id: &str) -> Result<()> {
        let conn = self.db.connection()?;

        let next_version: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM ai_context WHERE file_id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .unwrap_or(1);

        let entities_json = serde_json::to_string(&ctx.entities)
            .map_err(|e| NoemaError::Ai { detail: e.to_string() })?;
        let tags_json = serde_json::to_string(&ctx.suggested_tags)
            .map_err(|e| NoemaError::Ai { detail: e.to_string() })?;

        conn.execute(
            "INSERT INTO ai_context (file_id, version, summary, entities, suggested_tags, generated_at, model_id, user_edited)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params![
                file_id,
                next_version,
                ctx.summary,
                entities_json,
                tags_json,
                Utc::now().to_rfc3339(),
                model_id,
            ],
        )?;

        self.prune_old_versions(file_id, 10)?;

        debug!(file_id, version = next_version, "Saved AI context");
        Ok(())
    }

    pub fn get_context(&self, file_id: i64) -> Result<Option<FileContext>> {
        let conn = self.db.connection()?;

        let result = conn.query_row(
            "SELECT version, summary, entities, suggested_tags, generated_at, model_id, user_edited
             FROM ai_context WHERE file_id = ?1 ORDER BY version DESC LIMIT 1",
            params![file_id],
            |row| {
                Ok((
                    row.get::<_, u32>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, bool>(6)?,
                ))
            },
        );

        match result {
            Ok((version, summary, entities_json, tags_json, generated_at, model_id, user_edited)) => {
                let entities: Vec<Entity> = entities_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let tags: Vec<String> = tags_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                let generated_at = chrono::DateTime::parse_from_rfc3339(&generated_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Some(FileContext {
                    version,
                    summary,
                    entities,
                    tags,
                    user_edited,
                    generated_at,
                    model_id: model_id.unwrap_or_default(),
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_versions(&self, file_id: i64) -> Result<Vec<ContextVersion>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT version, summary, generated_at, user_edited
             FROM ai_context WHERE file_id = ?1 ORDER BY version DESC",
        )?;

        let rows = stmt.query_map(params![file_id], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
            ))
        })?;

        let mut versions = Vec::new();
        for row in rows {
            let (version, summary, generated_at, user_edited) = row?;
            let generated_at = chrono::DateTime::parse_from_rfc3339(&generated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            versions.push(ContextVersion {
                version,
                summary,
                generated_at,
                user_edited,
            });
        }

        Ok(versions)
    }

    pub fn update_user_edit(&self, file_id: i64, edit: &UserContextEdit) -> Result<()> {
        let current = self.get_context(file_id)?
            .ok_or(NoemaError::Ai { detail: "No existing context to edit".into() })?;

        let summary = edit.summary.as_deref().unwrap_or(&current.summary).to_string();

        let mut tags = current.tags.clone();
        tags.extend(edit.add_tags.iter().cloned());
        tags.retain(|t| !edit.remove_tags.contains(t));
        tags.sort();
        tags.dedup();

        let conn = self.db.connection()?;

        let next_version: u32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) + 1 FROM ai_context WHERE file_id = ?1",
                params![file_id],
                |row| row.get(0),
            )
            .unwrap_or(1);

        let entities_json = serde_json::to_string(&current.entities)
            .map_err(|e| NoemaError::Ai { detail: e.to_string() })?;
        let tags_json = serde_json::to_string(&tags)
            .map_err(|e| NoemaError::Ai { detail: e.to_string() })?;

        conn.execute(
            "INSERT INTO ai_context (file_id, version, summary, entities, suggested_tags, generated_at, model_id, user_edited)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            params![
                file_id,
                next_version,
                summary,
                entities_json,
                tags_json,
                Utc::now().to_rfc3339(),
                current.model_id,
            ],
        )?;

        self.prune_old_versions(file_id, 10)?;
        Ok(())
    }

    pub fn apply_suggested_tags(&self, file_id: i64, tags: &[String]) -> Result<()> {
        let conn = self.db.connection()?;

        for tag_name in tags {
            conn.execute(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)",
                params![tag_name],
            )?;

            conn.execute(
                "INSERT OR IGNORE INTO file_tags (file_id, tag_id, source)
                 SELECT ?1, id, 'ai' FROM tags WHERE name = ?2",
                params![file_id, tag_name],
            )?;
        }

        debug!(file_id, count = tags.len(), "Applied AI tags");
        Ok(())
    }

    fn prune_old_versions(&self, file_id: i64, max_versions: u32) -> Result<()> {
        let conn = self.db.connection()?;
        conn.execute(
            "DELETE FROM ai_context WHERE file_id = ?1 AND version <= (
                SELECT COALESCE(MAX(version), 0) - ?2 FROM ai_context WHERE file_id = ?1
            )",
            params![file_id, max_versions],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noema_core::db::Database;
    use tempfile::TempDir;

    fn setup_db() -> (Arc<Database>, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();
        db.run_migrations().unwrap();
        let conn = db.connection().unwrap();
        conn.execute(
            "INSERT INTO files (path, filename, size_bytes, created_at, modified_at, parent_dir)
             VALUES ('/test.md', 'test.md', 100, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '/')",
            [],
        ).unwrap();
        (Arc::new(db), dir)
    }

    #[test]
    fn test_save_and_get_context() {
        let (db, _dir) = setup_db();
        let store = ContextStore::new(db);

        let ctx = GeneratedContext {
            summary: "A test document".into(),
            entities: vec![Entity { text: "Rust".into(), entity_type: EntityType::Technology }],
            suggested_tags: vec!["programming".into(), "rust".into()],
            key_phrases: vec!["test document".into()],
        };

        store.save_context(1, &ctx, "stub-v1").unwrap();

        let retrieved = store.get_context(1).unwrap().unwrap();
        assert_eq!(retrieved.version, 1);
        assert_eq!(retrieved.summary, "A test document");
        assert_eq!(retrieved.tags.len(), 2);
        assert!(!retrieved.user_edited);
    }

    #[test]
    fn test_versioning() {
        let (db, _dir) = setup_db();
        let store = ContextStore::new(db);

        let ctx = GeneratedContext {
            summary: "Version 1".into(),
            entities: vec![],
            suggested_tags: vec![],
            key_phrases: vec![],
        };
        store.save_context(1, &ctx, "stub-v1").unwrap();

        let ctx2 = GeneratedContext {
            summary: "Version 2".into(),
            entities: vec![],
            suggested_tags: vec!["new-tag".into()],
            key_phrases: vec![],
        };
        store.save_context(1, &ctx2, "stub-v1").unwrap();

        let latest = store.get_context(1).unwrap().unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.summary, "Version 2");

        let versions = store.get_versions(1).unwrap();
        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn test_user_edit() {
        let (db, _dir) = setup_db();
        let store = ContextStore::new(db);

        let ctx = GeneratedContext {
            summary: "Original".into(),
            entities: vec![],
            suggested_tags: vec!["tag1".into(), "tag2".into()],
            key_phrases: vec![],
        };
        store.save_context(1, &ctx, "stub-v1").unwrap();

        let edit = UserContextEdit {
            summary: Some("User edited summary".into()),
            add_tags: vec!["tag3".into()],
            remove_tags: vec!["tag1".into()],
        };
        store.update_user_edit(1, &edit).unwrap();

        let latest = store.get_context(1).unwrap().unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.summary, "User edited summary");
        assert!(latest.user_edited);
        assert!(latest.tags.contains(&"tag2".to_string()));
        assert!(latest.tags.contains(&"tag3".to_string()));
        assert!(!latest.tags.contains(&"tag1".to_string()));
    }

    #[test]
    fn test_apply_tags() {
        let (db, _dir) = setup_db();
        let store = ContextStore::new(db.clone());

        store.apply_suggested_tags(1, &["ai-tag".into(), "another".into()]).unwrap();

        let conn = db.connection().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM file_tags WHERE file_id = 1 AND source = 'ai'",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 2);
    }
}

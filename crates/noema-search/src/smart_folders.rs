use std::sync::Arc;

use chrono::Utc;
use noema_core::db::Database;
use noema_core::error::Result;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolder {
    pub id: i64,
    pub name: String,
    pub query: SmartFolderQuery,
    pub icon: Option<String>,
    pub position: Option<i32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartFolderQuery {
    pub text: Option<String>,
    pub file_types: Vec<String>,
    pub tags: Vec<String>,
    pub in_path: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub after: Option<String>,
    pub before: Option<String>,
}

impl SmartFolderQuery {
    pub fn to_search_query(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref text) = self.text {
            parts.push(text.clone());
        }
        for ft in &self.file_types {
            parts.push(format!("type:{}", ft));
        }
        for tag in &self.tags {
            parts.push(format!("has:tag:{}", tag));
        }
        if let Some(ref p) = self.in_path {
            parts.push(format!("in:{}", p));
        }
        if let Some(sz) = self.min_size {
            parts.push(format!("size:>{}", sz));
        }
        if let Some(sz) = self.max_size {
            parts.push(format!("size:<{}", sz));
        }
        if let Some(ref d) = self.after {
            parts.push(format!("after:{}", d));
        }
        if let Some(ref d) = self.before {
            parts.push(format!("before:{}", d));
        }
        parts.join(" ")
    }
}

pub struct SmartFolderStore {
    db: Arc<Database>,
}

impl SmartFolderStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub fn create(&self, name: &str, query: &SmartFolderQuery, icon: Option<&str>) -> Result<SmartFolder> {
        let conn = self.db.connection()?;
        let query_json = serde_json::to_string(query)
            .map_err(|e| noema_core::error::NoemaError::Ai { detail: e.to_string() })?;
        let now = Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO virtual_folders (name, query_json, icon, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![name, query_json, icon, now],
        )?;

        let id = conn.last_insert_rowid();
        Ok(SmartFolder {
            id,
            name: name.to_string(),
            query: query.clone(),
            icon: icon.map(|s| s.to_string()),
            position: None,
            created_at: now,
        })
    }

    pub fn list(&self) -> Result<Vec<SmartFolder>> {
        let conn = self.db.connection()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, query_json, icon, position, created_at FROM virtual_folders ORDER BY position, name"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i32>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;

        let mut folders = Vec::new();
        for row in rows {
            let (id, name, query_json, icon, position, created_at) = row?;
            let query: SmartFolderQuery = serde_json::from_str(&query_json).unwrap_or(SmartFolderQuery {
                text: None, file_types: vec![], tags: vec![], in_path: None,
                min_size: None, max_size: None, after: None, before: None,
            });
            folders.push(SmartFolder { id, name, query, icon, position, created_at });
        }

        Ok(folders)
    }

    pub fn get(&self, id: i64) -> Result<Option<SmartFolder>> {
        let conn = self.db.connection()?;
        let result = conn.query_row(
            "SELECT id, name, query_json, icon, position, created_at FROM virtual_folders WHERE id = ?1",
            params![id],
            |row| Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i32>>(4)?,
                row.get::<_, String>(5)?,
            )),
        );

        match result {
            Ok((id, name, query_json, icon, position, created_at)) => {
                let query: SmartFolderQuery = serde_json::from_str(&query_json).unwrap_or(SmartFolderQuery {
                    text: None, file_types: vec![], tags: vec![], in_path: None,
                    min_size: None, max_size: None, after: None, before: None,
                });
                Ok(Some(SmartFolder { id, name, query, icon, position, created_at }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn update(&self, id: i64, name: Option<&str>, query: Option<&SmartFolderQuery>, icon: Option<&str>) -> Result<()> {
        let conn = self.db.connection()?;
        if let Some(n) = name {
            conn.execute("UPDATE virtual_folders SET name = ?1 WHERE id = ?2", params![n, id])?;
        }
        if let Some(q) = query {
            let json = serde_json::to_string(q)
                .map_err(|e| noema_core::error::NoemaError::Ai { detail: e.to_string() })?;
            conn.execute("UPDATE virtual_folders SET query_json = ?1 WHERE id = ?2", params![json, id])?;
        }
        if let Some(i) = icon {
            conn.execute("UPDATE virtual_folders SET icon = ?1 WHERE id = ?2", params![i, id])?;
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let conn = self.db.connection()?;
        conn.execute("DELETE FROM virtual_folders WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn reorder(&self, ids: &[i64]) -> Result<()> {
        let conn = self.db.connection()?;
        for (pos, id) in ids.iter().enumerate() {
            conn.execute(
                "UPDATE virtual_folders SET position = ?1 WHERE id = ?2",
                params![pos as i32, id],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (Arc<Database>, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("test.db")).unwrap();
        db.run_migrations().unwrap();
        (Arc::new(db), dir)
    }

    #[test]
    fn test_create_and_list() {
        let (db, _dir) = setup();
        let store = SmartFolderStore::new(db);

        let query = SmartFolderQuery {
            text: None,
            file_types: vec!["rs".into(), "toml".into()],
            tags: vec![],
            in_path: Some("/projects".into()),
            min_size: None,
            max_size: None,
            after: None,
            before: None,
        };

        let folder = store.create("Rust Files", &query, Some("code")).unwrap();
        assert_eq!(folder.name, "Rust Files");

        let all = store.list().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].query.file_types, vec!["rs", "toml"]);
    }

    #[test]
    fn test_update_and_delete() {
        let (db, _dir) = setup();
        let store = SmartFolderStore::new(db);

        let query = SmartFolderQuery {
            text: Some("meeting notes".into()),
            file_types: vec!["md".into()],
            tags: vec![], in_path: None, min_size: None, max_size: None, after: None, before: None,
        };

        let folder = store.create("Meetings", &query, None).unwrap();
        store.update(folder.id, Some("All Meetings"), None, Some("calendar_month")).unwrap();

        let updated = store.get(folder.id).unwrap().unwrap();
        assert_eq!(updated.name, "All Meetings");
        assert_eq!(updated.icon, Some("calendar_month".into()));

        store.delete(folder.id).unwrap();
        assert!(store.get(folder.id).unwrap().is_none());
    }

    #[test]
    fn test_query_to_search() {
        let query = SmartFolderQuery {
            text: Some("rust".into()),
            file_types: vec!["rs".into()],
            tags: vec!["important".into()],
            in_path: Some("/src".into()),
            min_size: None,
            max_size: Some(1048576),
            after: None,
            before: None,
        };

        let search = query.to_search_query();
        assert!(search.contains("rust"));
        assert!(search.contains("type:rs"));
        assert!(search.contains("has:tag:important"));
        assert!(search.contains("in:/src"));
        assert!(search.contains("size:<1048576"));
    }
}

//! KB-04 (Chronos) — sovereign local chat history (SQLite).
//!
//! Bare-metal local DB for project-based chat threads.
//!
//! - Source of truth for Studio web UI chat history (threads/projects/messages)
//! - Independent from the Sled KnowledgeStore so existing KB-04 events remain untouched.

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct ChronosSqlite {
    db_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ThreadRow {
    pub id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub last_message_at_ms: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MessageRow {
    pub id: String,
    pub thread_id: String,
    pub project_id: Option<String>,
    pub role: String,
    pub content: String,
    pub created_at_ms: i64,
    pub metadata_json: Option<String>,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl ChronosSqlite {
    pub fn new(db_path: PathBuf) -> Result<Self, rusqlite::Error> {
        let this = Self { db_path };
        this.init()?;
        Ok(this)
    }

    pub fn path(&self) -> &Path {
        &self.db_path
    }

    fn open(&self) -> Result<Connection, rusqlite::Error> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
        // Enforce FK constraints on every connection (SQLite default is OFF unless set).
        let _ = conn.pragma_update(None, "foreign_keys", "ON");
        Ok(conn)
    }

    fn init(&self) -> Result<(), rusqlite::Error> {
        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = self.open()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                project_id TEXT NULL,
                title TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                last_message_at_ms INTEGER NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_threads_project_id ON threads(project_id);
            CREATE INDEX IF NOT EXISTS idx_threads_last_message ON threads(last_message_at_ms);

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                project_id TEXT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                metadata_json TEXT NULL,
                FOREIGN KEY(thread_id) REFERENCES threads(id) ON DELETE CASCADE,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_thread_id ON messages(thread_id);
            CREATE INDEX IF NOT EXISTS idx_messages_project_id ON messages(project_id);
            CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at_ms);
            "#,
        )?;
        Ok(())
    }

    pub fn ensure_thread_exists(
        &self,
        thread_id: &str,
        title: &str,
        project_id: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.open()?;
        let ts = now_ms();
        conn.execute(
            r#"
            INSERT INTO threads (id, project_id, title, created_at_ms, updated_at_ms, last_message_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, NULL)
            ON CONFLICT(id) DO UPDATE SET
                project_id = COALESCE(excluded.project_id, threads.project_id),
                title = CASE WHEN threads.title = '' THEN excluded.title ELSE threads.title END,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![thread_id, project_id, title, ts, ts],
        )?;
        Ok(())
    }

    pub fn ensure_project_exists(
        &self,
        project_id: &str,
        name: &str,
    ) -> Result<ProjectRow, rusqlite::Error> {
        let conn = self.open()?;
        let ts = now_ms();
        let pid = project_id.trim();
        let n = name.trim();
        conn.execute(
            r#"
            INSERT INTO projects (id, name, created_at_ms, updated_at_ms)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                name = CASE WHEN excluded.name != '' THEN excluded.name ELSE projects.name END,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![pid, n, ts, ts],
        )?;
        Ok(ProjectRow {
            id: pid.to_string(),
            name: n.to_string(),
            created_at_ms: ts,
            updated_at_ms: ts,
        })
    }

    pub fn create_project(&self, name: &str) -> Result<ProjectRow, rusqlite::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let ts = now_ms();
        let conn = self.open()?;
        conn.execute(
            "INSERT INTO projects (id, name, created_at_ms, updated_at_ms) VALUES (?1, ?2, ?3, ?4)",
            params![id, name.trim(), ts, ts],
        )?;
        Ok(ProjectRow {
            id,
            name: name.trim().to_string(),
            created_at_ms: ts,
            updated_at_ms: ts,
        })
    }

    pub fn get_project_by_name(&self, name: &str) -> Result<Option<ProjectRow>, rusqlite::Error> {
        let conn = self.open()?;
        let needle = name.trim();
        if needle.is_empty() {
            return Ok(None);
        }
        let row = conn
            .query_row(
                "SELECT id, name, created_at_ms, updated_at_ms FROM projects WHERE lower(name) = lower(?1) LIMIT 1",
                params![needle],
                |r| {
                    Ok(ProjectRow {
                        id: r.get(0)?,
                        name: r.get(1)?,
                        created_at_ms: r.get(2)?,
                        updated_at_ms: r.get(3)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Upsert project by name (case-insensitive). Returns existing project when it matches by name.
    pub fn upsert_project_by_name(&self, name: &str) -> Result<ProjectRow, rusqlite::Error> {
        if let Some(existing) = self.get_project_by_name(name)? {
            let conn = self.open()?;
            let ts = now_ms();
            conn.execute(
                "UPDATE projects SET updated_at_ms = ?1 WHERE id = ?2",
                params![ts, existing.id],
            )?;
            return Ok(ProjectRow {
                updated_at_ms: ts,
                ..existing
            });
        }
        self.create_project(name)
    }

    pub fn list_projects(&self) -> Result<Vec<ProjectRow>, rusqlite::Error> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, created_at_ms, updated_at_ms FROM projects ORDER BY updated_at_ms DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ProjectRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    created_at_ms: r.get(2)?,
                    updated_at_ms: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_thread(&self, title: &str, project_id: Option<&str>) -> Result<ThreadRow, rusqlite::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let ts = now_ms();
        let conn = self.open()?;
        conn.execute(
            "INSERT INTO threads (id, project_id, title, created_at_ms, updated_at_ms, last_message_at_ms) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![id, project_id, title.trim(), ts, ts],
        )?;
        Ok(ThreadRow {
            id,
            project_id: project_id.map(|s| s.to_string()),
            title: title.trim().to_string(),
            created_at_ms: ts,
            updated_at_ms: ts,
            last_message_at_ms: None,
        })
    }

    pub fn list_threads(&self, project_id: Option<&str>, limit: usize) -> Result<Vec<ThreadRow>, rusqlite::Error> {
        let conn = self.open()?;
        let sql = if project_id.is_some() {
            "SELECT id, project_id, title, created_at_ms, updated_at_ms, last_message_at_ms FROM threads WHERE project_id = ?1 ORDER BY COALESCE(last_message_at_ms, updated_at_ms) DESC LIMIT ?2"
        } else {
            "SELECT id, project_id, title, created_at_ms, updated_at_ms, last_message_at_ms FROM threads WHERE project_id IS NULL ORDER BY COALESCE(last_message_at_ms, updated_at_ms) DESC LIMIT ?1"
        };
        let mut stmt = conn.prepare(sql)?;
        let rows = if let Some(pid) = project_id {
            stmt.query_map(params![pid, limit as i64], |r| {
                Ok(ThreadRow {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    title: r.get(2)?,
                    created_at_ms: r.get(3)?,
                    updated_at_ms: r.get(4)?,
                    last_message_at_ms: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![limit as i64], |r| {
                Ok(ThreadRow {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    title: r.get(2)?,
                    created_at_ms: r.get(3)?,
                    updated_at_ms: r.get(4)?,
                    last_message_at_ms: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    }

    pub fn list_threads_any(&self, limit: usize) -> Result<Vec<ThreadRow>, rusqlite::Error> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, created_at_ms, updated_at_ms, last_message_at_ms FROM threads ORDER BY COALESCE(last_message_at_ms, updated_at_ms) DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(ThreadRow {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    title: r.get(2)?,
                    created_at_ms: r.get(3)?,
                    updated_at_ms: r.get(4)?,
                    last_message_at_ms: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn rename_thread(&self, thread_id: &str, title: &str) -> Result<(), rusqlite::Error> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE threads SET title = ?1, updated_at_ms = ?2 WHERE id = ?3",
            params![title.trim(), now_ms(), thread_id],
        )?;
        Ok(())
    }

    pub fn set_thread_project(
        &self,
        thread_id: &str,
        project_id: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let conn = self.open()?;
        conn.execute(
            "UPDATE threads SET project_id = ?1, updated_at_ms = ?2 WHERE id = ?3",
            params![project_id, now_ms(), thread_id],
        )?;
        Ok(())
    }

    pub fn delete_thread(&self, thread_id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.open()?;
        conn.execute("DELETE FROM threads WHERE id = ?1", params![thread_id])?;
        Ok(())
    }

    pub fn append_message(
        &self,
        thread_id: &str,
        project_id: Option<&str>,
        role: &str,
        content: &str,
        metadata_json: Option<&str>,
    ) -> Result<MessageRow, rusqlite::Error> {
        let conn = self.open()?;
        let msg_id = uuid::Uuid::new_v4().to_string();
        let ts = now_ms();
        conn.execute(
            "INSERT INTO messages (id, thread_id, project_id, role, content, created_at_ms, metadata_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![msg_id, thread_id, project_id, role, content, ts, metadata_json],
        )?;
        conn.execute(
            "UPDATE threads SET updated_at_ms = ?1, last_message_at_ms = ?1 WHERE id = ?2",
            params![ts, thread_id],
        )?;

        Ok(MessageRow {
            id: msg_id,
            thread_id: thread_id.to_string(),
            project_id: project_id.map(|s| s.to_string()),
            role: role.to_string(),
            content: content.to_string(),
            created_at_ms: ts,
            metadata_json: metadata_json.map(|s| s.to_string()),
        })
    }

    pub fn list_messages(
        &self,
        thread_id: &str,
        limit: usize,
        before_ms: Option<i64>,
    ) -> Result<Vec<MessageRow>, rusqlite::Error> {
        let conn = self.open()?;
        let sql = if before_ms.is_some() {
            "SELECT id, thread_id, project_id, role, content, created_at_ms, metadata_json FROM messages WHERE thread_id = ?1 AND created_at_ms < ?2 ORDER BY created_at_ms DESC LIMIT ?3"
        } else {
            "SELECT id, thread_id, project_id, role, content, created_at_ms, metadata_json FROM messages WHERE thread_id = ?1 ORDER BY created_at_ms DESC LIMIT ?2"
        };
        let mut stmt = conn.prepare(sql)?;
        let mut rows: Vec<MessageRow> = if let Some(b) = before_ms {
            stmt.query_map(params![thread_id, b, limit as i64], |r| {
                Ok(MessageRow {
                    id: r.get(0)?,
                    thread_id: r.get(1)?,
                    project_id: r.get(2)?,
                    role: r.get(3)?,
                    content: r.get(4)?,
                    created_at_ms: r.get(5)?,
                    metadata_json: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![thread_id, limit as i64], |r| {
                Ok(MessageRow {
                    id: r.get(0)?,
                    thread_id: r.get(1)?,
                    project_id: r.get(2)?,
                    role: r.get(3)?,
                    content: r.get(4)?,
                    created_at_ms: r.get(5)?,
                    metadata_json: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        // Return oldest -> newest for UI rendering.
        rows.sort_by(|a, b| a.created_at_ms.cmp(&b.created_at_ms));
        Ok(rows)
    }

    pub fn get_project_name(&self, project_id: &str) -> Result<Option<String>, rusqlite::Error> {
        let conn = self.open()?;
        let name: Option<String> = conn
            .query_row(
                "SELECT name FROM projects WHERE id = ?1",
                params![project_id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(name)
    }
}


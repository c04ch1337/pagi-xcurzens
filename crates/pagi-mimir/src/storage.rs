//! Chronos-compatible meeting storage: meetings + meeting_transcripts.
//!
//! Uses the same SQLite DB as the gateway (data/pagi_chronos/chronos.sqlite)
//! and adds `meetings` and `meeting_transcripts` tables.

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use std::path::{Path, PathBuf};

/// One row from Chronos `projects` table (for lookup by name).
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// One row in the `meetings` table.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MeetingRow {
    pub id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub summary_path: Option<String>,
    pub created_at_ms: i64,
}

/// One row in the `meeting_transcripts` table.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MeetingTranscriptRow {
    pub id: String,
    pub meeting_id: String,
    pub speaker_id: Option<i64>,
    pub text: String,
    pub timestamp_sec: f64,
    pub created_at_ms: i64,
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Storage for meetings and transcripts (Chronos DB).
pub struct MeetingStorage {
    db_path: PathBuf,
}

impl MeetingStorage {
    /// Open or create the Chronos DB and ensure meetings tables exist.
    /// Use the same path as the gateway: e.g. `./data/pagi_chronos/chronos.sqlite`.
    pub fn new(db_path: PathBuf) -> Result<Self, rusqlite::Error> {
        let this = Self { db_path };
        this.init()?;
        Ok(this)
    }

    /// Default path: PAGI_STORAGE_PATH or ./data, then pagi_chronos/chronos.sqlite.
    pub fn default_path() -> PathBuf {
        let base = std::env::var("PAGI_STORAGE_PATH")
            .unwrap_or_else(|_| "./data".to_string());
        PathBuf::from(base).join("pagi_chronos").join("chronos.sqlite")
    }

    /// Open storage at the default Chronos path.
    pub fn open_default() -> Result<Self, rusqlite::Error> {
        Self::new(Self::default_path())
    }

    pub fn path(&self) -> &Path {
        &self.db_path
    }

    fn open(&self) -> Result<Connection, rusqlite::Error> {
        let conn = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;
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
            CREATE TABLE IF NOT EXISTS meetings (
                id TEXT PRIMARY KEY,
                project_id TEXT NULL,
                title TEXT NOT NULL,
                started_at_ms INTEGER NOT NULL,
                ended_at_ms INTEGER NULL,
                summary_path TEXT NULL,
                created_at_ms INTEGER NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL
            );
            CREATE INDEX IF NOT EXISTS idx_meetings_project_id ON meetings(project_id);
            CREATE INDEX IF NOT EXISTS idx_meetings_started_at ON meetings(started_at_ms);

            CREATE TABLE IF NOT EXISTS meeting_transcripts (
                id TEXT PRIMARY KEY,
                meeting_id TEXT NOT NULL,
                speaker_id INTEGER NULL,
                text TEXT NOT NULL,
                timestamp_sec REAL NOT NULL,
                created_at_ms INTEGER NOT NULL,
                FOREIGN KEY(meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_meeting_transcripts_meeting_id ON meeting_transcripts(meeting_id);
            "#,
        )?;
        Ok(())
    }

    /// Create a new meeting and return its row.
    pub fn create_meeting(
        &self,
        title: &str,
        project_id: Option<&str>,
    ) -> Result<MeetingRow, rusqlite::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let ts = now_ms();
        let conn = self.open()?;
        conn.execute(
            r#"
            INSERT INTO meetings (id, project_id, title, started_at_ms, ended_at_ms, summary_path, created_at_ms)
            VALUES (?1, ?2, ?3, ?4, NULL, NULL, ?5)
            "#,
            params![id, project_id, title.trim(), ts, ts],
        )?;
        Ok(MeetingRow {
            id: id.clone(),
            project_id: project_id.map(String::from),
            title: title.trim().to_string(),
            started_at_ms: ts,
            ended_at_ms: None,
            summary_path: None,
            created_at_ms: ts,
        })
    }

    /// End a meeting and optionally set summary path.
    pub fn end_meeting(
        &self,
        meeting_id: &str,
        summary_path: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let ts = now_ms();
        let conn = self.open()?;
        conn.execute(
            "UPDATE meetings SET ended_at_ms = ?1, summary_path = ?2 WHERE id = ?3",
            params![ts, summary_path, meeting_id],
        )?;
        Ok(())
    }

    /// Append a transcript segment.
    pub fn append_transcript(
        &self,
        meeting_id: &str,
        speaker_id: Option<i64>,
        text: &str,
        timestamp_sec: f64,
    ) -> Result<MeetingTranscriptRow, rusqlite::Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let ts = now_ms();
        let conn = self.open()?;
        conn.execute(
            r#"
            INSERT INTO meeting_transcripts (id, meeting_id, speaker_id, text, timestamp_sec, created_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![id, meeting_id, speaker_id, text.trim(), timestamp_sec, ts],
        )?;
        Ok(MeetingTranscriptRow {
            id,
            meeting_id: meeting_id.to_string(),
            speaker_id,
            text: text.trim().to_string(),
            timestamp_sec,
            created_at_ms: ts,
        })
    }

    /// List transcripts for a meeting, ordered by timestamp.
    pub fn list_transcripts(
        &self,
        meeting_id: &str,
    ) -> Result<Vec<MeetingTranscriptRow>, rusqlite::Error> {
        let conn = self.open()?;
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, speaker_id, text, timestamp_sec, created_at_ms
             FROM meeting_transcripts WHERE meeting_id = ?1 ORDER BY timestamp_sec ASC",
        )?;
        let rows = stmt
            .query_map(params![meeting_id], |r| {
                Ok(MeetingTranscriptRow {
                    id: r.get(0)?,
                    meeting_id: r.get(1)?,
                    speaker_id: r.get(2)?,
                    text: r.get(3)?,
                    timestamp_sec: r.get(4)?,
                    created_at_ms: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Get meeting by id.
    pub fn get_meeting(&self, meeting_id: &str) -> Result<Option<MeetingRow>, rusqlite::Error> {
        let conn = self.open()?;
        let row = conn
            .query_row(
                "SELECT id, project_id, title, started_at_ms, ended_at_ms, summary_path, created_at_ms
                 FROM meetings WHERE id = ?1",
                params![meeting_id],
                |r| {
                    Ok(MeetingRow {
                        id: r.get(0)?,
                        project_id: r.get(1)?,
                        title: r.get(2)?,
                        started_at_ms: r.get(3)?,
                        ended_at_ms: r.get(4)?,
                        summary_path: r.get(5)?,
                        created_at_ms: r.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(row)
    }

    /// Get project name for a project_id (from Chronos projects table).
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

    /// Get project by name (case-insensitive). From Chronos projects table.
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
}

//! Scribe — Archives adapter logic into KB-03 (Techne) for versioned knowledge.
//! Ensures the system can "remember" how to talk to a 2026-era system in the year 3026.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

/// KB-03 (Techne) — versioned knowledge base for decommissioned adapter logic.
const KB03_TECHNE_DIR: &str = "techne/KB-03";

/// Scribe archives adapter source into KB-03 before any delete.
pub struct Scribe {
    kb_root: std::path::PathBuf,
}

impl Scribe {
    /// Create a Scribe with optional KB root (default: ./data/knowledge_bases).
    pub fn new(kb_root: Option<impl AsRef<Path>>) -> Self {
        let kb_root = kb_root
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| Path::new("./data/knowledge_bases").to_path_buf());
        Self { kb_root }
    }

    /// Archive adapter logic into KB-03 (Techne) before decommission.
    /// Returns the path of the archived file.
    pub fn archive_to_kb03(&self, adapter_name: &str, content: &str) -> std::io::Result<std::path::PathBuf> {
        let techne_dir = self.kb_root.join(KB03_TECHNE_DIR);
        fs::create_dir_all(&techne_dir)?;
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let archive_name = format!("{}_{:016}.rs", adapter_name, timestamp);
        let archive_path = techne_dir.join(&archive_name);
        fs::write(&archive_path, content)?;
        Ok(archive_path)
    }
}

impl Default for Scribe {
    fn default() -> Self {
        Self::new(None::<&Path>)
    }
}

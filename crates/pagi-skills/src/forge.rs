//! Forge — SAM full adapter lifecycle authority.
//! Create, Evolve, Decommission. Integrity check on Modify with Rollback.
//! Protected skills (vital organs) cannot be purged or evolved without Sovereign-Key override.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::scribe::Scribe;

/// Default protected skill names. SAM cannot decommission or evolve these without Architect override.
fn default_protected_skills() -> Vec<String> {
    vec![
        "forge".into(),
        "scribe".into(),
        "gateway".into(),
        "auth".into(),
        "xcurzens_core".into(),
    ]
}

/// SAM's Forge: creates, evolves, and decommissions adapters.
/// Protected skills are immutable unless Force is used with a valid Sovereign-Key.
pub struct Forge {
    scribe: Scribe,
    workspace_root: std::path::PathBuf,
    protected_skills: Vec<String>,
    sovereign_key: Option<String>,
}

impl Forge {
    pub fn new(workspace_root: impl AsRef<Path>, kb_root: Option<impl AsRef<Path>>) -> Self {
        Self {
            scribe: Scribe::new(kb_root),
            workspace_root: workspace_root.as_ref().to_path_buf(),
            protected_skills: default_protected_skills(),
            sovereign_key: None,
        }
    }

    /// Builder: set the Sovereign-Key for Force override (e.g. from env `FORGE_SOVEREIGN_KEY`).
    pub fn with_sovereign_key(mut self, key: Option<impl Into<String>>) -> Self {
        self.sovereign_key = key.map(Into::into);
        self
    }

    /// Builder: set custom protected skill names (replaces default).
    pub fn with_protected_skills(mut self, names: Vec<String>) -> Self {
        self.protected_skills = names;
        self
    }

    /// Returns true if the file stem is a protected skill (vital organ).
    pub fn is_protected(&self, file_stem: &str) -> bool {
        self.protected_skills
            .iter()
            .any(|s| s.as_str() == file_stem)
    }

    /// Evolve an adapter: replace content and enforce integrity. On cargo check failure, rollback.
    /// AccessDenied if the adapter is protected (use evolve_force with Sovereign-Key to override).
    pub fn evolve(&self, rs_path: &Path, new_content: &str) -> Result<(), ForgeError> {
        self.enforce_protection(rs_path)?;
        self.modify_inner(rs_path, new_content)
    }

    /// Evolve with Force override. Bypasses protection only if `sovereign_key` matches the configured key.
    pub fn evolve_force(
        &self,
        rs_path: &Path,
        new_content: &str,
        sovereign_key: &str,
    ) -> Result<(), ForgeError> {
        if !self.allow_force(sovereign_key) {
            self.enforce_protection(rs_path)?;
        }
        self.modify_inner(rs_path, new_content)
    }

    /// Modify an adapter. Triggers mandatory cargo check; on failure, rollback to last stable state.
    /// AccessDenied if the adapter is protected.
    pub fn modify(&self, rs_path: &Path, new_content: &str) -> Result<(), ForgeError> {
        self.enforce_protection(rs_path)?;
        self.modify_inner(rs_path, new_content)
    }

    /// Modify with Force override (Sovereign-Key). Bypasses protection when key matches.
    pub fn modify_force(
        &self,
        rs_path: &Path,
        new_content: &str,
        sovereign_key: &str,
    ) -> Result<(), ForgeError> {
        if !self.allow_force(sovereign_key) {
            self.enforce_protection(rs_path)?;
        }
        self.modify_inner(rs_path, new_content)
    }

    fn enforce_protection(&self, rs_path: &Path) -> Result<(), ForgeError> {
        let stem = rs_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ForgeError::InvalidPath(rs_path.to_path_buf()))?;
        if self.is_protected(stem) {
            return Err(ForgeError::AccessDenied(stem.to_string()));
        }
        Ok(())
    }

    fn allow_force(&self, key: &str) -> bool {
        self.sovereign_key
            .as_ref()
            .map(|k| k.as_str() == key)
            .unwrap_or(false)
    }

    fn modify_inner(&self, rs_path: &Path, new_content: &str) -> Result<(), ForgeError> {
        let path = rs_path.to_path_buf();
        let backup = fs::read_to_string(&path).map_err(|e| ForgeError::Io(path.clone(), e))?;
        fs::write(&path, new_content).map_err(|e| ForgeError::Io(path.clone(), e))?;
        if !self.cargo_check() {
            fs::write(&path, &backup).map_err(|e| ForgeError::Rollback(path.clone(), e))?;
            return Err(ForgeError::CheckFailedRollback);
        }
        Ok(())
    }

    /// Decommission an adapter: archive to KB-03 (Techne) via Scribe, then delete .rs and remove mod entry.
    /// AccessDenied if the adapter is protected (use decommission_force with Sovereign-Key to override).
    pub fn decommission(&self, rs_path: &Path) -> Result<std::path::PathBuf, ForgeError> {
        self.enforce_protection(rs_path)?;
        self.decommission_inner(rs_path)
    }

    /// Decommission with Force override. Bypasses protection only when Sovereign-Key matches.
    pub fn decommission_force(
        &self,
        rs_path: &Path,
        sovereign_key: &str,
    ) -> Result<std::path::PathBuf, ForgeError> {
        if !self.allow_force(sovereign_key) {
            self.enforce_protection(rs_path)?;
        }
        self.decommission_inner(rs_path)
    }

    fn decommission_inner(&self, rs_path: &Path) -> Result<std::path::PathBuf, ForgeError> {
        let rs_path = rs_path.to_path_buf();
        let content =
            fs::read_to_string(&rs_path).map_err(|e| ForgeError::Io(rs_path.clone(), e))?;
        let adapter_name = rs_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| ForgeError::InvalidPath(rs_path.clone()))?;
        let archive_path = self
            .scribe
            .archive_to_kb03(adapter_name, &content)
            .map_err(|e| ForgeError::Scribe(adapter_name.to_string(), e))?;
        self.remove_mod_entry(&rs_path, adapter_name)?;
        fs::remove_file(&rs_path).map_err(|e| ForgeError::Io(rs_path.clone(), e))?;
        Ok(archive_path)
    }

    /// Remove `mod adapter_name;` or `pub mod adapter_name;` from the mod.rs in the same directory as the .rs file.
    fn remove_mod_entry(&self, rs_path: &Path, adapter_name: &str) -> Result<(), ForgeError> {
        let dir = rs_path
            .parent()
            .ok_or_else(|| ForgeError::InvalidPath(rs_path.to_path_buf()))?;
        let mod_rs = dir.join("mod.rs");
        if !mod_rs.exists() {
            return Ok(());
        }
        let content =
            fs::read_to_string(&mod_rs).map_err(|e| ForgeError::Io(mod_rs.clone(), e))?;
        let lines: Vec<&str> = content
            .lines()
            .filter(|line| {
                let t = line.trim();
                t != &format!("mod {};", adapter_name)
                    && t != &format!("pub mod {};", adapter_name)
            })
            .collect();
        let new_content = lines.join("\n");
        if new_content != content {
            fs::write(&mod_rs, new_content).map_err(|e| ForgeError::Io(mod_rs, e))?;
        }
        Ok(())
    }

    /// Run cargo check in workspace root. Returns true if success.
    fn cargo_check(&self) -> bool {
        Command::new("cargo")
            .arg("check")
            .current_dir(&self.workspace_root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[derive(Debug)]
pub enum ForgeError {
    Io(std::path::PathBuf, std::io::Error),
    InvalidPath(std::path::PathBuf),
    Scribe(String, std::io::Error),
    Rollback(std::path::PathBuf, std::io::Error),
    CheckFailedRollback,
    /// Vital organ protected; use Force with Sovereign-Key to override.
    AccessDenied(String),
}

impl std::fmt::Display for ForgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForgeError::Io(p, e) => write!(f, "IO {}: {}", p.display(), e),
            ForgeError::InvalidPath(p) => write!(f, "Invalid path: {}", p.display()),
            ForgeError::Scribe(name, e) => write!(f, "Scribe archive {}: {}", name, e),
            ForgeError::Rollback(p, e) => write!(f, "Rollback {}: {}", p.display(), e),
            ForgeError::CheckFailedRollback => {
                write!(f, "cargo check failed; rolled back to last stable state")
            }
            ForgeError::AccessDenied(name) => {
                write!(f, "Access denied: '{}' is a protected skill (vital organ). Use Force with Sovereign-Key to override.", name)
            }
        }
    }
}

impl std::error::Error for ForgeError {}

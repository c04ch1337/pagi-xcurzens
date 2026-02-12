//! Evolutionary Versioning & Rollback Manager
//!
//! Manages versioned skill patches with atomic symlink swapping and genetic memory.
//!
//! ## Versioned Storage
//!
//! Patches are stored as `{skill_name}_v{timestamp}.rs` in the patches directory.
//! A symlink `current_{skill_name}.rs` always points to the active version.
//! Compiled artifacts follow the same scheme: `{skill_name}_v{timestamp}.dll/.so`
//! with `current_{skill_name}.dll/.so` as the active symlink.
//!
//! ## Atomic Rollback
//!
//! Rollback is performed by atomically swapping the `current_*` symlink to point
//! to a previous version, then reloading the `.dll/.so` via `SkillLoader`.
//! On Windows, symlinks are emulated via file copy (atomic rename).
//!
//! ## Genetic Memory
//!
//! Every applied patch is hashed (SHA-256) and stored as "DNA" in the patch registry.
//! If the agent suggests a fix whose hash matches a previously rejected or rolled-back
//! patch, it is self-censored as an "Evolutionary Dead-End."

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::loader::SkillLoader;
use crate::skill::SkillError;

// ---------------------------------------------------------------------------
// Patch Version Record
// ---------------------------------------------------------------------------

/// Metadata for a single versioned patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchVersion {
    /// Skill name this patch targets.
    pub skill_name: String,
    /// Timestamp (epoch millis) when this version was created.
    pub timestamp_ms: i64,
    /// SHA-256 hash of the source code ("DNA").
    pub code_hash: String,
    /// Path to the `.rs` source file.
    pub source_path: PathBuf,
    /// Path to the compiled `.dll`/`.so` artifact (if compiled).
    pub artifact_path: Option<PathBuf>,
    /// Whether this version is currently active.
    pub is_active: bool,
    /// Status of this patch version.
    pub status: PatchStatus,
    /// Performance delta at the time of application (if measured).
    pub performance_delta: Option<PatchPerformanceDelta>,
    /// Human-readable description of what this patch fixes.
    pub description: String,
}

/// Status of a patch version in the evolutionary timeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatchStatus {
    /// Patch is pending validation.
    Pending,
    /// Patch passed validation and is applied.
    Applied,
    /// Patch was rolled back (reverted).
    RolledBack,
    /// Patch was rejected (compilation failure, smoke test crash, or operator decline).
    Rejected,
    /// Patch was auto-rejected as a "Syntactic Hallucination."
    SyntacticHallucination,
    /// Patch was rejected by Red-Team peer review (High severity).
    RedTeamRejected,
    /// Patch was rejected as a "Lethal Mutation" (Critical severity from Red-Team).
    LethalMutation,
}

/// Performance metrics captured when a patch was validated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchPerformanceDelta {
    pub cpu: String,
    pub mem: String,
    pub compiled: bool,
    pub smoke_test_passed: bool,
    /// Red-Team peer review result (Phase 4.75).
    /// `None` if peer review was not performed.
    #[serde(default)]
    pub security_audit: Option<SecurityAuditSummary>,
}

/// Summary of the Red-Team security audit, embedded in the PerformanceDelta
/// for telemetry correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditSummary {
    /// Whether the patch passed peer review.
    pub passed: bool,
    /// Overall severity from the review.
    pub overall_severity: String,
    /// The model that performed the review.
    pub reviewer_model: String,
    /// Number of findings.
    pub findings_count: usize,
    /// Human-readable summary.
    pub summary: String,
    /// Memory usage warning from the reviewer (if any).
    pub memory_warning: Option<String>,
}

// ---------------------------------------------------------------------------
// Genetic Memory: Dead-End Registry
// ---------------------------------------------------------------------------

/// Tracks the "DNA" (code hashes) of all patches to detect evolutionary dead-ends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneticMemory {
    /// Map of code_hash → DeadEndRecord for rejected/rolled-back patches.
    dead_ends: HashMap<String, DeadEndRecord>,
    /// Map of code_hash → skill_name for all known patches (including successful ones).
    known_dna: HashMap<String, String>,
}

/// Record of a dead-end patch (rejected or rolled back).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadEndRecord {
    /// The code hash that was rejected.
    pub code_hash: String,
    /// Skill name this was targeting.
    pub skill_name: String,
    /// Reason for rejection/rollback.
    pub reason: String,
    /// Timestamp when it was marked as dead-end.
    pub timestamp_ms: i64,
    /// How many times this hash has been suggested (for tracking recurrence).
    pub occurrence_count: u32,
}

impl GeneticMemory {
    pub fn new() -> Self {
        Self {
            dead_ends: HashMap::new(),
            known_dna: HashMap::new(),
        }
    }

    /// Register a patch's DNA (code hash) as known.
    pub fn register_dna(&mut self, code_hash: &str, skill_name: &str) {
        self.known_dna
            .insert(code_hash.to_string(), skill_name.to_string());
    }

    /// Mark a code hash as a dead-end (rejected or rolled back).
    pub fn mark_dead_end(&mut self, code_hash: &str, skill_name: &str, reason: &str) {
        let entry = self
            .dead_ends
            .entry(code_hash.to_string())
            .or_insert_with(|| DeadEndRecord {
                code_hash: code_hash.to_string(),
                skill_name: skill_name.to_string(),
                reason: reason.to_string(),
                timestamp_ms: now_epoch_ms(),
                occurrence_count: 0,
            });
        entry.occurrence_count += 1;
        entry.reason = reason.to_string();
        entry.timestamp_ms = now_epoch_ms();
    }

    /// Check if a code hash is a known dead-end. Returns the record if so.
    pub fn is_dead_end(&self, code_hash: &str) -> Option<&DeadEndRecord> {
        self.dead_ends.get(code_hash)
    }

    /// Returns all dead-end records.
    pub fn all_dead_ends(&self) -> Vec<&DeadEndRecord> {
        self.dead_ends.values().collect()
    }

    /// Returns the total number of known DNA entries.
    pub fn known_count(&self) -> usize {
        self.known_dna.len()
    }

    /// Returns the total number of dead-end entries.
    pub fn dead_end_count(&self) -> usize {
        self.dead_ends.len()
    }
}

impl Default for GeneticMemory {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Rollback Manager
// ---------------------------------------------------------------------------

/// Configuration for the RollbackManager.
#[derive(Debug, Clone)]
pub struct RollbackConfig {
    /// Directory where versioned patch source files are stored.
    pub patches_dir: PathBuf,
    /// Directory where compiled artifacts (.dll/.so) are stored.
    pub artifacts_dir: PathBuf,
    /// Maximum number of versions to keep per skill (0 = unlimited).
    pub max_versions_per_skill: usize,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            patches_dir: PathBuf::from("crates/pagi-skills/src/generated/patches"),
            artifacts_dir: PathBuf::from("data/pagi_evolution"),
            max_versions_per_skill: 50,
        }
    }
}

/// The Rollback Manager: handles versioned patch storage, atomic symlink swapping,
/// and genetic memory for evolutionary dead-end detection.
///
/// Thread-safe via internal `RwLock`.
pub struct RollbackManager {
    config: RollbackConfig,
    /// All known patch versions, keyed by skill_name → Vec<PatchVersion> (ordered by timestamp).
    versions: RwLock<HashMap<String, Vec<PatchVersion>>>,
    /// Genetic memory: tracks code hashes for dead-end detection.
    genetic_memory: RwLock<GeneticMemory>,
    /// Reference to the SkillLoader for hot-reloading after rollback.
    skill_loader: Arc<SkillLoader>,
}

impl RollbackManager {
    /// Create a new RollbackManager with the given configuration and skill loader.
    pub fn new(config: RollbackConfig, skill_loader: Arc<SkillLoader>) -> Self {
        let manager = Self {
            config,
            versions: RwLock::new(HashMap::new()),
            genetic_memory: RwLock::new(GeneticMemory::new()),
            skill_loader,
        };

        // Scan existing patches directory to rebuild version history.
        if let Err(e) = manager.scan_existing_patches() {
            warn!(
                target: "pagi::rollback",
                error = %e,
                "Failed to scan existing patches on startup"
            );
        }

        manager
    }

    /// Create with default configuration.
    pub fn with_defaults(skill_loader: Arc<SkillLoader>) -> Self {
        Self::new(RollbackConfig::default(), skill_loader)
    }

    // -----------------------------------------------------------------------
    // Versioned Storage
    // -----------------------------------------------------------------------

    /// Save a new versioned patch. Returns the PatchVersion record.
    ///
    /// This:
    /// 1. Computes the SHA-256 hash of the code.
    /// 2. Checks genetic memory for dead-ends.
    /// 3. Writes `{skill_name}_v{timestamp}.rs` to the patches directory.
    /// 4. Updates the `current_{skill_name}.rs` symlink.
    /// 5. Registers the DNA in genetic memory.
    pub fn save_versioned_patch(
        &self,
        skill_name: &str,
        code: &str,
        description: &str,
        performance_delta: Option<PatchPerformanceDelta>,
    ) -> Result<PatchVersion, SkillError> {
        let code_hash = compute_sha256(code);

        // Check genetic memory for dead-ends.
        {
            let gm = self
                .genetic_memory
                .read()
                .map_err(|e| SkillError::Load(format!("Genetic memory lock: {}", e)))?;
            if let Some(dead_end) = gm.is_dead_end(&code_hash) {
                return Err(SkillError::Load(format!(
                    "Evolutionary Dead-End: This patch (hash {}) was previously rejected for '{}'. \
                     Reason: {}. Occurrences: {}.",
                    &code_hash[..12],
                    dead_end.skill_name,
                    dead_end.reason,
                    dead_end.occurrence_count
                )));
            }
        }

        // Create patches directory if needed.
        std::fs::create_dir_all(&self.config.patches_dir)
            .map_err(|e| SkillError::Load(format!("Failed to create patches dir: {}", e)))?;

        let timestamp = now_epoch_ms();
        let sanitized = sanitize_name(skill_name);
        let filename = format!("{}_v{}.rs", sanitized, timestamp);
        let source_path = self.config.patches_dir.join(&filename);

        // Write the versioned source file.
        std::fs::write(&source_path, code)
            .map_err(|e| SkillError::Load(format!("Failed to write patch: {}", e)))?;

        // Update the current symlink.
        let current_link = self
            .config
            .patches_dir
            .join(format!("current_{}.rs", sanitized));
        atomic_symlink_update(&source_path, &current_link)?;

        let version = PatchVersion {
            skill_name: skill_name.to_string(),
            timestamp_ms: timestamp,
            code_hash: code_hash.clone(),
            source_path: source_path.clone(),
            artifact_path: None,
            is_active: true,
            status: PatchStatus::Applied,
            performance_delta,
            description: description.to_string(),
        };

        // Update version registry.
        {
            let mut versions = self
                .versions
                .write()
                .map_err(|e| SkillError::Load(format!("Version lock: {}", e)))?;
            let skill_versions = versions.entry(skill_name.to_string()).or_default();

            // Deactivate all previous versions.
            for v in skill_versions.iter_mut() {
                v.is_active = false;
            }

            skill_versions.push(version.clone());

            // Enforce max versions limit.
            if self.config.max_versions_per_skill > 0
                && skill_versions.len() > self.config.max_versions_per_skill
            {
                let excess = skill_versions.len() - self.config.max_versions_per_skill;
                // Remove oldest non-active versions.
                let mut removed = 0;
                skill_versions.retain(|v| {
                    if removed >= excess || v.is_active {
                        true
                    } else {
                        // Clean up the source file.
                        let _ = std::fs::remove_file(&v.source_path);
                        if let Some(ref artifact) = v.artifact_path {
                            let _ = std::fs::remove_file(artifact);
                        }
                        removed += 1;
                        false
                    }
                });
            }
        }

        // Register DNA in genetic memory.
        {
            let mut gm = self
                .genetic_memory
                .write()
                .map_err(|e| SkillError::Load(format!("Genetic memory lock: {}", e)))?;
            gm.register_dna(&code_hash, skill_name);
        }

        info!(
            target: "pagi::rollback",
            skill = skill_name,
            hash = &code_hash[..12],
            path = %source_path.display(),
            "Versioned patch saved"
        );

        Ok(version)
    }

    /// Register a compiled artifact for a patch version.
    pub fn register_artifact(
        &self,
        skill_name: &str,
        timestamp_ms: i64,
        artifact_path: PathBuf,
    ) -> Result<(), SkillError> {
        let mut versions = self
            .versions
            .write()
            .map_err(|e| SkillError::Load(format!("Version lock: {}", e)))?;

        if let Some(skill_versions) = versions.get_mut(skill_name) {
            if let Some(version) = skill_versions
                .iter_mut()
                .find(|v| v.timestamp_ms == timestamp_ms)
            {
                // Update the current artifact symlink.
                let sanitized = sanitize_name(skill_name);
                let ext = if cfg!(target_os = "windows") {
                    "dll"
                } else {
                    "so"
                };
                let current_artifact_link = self
                    .config
                    .artifacts_dir
                    .join(format!("current_{}.{}", sanitized, ext));

                std::fs::create_dir_all(&self.config.artifacts_dir)
                    .map_err(|e| SkillError::Load(format!("Failed to create artifacts dir: {}", e)))?;

                atomic_symlink_update(&artifact_path, &current_artifact_link)?;
                version.artifact_path = Some(artifact_path);

                info!(
                    target: "pagi::rollback",
                    skill = skill_name,
                    timestamp = timestamp_ms,
                    "Artifact registered and symlink updated"
                );
                return Ok(());
            }
        }

        Err(SkillError::Load(format!(
            "No version found for skill '{}' at timestamp {}",
            skill_name, timestamp_ms
        )))
    }

    // -----------------------------------------------------------------------
    // Rollback
    // -----------------------------------------------------------------------

    /// Roll back a skill to a previous version.
    ///
    /// This:
    /// 1. Finds the previous version for the skill.
    /// 2. Atomically swaps the `current_*` symlink to point to it.
    /// 3. Reloads the `.dll/.so` via SkillLoader (if artifact exists).
    /// 4. Marks the current version as "RolledBack" and the target as "Applied".
    /// 5. Records the rolled-back version's hash as a dead-end in genetic memory.
    ///
    /// If `target_timestamp` is None, rolls back to the most recent non-active version.
    pub fn rollback_skill(
        &self,
        skill_name: &str,
        target_timestamp: Option<i64>,
        reason: &str,
    ) -> Result<PatchVersion, SkillError> {
        let mut versions = self
            .versions
            .write()
            .map_err(|e| SkillError::Load(format!("Version lock: {}", e)))?;

        let skill_versions = versions.get_mut(skill_name).ok_or_else(|| {
            SkillError::Load(format!("No versions found for skill '{}'", skill_name))
        })?;

        if skill_versions.len() < 2 {
            return Err(SkillError::Load(format!(
                "Cannot rollback '{}': only {} version(s) available (need at least 2)",
                skill_name,
                skill_versions.len()
            )));
        }

        // Find the currently active version.
        let active_idx = skill_versions
            .iter()
            .position(|v| v.is_active)
            .ok_or_else(|| {
                SkillError::Load(format!("No active version found for skill '{}'", skill_name))
            })?;

        // Find the target version.
        let target_idx = if let Some(ts) = target_timestamp {
            skill_versions
                .iter()
                .position(|v| v.timestamp_ms == ts)
                .ok_or_else(|| {
                    SkillError::Load(format!(
                        "No version found for skill '{}' at timestamp {}",
                        skill_name, ts
                    ))
                })?
        } else {
            // Default: roll back to the version just before the active one.
            if active_idx == 0 {
                return Err(SkillError::Load(format!(
                    "Cannot rollback '{}': active version is the oldest",
                    skill_name
                )));
            }
            active_idx - 1
        };

        if target_idx == active_idx {
            return Err(SkillError::Load(format!(
                "Target version is already active for skill '{}'",
                skill_name
            )));
        }

        // Get the hash of the version being rolled back (for dead-end marking).
        let rolled_back_hash = skill_versions[active_idx].code_hash.clone();

        // Swap: deactivate current, activate target.
        skill_versions[active_idx].is_active = false;
        skill_versions[active_idx].status = PatchStatus::RolledBack;
        skill_versions[target_idx].is_active = true;
        skill_versions[target_idx].status = PatchStatus::Applied;

        let target_version = skill_versions[target_idx].clone();

        // Update source symlink.
        let sanitized = sanitize_name(skill_name);
        let current_source_link = self
            .config
            .patches_dir
            .join(format!("current_{}.rs", sanitized));
        atomic_symlink_update(&target_version.source_path, &current_source_link)?;

        // Update artifact symlink and reload if artifact exists.
        if let Some(ref artifact_path) = target_version.artifact_path {
            if artifact_path.exists() {
                let ext = if cfg!(target_os = "windows") {
                    "dll"
                } else {
                    "so"
                };
                let current_artifact_link = self
                    .config
                    .artifacts_dir
                    .join(format!("current_{}.{}", sanitized, ext));
                atomic_symlink_update(artifact_path, &current_artifact_link)?;

                // Hot-reload the previous version's library.
                if let Err(e) = self
                    .skill_loader
                    .load(artifact_path, skill_name.to_string())
                {
                    warn!(
                        target: "pagi::rollback",
                        skill = skill_name,
                        error = %e,
                        "Failed to hot-reload rolled-back artifact (skill may need manual reload)"
                    );
                } else {
                    info!(
                        target: "pagi::rollback",
                        skill = skill_name,
                        artifact = %artifact_path.display(),
                        "Hot-reloaded rolled-back artifact"
                    );
                }
            }
        }

        info!(
            target: "pagi::rollback",
            skill = skill_name,
            from_ts = skill_versions[active_idx].timestamp_ms,
            to_ts = target_version.timestamp_ms,
            reason = reason,
            "Skill rolled back successfully"
        );

        // Drop the write lock before acquiring genetic memory lock.
        drop(versions);

        // Mark the rolled-back version's hash as a dead-end.
        {
            let mut gm = self
                .genetic_memory
                .write()
                .map_err(|e| SkillError::Load(format!("Genetic memory lock: {}", e)))?;
            gm.mark_dead_end(
                &rolled_back_hash,
                skill_name,
                &format!("Rolled back: {}", reason),
            );
        }

        Ok(target_version)
    }

    // -----------------------------------------------------------------------
    // Genetic Memory: Dead-End Detection
    // -----------------------------------------------------------------------

    /// Check if a code string is a known evolutionary dead-end.
    /// Returns the DeadEndRecord if the hash matches a previously rejected/rolled-back patch.
    pub fn check_dead_end(&self, code: &str) -> Option<DeadEndRecord> {
        let hash = compute_sha256(code);
        let gm = self.genetic_memory.read().ok()?;
        gm.is_dead_end(&hash).cloned()
    }

    /// Mark a code hash as a dead-end (e.g., after operator rejection or auto-rejection).
    pub fn mark_dead_end(
        &self,
        code: &str,
        skill_name: &str,
        reason: &str,
    ) -> Result<(), SkillError> {
        let hash = compute_sha256(code);
        let mut gm = self
            .genetic_memory
            .write()
            .map_err(|e| SkillError::Load(format!("Genetic memory lock: {}", e)))?;
        gm.mark_dead_end(&hash, skill_name, reason);
        info!(
            target: "pagi::rollback::genetic",
            hash = &hash[..12],
            skill = skill_name,
            reason = reason,
            "Marked as evolutionary dead-end"
        );
        Ok(())
    }

    /// Mark a patch version as rejected (updates status and genetic memory).
    pub fn mark_rejected(
        &self,
        skill_name: &str,
        code: &str,
        reason: &str,
        is_hallucination: bool,
    ) -> Result<(), SkillError> {
        let hash = compute_sha256(code);

        // Update version status if it exists.
        if let Ok(mut versions) = self.versions.write() {
            if let Some(skill_versions) = versions.get_mut(skill_name) {
                for v in skill_versions.iter_mut() {
                    if v.code_hash == hash {
                        v.status = if is_hallucination {
                            PatchStatus::SyntacticHallucination
                        } else {
                            PatchStatus::Rejected
                        };
                        v.is_active = false;
                    }
                }
            }
        }

        // Mark in genetic memory.
        self.mark_dead_end(code, skill_name, reason)
    }

    // -----------------------------------------------------------------------
    // Query API
    // -----------------------------------------------------------------------

    /// Get all versions for a skill, ordered by timestamp (oldest first).
    pub fn get_versions(&self, skill_name: &str) -> Vec<PatchVersion> {
        self.versions
            .read()
            .ok()
            .and_then(|v| v.get(skill_name).cloned())
            .unwrap_or_default()
    }

    /// Get the currently active version for a skill.
    pub fn get_active_version(&self, skill_name: &str) -> Option<PatchVersion> {
        self.versions
            .read()
            .ok()
            .and_then(|v| {
                v.get(skill_name)
                    .and_then(|versions| versions.iter().find(|v| v.is_active).cloned())
            })
    }

    /// Get all skills that have versioned patches.
    pub fn get_versioned_skills(&self) -> Vec<String> {
        self.versions
            .read()
            .ok()
            .map(|v| v.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the full patch history across all skills, sorted by timestamp (newest first).
    pub fn get_full_history(&self) -> Vec<PatchVersion> {
        let mut all: Vec<PatchVersion> = self
            .versions
            .read()
            .ok()
            .map(|v| v.values().flatten().cloned().collect())
            .unwrap_or_default();
        all.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
        all
    }

    /// Get genetic memory statistics.
    pub fn genetic_memory_stats(&self) -> (usize, usize) {
        self.genetic_memory
            .read()
            .ok()
            .map(|gm| (gm.known_count(), gm.dead_end_count()))
            .unwrap_or((0, 0))
    }

    /// Get all dead-end records.
    pub fn get_dead_ends(&self) -> Vec<DeadEndRecord> {
        self.genetic_memory
            .read()
            .ok()
            .map(|gm| gm.all_dead_ends().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    // -----------------------------------------------------------------------
    // Internal: Scan existing patches
    // -----------------------------------------------------------------------

    /// Scan the patches directory for existing versioned files and rebuild the registry.
    fn scan_existing_patches(&self) -> Result<(), SkillError> {
        let patches_dir = &self.config.patches_dir;
        if !patches_dir.exists() {
            debug!(
                target: "pagi::rollback",
                path = %patches_dir.display(),
                "Patches directory does not exist yet — nothing to scan"
            );
            return Ok(());
        }

        let entries = std::fs::read_dir(patches_dir)
            .map_err(|e| SkillError::Load(format!("Failed to read patches dir: {}", e)))?;

        let mut versions = self
            .versions
            .write()
            .map_err(|e| SkillError::Load(format!("Version lock: {}", e)))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let filename = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Skip current_* symlinks/copies.
            if filename.starts_with("current_") {
                continue;
            }

            // Parse versioned filename: {skill_name}_v{timestamp}.rs
            if let Some(parsed) = parse_versioned_filename(&filename) {
                let (skill_name, timestamp) = parsed;

                // Read the source to compute hash.
                let code = std::fs::read_to_string(&path).unwrap_or_default();
                let code_hash = compute_sha256(&code);

                // Check if this is the current active version.
                let sanitized = sanitize_name(&skill_name);
                let current_link = patches_dir.join(format!("current_{}.rs", sanitized));
                let is_active = if current_link.exists() {
                    // Read the current link target and compare.
                    match std::fs::read_to_string(&current_link) {
                        Ok(current_code) => compute_sha256(&current_code) == code_hash,
                        Err(_) => false,
                    }
                } else {
                    false
                };

                let version = PatchVersion {
                    skill_name: skill_name.clone(),
                    timestamp_ms: timestamp,
                    code_hash: code_hash.clone(),
                    source_path: path.clone(),
                    artifact_path: None,
                    is_active,
                    status: if is_active {
                        PatchStatus::Applied
                    } else {
                        PatchStatus::Applied // Historical — we don't know the original status.
                    },
                    performance_delta: None,
                    description: format!("Scanned from existing file: {}", filename),
                };

                let skill_versions = versions.entry(skill_name).or_default();
                skill_versions.push(version);

                // Register DNA.
                if let Ok(mut gm) = self.genetic_memory.write() {
                    gm.register_dna(&code_hash, &skill_versions.last().unwrap().skill_name);
                }
            }
        }

        // Sort each skill's versions by timestamp.
        for skill_versions in versions.values_mut() {
            skill_versions.sort_by_key(|v| v.timestamp_ms);
        }

        let total_skills = versions.len();
        let total_versions: usize = versions.values().map(|v| v.len()).sum();
        info!(
            target: "pagi::rollback",
            skills = total_skills,
            versions = total_versions,
            "Scanned existing patches"
        );

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Atomic Symlink Update (cross-platform)
// ---------------------------------------------------------------------------

/// Atomically update a symlink (or file copy on Windows) to point to a new target.
///
/// On Unix: Uses `symlink` + `rename` for atomic swap.
/// On Windows: Uses file copy + atomic rename (true symlinks require elevated privileges).
fn atomic_symlink_update(target: &Path, link_path: &Path) -> Result<(), SkillError> {
    // Ensure parent directory exists.
    if let Some(parent) = link_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| SkillError::Load(format!("Failed to create parent dir: {}", e)))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        // Create a temporary symlink, then atomically rename it.
        let temp_link = link_path.with_extension("tmp_symlink");
        let _ = std::fs::remove_file(&temp_link);

        symlink(target, &temp_link)
            .map_err(|e| SkillError::Load(format!("Failed to create symlink: {}", e)))?;

        std::fs::rename(&temp_link, link_path)
            .map_err(|e| SkillError::Load(format!("Failed to rename symlink: {}", e)))?;
    }

    #[cfg(windows)]
    {
        // On Windows, use file copy + atomic rename instead of symlinks
        // (symlinks require elevated privileges or developer mode).
        let temp_copy = link_path.with_extension("tmp_copy");
        let _ = std::fs::remove_file(&temp_copy);

        std::fs::copy(target, &temp_copy)
            .map_err(|e| SkillError::Load(format!("Failed to copy file: {}", e)))?;

        // Atomic rename: remove old, rename temp.
        let _ = std::fs::remove_file(link_path);
        std::fs::rename(&temp_copy, link_path)
            .map_err(|e| SkillError::Load(format!("Failed to rename file: {}", e)))?;
    }

    debug!(
        target: "pagi::rollback",
        target = %target.display(),
        link = %link_path.display(),
        "Symlink updated"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute SHA-256 hash of a string, returning a hex-encoded string.
fn compute_sha256(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Use a simple hash for now (SHA-256 would require a crypto dependency).
    // For production, replace with `sha2` crate.
    // We use a double-hash approach for better distribution.
    let mut hasher1 = DefaultHasher::new();
    input.hash(&mut hasher1);
    let h1 = hasher1.finish();

    let mut hasher2 = DefaultHasher::new();
    format!("{}{}", input, h1).hash(&mut hasher2);
    let h2 = hasher2.finish();

    format!("{:016x}{:016x}", h1, h2)
}

/// Sanitize a skill name for use in filenames.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

/// Parse a versioned filename like `{skill_name}_v{timestamp}.rs`.
/// Returns `(skill_name, timestamp)` if successful.
fn parse_versioned_filename(filename: &str) -> Option<(String, i64)> {
    let stem = filename.strip_suffix(".rs")?;

    // Find the last `_v` followed by digits.
    let v_pos = stem.rfind("_v")?;
    let timestamp_str = &stem[v_pos + 2..];
    let timestamp: i64 = timestamp_str.parse().ok()?;
    let skill_name = stem[..v_pos].to_string();

    if skill_name.is_empty() {
        return None;
    }

    Some((skill_name, timestamp))
}

/// Current epoch milliseconds.
fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let h1 = compute_sha256("hello world");
        let h2 = compute_sha256("hello world");
        let h3 = compute_sha256("hello world!");
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert_eq!(h1.len(), 32); // 16 hex chars * 2
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("FileSystemSkill"), "filesystemskill");
        assert_eq!(sanitize_name("my-skill.v2"), "my_skill_v2");
        assert_eq!(sanitize_name("hello world!"), "hello_world_");
    }

    #[test]
    fn test_parse_versioned_filename() {
        let (name, ts) = parse_versioned_filename("patch_fs_tools_v1707307200000.rs").unwrap();
        assert_eq!(name, "patch_fs_tools");
        assert_eq!(ts, 1707307200000);

        assert!(parse_versioned_filename("current_fs_tools.rs").is_none());
        assert!(parse_versioned_filename("not_versioned.rs").is_none());
        assert!(parse_versioned_filename("_v123.rs").is_none());
    }

    #[test]
    fn test_genetic_memory() {
        let mut gm = GeneticMemory::new();

        gm.register_dna("hash1", "skill_a");
        assert_eq!(gm.known_count(), 1);
        assert_eq!(gm.dead_end_count(), 0);

        gm.mark_dead_end("hash1", "skill_a", "compilation failed");
        assert_eq!(gm.dead_end_count(), 1);

        let dead = gm.is_dead_end("hash1").unwrap();
        assert_eq!(dead.occurrence_count, 1);
        assert_eq!(dead.reason, "compilation failed");

        // Mark again — occurrence count should increase.
        gm.mark_dead_end("hash1", "skill_a", "still broken");
        let dead = gm.is_dead_end("hash1").unwrap();
        assert_eq!(dead.occurrence_count, 2);
        assert_eq!(dead.reason, "still broken");

        assert!(gm.is_dead_end("hash_unknown").is_none());
    }

    #[test]
    fn test_rollback_manager_save_and_query() {
        let loader = Arc::new(SkillLoader::new());
        let temp_dir = tempfile::tempdir().unwrap();
        let config = RollbackConfig {
            patches_dir: temp_dir.path().join("patches"),
            artifacts_dir: temp_dir.path().join("artifacts"),
            max_versions_per_skill: 10,
        };

        let manager = RollbackManager::new(config, loader);

        // Save first version.
        let v1 = manager
            .save_versioned_patch("test_skill", "fn v1() {}", "First version", None)
            .unwrap();
        assert!(v1.is_active);
        assert_eq!(v1.status, PatchStatus::Applied);

        // Save second version.
        let v2 = manager
            .save_versioned_patch("test_skill", "fn v2() {}", "Second version", None)
            .unwrap();
        assert!(v2.is_active);

        // Query versions.
        let versions = manager.get_versions("test_skill");
        assert_eq!(versions.len(), 2);
        assert!(!versions[0].is_active); // v1 deactivated
        assert!(versions[1].is_active); // v2 active

        // Active version should be v2.
        let active = manager.get_active_version("test_skill").unwrap();
        assert_eq!(active.timestamp_ms, v2.timestamp_ms);
    }

    #[test]
    fn test_rollback_manager_rollback() {
        let loader = Arc::new(SkillLoader::new());
        let temp_dir = tempfile::tempdir().unwrap();
        let config = RollbackConfig {
            patches_dir: temp_dir.path().join("patches"),
            artifacts_dir: temp_dir.path().join("artifacts"),
            max_versions_per_skill: 10,
        };

        let manager = RollbackManager::new(config, loader);

        // Save two versions.
        let _v1 = manager
            .save_versioned_patch("test_skill", "fn v1() {}", "First", None)
            .unwrap();
        let _v2 = manager
            .save_versioned_patch("test_skill", "fn v2() {}", "Second", None)
            .unwrap();

        // Rollback to v1.
        let rolled_back = manager
            .rollback_skill("test_skill", None, "Testing rollback")
            .unwrap();
        assert!(rolled_back.is_active);

        // v2's hash should now be a dead-end.
        let dead_end = manager.check_dead_end("fn v2() {}");
        assert!(dead_end.is_some());
        assert!(dead_end.unwrap().reason.contains("Rolled back"));
    }

    #[test]
    fn test_dead_end_blocks_save() {
        let loader = Arc::new(SkillLoader::new());
        let temp_dir = tempfile::tempdir().unwrap();
        let config = RollbackConfig {
            patches_dir: temp_dir.path().join("patches"),
            artifacts_dir: temp_dir.path().join("artifacts"),
            max_versions_per_skill: 10,
        };

        let manager = RollbackManager::new(config, loader);

        // Mark some code as a dead-end.
        manager
            .mark_dead_end("fn broken() {}", "test_skill", "always crashes")
            .unwrap();

        // Trying to save the same code should fail.
        let result =
            manager.save_versioned_patch("test_skill", "fn broken() {}", "Retry", None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Evolutionary Dead-End"));
    }

    #[test]
    fn test_full_history() {
        let loader = Arc::new(SkillLoader::new());
        let temp_dir = tempfile::tempdir().unwrap();
        let config = RollbackConfig {
            patches_dir: temp_dir.path().join("patches"),
            artifacts_dir: temp_dir.path().join("artifacts"),
            max_versions_per_skill: 10,
        };

        let manager = RollbackManager::new(config, loader);

        manager
            .save_versioned_patch("skill_a", "fn a1() {}", "A v1", None)
            .unwrap();
        manager
            .save_versioned_patch("skill_b", "fn b1() {}", "B v1", None)
            .unwrap();
        manager
            .save_versioned_patch("skill_a", "fn a2() {}", "A v2", None)
            .unwrap();

        let history = manager.get_full_history();
        assert_eq!(history.len(), 3);
        // Newest first.
        assert!(history[0].timestamp_ms >= history[1].timestamp_ms);
    }
}

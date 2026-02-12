//! 3-Tier Skills Infrastructure: manifest schema, directory scanner, and permission interceptor.
//!
//! - **Tier 1 (core):** Human-authored, local. Only core may access KB-01 (Soul) and KB-09 (Shadow/PII).
//! - **Tier 2 (import):** Community patterns, quarantined. No KB-01 or KB-09.
//! - **Tier 3 (generated):** Orchestrator-generated (ephemeral). No KB-01 or KB-09.
//!
//! The Orchestrator should call `validate_skill_permissions` before executing a skill that touches a KB layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

/// Trust tier for the 3-tier model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustTier {
    /// Tier 1: Human-authored, local. May access KB-01 and KB-09.
    Core,
    /// Tier 2: Community patterns, quarantined. No KB-01 or KB-09.
    Import,
    /// Tier 3: Orchestrator-generated (ephemeral). No KB-01 or KB-09.
    Generated,
}

impl TrustTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustTier::Core => "core",
            TrustTier::Import => "import",
            TrustTier::Generated => "generated",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "core" => TrustTier::Core,
            "import" => TrustTier::Import,
            "generated" => TrustTier::Generated,
            _ => TrustTier::Import,
        }
    }
}

/// Single skill entry in a tier manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifestEntry {
    pub skill_id: String,
    pub kb_layers_allowed: Vec<u8>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Per-tier manifest (e.g. core/manifest.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierManifest {
    #[serde(default)]
    pub trust_tier: String,
    pub skills: Vec<SkillManifestEntry>,
    #[serde(default)]
    pub description: Option<String>,
}

/// In-memory entry for a loaded skill (tier + allowed KB layers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInventoryEntry {
    pub skill_id: String,
    pub trust_tier: String,
    pub kb_layers_allowed: Vec<u8>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Registry that scans `core/`, `import/`, `ephemeral/` and loads manifest.json from each.
/// Provides permission validation: only `core` may access KB-01 or KB-09.
#[derive(Debug, Default)]
pub struct SkillManifestRegistry {
    /// skill_id -> (TrustTier, kb_layers_allowed)
    index: RwLock<HashMap<String, (TrustTier, Vec<u8>)>>,
    /// Ordered list for API (GET /api/v1/skills).
    inventory: RwLock<Vec<SkillInventoryEntry>>,
}

impl SkillManifestRegistry {
    pub fn new() -> Self {
        Self {
            index: RwLock::new(HashMap::new()),
            inventory: RwLock::new(Vec::new()),
        }
    }

    /// Load manifests from the three tier directories under `skills_root`.
    /// `skills_root` should be the path to `crates/pagi-skills` (or equivalent).
    pub fn load_from_dir(skills_root: &Path) -> std::io::Result<Self> {
        let mut index = HashMap::new();
        let mut inventory = Vec::new();

        for (dir_name, _) in [
            ("core", TrustTier::Core),
            ("import", TrustTier::Import),
            ("ephemeral", TrustTier::Generated),
        ] {
            let manifest_path = skills_root.join(dir_name).join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }
            let bytes = std::fs::read(&manifest_path)
                .map_err(|e| std::io::Error::new(e.kind(), format!("{}: {}", manifest_path.display(), e)))?;
            let manifest: TierManifest = serde_json::from_slice(&bytes).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {}", manifest_path.display(), e))
            })?;
            let tier = TrustTier::from_str(&manifest.trust_tier);
            for entry in manifest.skills {
                let layers = entry.kb_layers_allowed.clone();
                index.insert(entry.skill_id.clone(), (tier, layers.clone()));
                inventory.push(SkillInventoryEntry {
                    skill_id: entry.skill_id,
                    trust_tier: tier.as_str().to_string(),
                    kb_layers_allowed: layers,
                    description: entry.description,
                });
            }
        }

        Ok(Self {
            index: RwLock::new(index),
            inventory: RwLock::new(inventory),
        })
    }

    /// Returns whether the given skill is allowed to access the given KB layer (1..=9).
    /// - When `strict_mode` is true: only Core (Tier 1) may touch any KB layer; Tier 2/3 are blocked for all KBs.
    /// - When `strict_mode` is false: only Core may access KB-01 or KB-09; Tier 2 may touch 2..=8 per manifest.
    /// - The skill must be in the registry and list the layer in `kb_layers_allowed`.
    pub fn validate_skill_permissions(
        &self,
        skill_id: &str,
        target_kb_layer: u8,
        strict_mode: bool,
    ) -> bool {
        if target_kb_layer == 0 || target_kb_layer > 9 {
            return false;
        }
        let guard = match self.index.read() {
            Ok(g) => g,
            Err(_) => return false,
        };
        let Some((tier, allowed)) = guard.get(skill_id) else {
            return false;
        };
        if !allowed.contains(&target_kb_layer) {
            return false;
        }
        if strict_mode {
            return *tier == TrustTier::Core;
        }
        // Restriction: only core may access KB-01 (Soul) or KB-09 (Shadow/PII).
        if target_kb_layer == 1 || target_kb_layer == 9 {
            return *tier == TrustTier::Core;
        }
        true
    }

    /// List all skills with trust status (for GET /api/v1/skills).
    pub fn list_inventory(&self) -> Vec<SkillInventoryEntry> {
        self.inventory
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Promote a skill from ephemeral to core by moving its manifest entry.
    /// Updates the in-memory registry; caller may persist to disk by writing back manifest files.
    /// Returns true if the skill was found in generated tier and moved to core.
    pub fn promote_to_core(&self, skill_id: &str) -> bool {
        let layers = {
            let idx_guard = match self.index.read() {
                Ok(g) => g,
                Err(_) => return false,
            };
            let (tier, layers) = match idx_guard.get(skill_id) {
                Some((t, l)) => (*t, l.clone()),
                None => return false,
            };
            if tier != TrustTier::Generated {
                return false;
            }
            layers
        };
        {
            let mut idx_guard = match self.index.write() {
                Ok(g) => g,
                Err(_) => return false,
            };
            idx_guard.insert(skill_id.to_string(), (TrustTier::Core, layers));
        }
        if let Ok(mut inv_guard) = self.inventory.write() {
            if let Some(entry) = inv_guard.iter_mut().find(|e| e.skill_id == skill_id) {
                entry.trust_tier = "core".to_string();
            }
        }
        true
    }
}

/// Validates skill access to a KB layer. Use from Orchestrator before dispatching a skill
/// that will touch a given KB layer (e.g. before ExecuteSkill or QueryKnowledge).
/// When `strict_mode` is true, only Core (Tier 1) skills may access any KB layer.
pub fn validate_skill_permissions(
    registry: &SkillManifestRegistry,
    skill_id: &str,
    target_kb_layer: u8,
    strict_mode: bool,
) -> bool {
    registry.validate_skill_permissions(skill_id, target_kb_layer, strict_mode)
}

/// Error returned when a skill attempts to access a KB layer it is not allowed to touch
/// (e.g. Tier 3 generated skill touching KB-01 or KB-09). Logged in KB-08 as "Failed Leak Attempt".
#[derive(Debug, Clone)]
pub struct SovereigntyViolation {
    pub skill_id: String,
    pub kb_layer: u8,
}

impl std::fmt::Display for SovereigntyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SovereigntyViolation: skill '{}' is not allowed to access KB-{} (Sovereignty Firewall)",
            self.skill_id, self.kb_layer
        )
    }
}

impl std::error::Error for SovereigntyViolation {}

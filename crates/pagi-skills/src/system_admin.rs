//! Sovereign Admin: Hardware Awareness and Secure Credentialing
//!
//! Provides the "Admin (Action)" layer for the Master Orchestrator:
//! - **GetHardwareStats**: Real-time CPU/RAM/Disk health for the Architect (sysinfo).
//! - **SecureVault**: OS keychain-backed storage for API keys (keyring).
//!
//! Enables Phoenix to say: *"Jamey, while analyzing the PROOFPOINT logs, I noticed your
//! Disk I/O is peaking at 95%. I've diagrammed the process bottleneck below."*

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use pagi_core::TenantContext;

use crate::system::SystemTelemetry;

// ---------------------------------------------------------------------------
// GetHardwareStats: Vitality summary for the Architect
// ---------------------------------------------------------------------------

/// Compact hardware vitality payload for JSON Diagram Envelope and Counselor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareVitality {
    /// CPU usage (0–100%)
    pub cpu_usage_pct: f32,
    /// RAM used / total as percentage
    pub ram_used_pct: f32,
    /// Total RAM (bytes)
    pub ram_total_bytes: u64,
    /// Used RAM (bytes)
    pub ram_used_bytes: u64,
    /// Per-disk health: name, used_pct, total_gb, available_gb
    pub disks: Vec<DiskVitality>,
    /// Human-readable one-liner
    pub summary: String,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskVitality {
    pub name: String,
    pub mount_point: String,
    pub used_pct: f32,
    pub total_gb: f64,
    pub available_gb: f64,
}

/// GetHardwareStatsSkill: Returns CPU/RAM/Disk health to the Architect.
///
/// Use when the user asks about "system vitality", "system health", "CPU", "RAM", or "disk".
/// Output is suitable for injection into a JSON Diagram Envelope (e.g. Mermaid flowchart).
pub struct GetHardwareStatsSkill {
    telemetry: Arc<SystemTelemetry>,
}

impl GetHardwareStatsSkill {
    pub fn new(telemetry: Arc<SystemTelemetry>) -> Self {
        Self { telemetry }
    }
}

#[async_trait]
impl pagi_core::AgentSkill for GetHardwareStatsSkill {
    fn name(&self) -> &str {
        "GetHardwareStats"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        _payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        self.telemetry.refresh().await;

        let cpu = self.telemetry.get_cpu_info().await;
        let memory = self.telemetry.get_memory_info().await;
        let disks = self.telemetry.get_disks().await;

        let ram_used_pct = if memory.total_memory > 0 {
            (memory.used_memory as f64 / memory.total_memory as f64) * 100.0
        } else {
            0.0
        };

        let disks_vitality: Vec<DiskVitality> = disks
            .iter()
            .map(|d| {
                let total_gb = d.total_space as f64 / 1_073_741_824.0;
                let available_gb = d.available_space as f64 / 1_073_741_824.0;
                let used_pct = if d.total_space > 0 {
                    ((d.total_space - d.available_space) as f64 / d.total_space as f64) * 100.0
                } else {
                    0.0
                };
                DiskVitality {
                    name: d.name.clone(),
                    mount_point: d.mount_point.clone(),
                    used_pct: used_pct as f32,
                    total_gb,
                    available_gb,
                }
            })
            .collect();

        let summary = format!(
            "CPU {:.1}% | RAM {:.1}% ({:.1} GB / {:.1} GB) | {} disk(s)",
            cpu.usage,
            ram_used_pct,
            memory.used_memory as f64 / 1_073_741_824.0,
            memory.total_memory as f64 / 1_073_741_824.0,
            disks_vitality.len()
        );

        let vitality = HardwareVitality {
            cpu_usage_pct: cpu.usage,
            ram_used_pct: ram_used_pct as f32,
            ram_total_bytes: memory.total_memory,
            ram_used_bytes: memory.used_memory,
            disks: disks_vitality,
            summary: summary.clone(),
            timestamp: chrono::Utc::now(),
        };

        Ok(serde_json::to_value(vitality)?)
    }
}

// ---------------------------------------------------------------------------
// SecureVault: OS keychain-backed API key storage
// ---------------------------------------------------------------------------

/// SecureVault: Store and retrieve API keys via the OS keychain (keyring).
///
/// Use for OpenRouter, Local LLM, and other credentials so that file access
/// alone does not expose the "lifeblood" of the system.
pub struct SecureVault;

impl SecureVault {
    pub fn new() -> Self {
        Self
    }

    /// Service name used for keyring entries (sovereign perimeter).
    const SERVICE: &'static str = "pagi-phoenix";

    /// Store a secret (e.g. OPENROUTER_API_KEY) under a key name.
    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(Self::SERVICE, key)
            .map_err(|e| format!("keyring entry failed: {}", e))?;
        entry.set_password(value).map_err(|e| format!("keyring set failed: {}", e))?;
        Ok(())
    }

    /// Retrieve a secret by key name.
    pub fn get(&self, key: &str) -> Result<String, String> {
        let entry = keyring::Entry::new(Self::SERVICE, key)
            .map_err(|e| format!("keyring entry failed: {}", e))?;
        entry.get_password().map_err(|e| format!("keyring get failed: {}", e))
    }

    /// Delete a stored secret.
    pub fn delete(&self, key: &str) -> Result<(), String> {
        let entry = keyring::Entry::new(Self::SERVICE, key)
            .map_err(|e| format!("keyring entry failed: {}", e))?;
        entry.delete_credential().map_err(|e| format!("keyring delete failed: {}", e))?;
        Ok(())
    }
}

impl Default for SecureVault {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve an API key: try OS keychain (SecureVault) first, then `std::env::var(key)`.
/// Use for OpenRouter and other credentials so the Orchestrator can pull from the vault.
pub fn resolve_api_key_from_vault_or_env(key: &str) -> Option<String> {
    let vault = SecureVault::new();
    vault.get(key).ok().or_else(|| std::env::var(key).ok())
}

/// SecureVaultSkill: AgentSkill interface for SecureVault (get/set/delete by key).
pub struct SecureVaultSkill {
    vault: Arc<SecureVault>,
}

impl SecureVaultSkill {
    pub fn new(vault: Arc<SecureVault>) -> Self {
        Self { vault }
    }
}

#[async_trait]
impl pagi_core::AgentSkill for SecureVaultSkill {
    fn name(&self) -> &str {
        "SecureVault"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("SecureVault requires payload: { operation, key, value? }")?;
        let operation = payload
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'operation' (get, set, delete)")?;
        let key = payload
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'key'")?;

        let result = match operation {
            "get" => {
                match self.vault.get(key) {
                    Ok(v) => serde_json::json!({ "success": true, "key": key, "value": v }),
                    Err(e) => serde_json::json!({ "success": false, "key": key, "error": e }),
                }
            }
            "set" => {
                let value = payload
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'value' for set")?;
                match self.vault.set(key, value) {
                    Ok(()) => serde_json::json!({ "success": true, "key": key }),
                    Err(e) => serde_json::json!({ "success": false, "key": key, "error": e }),
                }
            }
            "delete" => match self.vault.delete(key) {
                Ok(()) => serde_json::json!({ "success": true, "key": key }),
                Err(e) => serde_json::json!({ "success": false, "key": key, "error": e }),
            },
            _ => return Err(format!("Unknown operation: {}", operation).into()),
        };

        Ok(result)
    }
}

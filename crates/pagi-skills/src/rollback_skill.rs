//! RollbackSkill: Agent-callable skill for evolutionary rollback.
//!
//! Allows the agent (or operator) to:
//! - Roll back a skill to a previous version.
//! - Query the version history for a skill.
//! - Check if a proposed patch is an evolutionary dead-end.
//! - List all versioned skills and their current status.
//!
//! ## Actions
//!
//! | Action | Description |
//! |--------|-------------|
//! | `rollback` | Revert a skill to a previous version. |
//! | `history` | Get the version history for a skill. |
//! | `check_dead_end` | Check if code is a known dead-end. |
//! | `list_skills` | List all skills with versioned patches. |
//! | `full_history` | Get the full patch history across all skills. |
//! | `dead_ends` | List all evolutionary dead-ends. |

use std::sync::Arc;

use pagi_core::{AgentSkill, TenantContext};
use pagi_evolution::RollbackManager;
use tracing::{info, warn};

/// AgentSkill wrapper for the RollbackManager.
///
/// Registered in the SkillRegistry so the Orchestrator can dispatch
/// rollback commands through the standard skill interface.
pub struct RollbackSkill {
    rollback_manager: Arc<RollbackManager>,
}

impl RollbackSkill {
    pub fn new(rollback_manager: Arc<RollbackManager>) -> Self {
        Self { rollback_manager }
    }
}

#[async_trait::async_trait]
impl AgentSkill for RollbackSkill {
    fn name(&self) -> &str {
        "RollbackSkill"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("Missing payload")?;
        let action = payload
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'action' field")?;

        match action {
            // -----------------------------------------------------------------
            // rollback: Revert a skill to a previous version
            // -----------------------------------------------------------------
            "rollback" => {
                let skill_name = payload
                    .get("skill")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'skill' field")?;

                let target_timestamp = payload
                    .get("target_timestamp")
                    .and_then(|v| v.as_i64());

                let reason = payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Operator-initiated rollback");

                info!(
                    target: "pagi::rollback_skill",
                    skill = skill_name,
                    reason = reason,
                    "Rollback requested"
                );

                match self.rollback_manager.rollback_skill(
                    skill_name,
                    target_timestamp,
                    reason,
                ) {
                    Ok(version) => Ok(serde_json::json!({
                        "status": "success",
                        "message": format!(
                            "Rolled back '{}' to version {} (timestamp {})",
                            skill_name,
                            version.source_path.display(),
                            version.timestamp_ms
                        ),
                        "version": {
                            "skill_name": version.skill_name,
                            "timestamp_ms": version.timestamp_ms,
                            "code_hash": version.code_hash,
                            "source_path": version.source_path.to_string_lossy(),
                            "is_active": version.is_active,
                            "status": format!("{:?}", version.status),
                            "description": version.description,
                        }
                    })),
                    Err(e) => {
                        warn!(
                            target: "pagi::rollback_skill",
                            skill = skill_name,
                            error = %e,
                            "Rollback failed"
                        );
                        Ok(serde_json::json!({
                            "status": "error",
                            "message": format!("Rollback failed: {}", e)
                        }))
                    }
                }
            }

            // -----------------------------------------------------------------
            // history: Get version history for a skill
            // -----------------------------------------------------------------
            "history" => {
                let skill_name = payload
                    .get("skill")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'skill' field")?;

                let versions = self.rollback_manager.get_versions(skill_name);

                let version_list: Vec<serde_json::Value> = versions
                    .iter()
                    .map(|v| {
                        serde_json::json!({
                            "skill_name": v.skill_name,
                            "timestamp_ms": v.timestamp_ms,
                            "code_hash": v.code_hash,
                            "source_path": v.source_path.to_string_lossy(),
                            "artifact_path": v.artifact_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                            "is_active": v.is_active,
                            "status": format!("{:?}", v.status),
                            "description": v.description,
                            "performance_delta": v.performance_delta.as_ref().map(|pd| serde_json::json!({
                                "cpu": pd.cpu,
                                "mem": pd.mem,
                                "compiled": pd.compiled,
                                "smoke_test_passed": pd.smoke_test_passed,
                                "security_audit": pd.security_audit.as_ref().map(|sa| serde_json::json!({
                                    "passed": sa.passed,
                                    "overall_severity": sa.overall_severity,
                                    "reviewer_model": sa.reviewer_model,
                                    "findings_count": sa.findings_count,
                                    "summary": sa.summary,
                                    "memory_warning": sa.memory_warning,
                                })),
                            })),
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "status": "success",
                    "skill": skill_name,
                    "version_count": version_list.len(),
                    "versions": version_list,
                }))
            }

            // -----------------------------------------------------------------
            // check_dead_end: Check if code is a known evolutionary dead-end
            // -----------------------------------------------------------------
            "check_dead_end" => {
                let code = payload
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'code' field")?;

                match self.rollback_manager.check_dead_end(code) {
                    Some(dead_end) => Ok(serde_json::json!({
                        "status": "dead_end",
                        "message": format!(
                            "This code is an Evolutionary Dead-End for '{}'. Reason: {}. Occurrences: {}.",
                            dead_end.skill_name, dead_end.reason, dead_end.occurrence_count
                        ),
                        "record": {
                            "code_hash": dead_end.code_hash,
                            "skill_name": dead_end.skill_name,
                            "reason": dead_end.reason,
                            "timestamp_ms": dead_end.timestamp_ms,
                            "occurrence_count": dead_end.occurrence_count,
                        }
                    })),
                    None => Ok(serde_json::json!({
                        "status": "clear",
                        "message": "This code is not a known dead-end."
                    })),
                }
            }

            // -----------------------------------------------------------------
            // list_skills: List all skills with versioned patches
            // -----------------------------------------------------------------
            "list_skills" => {
                let skills = self.rollback_manager.get_versioned_skills();
                let skill_info: Vec<serde_json::Value> = skills
                    .iter()
                    .map(|name| {
                        let active = self.rollback_manager.get_active_version(name);
                        let version_count = self.rollback_manager.get_versions(name).len();
                        serde_json::json!({
                            "skill_name": name,
                            "version_count": version_count,
                            "active_version": active.map(|v| serde_json::json!({
                                "timestamp_ms": v.timestamp_ms,
                                "code_hash": v.code_hash,
                                "status": format!("{:?}", v.status),
                                "description": v.description,
                            })),
                        })
                    })
                    .collect();

                let (known, dead_ends) = self.rollback_manager.genetic_memory_stats();

                Ok(serde_json::json!({
                    "status": "success",
                    "skills": skill_info,
                    "total_skills": skill_info.len(),
                    "genetic_memory": {
                        "known_dna": known,
                        "dead_ends": dead_ends,
                    }
                }))
            }

            // -----------------------------------------------------------------
            // full_history: Get the full patch history across all skills
            // -----------------------------------------------------------------
            "full_history" => {
                let limit = payload
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(50) as usize;

                let history = self.rollback_manager.get_full_history();
                let truncated: Vec<serde_json::Value> = history
                    .iter()
                    .take(limit)
                    .map(|v| {
                        serde_json::json!({
                            "skill_name": v.skill_name,
                            "timestamp_ms": v.timestamp_ms,
                            "code_hash": v.code_hash,
                            "is_active": v.is_active,
                            "status": format!("{:?}", v.status),
                            "description": v.description,
                            "performance_delta": v.performance_delta.as_ref().map(|pd| serde_json::json!({
                                "cpu": pd.cpu,
                                "mem": pd.mem,
                                "compiled": pd.compiled,
                                "smoke_test_passed": pd.smoke_test_passed,
                                "security_audit": pd.security_audit.as_ref().map(|sa| serde_json::json!({
                                    "passed": sa.passed,
                                    "overall_severity": sa.overall_severity,
                                    "reviewer_model": sa.reviewer_model,
                                    "findings_count": sa.findings_count,
                                    "summary": sa.summary,
                                    "memory_warning": sa.memory_warning,
                                })),
                            })),
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "status": "success",
                    "total": history.len(),
                    "returned": truncated.len(),
                    "history": truncated,
                }))
            }

            // -----------------------------------------------------------------
            // dead_ends: List all evolutionary dead-ends
            // -----------------------------------------------------------------
            "dead_ends" => {
                let dead_ends = self.rollback_manager.get_dead_ends();
                let records: Vec<serde_json::Value> = dead_ends
                    .iter()
                    .map(|d| {
                        serde_json::json!({
                            "code_hash": d.code_hash,
                            "skill_name": d.skill_name,
                            "reason": d.reason,
                            "timestamp_ms": d.timestamp_ms,
                            "occurrence_count": d.occurrence_count,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({
                    "status": "success",
                    "total": records.len(),
                    "dead_ends": records,
                }))
            }

            _ => Err(format!("Unknown RollbackSkill action: {}", action).into()),
        }
    }
}

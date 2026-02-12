//! RedTeamSkill: Agent-callable skill for adversarial peer review.
//!
//! Wraps the `RedTeamAnalyzer` and `ConsensusGate` from `pagi-evolution`
//! into the standard `AgentSkill` interface so the Orchestrator can dispatch
//! security reviews through the skill registry.
//!
//! ## Actions
//!
//! | Action | Description |
//! |--------|-------------|
//! | `review` | Submit code for adversarial peer review. |
//! | `consensus` | Run the full consensus gate (review + decision). |
//! | `heuristic` | Run heuristic-only analysis (no LLM call). |
//!
//! ## Integration with Maintenance Loop
//!
//! During **Phase 4.75: Peer Review**, the maintenance loop calls the `consensus`
//! action. If the result is `approved: false`, the patch is auto-rejected and
//! (if lethal) marked in Genetic Memory.

use std::sync::Arc;

use pagi_core::{AgentSkill, TenantContext};
use pagi_evolution::{
    ConsensusGate, ConsensusResult, RedTeamAnalyzer, RedTeamConfig, RollbackManager,
    SecurityVerdict,
};
use tracing::{info, warn};

/// Agent skill for adversarial peer review of proposed patches.
///
/// Registered in the SkillRegistry so the Orchestrator and Maintenance Loop
/// can dispatch security reviews through the standard skill interface.
pub struct RedTeamSkill {
    analyzer: Arc<RedTeamAnalyzer>,
    gate: ConsensusGate,
    rollback_manager: Option<Arc<RollbackManager>>,
}

impl RedTeamSkill {
    /// Create a new RedTeamSkill with the given analyzer and optional rollback manager.
    ///
    /// If a `RollbackManager` is provided, lethal mutations will be automatically
    /// recorded in Genetic Memory.
    pub fn new(
        analyzer: Arc<RedTeamAnalyzer>,
        rollback_manager: Option<Arc<RollbackManager>>,
    ) -> Self {
        Self {
            analyzer,
            gate: ConsensusGate::default(),
            rollback_manager,
        }
    }

    /// Create with default configuration from environment variables.
    pub fn from_env(rollback_manager: Option<Arc<RollbackManager>>) -> Self {
        Self {
            analyzer: Arc::new(RedTeamAnalyzer::from_env()),
            gate: ConsensusGate::default(),
            rollback_manager,
        }
    }

    /// Run the full consensus pipeline: review + gate evaluation + genetic memory update.
    ///
    /// This is the primary entry point for **Phase 4.75** of the maintenance loop.
    pub async fn run_consensus(
        &self,
        skill_name: &str,
        code: &str,
        patch_description: &str,
    ) -> ConsensusResult {
        // Step 1: Adversarial review.
        let verdict = self
            .analyzer
            .review_patch(skill_name, code, patch_description)
            .await;

        // Step 2: Consensus gate evaluation.
        let result = self.gate.evaluate(verdict);

        // Step 3: If lethal, mark in Genetic Memory.
        if result.mark_lethal {
            if let Some(ref rm) = self.rollback_manager {
                if let Err(e) = rm.mark_dead_end(
                    code,
                    skill_name,
                    &format!("Lethal Mutation (Red-Team): {}", result.reason),
                ) {
                    warn!(
                        target: "pagi::redteam_skill",
                        error = %e,
                        "Failed to mark lethal mutation in Genetic Memory"
                    );
                } else {
                    info!(
                        target: "pagi::redteam_skill",
                        skill = skill_name,
                        "Lethal Mutation recorded in Genetic Memory"
                    );
                }
            }
        }

        // Step 4: If rejected (non-lethal), still mark as dead-end.
        if !result.approved && !result.mark_lethal {
            if let Some(ref rm) = self.rollback_manager {
                let _ = rm.mark_dead_end(
                    code,
                    skill_name,
                    &format!("Red-Team Rejected: {}", result.reason),
                );
            }
        }

        info!(
            target: "pagi::redteam_skill",
            skill = skill_name,
            approved = result.approved,
            lethal = result.mark_lethal,
            severity = %result.verdict.overall_severity,
            "Consensus gate result"
        );

        result
    }

    /// Get a reference to the underlying analyzer.
    pub fn analyzer(&self) -> &RedTeamAnalyzer {
        &self.analyzer
    }
}

#[async_trait::async_trait]
impl AgentSkill for RedTeamSkill {
    fn name(&self) -> &str {
        "RedTeamSkill"
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

        let skill_name = payload
            .get("skill")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let code = payload
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'code' field")?;

        let description = payload
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("Proposed patch");

        match action {
            // -----------------------------------------------------------------
            // review: Submit code for adversarial peer review (verdict only)
            // -----------------------------------------------------------------
            "review" => {
                let verdict = self
                    .analyzer
                    .review_patch(skill_name, code, description)
                    .await;

                Ok(verdict_to_json(&verdict))
            }

            // -----------------------------------------------------------------
            // consensus: Full consensus gate (review + decision + genetic memory)
            // -----------------------------------------------------------------
            "consensus" => {
                let result = self.run_consensus(skill_name, code, description).await;

                Ok(serde_json::json!({
                    "status": if result.approved { "approved" } else { "rejected" },
                    "approved": result.approved,
                    "reason": result.reason,
                    "mark_lethal": result.mark_lethal,
                    "verdict": verdict_to_json(&result.verdict),
                }))
            }

            // -----------------------------------------------------------------
            // heuristic: Heuristic-only analysis (no LLM call)
            // -----------------------------------------------------------------
            "heuristic" => {
                // Access the heuristic directly via a temporary analyzer.
                let config = RedTeamConfig::default();
                let temp_analyzer = RedTeamAnalyzer::new(config);
                // We need to use the public review_patch which falls back to heuristic
                // when no API key is available. For explicit heuristic, we set no key.
                let verdict = temp_analyzer
                    .review_patch(skill_name, code, description)
                    .await;

                Ok(verdict_to_json(&verdict))
            }

            _ => Err(format!("Unknown RedTeamSkill action: {}", action).into()),
        }
    }
}

/// Convert a SecurityVerdict to a JSON value for API responses.
fn verdict_to_json(verdict: &SecurityVerdict) -> serde_json::Value {
    let findings: Vec<serde_json::Value> = verdict
        .findings
        .iter()
        .map(|f| {
            serde_json::json!({
                "category": f.category,
                "severity": format!("{}", f.severity),
                "description": f.description,
                "affected_region": f.affected_region,
                "remediation": f.remediation,
            })
        })
        .collect();

    serde_json::json!({
        "overall_severity": format!("{}", verdict.overall_severity),
        "passed": verdict.passed,
        "summary": verdict.summary,
        "reviewer_model": verdict.reviewer_model,
        "reviewed_at_ms": verdict.reviewed_at_ms,
        "memory_warning": verdict.memory_warning,
        "findings_count": verdict.findings.len(),
        "findings": findings,
    })
}

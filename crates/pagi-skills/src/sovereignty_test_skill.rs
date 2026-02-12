//! Sovereignty Test Case: skill that uses Command without calling KB-05 validation.
//!
//! This module exists only to verify the `--audit` and `--heal` flow:
//! - **Audit:** Flags this file in `skills_without_kb05` (Command present, no validate_security call).
//! - **Heal:** Injects `self.validate_security(knowledge, &params).await?` before the Command line.
//!   The trait default for `validate_security` is used (we do not override it here), so the fix compiles.
//!
//! Run from repo root: `cargo run -p pagi-gateway -- --heal` to confirm identify → wrap → verify.

use pagi_core::{KnowledgeStore, LiveSkill, SkillPriority, EnergyCost, TenantContext};

/// Sacrificial skill for sovereignty heal verification. Do not add a validate_security call
/// in execute(); the heal flow injects it.
pub struct SovereigntyTestSkill;

#[async_trait::async_trait]
impl LiveSkill for SovereigntyTestSkill {
    fn name(&self) -> &str {
        "sovereignty_test"
    }

    fn description(&self) -> &str {
        "Test fixture for --heal: Command without KB-05 check (intentional)."
    }

    fn priority(&self) -> SkillPriority {
        SkillPriority::Low
    }

    fn energy_cost(&self) -> EnergyCost {
        EnergyCost::Minimal
    }

    // Do NOT override requires_security_check or validate_security so this file has no
    // "validate_security" / "requires_security_check" / "KB-05" and audit flags it.
    // string and audit flags it. Heal injects the call; trait default provides the method.

    async fn execute(
        &self,
        _ctx: &TenantContext,
        knowledge: &KnowledgeStore,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        // No validate_security call here - for --heal test. Heal will inject it before the next line.
        let output = tokio::process::Command::new("echo")
            .args(["sovereignty", "heal", "test"])
            .output()
            .await
            .map_err(|e| format!("command failed: {}", e))?;
        let _ = (knowledge, params);
        Ok(serde_json::json!({
            "status": "ok",
            "sovereignty_test": true,
            "command_success": output.status.success(),
        }))
    }
}

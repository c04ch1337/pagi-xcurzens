//! **Identity Setup Skill** — Core (Tier 1) onboarding: save user personality, vulnerabilities, and communication style to KB-01.
//!
//! Human-centric: "Who are you?", "What drains you?", "How should we talk?"
//! Persists to KB-01 (Pneuma) under `user_profile` for persona and discovery. Only this skill
//! should have write-access to the personality profile in KB-01 (enforced via core manifest).

use pagi_core::{AgentSkill, EventRecord, KnowledgeStore, TenantContext, KB01_USER_PROFILE_KEY};
use serde::Deserialize;
use std::sync::Arc;

const SKILL_NAME: &str = "IdentitySetup";

/// Slot ID for KB-01 (Pneuma / Identity).
const PNEUMA_SLOT: u8 = 1;

/// Input payload for the Identity Setup skill.
#[derive(Debug, Deserialize)]
struct IdentitySetupArgs {
    /// Who are you? e.g. "Pisces Sun, ENFP" or "Birthday: March 15, MBTI: INFP".
    #[serde(default)]
    personality_profile: String,

    /// What drains you? Recurring patterns, e.g. ["People pleasing", "Work burnout", "Difficulty saying no"].
    #[serde(default)]
    energy_drains: Vec<String>,

    /// How should we talk? e.g. "Direct and analytical", "Gentle and supportive", "Purely logic-driven".
    #[serde(default)]
    communication_style: String,
}

pub struct IdentitySetup {
    store: Arc<KnowledgeStore>,
}

impl IdentitySetup {
    pub fn new(store: Arc<KnowledgeStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl AgentSkill for IdentitySetup {
    fn name(&self) -> &str {
        SKILL_NAME
    }

    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("IdentitySetup requires payload: { personality_profile?, energy_drains?, communication_style? }")?;
        let args: IdentitySetupArgs = serde_json::from_value(payload)?;

        // Build the profile object for KB-01. Use keys that existing code expects (user_profile is read as object with k/v pairs).
        let profile = serde_json::json!({
            "personality_profile": args.personality_profile.trim().to_string(),
            "energy_drains": args.energy_drains,
            "communication_style": args.communication_style.trim().to_string(),
        });
        let bytes = serde_json::to_vec(&profile)?;

        self.store.insert(PNEUMA_SLOT, KB01_USER_PROFILE_KEY, &bytes)?;

        let agent_id = ctx.resolved_agent_id();

        // Chronos: log that identity was set (no PII in message).
        let event = EventRecord::now(
            "Chronos",
            "Identity Setup: user profile (personality, energy_drains, communication_style) saved to KB-01 (private storage).",
        )
        .with_skill(SKILL_NAME)
        .with_outcome("identity_setup");
        let _ = self.store.append_chronos_event(agent_id, &event);

        Ok(serde_json::json!({
            "status": "ok",
            "skill": SKILL_NAME,
            "slot_id": PNEUMA_SLOT,
            "chronos_logged": true,
            "message": "Profile Securely Saved to Private Storage.",
        }))
    }
}

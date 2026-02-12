//! CounselorSkill: Proactive mental-health and intervention layer
//!
//! Triggers when InputVelocity is high or HistoryHarvester signals "negative looping".
//! In Counselor mode, can propose Sovereign Reset (minimize windows + health reminder).
//! Provides Spirit/Mind/Body (1–10) balance-check prompt for the UI.

use pagi_core::{AgentSkill, TenantContext};
use serde::{Deserialize, Serialize};
use tracing::info;

const SKILL_NAME: &str = "CounselorSkill";

/// Velocity metrics passed from InputVelocitySensor for proactive gate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounselorVelocityInput {
    pub velocity_score: f64,
    pub is_rage_detected: bool,
    #[serde(default)]
    pub keystrokes_per_second: f64,
    #[serde(default)]
    pub mouse_clicks_per_second: f64,
}

/// Payload for CounselorSkill execute
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CounselorPayload {
    /// When true, evaluate proactive ping (velocity + negative_looping)
    #[serde(default)]
    pub check_proactive: bool,
    /// Current velocity metrics (from InputVelocitySensor)
    pub velocity_metrics: Option<CounselorVelocityInput>,
    /// True when HistoryHarvester (or other analysis) detected negative looping
    #[serde(default)]
    pub negative_looping: bool,
    /// Current persona is Counselor (otherwise no intervention)
    #[serde(default)]
    pub counselor_mode: bool,
    /// Request Spirit/Mind/Body prompt only
    #[serde(default)]
    pub spirit_mind_body_prompt_only: bool,
    /// User birth sign (from KB-01/Archetype). Used for Savior Override: Pisces/Provider → High-Value Vulnerability alert.
    #[serde(default)]
    pub birth_sign: Option<String>,
}

/// Counselor skill: proactive ping and Spirit/Mind/Body quantification.
/// In Counselor mode with rage detected, returns sovereign_reset_suggested; the gateway
/// may then call PhysicalGuard (minimize windows) and show a health reminder.
pub struct CounselorSkill;

impl CounselorSkill {
    pub fn new() -> Self {
        Self
    }

    /// Returns the prompt text for a Spirit/Mind/Body (1–10) balance check.
    /// Use this for the 4-hour heartbeat or proactive ping.
    pub fn spirit_mind_body_prompt(user_name: Option<&str>) -> String {
        let name = user_name.unwrap_or("there").trim();
        if name.is_empty() {
            "Checking in. Spirit / Mind / Body balance—where are you at? (1–10 for each, or a short sentence.)".to_string()
        } else {
            format!(
                "{}, checking in. Spirit/Mind/Body balance check—where are we at? (1–10 for each, or a short sentence.)",
                name
            )
        }
    }

    /// Logic gate: should we trigger a proactive ping? (high velocity or negative looping, and counselor mode)
    fn should_proactive_ping(payload: &CounselorPayload) -> bool {
        if !payload.counselor_mode {
            return false;
        }
        let velocity_high = payload
            .velocity_metrics
            .as_ref()
            .map(|m| m.is_rage_detected || m.velocity_score >= 60.0)
            .unwrap_or(false);
        velocity_high || payload.negative_looping
    }

    /// In Counselor mode with rage, return Sovereign Reset suggestion (gateway may call PhysicalGuard).
    fn maybe_sovereign_reset(&self, payload: &CounselorPayload) -> Option<serde_json::Value> {
        if !payload.counselor_mode {
            return None;
        }
        let rage = payload
            .velocity_metrics
            .as_ref()
            .map(|m| m.is_rage_detected)
            .unwrap_or(false);
        if !rage {
            return None;
        }
        info!(
            target: "pagi::sentinel::counselor",
            "Rage detected in Counselor mode; proposing Sovereign Reset (minimize + health reminder)"
        );
        Some(serde_json::json!({
            "sovereign_reset_suggested": true,
            "message": "Rage detected. Consider a Sovereign Reset: minimize windows, take a breath, water/movement.",
            "health_reminder": "Water, movement, or a short break can help."
        }))
    }
}

impl Default for CounselorSkill {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentSkill for CounselorSkill {
    fn name(&self) -> &str {
        SKILL_NAME
    }

    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload: CounselorPayload = payload
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        if payload.spirit_mind_body_prompt_only {
            let user = ctx.tenant_id.as_str();
            let prompt = Self::spirit_mind_body_prompt(Some(user));
            return Ok(serde_json::json!({
                "status": "ok",
                "skill": SKILL_NAME,
                "spirit_mind_body_prompt": prompt,
            }));
        }

        let should_ping = Self::should_proactive_ping(&payload);
        if should_ping {
            info!(
                target: "pagi::sentinel::counselor",
                "Proactive Counselor ping triggered (velocity high or negative looping)"
            );
        }

        let spirit_mind_body_prompt = Self::spirit_mind_body_prompt(Some(ctx.tenant_id.as_str()));
        let sovereign_reset = self.maybe_sovereign_reset(&payload);
        let savior_override = payload.birth_sign.as_deref().map(|s| s.eq_ignore_ascii_case("pisces")).unwrap_or(false)
            && payload.counselor_mode;

        let mut out = serde_json::json!({
            "status": "ok",
            "skill": SKILL_NAME,
            "should_ping": should_ping,
            "spirit_mind_body_prompt": spirit_mind_body_prompt,
        });
        if let Some(obj) = out.as_object_mut() {
            if let Some(reset) = sovereign_reset {
                if let Some(map) = reset.as_object() {
                    for (k, v) in map {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
            if savior_override {
                obj.insert(
                    "high_value_vulnerability_alert".to_string(),
                    serde_json::json!(true),
                );
                obj.insert(
                    "high_value_vulnerability_message".to_string(),
                    serde_json::json!("Pisces/Provider archetype detected. High-value vulnerability: watch for over-giving and boundary erosion."),
                );
            }
        }
        Ok(out)
    }
}

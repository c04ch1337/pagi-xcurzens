//! Mimir meeting layer: Pre-Flight Audio Check and meeting readiness.
//!
//! PreFlightAudioSkill scans cpal devices and returns a JSON report so Phoenix can advise
//! e.g. "Loopback not detected. Please enable 'Stereo Mix' in Windows Sound Settings."

use async_trait::async_trait;
use pagi_core::TenantContext;
use pagi_mimir::run_preflight_audio_check;
use serde_json::json;

/// PreFlightAudioSkill: Returns JSON report of mic/loopback availability.
///
/// Use before starting a meeting recording. If loopback is missing, the report includes
/// user_advice so Phoenix can say: "Loopback not detected. Please enable 'Stereo Mix'…"
pub struct PreFlightAudioSkill;

#[async_trait]
impl pagi_core::AgentSkill for PreFlightAudioSkill {
    fn name(&self) -> &str {
        "PreFlightAudio"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        _payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let report = run_preflight_audio_check();
        Ok(json!({
            "loopback_active": report.loopback_active,
            "mic_active": report.mic_active,
            "detected_devices": report.detected_devices,
            "user_advice": report.user_advice,
        }))
    }
}

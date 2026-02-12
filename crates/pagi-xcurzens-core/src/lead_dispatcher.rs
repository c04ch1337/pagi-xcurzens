//! LeadDispatcher — XCURZENS service mapped from SovereignOperatorSkill.
//! Writes new business leads to KB-07 (Relations) for Jamey (Root Sovereign).
//! Triggers partner notifications on High Intent (webhook + ALERTS for Command Center).

use std::path::Path;

use crate::identity_orchestrator::ROOT_SOVEREIGN;
use crate::relations::KB07Relations;

/// SovereignOperatorSkill is mapped to LeadDispatcher for XCURZENS.
/// All lead traffic is dispatched to KB-07 (Relations) under the Root Sovereign (Jamey).
pub struct LeadDispatcher {
    relations: KB07Relations,
}

impl LeadDispatcher {
    /// Create a LeadDispatcher with KB-07 at the given path (Bare Metal; no Docker).
    pub fn new(kb07_path: Option<impl AsRef<Path>>) -> sled::Result<Self> {
        let relations = KB07Relations::open(kb07_path)?;
        Ok(Self { relations })
    }

    /// Dispatch a new business lead into KB-07 for Jamey.
    pub fn dispatch_lead(&self, lead_id: &str, payload: &[u8]) -> sled::Result<Option<sled::IVec>> {
        self.relations.log_lead(lead_id, payload)
    }

    /// Dispatch with a JSON-like payload (e.g. from SovereignOperatorSkill output).
    pub fn dispatch_lead_str(&self, lead_id: &str, payload: &str) -> sled::Result<Option<sled::IVec>> {
        self.dispatch_lead(lead_id, payload.as_bytes())
    }

    /// On High Intent: write alert for Jamey, POST to partner webhook (if configured).
    /// ROOT_SOVEREIGN (Jamey) always receives a copy via the ALERTS log in KB-07.
    pub async fn trigger_partner_notification(
        &self,
        lead_id: &str,
        reply_snippet: &str,
        city: Option<&str>,
        weather: Option<&str>,
        webhook_override: Option<&str>,
    ) -> sled::Result<String> {
        let payload = serde_json::json!({
            "lead_id": lead_id,
            "intent": "high",
            "reply_snippet": reply_snippet,
            "city": city,
            "weather": weather,
            "for_sovereign": ROOT_SOVEREIGN,
        });
        let payload_bytes = payload.to_string();

        let alert_key = format!("alert_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0));
        self.relations.log_alert(&alert_key, payload_bytes.as_bytes())?;

        let partner_id = if let Some(url) = webhook_override {
            let client = reqwest::Client::new();
            let _ = client.post(url).json(&payload).send().await;
            tracing::info!(
                "[SYSTEM] High-Intent Lead Detected! Dispatching bandwidth to Partner (webhook override)."
            );
            "webhook".to_string()
        } else if let Ok(partners) = self.relations.get_partners() {
            let first = partners.into_iter().next();
            if let Some((id, value)) = first {
                let value_str = String::from_utf8(value).unwrap_or_default();
                if let Ok(partner) = serde_json::from_str::<serde_json::Value>(&value_str) {
                    if let Some(url) = partner.get("webhook_url").and_then(|v| v.as_str()) {
                        let client = reqwest::Client::new();
                        let _ = client.post(url).json(&payload).send().await;
                        tracing::info!(
                            "[SYSTEM] High-Intent Lead Detected! Dispatching bandwidth to Partner: {}.",
                            id
                        );
                        return Ok(id);
                    }
                }
                tracing::info!(
                    "[SYSTEM] High-Intent Lead Detected! Dispatching bandwidth to Partner: {}.",
                    id
                );
                return Ok(id);
            }
            tracing::info!(
                "[SYSTEM] High-Intent Lead logged to ALERTS for {} (no partner webhook).",
                ROOT_SOVEREIGN
            );
            "none".to_string()
        } else {
            tracing::info!(
                "[SYSTEM] High-Intent Lead logged to ALERTS for {} (no partner webhook).",
                ROOT_SOVEREIGN
            );
            "none".to_string()
        };

        Ok(partner_id)
    }
}

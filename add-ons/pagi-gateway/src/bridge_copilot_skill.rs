//! BridgeCopilotSkill: "Phoenix, ask Copilot to draft a memo from this meeting."
//! Grabs latest transcript from Chronos, redacts via SAORedactor, sends to Copilot sidebar, returns response.
//! Implements AgentSkill for the main orchestrator registry.

#![cfg(all(windows, feature = "bridge-ms"))]

use async_trait::async_trait;
use pagi_core::{AgentSkill, TenantContext};
use std::path::PathBuf;
use std::sync::Arc;

use crate::chronos_sqlite::ChronosSqlite;

pub struct BridgeCopilotSkill {
    chronos_db: Arc<ChronosSqlite>,
    data_dir: PathBuf,
}

impl BridgeCopilotSkill {
    pub fn new(chronos_db: Arc<ChronosSqlite>, data_dir: PathBuf) -> Self {
        Self { chronos_db, data_dir }
    }
}

#[async_trait]
impl AgentSkill for BridgeCopilotSkill {
    fn name(&self) -> &str {
        "bridge_copilot"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let params = payload.unwrap_or(serde_json::json!({}));
        let thread_id: Option<String> = params.get("thread_id").and_then(|v| v.as_str()).map(String::from);
        let prompt_override: Option<String> = params.get("prompt").and_then(|v| v.as_str()).map(String::from);

        let chronos_db = Arc::clone(&self.chronos_db);
        let data_dir = self.data_dir.clone();

        let (transcript, thread_id_used) = tokio::task::spawn_blocking(move || {
            let thread_id = thread_id.or_else(|| {
                chronos_db
                    .list_threads_any(1)
                    .ok()
                    .and_then(|t| t.into_iter().next())
                    .map(|t| t.id)
            });
            let thread_id = match thread_id {
                Some(id) => id,
                None => return (String::new(), None),
            };
            let messages = chronos_db
                .list_messages(&thread_id, 500, None)
                .unwrap_or_default();
            let transcript: String = messages
                .iter()
                .map(|m| format!("[{}] {}", m.role, m.content))
                .collect::<Vec<_>>()
                .join("\n\n");
            (transcript, Some(thread_id))
        })
        .await
        .map_err(|e| format!("spawn_blocking: {}", e))?;

        if transcript.trim().is_empty() {
            return Ok(serde_json::json!({
                "success": false,
                "error": "No transcript found. Start a meeting (Mimir) or provide thread_id with messages.",
                "thread_id": thread_id_used
            }));
        }

        let default_prompt = "Draft a concise memo from this meeting transcript. Use clear sections (e.g. Attendees, Decisions, Action Items).\n\n--- Transcript ---\n\n";
        let prompt_body = prompt_override.unwrap_or_else(|| default_prompt.to_string());
        let full_prompt = format!("{}{}", prompt_body, transcript);

        let data_dir_clone = data_dir.clone();
        let response = tokio::task::spawn_blocking(move || {
            let bridge = pagi_bridge_ms::BridgeOrchestrator::with_redactor_from_path(&data_dir_clone)
                .map_err(|e| format!("BridgeOrchestrator: {}", e))?;
            bridge.send_to_copilot(full_prompt)
        })
        .await
        .map_err(|e| format!("spawn_blocking: {}", e))?
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error + Send + Sync>)?;

        Ok(serde_json::json!({
            "success": true,
            "response": response,
            "thread_id": thread_id_used,
            "transcript_length": transcript.len()
        }))
    }
}

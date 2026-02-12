//! Project Context Synthesis (KB-Linker): cross-reference meeting summaries with infrastructure vitality.
//!
//! **SynthesizeMeetingContext** takes a meeting summary (Markdown path) and the current project context,
//! reads the latest sanitized summary, queries real-time machine health (sysinfo via SystemTelemetry),
//! and produces a Mermaid "Sovereign Vitality" diagram comparing meeting goals with actual system state.
//! Enables the Architect's View to flag discrepancies (e.g. "Upgrading RAM" discussed while RAM is at 90%).

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use pagi_core::TenantContext;

use crate::system_admin::DiskVitality;
use crate::system::SystemTelemetry;

// ---------------------------------------------------------------------------
// Params and result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeParams {
    /// Path to the meeting summary Markdown file (sanitized, from Mimir stop).
    pub summary_path: String,
    /// Optional project_id for labeling the synthesis.
    #[serde(default)]
    pub project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisAlert {
    pub kind: String,
    pub message: String,
    pub meeting_mention: Option<String>,
    pub current_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizeMeetingContextResult {
    /// Mermaid diagram comparing meeting context vs infrastructure vitality.
    pub mermaid: String,
    /// Short narrative summary for the Architect's View.
    pub summary: String,
    /// Alerts when meeting discussed topics that conflict with current machine state.
    pub alerts: Vec<SynthesisAlert>,
    /// Hardware snapshot used for the synthesis.
    pub vitality_snapshot: VitalitySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VitalitySnapshot {
    pub cpu_usage_pct: f32,
    pub ram_used_pct: f32,
    pub ram_summary: String,
    pub disks: Vec<DiskVitality>,
}

// ---------------------------------------------------------------------------
// Skill implementation
// ---------------------------------------------------------------------------

/// SynthesizeMeetingContext: cross-reference a sanitized meeting summary with real-time infrastructure (KB-02 / sysinfo).
pub struct SynthesizeMeetingContextSkill {
    telemetry: Arc<SystemTelemetry>,
}

impl SynthesizeMeetingContextSkill {
    pub fn new(telemetry: Arc<SystemTelemetry>) -> Self {
        Self { telemetry }
    }

    /// Build Mermaid diagram and alerts from meeting text and vitality.
    fn build_mermaid_and_alerts(
        meeting_excerpt: &str,
        vitality: &VitalitySnapshot,
        project_id: Option<&str>,
    ) -> (String, Vec<SynthesisAlert>) {
        let mut alerts = Vec::new();
        let excerpt_lower = meeting_excerpt.to_lowercase();

        // Cross-reference: meeting mentions vs current state
        if (excerpt_lower.contains("ram") || excerpt_lower.contains("memory") || excerpt_lower.contains("upgrading"))
            && vitality.ram_used_pct >= 85.0
        {
            alerts.push(SynthesisAlert {
                kind: "System Alert".to_string(),
                message: format!(
                    "Meeting discussed RAM/memory; current utilization is {:.1}% — consider upgrade or cleanup.",
                    vitality.ram_used_pct
                ),
                meeting_mention: Some("RAM / memory / upgrading".to_string()),
                current_value: format!("{:.1}% RAM used", vitality.ram_used_pct),
            });
        }
        if (excerpt_lower.contains("cpu") || excerpt_lower.contains("processor") || excerpt_lower.contains("load"))
            && vitality.cpu_usage_pct >= 80.0
        {
            alerts.push(SynthesisAlert {
                kind: "System Alert".to_string(),
                message: format!(
                    "Meeting discussed CPU/load; current CPU usage is {:.1}%.",
                    vitality.cpu_usage_pct
                ),
                meeting_mention: Some("CPU / processor / load".to_string()),
                current_value: format!("{:.1}% CPU", vitality.cpu_usage_pct),
            });
        }
        for d in &vitality.disks {
            if d.used_pct >= 90.0
                && (excerpt_lower.contains("disk") || excerpt_lower.contains("storage") || excerpt_lower.contains("space"))
            {
                alerts.push(SynthesisAlert {
                    kind: "System Alert".to_string(),
                    message: format!(
                        "Meeting discussed storage; disk {} is at {:.1}% capacity.",
                        d.name, d.used_pct
                    ),
                    meeting_mention: Some("disk / storage / space".to_string()),
                    current_value: format!("{} {:.1}%", d.name, d.used_pct),
                });
            }
        }

        // Mermaid: Meeting context vs Sovereign Vitality (node labels avoid parentheses for compatibility)
        let project_label = project_id.unwrap_or("Current Project");
        let alert_node = if alerts.is_empty() {
            "Vitality OK"
        } else {
            "Alerts"
        };
        let ram_label = format!("RAM {:.0}%", vitality.ram_used_pct);
        let mermaid = format!(
            r#"flowchart LR
    subgraph Meeting["Meeting Summary"]
        A["{0}"]
    end
    subgraph Infrastructure["Sovereign Vitality"]
        B["CPU {1:.0}%"]
        C["{2}"]
    end
    A --> E["Cross-Reference"]
    B --> E
    C --> E
    E --> F["{3}"]
    style F fill:{4}
"#,
            project_label.replace('"', "'"),
            vitality.cpu_usage_pct,
            ram_label,
            alert_node,
            if alerts.is_empty() { "#d4edda" } else { "#f8d7da" }
        );

        (mermaid, alerts)
    }
}

#[async_trait]
impl pagi_core::AgentSkill for SynthesizeMeetingContextSkill {
    fn name(&self) -> &str {
        "SynthesizeMeetingContext"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let params: SynthesizeParams = match payload {
            Some(p) => serde_json::from_value(p).map_err(|e| format!("Invalid params: {}", e))?,
            None => return Err("SynthesizeMeetingContext requires summary_path".into()),
        };

        let path = Path::new(params.summary_path.trim());
        let summary_content = std::fs::read_to_string(path)
            .map_err(|e| format!("Could not read summary at {}: {}", path.display(), e))?;

        // First ~2000 chars for cross-reference (transcript + headings)
        let meeting_excerpt = summary_content.chars().take(2000).collect::<String>();

        // Real-time vitality (KB-02 / Infrastructure view via sysinfo)
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

        let ram_summary = format!(
            "{:.1}% ({:.1} / {:.1} GB)",
            ram_used_pct,
            memory.used_memory as f64 / 1_073_741_824.0,
            memory.total_memory as f64 / 1_073_741_824.0
        );

        let vitality_snapshot = VitalitySnapshot {
            cpu_usage_pct: cpu.usage,
            ram_used_pct: ram_used_pct as f32,
            ram_summary: ram_summary.clone(),
            disks: disks_vitality.clone(),
        };

        let (mermaid, alerts) = Self::build_mermaid_and_alerts(
            &meeting_excerpt,
            &vitality_snapshot,
            params.project_id.as_deref(),
        );

        let summary = if alerts.is_empty() {
            format!(
                "Meeting context synthesized with Sovereign Vitality. No discrepancies: CPU {:.1}%, RAM {}.",
                vitality_snapshot.cpu_usage_pct, vitality_snapshot.ram_summary
            )
        } else {
            format!(
                "Synthesis complete. {} system alert(s): meeting discussed infrastructure topics while current state shows high utilization — see diagram.",
                alerts.len()
            )
        };

        let result = SynthesizeMeetingContextResult {
            mermaid,
            summary,
            alerts,
            vitality_snapshot,
        };

        Ok(serde_json::to_value(result)?)
    }
}

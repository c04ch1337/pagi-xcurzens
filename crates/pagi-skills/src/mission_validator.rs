//! Mission Submission Validator Skill
//!
//! Validates a beta tester "submission bundle" for Operation First Rise.
//!
//! Input payload (flexible; keys are optional but recommended):
//! {
//!   "density_mode": "concise",
//!   "json_envelope": { ... } | "{...}" | { "status": "ok", "response": "{...}" },
//!   "sidecar_logs": "...",
//!   "diagramviewer_screenshot_description": "..."
//! }
//!
//! Output:
//! {
//!   "status": "verified" | "invalid",
//!   "rank": "Gold" | "Silver" | "Bronze" | "Retry",
//!   "checks": { ... },
//!   "notes": [ ... ]
//! }

use pagi_core::{AgentSkill, TenantContext};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const SKILL_NAME: &str = "MissionValidator";

#[derive(Debug, Deserialize)]
struct MissionBundle {
    #[serde(default)]
    density_mode: Option<String>,

    /// Raw JSON envelope, full gateway response, or a string containing JSON.
    #[serde(default)]
    json_envelope: Option<serde_json::Value>,

    #[serde(default)]
    sidecar_logs: Option<String>,

    #[serde(default)]
    diagramviewer_screenshot_description: Option<String>,
}

#[derive(Debug, Serialize)]
struct Checks {
    density_concise_ok: bool,
    json_schema_ok: bool,
    mermaid_dark_theme_ok: bool,
    sidecar_qdrant_ok: bool,
    qdrant_pid: Option<u32>,
    qdrant_port_6333_seen: bool,
}

#[derive(Debug, Serialize)]
struct ValidationReport {
    status: String,
    rank: String,
    checks: Checks,
    notes: Vec<String>,
}

fn is_concise_density(s: &str) -> bool {
    matches!(s.trim().to_lowercase().as_str(), "concise" | "sovereign" | "concise_sovereign")
}

fn extract_first_json_object(s: &str) -> Option<String> {
    // Naive, but robust enough for gateway responses that start with a JSON object.
    let mut started = false;
    let mut depth: i32 = 0;
    let mut start_idx: usize = 0;
    for (i, ch) in s.char_indices() {
        if !started {
            if ch == '{' {
                started = true;
                depth = 1;
                start_idx = i;
            }
            continue;
        }
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[start_idx..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn normalize_envelope(val: &serde_json::Value) -> Option<serde_json::Value> {
    // Accept direct envelope: {"type":"diagram",...}
    if val.get("type").is_some() && val.get("format").is_some() && val.get("content").is_some() {
        return Some(val.clone());
    }

    // Accept full gateway response: {"status":"ok","response":"{...}"}
    if let Some(resp) = val.get("response") {
        if let Some(s) = resp.as_str() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(s) {
                if v.get("type").is_some() {
                    return Some(v);
                }
            }
            if let Some(obj) = extract_first_json_object(s) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&obj) {
                    if v.get("type").is_some() {
                        return Some(v);
                    }
                }
            }
        }
    }

    None
}

fn validate_diagram_envelope(envelope: &serde_json::Value) -> (bool, bool, Vec<String>) {
    let mut notes = Vec::new();
    let ty_ok = envelope
        .get("type")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("diagram"))
        .unwrap_or(false);
    if !ty_ok {
        notes.push("Envelope missing or invalid field: type=diagram".to_string());
    }

    let format_ok = envelope
        .get("format")
        .and_then(|v| v.as_str())
        .map(|s| s.eq_ignore_ascii_case("mermaid"))
        .unwrap_or(false);
    if !format_ok {
        notes.push("Envelope missing or invalid field: format=mermaid".to_string());
    }

    let content = envelope.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let content_ok = !content.trim().is_empty();
    if !content_ok {
        notes.push("Envelope missing or invalid field: content (Mermaid source)".to_string());
    }

    // Dark theme init: allow a couple syntaxes but require theme: dark to be present.
    let dark_ok = {
        let c = content;
        c.contains("%%{init")
            && (c.contains("theme': 'dark'")
                || c.contains("theme\": \"dark\"")
                || c.contains("theme: 'dark'")
                || c.contains("theme: \"dark\""))
    };
    if !dark_ok {
        notes.push("Mermaid content missing dark theme init (expected %%{init: {'theme': 'dark'}}%%)".to_string());
    }

    (ty_ok && format_ok && content_ok, dark_ok, notes)
}

fn analyze_qdrant_logs(logs: &str) -> (bool, bool, Option<u32>, Vec<String>) {
    let mut notes = Vec::new();
    let l = logs.to_lowercase();

    let port_seen = l.contains("6333");
    if !port_seen {
        notes.push("Sidecar logs: did not find port 6333 evidence".to_string());
    }

    let healthy = l.contains("healthy") || l.contains("health: ok") || l.contains("ready");
    if !healthy {
        notes.push("Sidecar logs: did not find a clear health/ready signal".to_string());
    }

    let pid = {
        // Search for "pid" token and parse digits after it.
        let bytes = logs.as_bytes();
        let mut i = 0usize;
        let mut parsed: Option<u32> = None;
        while i + 2 < bytes.len() {
            if (bytes[i] == b'P' || bytes[i] == b'p')
                && (bytes[i + 1] == b'I' || bytes[i + 1] == b'i')
                && (bytes[i + 2] == b'D' || bytes[i + 2] == b'd')
            {
                // Move forward to first digit
                let mut j = i + 3;
                while j < bytes.len() && !(bytes[j] as char).is_ascii_digit() {
                    j += 1;
                }
                let start = j;
                while j < bytes.len() && (bytes[j] as char).is_ascii_digit() {
                    j += 1;
                }
                if start < j {
                    if let Some(n) = std::str::from_utf8(&bytes[start..j]).ok().and_then(|s| s.parse::<u32>().ok()) {
                        parsed = Some(n);
                        break;
                    }
                }
            }
            i += 1;
        }
        parsed
    };
    if pid.is_none() {
        notes.push("Sidecar logs: PID not found (PID optional, but helps verification)".to_string());
    }

    (port_seen && healthy, port_seen, pid, notes)
}

async fn poll_qdrant_health_3() -> (bool, Option<usize>, String) {
    // NOTE: This is a *local* stability wrapper. It is intentionally hard-coded to
    // 127.0.0.1 to match the Sovereignty Firewall model.
    let url = "http://127.0.0.1:6333/health";
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => return (false, None, format!("Qdrant health poll: HTTP client build failed ({})", e)),
    };

    // Backoff schedule: immediate, 500ms, 1500ms.
    let backoffs = [Duration::from_millis(0), Duration::from_millis(500), Duration::from_millis(1500)];
    for (i, backoff) in backoffs.iter().enumerate() {
        if *backoff != Duration::from_millis(0) {
            tokio::time::sleep(*backoff).await;
        }
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                return (
                    true,
                    Some(i + 1),
                    format!("Qdrant health poll: OK (attempt {}/{})", i + 1, backoffs.len()),
                );
            }
            Ok(resp) => {
                // Continue retrying; capture status in final message.
                if i + 1 == backoffs.len() {
                    return (
                        false,
                        Some(i + 1),
                        format!(
                            "Qdrant health poll: unexpected HTTP status {} (attempt {}/{})",
                            resp.status(),
                            i + 1,
                            backoffs.len()
                        ),
                    );
                }
            }
            Err(e) => {
                if i + 1 == backoffs.len() {
                    return (
                        false,
                        Some(i + 1),
                        format!("Qdrant health poll: request failed ({})", e),
                    );
                }
            }
        }
    }

    (false, None, "Qdrant health poll: failed".to_string())
}

fn rank_from_checks(
    density_ok: bool,
    schema_ok: bool,
    dark_ok: bool,
    qdrant_ok: bool,
    has_screenshot_desc: bool,
) -> String {
    if !schema_ok {
        return "Retry".to_string();
    }

    if qdrant_ok && density_ok && dark_ok && has_screenshot_desc {
        return "Gold".to_string();
    }

    if qdrant_ok && schema_ok {
        return "Silver".to_string();
    }

    "Bronze".to_string()
}

pub struct MissionValidatorSkill;

impl MissionValidatorSkill {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl AgentSkill for MissionValidatorSkill {
    fn name(&self) -> &str {
        SKILL_NAME
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("MissionValidator requires a JSON payload")?;
        let bundle: MissionBundle = serde_json::from_value(payload.clone()).unwrap_or(MissionBundle {
            density_mode: payload.get("density_mode").and_then(|v| v.as_str()).map(|s| s.to_string()),
            json_envelope: payload.get("json_envelope").cloned().or_else(|| payload.get("raw_json").cloned()),
            sidecar_logs: payload.get("sidecar_logs").and_then(|v| v.as_str()).map(|s| s.to_string()),
            diagramviewer_screenshot_description: payload
                .get("diagramviewer_screenshot_description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        });

        let mut notes: Vec<String> = Vec::new();

        // Density mode
        let density_ok = bundle
            .density_mode
            .as_deref()
            .map(is_concise_density)
            .unwrap_or(false);
        if !density_ok {
            notes.push("density_mode is not clearly 'concise' (required for Architect’s View proof)".to_string());
        }

        // JSON envelope
        let mut schema_ok = false;
        let mut dark_ok = false;
        if let Some(raw) = bundle.json_envelope.as_ref() {
            let normalized = match raw {
                serde_json::Value::String(s) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(s) {
                        normalize_envelope(&v)
                    } else if let Some(obj) = extract_first_json_object(s) {
                        serde_json::from_str::<serde_json::Value>(&obj).ok().and_then(|v| normalize_envelope(&v))
                    } else {
                        None
                    }
                }
                _ => normalize_envelope(raw),
            };

            if let Some(env) = normalized {
                let (ok, dark, env_notes) = validate_diagram_envelope(&env);
                schema_ok = ok;
                dark_ok = dark;
                notes.extend(env_notes);
            } else {
                notes.push("Could not normalize/locate a JSON Diagram Envelope in json_envelope field".to_string());
            }
        } else {
            notes.push("Missing json_envelope artifact".to_string());
        }

        // Sidecar logs
        let (qdrant_ok_by_logs, port_seen_by_logs, pid, log_notes) = match bundle.sidecar_logs.as_deref() {
            Some(s) if !s.trim().is_empty() => analyze_qdrant_logs(s),
            _ => {
                notes.push("Missing sidecar_logs artifact".to_string());
                (false, false, None, vec![])
            }
        };
        notes.extend(log_notes);

        // Stability Wrapper (Day-0 bandwidth optimization): if log parsing is missing/weak,
        // do a short local poll before flagging Qdrant failure.
        let (qdrant_ok, port_seen) = if qdrant_ok_by_logs {
            (true, port_seen_by_logs)
        } else {
            let (live_ok, _attempt, msg) = poll_qdrant_health_3().await;
            notes.push(msg);
            if live_ok {
                // If we got a local 200 OK, we can treat Qdrant as healthy even if the
                // user-provided sidecar logs are incomplete/noisy.
                (true, true)
            } else {
                (false, port_seen_by_logs)
            }
        };

        let has_screenshot_desc = bundle
            .diagramviewer_screenshot_description
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_screenshot_desc {
            notes.push("Missing diagramviewer_screenshot_description artifact (describe what you saw in the DiagramViewer)".to_string());
        }

        let rank = rank_from_checks(density_ok, schema_ok, dark_ok, qdrant_ok, has_screenshot_desc);
        let status = if rank == "Retry" { "invalid" } else { "verified" };

        let report = ValidationReport {
            status: status.to_string(),
            rank,
            checks: Checks {
                density_concise_ok: density_ok,
                json_schema_ok: schema_ok,
                mermaid_dark_theme_ok: dark_ok,
                sidecar_qdrant_ok: qdrant_ok,
                qdrant_pid: pid,
                qdrant_port_6333_seen: port_seen,
            },
            notes,
        };

        Ok(serde_json::to_value(report)?)
    }
}


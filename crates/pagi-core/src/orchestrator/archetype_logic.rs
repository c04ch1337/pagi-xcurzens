//! Astro-Logic Processing Engine for Phoenix Marie.
//!
//! Processes KB-01 user profile to produce runtime directives (Pisces/Savior monitoring,
//! tone/verbosity overrides) and syncs sovereignty_leaks to KB-05 for subject ranking.
//! Gated by PAGI_ASTRO_LOGIC_ENABLED (SovereignConfig).

use crate::knowledge::KnowledgeStore;
use crate::shared::SovereignConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// KB-05 (Techne) slot for Social Protocols / sovereignty leak triggers.
const KB05_SLOT: u8 = 5;
/// Key in KB-05 where sovereignty_leak keywords are stored for auto-ranking subjects.
pub const SOVEREIGNTY_LEAK_TRIGGERS_KEY: &str = "sovereignty_leak_triggers";

/// Read sovereignty_leak_triggers from KB-05 (written by process_archetype_triggers from KB-01).
/// Returns an empty vec if not set or on parse error.
pub fn get_sovereignty_leak_triggers(store: &KnowledgeStore) -> Vec<String> {
    let bytes = match store.get(KB05_SLOT, SOVEREIGNTY_LEAK_TRIGGERS_KEY) {
        Ok(Some(b)) if !b.is_empty() => b,
        _ => return Vec::new(),
    };
    match serde_json::from_slice::<Vec<String>>(&bytes) {
        Ok(v) => v,
        _ => Vec::new(),
    }
}

/// Result of processing archetype triggers: directive to append to system prompt and optional LLM overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchetypeTriggerResult {
    /// Directive text to append to the dynamic system prompt (empty if nothing to add).
    pub directive: String,
    /// When "Strictly Technical", suggest lower temperature and minimal verbosity.
    pub temperature_override: Option<f32>,
    /// Suggested PAGI_VERBOSITY level: "minimal" | "normal" | "high". Caller may map to env or LLM params.
    pub verbosity_override: Option<String>,
}

/// Process KB-01 user profile and return directive + overrides. Writes sovereignty_leaks to KB-05.
///
/// - If `astro_archetype` contains "Pisces", injects Savior-Complex monitoring directive.
/// - If `tone_preference` is "Strictly Technical", sets temperature_override and verbosity_override.
/// - Maps `sovereignty_leaks` to KB-05 (Social Protocols) for subject auto-ranking.
///
/// When PAGI_ASTRO_LOGIC_ENABLED is false, returns default (no directives applied).
pub fn process_archetype_triggers(
    store: &KnowledgeStore,
    kb01_data: &Value,
) -> ArchetypeTriggerResult {
    if !SovereignConfig::from_env().astro_logic_enabled {
        return ArchetypeTriggerResult::default();
    }
    let mut result = ArchetypeTriggerResult::default();
    let obj = match kb01_data.as_object() {
        Some(o) => o,
        None => return result,
    };

    // 1) Pisces → Savior-Complex resource drain monitoring
    let astro = obj
        .get("astro_archetype")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let astro_lower = astro.to_lowercase();
    if astro_lower.contains("pisces") {
        result.directive.push_str(
            "\n\n=== ASTRO-LOGIC (KB-01) ===\n\
             Monitor for Savior-Complex resource drains. User archetype indicates Pisces; \
             prioritize boundary-focused advice and flag situations where over-giving or \
             rescuing others may drain the user's sovereignty.\n",
        );
    }

    // 2) Strictly Technical → lower temperature and minimal verbosity
    let tone = obj
        .get("tone_preference")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if tone.eq_ignore_ascii_case("Strictly Technical") {
        result.temperature_override = Some(0.3);
        result.verbosity_override = Some("minimal".to_string());
        if !result.directive.is_empty() {
            result.directive.push_str("\n");
        }
        result.directive.push_str(
            "=== TONE (KB-01) ===\n\
             User prefers Strictly Technical tone. Be concise, factual, and low-verbosity. \
             Avoid therapeutic elaboration unless the user asks for it.\n",
        );
    }

    // 3) sovereignty_leaks → write to KB-05 for Social Protocols / subject auto-ranking
    let leaks = obj
        .get("sovereignty_leaks")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    if let Some(leaks_str) = leaks {
        // Parse keywords: comma/semicolon/newline separated, trimmed
        let keywords: Vec<String> = leaks_str
            .split(&[',', ';', '\n'][..])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !keywords.is_empty() {
            if let Ok(bytes) = serde_json::to_vec(&keywords) {
                let _ = store.insert(KB05_SLOT, SOVEREIGNTY_LEAK_TRIGGERS_KEY, &bytes);
            }
            if !result.directive.is_empty() {
                result.directive.push_str("\n");
            }
            result.directive.push_str(
                "=== SOVEREIGNTY LEAKS (KB-01 → KB-05) ===\n\
                 User has specified sovereignty leaks to monitor. When discussing people or \
                 situations, rank subjects that trigger these leaks higher for Gray Rock / \
                 boundary protocols. Keywords are synced to KB-05 (Social Protocols).\n",
            );
        }
    }

    result
}

/// Derive a short "Active Archetype" label from KB-01 user profile for UI display (e.g. "Pisces-Protector", "Technical").
pub fn active_archetype_label(kb01_data: &Value) -> Option<String> {
    let obj = kb01_data.as_object()?;
    let astro = obj.get("astro_archetype").and_then(|v| v.as_str()).unwrap_or("");
    let tone = obj.get("tone_preference").and_then(|v| v.as_str()).unwrap_or("");
    let astro_trim = astro.trim();
    let tone_trim = tone.trim();

    if astro_trim.is_empty() && tone_trim.is_empty() {
        return None;
    }
    let part1: String = if astro_lower_contains(astro_trim, "pisces") {
        "Pisces-Protector".to_string()
    } else if !astro_trim.is_empty() {
        astro_trim.split(',').next().unwrap_or(astro_trim).trim().to_string()
    } else {
        String::new()
    };
    let part2: String = if tone_trim.eq_ignore_ascii_case("Strictly Technical") {
        "Technical".to_string()
    } else if tone_trim.eq_ignore_ascii_case("Therapeutic Peer") {
        "Peer".to_string()
    } else if !tone_trim.is_empty() {
        tone_trim.to_string()
    } else {
        String::new()
    };
    let label = match (part1.is_empty(), part2.is_empty()) {
        (false, false) => format!("{} · {}", part1, part2),
        (false, true) => part1,
        (true, false) => part2,
        (true, true) => return None,
    };
    Some(label)
}

fn astro_lower_contains(s: &str, sub: &str) -> bool {
    s.to_lowercase().contains(sub)
}

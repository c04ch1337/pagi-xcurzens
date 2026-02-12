//! **Evening Audit** — Closing the daily Sovereignty Cycle.
//!
//! After the configured "End of Day" hour (default 6 PM UTC), prompts the user once per day
//! with a gentle reflective question tied to the morning's energy focus. Captured response
//! (success / challenge + optional lesson) is stored in KB-08 for weekly synthesis.
//! Tone: non-judgmental ("What did we learn?" not "Why did you fail?").

use pagi_core::{KnowledgeStore, KB01_USER_PROFILE_KEY};

const PNEUMA_SLOT: u8 = 1;
const SOMA_SLOT: u8 = 8;

/// Key in KB-08: date when we last showed the evening audit prompt (YYYY-MM-DD).
pub const EVENING_AUDIT_PROMPT_SHOWN_KEY: &str = "evening_audit/prompt_shown_date";
/// Key in KB-08: date when user last submitted an evening audit (YYYY-MM-DD).
pub const EVENING_AUDIT_LAST_DATE_KEY: &str = "evening_audit/last_date";
/// Key prefix in KB-08: per-date audit payload. Key = `evening_audit/by_date/{YYYY-MM-DD}`.
pub const EVENING_AUDIT_BY_DATE_PREFIX: &str = "evening_audit/by_date/";

/// Status of the evening audit: success (protected energy) or challenge (struggled; lesson captured).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EveningAuditStatus {
    Success,
    Challenge,
}

/// Returns the evening audit reflective question if:
/// - It's past `audit_start_hour` (UTC);
/// - We haven't already shown the prompt today (`evening_audit/prompt_shown_date` != today);
/// - User hasn't already submitted today (`evening_audit/last_date` != today).
/// Question is based on KB-01 energy_drains so it ties to the morning briefing. Gentle, non-judgmental tone.
pub fn get_evening_audit_prompt(
    store: &KnowledgeStore,
    today: &str,
    current_hour_utc: u8,
    audit_start_hour: u8,
    enabled: bool,
) -> Option<String> {
    if !enabled || current_hour_utc < audit_start_hour {
        return None;
    }
    let prompt_shown: String = store
        .get(SOMA_SLOT, EVENING_AUDIT_PROMPT_SHOWN_KEY)
        .ok()
        .flatten()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default();
    if prompt_shown == today {
        return None;
    }
    let last_date: String = store
        .get(SOMA_SLOT, EVENING_AUDIT_LAST_DATE_KEY)
        .ok()
        .flatten()
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_default();
    if last_date == today {
        return None;
    }

    let focus = evening_focus_from_profile(store);
    let question = if focus.is_empty() {
        "Before we wrap: did you manage to protect your energy today? What did we learn?".to_string()
    } else {
        format!(
            "Earlier we talked about {}—how did that go today? What did we learn?",
            focus
        )
    };
    Some(question)
}

/// Marks that the evening audit prompt was shown today so we don't repeat it. Call after prepending the question.
pub fn mark_evening_audit_prompt_shown(store: &KnowledgeStore, today: &str) -> Result<(), sled::Error> {
    store.insert(SOMA_SLOT, EVENING_AUDIT_PROMPT_SHOWN_KEY, today.as_bytes())?;
    Ok(())
}

/// Records the user's evening audit response in KB-08: by_date entry + last_date.
/// `status`: "success" or "challenge"; `lesson`: optional free text.
pub fn record_evening_audit(
    store: &KnowledgeStore,
    date: &str,
    status: &str,
    lesson: Option<&str>,
) -> Result<(), sled::Error> {
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let value = serde_json::json!({
        "date": date,
        "status": status,
        "lesson": lesson.unwrap_or("").to_string(),
        "timestamp_ms": timestamp_ms,
    });
    let key = format!("{}{}", EVENING_AUDIT_BY_DATE_PREFIX, date);
    let bytes = value.to_string().into_bytes();
    store.insert(SOMA_SLOT, &key, &bytes)?;
    store.insert(SOMA_SLOT, EVENING_AUDIT_LAST_DATE_KEY, date.as_bytes())?;
    tracing::info!(
        target: "pagi::evening_audit",
        date = %date,
        status = %status,
        "Evening audit recorded in KB-08"
    );
    Ok(())
}

/// Builds a short "focus" phrase from KB-01 energy_drains for the reflective question (e.g. "staying firm on boundaries").
fn evening_focus_from_profile(store: &KnowledgeStore) -> String {
    let bytes = match store.get(PNEUMA_SLOT, KB01_USER_PROFILE_KEY).ok().flatten() {
        Some(b) if !b.is_empty() => b,
        _ => return String::new(),
    };
    let profile: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };
    let drains: Vec<String> = profile
        .get("energy_drains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    if drains.is_empty() {
        return String::new();
    }
    if drains.len() == 1 {
        return format!("protecting your energy around {}", drains[0].to_lowercase());
    }
    format!(
        "staying firm on boundaries (e.g. {})",
        drains[0].to_lowercase()
    )
}

/// Reads the last 7 days of evening audit entries from KB-08 for weekly synthesis.
pub fn get_last_7_audits(store: &KnowledgeStore) -> Result<Vec<(String, String, String)>, sled::Error> {
    let kv = store.scan_kv(SOMA_SLOT)?;
    let mut entries: Vec<(String, String, String)> = Vec::new();
    for (key, value) in kv {
        if !key.starts_with(EVENING_AUDIT_BY_DATE_PREFIX) {
            continue;
        }
        let date = key
            .trim_start_matches(EVENING_AUDIT_BY_DATE_PREFIX)
            .to_string();
        let val: serde_json::Value = match serde_json::from_slice(&value) {
            Ok(v) => v,
            _ => continue,
        };
        let status = val
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let lesson = val
            .get("lesson")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        entries.push((date, status, lesson));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let cutoff = chrono::Utc::now() - chrono::Duration::days(7);
    let cutoff_str = cutoff.format("%Y-%m-%d").to_string();
    entries.retain(|(date, _, _)| date.as_str() >= cutoff_str.as_str());
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_prompt_before_hour() {
        let kb_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::open_path(kb_dir.path()).unwrap();
        let q = get_evening_audit_prompt(&store, "2026-02-09", 14, 18, true);
        assert!(q.is_none());
    }

    #[test]
    fn prompt_after_hour_when_not_shown() {
        let kb_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::open_path(kb_dir.path()).unwrap();
        let q = get_evening_audit_prompt(&store, "2026-02-09", 19, 18, true);
        assert!(q.is_some());
        let q = q.unwrap();
        assert!(q.to_lowercase().contains("learn"));
    }
}

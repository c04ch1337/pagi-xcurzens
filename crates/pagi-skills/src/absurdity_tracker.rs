//! AbsurdityTracker: The Absurdity Log (KB-8 / SAO).
//!
//! "A fresh smile does not erase a corrupted history."
//! System failures (disrespect, logic gaps) from KB-8 are injected into prompt context
//! when the same Subject is detected.

use pagi_core::{KnowledgeStore, KbType};
use serde::{Deserialize, Serialize};

const ABSURDITY_PREFIX: &str = "absurdity_log/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbsurdityEntry {
    pub subject_id: String,
    pub message: String,
    pub timestamp_ms: u64,
}

/// Log a system failure (disrespect, logic gap) for a subject. Stored in KB-8 (Soma).
pub fn log_failure(
    knowledge: &KnowledgeStore,
    subject_id: &str,
    message: &str,
) -> Result<(), String> {
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let key = format!("{}{}_{}", ABSURDITY_PREFIX, timestamp_ms, sanitize_subject(subject_id));
    let entry = AbsurdityEntry {
        subject_id: subject_id.to_string(),
        message: message.to_string(),
        timestamp_ms,
    };
    let bytes = serde_json::to_vec(&entry).map_err(|e| e.to_string())?;
    let soma_slot = KbType::Soma.slot_id();
    knowledge.insert(soma_slot, &key, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}

/// Retrieve past failures for a subject to inject into prompt context.
pub fn get_failures_for_subject(
    knowledge: &KnowledgeStore,
    subject_id: &str,
    limit: usize,
) -> Result<Vec<String>, String> {
    let soma_slot = KbType::Soma.slot_id();
    let kv = knowledge.scan_kv(soma_slot).map_err(|e| e.to_string())?;
    let mut entries: Vec<(u64, String)> = kv
        .into_iter()
        .filter(|(k, _)| k.starts_with(ABSURDITY_PREFIX))
        .filter_map(|(_, v)| serde_json::from_slice::<AbsurdityEntry>(&v).ok())
        .filter(|e| e.subject_id == subject_id)
        .map(|e| (e.timestamp_ms, e.message))
        .collect();
    entries.sort_by_key(|(ts, _)| std::cmp::Reverse(*ts));
    Ok(entries.into_iter().take(limit).map(|(_, m)| m).collect())
}

/// Count total absurdity log entries (for Sovereign Domain Integrity metric).
pub fn count_entries(knowledge: &KnowledgeStore) -> Result<usize, String> {
    let soma_slot = KbType::Soma.slot_id();
    let kv = knowledge.scan_kv(soma_slot).map_err(|e| e.to_string())?;
    Ok(kv.iter().filter(|(k, _)| k.starts_with(ABSURDITY_PREFIX)).count())
}

/// Build context string to inject: principle + past failures for subject.
pub fn build_context_injection(
    knowledge: &KnowledgeStore,
    subject_id: &str,
    max_entries: usize,
) -> Result<String, String> {
    let failures = get_failures_for_subject(knowledge, subject_id, max_entries)?;
    if failures.is_empty() {
        return Ok(String::new());
    }
    let mut out = String::from("Absurdity Log (KB-8): A fresh smile does not erase a corrupted history. Past system failures for this subject:\n");
    for (i, m) in failures.into_iter().enumerate() {
        out.push_str(&format!("- [{}] {}\n", i + 1, m));
    }
    Ok(out)
}

fn sanitize_subject(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(64)
        .collect()
}

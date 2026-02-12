//! WellnessReportSkill: 7-day Soma (KB-8) balance aggregation with Jungian individuation scoring.
//!
//! Uses a rolling 7-day window of Spirit/Mind/Body entries, computes pillar averages,
//! standard-deviation-based integration score, and optional archetypal flags
//! (Puer Aeternus, Shadow Dominance).

use pagi_core::{KnowledgeStore, KbType};
use std::collections::HashMap;

/// One parsed Soma balance entry (Spirit, Mind, Body 1–10, timestamp).
#[derive(Debug, Clone)]
struct SomaEntry {
    spirit: f32,
    mind: f32,
    body: f32,
    timestamp_ms: u64,
}

/// Wellness report returned by the skill.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WellnessReport {
    /// 7-day averages: "Spirit", "Mind", "Body".
    pub pillars: HashMap<String, f32>,
    /// 0.0–1.0; higher = better balance (lower std between pillars).
    pub individuation_score: f32,
    /// Short "Individuation" summary.
    pub summary: String,
    /// True if any pillar average < 3.0.
    pub is_critical: bool,
    /// Optional archetypal flags (e.g. "Puer Aeternus", "Shadow Dominance").
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<String>,
    /// Number of entries used in the 7-day window.
    pub entries_used: usize,
}

const SOMA_BALANCE_PREFIX: &str = "soma/balance_check";
const SEVEN_DAYS_MS: u64 = 7 * 24 * 60 * 60 * 1000;
/// Max std for three values in [1, 10]: e.g. (1,1,10) -> variance 18, std ~4.24.
const MAX_STD: f32 = 5.0;
const CRITICAL_THRESHOLD: f32 = 3.0;
/// Puer Aeternus: (Spirit + Mind) / 2 - Body > this => high spirit/mind, low body.
const PUER_THRESHOLD: f32 = 2.5;
/// Shadow Dominance: all three pillars below this on average.
const SHADOW_LOW_THRESHOLD: f32 = 4.0;

fn parse_soma_entry(_key: &str, bytes: &[u8]) -> Option<SomaEntry> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let obj = value.as_object()?;
    let spirit = obj.get("spirit")?.as_f64()? as f32;
    let mind = obj.get("mind")?.as_f64()? as f32;
    let body = obj.get("body")?.as_f64()? as f32;
    let timestamp_ms = obj
        .get("timestamp_ms")
        .and_then(|t| t.as_u64())
        .or_else(|| t_as_i64(obj.get("timestamp_ms")).and_then(|t| u64::try_from(t).ok()))
        .unwrap_or(0);
    if !(1.0..=10.0).contains(&spirit) || !(1.0..=10.0).contains(&mind) || !(1.0..=10.0).contains(&body) {
        return None;
    }
    Some(SomaEntry { spirit, mind, body, timestamp_ms })
}

fn t_as_i64(v: Option<&serde_json::Value>) -> Option<i64> {
    v.and_then(|t| t.as_i64())
}

/// Returns standard deviation of three values (population std).
fn std_three(a: f32, b: f32, c: f32) -> f32 {
    let mean = (a + b + c) / 3.0;
    let var = ((a - mean).powi(2) + (b - mean).powi(2) + (c - mean).powi(2)) / 3.0;
    var.sqrt()
}

/// Generates the wellness report from the last 7 days of KB-8 (Soma) balance entries.
pub fn generate_report(knowledge: &KnowledgeStore) -> Result<WellnessReport, String> {
    let soma_slot = KbType::Soma.slot_id();
    let kv = knowledge.scan_kv(soma_slot).map_err(|e| e.to_string())?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let cutoff_ms = now_ms.saturating_sub(SEVEN_DAYS_MS);

    let mut entries: Vec<SomaEntry> = kv
        .into_iter()
        .filter(|(k, _)| k == SOMA_BALANCE_PREFIX || k.starts_with(&format!("{}/", SOMA_BALANCE_PREFIX)))
        .filter_map(|(k, v)| parse_soma_entry(&k, &v))
        .filter(|e| e.timestamp_ms >= cutoff_ms)
        .collect();
    entries.sort_by_key(|e| e.timestamp_ms);

    let entries_used = entries.len();
    if entries_used == 0 {
        return Ok(WellnessReport {
            pillars: HashMap::from([
                ("Spirit".to_string(), 0.0),
                ("Mind".to_string(), 0.0),
                ("Body".to_string(), 0.0),
            ]),
            individuation_score: 0.0,
            summary: "No Soma balance entries in the last 7 days. Use the Balance Check (Spirit/Mind/Body) to start your Sovereign Health Log.".to_string(),
            is_critical: true,
            flags: vec![],
            entries_used: 0,
        });
    }

    let spirit_avg = entries.iter().map(|e| e.spirit).sum::<f32>() / entries_used as f32;
    let mind_avg = entries.iter().map(|e| e.mind).sum::<f32>() / entries_used as f32;
    let body_avg = entries.iter().map(|e| e.body).sum::<f32>() / entries_used as f32;

    let mut pillars = HashMap::new();
    pillars.insert("Spirit".to_string(), (spirit_avg * 100.0).round() / 100.0);
    pillars.insert("Mind".to_string(), (mind_avg * 100.0).round() / 100.0);
    pillars.insert("Body".to_string(), (body_avg * 100.0).round() / 100.0);

    let std = std_three(spirit_avg, mind_avg, body_avg);
    let raw_score = 1.0 - (std / MAX_STD);
    let individuation_score = raw_score.clamp(0.0, 1.0);

    let is_critical = spirit_avg < CRITICAL_THRESHOLD || mind_avg < CRITICAL_THRESHOLD || body_avg < CRITICAL_THRESHOLD;

    let mut flags = Vec::new();
    let upper_avg = (spirit_avg + mind_avg) / 2.0;
    if upper_avg - body_avg > PUER_THRESHOLD {
        flags.push("Puer Aeternus".to_string());
    }
    if spirit_avg < SHADOW_LOW_THRESHOLD && mind_avg < SHADOW_LOW_THRESHOLD && body_avg < SHADOW_LOW_THRESHOLD {
        flags.push("Shadow Dominance".to_string());
    }

    let summary = build_summary(individuation_score, is_critical, &pillars, &flags, entries_used);

    Ok(WellnessReport {
        pillars,
        individuation_score,
        summary,
        is_critical,
        flags,
        entries_used,
    })
}

fn build_summary(
    score: f32,
    is_critical: bool,
    pillars: &HashMap<String, f32>,
    flags: &[String],
    n: usize,
) -> String {
    if is_critical {
        return format!(
            "Critical: at least one pillar (Spirit/Mind/Body) is below {}. Prioritise grounding and self-care. Data from {} entries.",
            CRITICAL_THRESHOLD, n
        );
    }
    let integration = if score >= 0.7 {
        "Good integration"
    } else if score >= 0.4 {
        "Moderate integration"
    } else {
        "Tension of opposites"
    };
    let spirit = pillars.get("Spirit").copied().unwrap_or(0.0);
    let mind = pillars.get("Mind").copied().unwrap_or(0.0);
    let body = pillars.get("Body").copied().unwrap_or(0.0);
    let mut parts = vec![format!(
        "{} across Spirit ({:.1}), Mind ({:.1}), Body ({:.1}). Individuation score {:.2}. Based on {} entries.",
        integration, spirit, mind, body, score, n
    )];
    if !flags.is_empty() {
        parts.push(format!("Archetypal flags: {}.", flags.join(", ")));
    }
    parts.join(" ")
}

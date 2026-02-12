//! Sovereign Health Report (KB-08 Analytics).
//!
//! Transforms KB-08 (Absurdity Log / Soma) from a data dump into a strategic intelligence
//! briefing. Correlates success metrics with transit data to identify patterns between
//! emotional state, external transits, and boundary enforcements.

use crate::knowledge::{KnowledgeStore, ARCHETYPE_USAGE_PREFIX, SUCCESS_METRIC_PREFIX};
use crate::orchestrator::init::KB01_USER_PROFILE_KEY;
use chrono::{Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};

const PNEUMA_SLOT: u8 = 1;
const SOMA_SLOT: u8 = 8;
const REPORT_DAYS: i64 = 7;
const EVENING_AUDIT_BY_DATE_PREFIX: &str = "evening_audit/by_date/";
/// KB-08 key prefix for daily vitality (sleep/activity). Written by Vitality Shield; format vitality/daily/YYYY-MM-DD.
pub const VITALITY_DAILY_PREFIX: &str = "vitality/daily/";

/// One successful intervention logged in KB-08 (e.g. boundary enforced, Gray Rock applied).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldedEvent {
    pub timestamp_ms: u64,
    pub message: String,
    pub category: String,
    pub date: String,
}

/// Success metrics grouped by inferred category (Financial Boundary, Emotional Drain, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeakStats {
    pub by_category: std::collections::HashMap<String, u32>,
    pub total_shielded: u32,
}

/// Transit correlation entry: high-risk day on which a sovereignty event was logged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitCorrelationEntry {
    pub date: String,
    pub transit_summary: String,
    pub event_kind: String,
}

/// Sovereign Health Report: weekly KB-08 analytics for the Briefing Room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Success metrics grouped by category; total shielded count.
    pub leak_stats: LeakStats,
    /// 0.0–1.0: how much high-risk transits correlated with sovereignty events (higher = more awareness on hard days).
    pub transit_vulnerability_score: f32,
    /// High-risk days in the window with at least one sovereignty event (transit impact).
    pub transit_correlations: Vec<TransitCorrelationEntry>,
    /// Ratio of boundary/Gray Rock–type interventions vs technical execution (0.0–1.0).
    pub efficiency_score: f32,
    /// Phoenix Marie's executive summary for the week.
    pub phoenix_summary: String,
    /// User-facing label from KB-01 (e.g. "Pisces-Protector") for personalization.
    pub archetype_label: Option<String>,
    /// Display name from KB-01 when available.
    pub user_name: Option<String>,
    /// List of shielded events (success metrics) in the last 7 days.
    pub shielded_events: Vec<ShieldedEvent>,
    /// Start of the report window (YYYY-MM-DD).
    pub window_start: String,
    /// End of the report window (YYYY-MM-DD).
    pub window_end: String,
    /// Which archetypes were most active this week (e.g. 70% Virgo, 20% Pisces, 10% Capricorn). For Humanity/Archetype shift reporting.
    pub archetype_usage_breakdown: Option<ArchetypeUsageBreakdown>,
    /// 0.0–1.0 average rest (sleep) score over the week when Vitality Shield data is present. None if no vitality/daily data.
    pub vitality_score: Option<f32>,
    /// Rest (sleep hours) vs output (turns + shielded) per day for Briefing Room "Rest vs. Output" correlation graph.
    pub rest_vs_output: Vec<RestVsOutputEntry>,
}

/// One day's rest (sleep) and output (chat turns + shielded events) for Rest vs. Output correlation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestVsOutputEntry {
    pub date: String,
    /// Sleep hours (0–12 typical). 0 when unknown.
    pub rest_score: f32,
    /// Normalized output: e.g. chat turns + shielded count for that day (scale arbitrary; higher = more output).
    pub output_score: f32,
}

/// Per-archetype usage in the report window (counts and human-readable summary).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchetypeUsageBreakdown {
    /// Count per archetype name (e.g. "virgo" => 14, "pisces" => 4).
    pub by_archetype: std::collections::HashMap<String, u32>,
    /// Total turns in window.
    pub total_turns: u32,
    /// Human-readable summary, e.g. "70% Virgo (Technical), 20% Pisces (Emotional), 10% Capricorn (Strategy)."
    pub summary: String,
}

/// Records which archetype was used for this turn (for Weekly Health Report breakdown). Call from gateway after each chat response.
pub fn record_archetype_usage(store: &KnowledgeStore, archetype: &str) -> Result<(), sled::Error> {
    let ts_ms = Utc::now().timestamp_millis() as u64;
    let key = format!("{}{:016x}", ARCHETYPE_USAGE_PREFIX, ts_ms);
    let value = serde_json::json!({
        "archetype": archetype.trim().to_lowercase(),
        "timestamp_ms": ts_ms
    });
    store.insert(SOMA_SLOT, &key, &value.to_string().into_bytes())?;
    Ok(())
}

/// Infer category from success metric message (e.g. "Financial Boundary", "Emotional Drain").
fn infer_category(message: &str) -> String {
    let lower = message.to_lowercase();
    if lower.contains("financial") || lower.contains("money") || lower.contains("budget") {
        return "Financial Boundary".to_string();
    }
    if lower.contains("emotional") || lower.contains("drain") || lower.contains("savior") || lower.contains("empathy") {
        return "Emotional Drain".to_string();
    }
    if lower.contains("gray rock") || lower.contains("grey rock") || lower.contains("boundary") {
        return "Gray Rock".to_string();
    }
    if lower.contains("technical") || lower.contains("code") || lower.contains("execution") || lower.contains("focus") {
        return "Technical Execution".to_string();
    }
    "Sovereignty Leak Addressed".to_string()
}

/// Classify message as boundary/Gray Rock (true) vs technical execution (false) for efficiency score.
fn is_boundary_or_gray_rock(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("gray rock") || lower.contains("grey rock") || lower.contains("boundary")
        || lower.contains("savior") || lower.contains("drain") || lower.contains("emotional")
        || lower.contains("financial") || lower.contains("sovereignty")
}

/// Generate the weekly Sovereign Health Report from KB-08 and KB-01.
pub fn generate_weekly_report(store: &KnowledgeStore) -> Result<HealthReport, String> {
    let now = Utc::now();
    let end_ts_ms = now.timestamp_millis() as u64;
    let start_ts_ms = (now - Duration::days(REPORT_DAYS)).timestamp_millis() as u64;

    let window_start = (now - Duration::days(REPORT_DAYS)).format("%Y-%m-%d").to_string();
    let window_end = now.format("%Y-%m-%d").to_string();

    let kv = store.scan_kv(8).map_err(|e| e.to_string())?;

    let mut shielded_events: Vec<ShieldedEvent> = Vec::new();
    let mut by_category: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    let mut boundary_count = 0u32;
    let mut technical_count = 0u32;

    for (key, value) in &kv {
        if key.starts_with(SUCCESS_METRIC_PREFIX) {
            let val: serde_json::Value = match serde_json::from_slice(value) {
                Ok(v) => v,
                _ => continue,
            };
            let ts = match val.get("timestamp_ms").and_then(|v| v.as_u64()) {
                Some(t) => t,
                _ => continue,
            };
            if ts < start_ts_ms || ts > end_ts_ms {
                continue;
            }
            let message = val
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let category = val
                .get("category")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| infer_category(&message));
            let date = ms_to_date(ts);
            shielded_events.push(ShieldedEvent {
                timestamp_ms: ts,
                message: message.clone(),
                category: category.clone(),
                date,
            });
            *by_category.entry(category).or_insert(0) += 1;
            if is_boundary_or_gray_rock(&message) {
                boundary_count += 1;
            } else {
                technical_count += 1;
            }
        }
    }

    shielded_events.sort_by_key(|e| std::cmp::Reverse(e.timestamp_ms));

    let total_shielded = shielded_events.len() as u32;
    let efficiency_denom = boundary_count + technical_count;
    let efficiency_score = if efficiency_denom > 0 {
        (boundary_count as f32) / (efficiency_denom as f32)
    } else {
        0.5
    };

    let mut transit_correlations: Vec<TransitCorrelationEntry> = Vec::new();
    for (key, value) in &kv {
        if key.starts_with("transit_correlation/") {
            let val: serde_json::Value = match serde_json::from_slice(value) {
                Ok(v) => v,
                _ => continue,
            };
            let date = match val.get("date").and_then(|v| v.as_str()) {
                Some(d) => d.to_string(),
                _ => continue,
            };
            if date < window_start || date > window_end {
                continue;
            }
            let transit_summary = val
                .get("transit_summary")
                .and_then(|v| v.as_str())
                .unwrap_or("High Risk")
                .to_string();
            let event_kind = val
                .get("event_kind")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            transit_correlations.push(TransitCorrelationEntry {
                date,
                transit_summary,
                event_kind,
            });
        }
    }
    transit_correlations.sort_by(|a, b| a.date.cmp(&b.date));

    let high_risk_days_in_window = REPORT_DAYS; // we don't store daily risk history, only correlation events
    let transit_vulnerability_score = if high_risk_days_in_window > 0 && !transit_correlations.is_empty() {
        (transit_correlations.len() as f32).min(7.0) / 7.0
    } else {
        0.0
    };

    let (archetype_label, user_name) = kb01_user_context(store);

    let archetype_usage_breakdown = aggregate_archetype_usage(&kv, start_ts_ms, end_ts_ms);

    let (vitality_score, rest_vs_output) = aggregate_rest_vs_output(
        &kv,
        &shielded_events,
        &window_start,
        &window_end,
    );

    let phoenix_summary = build_phoenix_summary(
        total_shielded,
        &by_category,
        transit_correlations.len(),
        efficiency_score,
        archetype_label.as_deref(),
        archetype_usage_breakdown.as_ref(),
    );

    Ok(HealthReport {
        leak_stats: LeakStats {
            by_category,
            total_shielded,
        },
        transit_vulnerability_score,
        transit_correlations,
        efficiency_score,
        phoenix_summary,
        archetype_label,
        user_name,
        shielded_events,
        window_start,
        window_end,
        archetype_usage_breakdown,
        vitality_score,
        rest_vs_output,
    })
}

fn aggregate_archetype_usage(
    kv: &[(String, Vec<u8>)],
    start_ts_ms: u64,
    end_ts_ms: u64,
) -> Option<ArchetypeUsageBreakdown> {
    let mut by_archetype: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for (key_str, value) in kv {
        if !key_str.starts_with(ARCHETYPE_USAGE_PREFIX) {
            continue;
        }
        let val: serde_json::Value = match serde_json::from_slice(value) {
            Ok(v) => v,
            _ => continue,
        };
        let ts = match val.get("timestamp_ms").and_then(|v| v.as_u64()) {
            Some(t) => t,
            _ => continue,
        };
        if ts < start_ts_ms || ts > end_ts_ms {
            continue;
        }
        let name = val
            .get("archetype")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .trim()
            .to_lowercase();
        if !name.is_empty() {
            *by_archetype.entry(name).or_insert(0) += 1;
        }
    }
    let total_turns: u32 = by_archetype.values().sum();
    if total_turns == 0 {
        return None;
    }
    let mut pairs: Vec<_> = by_archetype.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1));
    let summary = pairs
        .iter()
        .map(|(k, v)| {
            let count = **v;
            let pct = (count as f32 / total_turns as f32 * 100.0).round() as u32;
            let label = match k.as_str() {
                "virgo" => "Virgo (Technical)",
                "pisces" => "Pisces (Emotional)",
                "capricorn" => "Capricorn (Strategy)",
                "scorpio" => "Scorpio",
                "libra" => "Libra",
                "cancer" => "Cancer",
                "leo" => "Leo",
                _ => k.as_str(),
            };
            format!("{}% {}", pct, label)
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(ArchetypeUsageBreakdown {
        by_archetype,
        total_turns,
        summary,
    })
}

fn ms_to_date(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    if let Some(dt) = Utc.timestamp_opt(secs, 0).single() {
        dt.format("%Y-%m-%d").to_string()
    } else {
        String::new()
    }
}

/// Aggregates vitality/daily/* and archetype_usage + shielded by date to produce vitality_score and rest_vs_output.
fn aggregate_rest_vs_output(
    kv: &[(String, Vec<u8>)],
    shielded_events: &[ShieldedEvent],
    window_start: &str,
    window_end: &str,
) -> (Option<f32>, Vec<RestVsOutputEntry>) {
    let mut rest_by_date: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    for (key, value) in kv {
        if !key.starts_with(VITALITY_DAILY_PREFIX) {
            continue;
        }
        let date = key.trim_start_matches(VITALITY_DAILY_PREFIX).to_string();
        let val: serde_json::Value = match serde_json::from_slice(value) {
            Ok(v) => v,
            _ => continue,
        };
        let sleep = val
            .get("sleep_hours_last_24")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .unwrap_or(0.0);
        rest_by_date.insert(date, sleep);
    }

    let mut output_by_date: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    for ev in shielded_events {
        *output_by_date.entry(ev.date.clone()).or_insert(0.0) += 1.0;
    }
    for (key, value) in kv {
        if !key.starts_with(ARCHETYPE_USAGE_PREFIX) {
            continue;
        }
        let val: serde_json::Value = match serde_json::from_slice(value) {
            Ok(v) => v,
            _ => continue,
        };
        let ts = match val.get("timestamp_ms").and_then(|v| v.as_u64()) {
            Some(t) => t,
            _ => continue,
        };
        let date = ms_to_date(ts);
        if date.as_str() >= window_start && date.as_str() <= window_end {
            *output_by_date.entry(date).or_insert(0.0) += 1.0;
        }
    }

    let mut rest_vs_output: Vec<RestVsOutputEntry> = Vec::new();
    let mut sum_rest = 0.0f32;
    let mut rest_count = 0u32;
    let start_naive = match chrono::NaiveDate::parse_from_str(window_start, "%Y-%m-%d") {
        Ok(d) => d,
        _ => return (None, rest_vs_output),
    };
    let end_naive = chrono::NaiveDate::parse_from_str(window_end, "%Y-%m-%d").unwrap_or(start_naive);
    let mut d = start_naive;
    loop {
        let date = d.format("%Y-%m-%d").to_string();
        let rest_score = rest_by_date.get(&date).copied().unwrap_or(0.0);
        if rest_score > 0.0 {
            sum_rest += rest_score;
            rest_count += 1;
        }
        let output_score = output_by_date.get(&date).copied().unwrap_or(0.0);
        rest_vs_output.push(RestVsOutputEntry {
            date,
            rest_score,
            output_score,
        });
        if d >= end_naive {
            break;
        }
        d = match d.succ_opt() {
            Some(next) => next,
            None => break,
        };
    }

    let vitality_score = if rest_count > 0 {
        Some(sum_rest / (rest_count as f32))
    } else {
        None
    };
    (vitality_score, rest_vs_output)
}

fn kb01_user_context(store: &KnowledgeStore) -> (Option<String>, Option<String>) {
    let bytes: Vec<u8> = match store.get(PNEUMA_SLOT, KB01_USER_PROFILE_KEY) {
        Ok(Some(b)) => b,
        _ => return (None, None),
    };
    let val: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        _ => return (None, None),
    };
    let obj = match val.as_object() {
        Some(o) => o,
        _ => return (None, None),
    };
    let archetype = obj
        .get("astro_archetype")
        .or_else(|| obj.get("archetype"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let name = obj
        .get("name")
        .or_else(|| obj.get("user_name"))
        .and_then(|v| v.as_str())
        .map(String::from);
    (archetype, name)
}

fn build_phoenix_summary(
    total_shielded: u32,
    by_category: &std::collections::HashMap<String, u32>,
    transit_events: usize,
    efficiency_score: f32,
    archetype: Option<&str>,
    archetype_usage: Option<&ArchetypeUsageBreakdown>,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    if total_shielded > 0 {
        lines.push(format!(
            "This week you logged {} successful boundary enforcement{}.",
            total_shielded,
            if total_shielded == 1 { "" } else { "s" }
        ));
        if !by_category.is_empty() {
            let top: Vec<_> = by_category.iter().collect();
            let categories: String = top
                .iter()
                .map(|(k, v)| format!("{} ({})", k, v))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("Categories: {}.", categories));
        }
    } else {
        lines.push("No shielded events were logged in the last 7 days. Consider logging when you enforce a boundary or apply Gray Rock.".to_string());
    }
    if transit_events > 0 {
        lines.push(format!(
            "High-risk transit days with sovereignty events: {}. Your system is correlating transits with your responses—use this to prepare for similar days ahead.",
            transit_events
        ));
    }
    if efficiency_score >= 0.6 {
        lines.push("Efficiency: You leaned into boundary and Gray Rock advice this week. Strong sovereign posture.".to_string());
    } else if efficiency_score <= 0.4 && (total_shielded > 0) {
        lines.push("Efficiency: More technical execution than boundary focus this week. Balance is key during difficult transits.".to_string());
    }
    if let Some(usage) = archetype_usage {
        if usage.total_turns > 0 {
            lines.push(format!(
                "Archetype focus this week: {}. Use this to see how your Humanity and domain (Technical / Emotional / Strategy) shifted over time.",
                usage.summary
            ));
        }
    }
    if let Some(a) = archetype {
        if a.to_lowercase().contains("pisces") {
            lines.push("Pisces strengths: Your empathy is an asset; on high-risk days, channel it into clear boundaries rather than over-giving. You held the line this week.".to_string());
        }
    }
    if lines.is_empty() {
        lines.push("Review your KB-08 success metrics and transit correlations in the Briefing Room to spot patterns. Prepare for the week ahead.".to_string());
    }
    lines.join(" ")
}

/// Aggregates the last 7 days of evening audit entries (KB-08) into a short sovereignty summary.
/// E.g. "You were 80% successful in protecting energy this week (4 of 5 reflections)."
/// Feeds the Sovereign Health Report / weekly briefing. Non-judgmental tone.
pub fn generate_weekly_sovereignty_report(store: &KnowledgeStore) -> String {
    let now = Utc::now();
    let cutoff = (now - Duration::days(REPORT_DAYS)).format("%Y-%m-%d").to_string();
    let kv = match store.scan_kv(SOMA_SLOT) {
        Ok(k) => k,
        Err(_) => return String::new(),
    };
    let mut success_count = 0u32;
    let mut challenge_count = 0u32;
    let mut total = 0u32;
    for (key, value) in &kv {
        if !key.starts_with(EVENING_AUDIT_BY_DATE_PREFIX) {
            continue;
        }
        let date = key
            .trim_start_matches(EVENING_AUDIT_BY_DATE_PREFIX)
            .to_string();
        if date < cutoff {
            continue;
        }
        let val: serde_json::Value = match serde_json::from_slice(value) {
            Ok(v) => v,
            _ => continue,
        };
        total += 1;
        let status = val.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if status.eq_ignore_ascii_case("success") {
            success_count += 1;
        } else if status.eq_ignore_ascii_case("challenge") {
            challenge_count += 1;
        }
    }
    if total == 0 {
        return "No evening reflections this week. When you use the evening audit, your trends will appear here.".to_string();
    }
    let pct = if total > 0 {
        (success_count as f32 / total as f32 * 100.0).round() as u32
    } else {
        0
    };
    format!(
        "You reflected on your energy {} time{} this week: {} success{}, {} challenge{}. You were {}% successful in protecting energy.",
        total,
        if total == 1 { "" } else { "s" },
        success_count,
        if success_count == 1 { "" } else { "es" },
        challenge_count,
        if challenge_count == 1 { "" } else { "s" },
        pct,
    )
}

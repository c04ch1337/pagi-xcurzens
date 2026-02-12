//! **Daily Check-in** — Morning Briefing protocol.
//!
//! Bridges environmental scan (Astro-Transit from KB-07 / astro-weather) with the user's
//! internal stability (personality profile and energy drains from KB-01). Produces a
//! concise 1–2 sentence boundary tip for the first interaction of the day when
//! `PAGI_DAILY_CHECKIN_ENABLED` is true. Human-centric language (e.g. "energy habits"
//! not "sovereignty leaks").

use crate::ms_graph::{is_low_sleep, UserVitality};
use pagi_core::{AstroWeatherState, KnowledgeStore, TransitRiskLevel, KB01_USER_PROFILE_KEY};

const PNEUMA_SLOT: u8 = 1;

/// Key in KB-08 (Soma) for the last date a morning briefing was shown (value: "YYYY-MM-DD").
pub const DAILY_CHECKIN_LAST_DATE_KEY: &str = "daily_checkin/last_date";

/// Generates a 1–2 sentence morning briefing by combining:
/// - **Environmental scan:** current transit risk (from astro-weather).
/// - **Vulnerability match:** user's "energy_drains" from KB-01 profile.
/// - **Sovereignty tip:** human-centric boundary strategy for the day.
/// - **Vitality Shield:** when vitality indicates sleep < 6h, appends "I notice your sleep was low. I'll stay brief today to save your energy."
///
/// Uses "energy habits" and "boundaries" language. Call on first user interaction
/// of the day when `PAGI_DAILY_CHECKIN_ENABLED` is true; then persist today's date
/// to KB-08 under `DAILY_CHECKIN_LAST_DATE_KEY` so it runs only once per day.
pub fn generate_morning_briefing(
    store: &KnowledgeStore,
    astro: &AstroWeatherState,
    vitality: Option<&UserVitality>,
) -> String {
    let low_sleep_suffix = if is_low_sleep(vitality) {
        " I notice your sleep was low. I'll stay brief today to save your energy. "
    } else {
        ""
    };
    let bytes = match store.get(PNEUMA_SLOT, KB01_USER_PROFILE_KEY).ok().flatten() {
        Some(b) if !b.is_empty() => b,
        _ => {
            return if astro.risk.is_high_risk() {
                format!("Good morning. Today's environment may bring more tension than usual. Keep your boundaries firm.{}", low_sleep_suffix)
            } else {
                format!("Good morning. No particular energy alerts today.{}", low_sleep_suffix)
            };
        }
    };

    let profile: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            return if astro.risk.is_high_risk() {
                format!("Good morning. Today might feel more pressured; hold your boundaries.{}", low_sleep_suffix)
            } else {
                format!("Good morning.{}", low_sleep_suffix)
            };
        }
    };

    let energy_drains: Vec<String> = profile
        .get("energy_drains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.trim().to_lowercase()))
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let has_high_risk = astro.risk.is_high_risk();
    let elevated = matches!(astro.risk, TransitRiskLevel::Elevated);

    if !has_high_risk && !elevated {
        return if low_sleep_suffix.is_empty() {
            "Good morning. No particular energy alerts today. ".to_string()
        } else {
            format!("Good morning.{}", low_sleep_suffix)
        };
    }

    // Match transit pressure to common energy-drain themes for a tailored tip
    let transit_lower = astro.transit_summary.to_lowercase();
    let advice_lower = astro.advice.to_lowercase();
    let pressure_keywords = [
        "pressure",
        "over-commit",
        "saying no",
        "people pleas",
        "burnout",
        "boundary",
        "irritab",
        "tension",
        "stress",
    ];

    let drain_matches_pressure = energy_drains.iter().any(|drain| {
        pressure_keywords
            .iter()
            .any(|kw| drain.contains(kw) || kw.contains(&drain.as_str()))
    });
    let transit_suggests_pressure = pressure_keywords
        .iter()
        .any(|kw| transit_lower.contains(kw) || advice_lower.contains(kw));

    if drain_matches_pressure || transit_suggests_pressure || has_high_risk {
        return format!(
            "Good morning. Before we dive in: today's environment might make you feel a bit more pressured to over-commit. Keep your boundaries firm today.{}",
            low_sleep_suffix
        );
    }

    // Elevated or high risk but no specific drain match: generic supportive line
    let base = if has_high_risk {
        "Good morning. Today may bring more tension than usual; go easy on yourself and hold your boundaries. "
    } else {
        "Good morning. A gentle heads-up: you might feel a bit more stretched today. Protect your energy. "
    };
    format!("{}{}", base, low_sleep_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_profile_stable_returns_short_greeting() {
        let kb_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::open_path(kb_dir.path()).unwrap();
        let astro = AstroWeatherState::default();
        let s = generate_morning_briefing(&store, &astro, None);
        assert!(s.starts_with("Good morning"));
        assert!(!s.contains("sovereignty"));
    }

    #[test]
    fn high_risk_no_profile_mentions_boundaries() {
        let kb_dir = tempfile::tempdir().unwrap();
        let store = KnowledgeStore::open_path(kb_dir.path()).unwrap();
        let astro = AstroWeatherState {
            risk: TransitRiskLevel::HighRisk,
            transit_summary: "Mars square Sun".to_string(),
            advice: "Lean into Gray Rock".to_string(),
            updated_at_ms: 0,
        };
        let s = generate_morning_briefing(&store, &astro, None);
        assert!(s.to_lowercase().contains("boundary"));
    }
}

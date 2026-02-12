//! Astro-Weather: Planetary Transit Scraper & Proactive Warning System.
//!
//! Compares current transits (from a lightweight model or optional API) against the user's
//! birth chart in KB-01 (Sun/Moon/Rising). When a "harsh transit" is detected (e.g. Mars
//! Square Sun), produces a SystemAlert-level warning and advice for the SYSTEM_PROMPT.
//! High-risk days can be correlated with KB-08 (Sovereignty Leaks) to refine predictive accuracy.

use crate::knowledge::KnowledgeStore;
use crate::orchestrator::init::KB01_USER_PROFILE_KEY;
use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};

const PNEUMA_SLOT: u8 = 1;

/// Risk level for today's transits relative to the user's chart (KB-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitRiskLevel {
    /// No significant harsh transits; standard boundary advice.
    Stable,
    /// Some tension; gentle reminder to lean into Gray Rock when needed.
    Elevated,
    /// Harsh transit (e.g. Mars Square Sun): high irritability and sovereignty leak risk.
    HighRisk,
}

impl Default for TransitRiskLevel {
    fn default() -> Self {
        Self::Stable
    }
}

impl TransitRiskLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Elevated => "elevated",
            Self::HighRisk => "high_risk",
        }
    }

    pub fn is_high_risk(&self) -> bool {
        matches!(self, Self::HighRisk)
    }
}

/// Cached astro-weather state for SYSTEM_PROMPT injection and UI widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstroWeatherState {
    pub risk: TransitRiskLevel,
    /// Short label for UI (e.g. "Mars Square Sun" or "No harsh transits").
    pub transit_summary: String,
    /// Advice for the LLM (e.g. "Lean into Gray Rock protocols").
    pub advice: String,
    /// Unix timestamp (ms) when this state was computed.
    pub updated_at_ms: u64,
}

impl Default for AstroWeatherState {
    fn default() -> Self {
        Self {
            risk: TransitRiskLevel::Stable,
            transit_summary: String::from("No transit data"),
            advice: String::from("Standard boundary awareness."),
            updated_at_ms: 0,
        }
    }
}

/// Zodiac sign index (0–11) for aspect math. Order: Aries=0 .. Pisces=11.
fn sign_index(sign: &str) -> Option<u8> {
    let s = sign.trim().to_lowercase();
    let idx = match s.as_str() {
        "aries" => 0,
        "taurus" => 1,
        "gemini" => 2,
        "cancer" => 3,
        "leo" => 4,
        "virgo" => 5,
        "libra" => 6,
        "scorpio" => 7,
        "sagittarius" => 8,
        "capricorn" => 9,
        "aquarius" => 10,
        "pisces" => 11,
        _ => return None,
    };
    Some(idx)
}

/// Aspect distance in signs: 0 = conj, 3 = square, 6 = opposition, 4 = trine (easy).
fn aspect_distance(a: u8, b: u8) -> u8 {
    let d = if a >= b { a - b } else { b - a };
    if d <= 6 { d } else { 12 - d }
}

/// Harsh aspects: square (3), opposition (6). Easy: trine (4), sextile (2). Conjunction (0) can be tense for Mars.
fn is_harsh_aspect(distance: u8) -> bool {
    matches!(distance, 3 | 6) || (distance == 0) // Mars conjunct Sun/Moon/Rising = tense
}

/// Parses KB-01 user_profile for Sun/Moon/Rising. Accepts keys: sun, moon, rising, or archetype (e.g. "Pisces/Virgo/Gemini").
fn user_chart_from_store(store: &KnowledgeStore) -> Option<UserChart> {
    let bytes = store.get(PNEUMA_SLOT, KB01_USER_PROFILE_KEY).ok().flatten()?;
    if bytes.is_empty() {
        return None;
    }
    let val = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    let obj = val.as_object()?;

    let sun = obj
        .get("sun")
        .or_else(|| obj.get("Sun"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase().to_string());
    let moon = obj
        .get("moon")
        .or_else(|| obj.get("Moon"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase().to_string());
    let rising = obj
        .get("rising")
        .or_else(|| obj.get("Rising"))
        .or_else(|| obj.get("ascendant"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_lowercase().to_string());

    // Fallback: single "archetype" or "archetype_raw" like "Pisces/Virgo/Gemini"
    let (sun, moon, rising) = if sun.is_some() || moon.is_some() || rising.is_some() {
        (sun, moon, rising)
    } else {
        let raw = obj
            .get("archetype")
            .or_else(|| obj.get("archetype_raw"))
            .and_then(|v| v.as_str())?;
        let parts: Vec<&str> = raw.split('/').map(str::trim).collect();
        let sun = parts.get(0).map(|s| s.to_lowercase().to_string());
        let moon = parts.get(1).map(|s| s.to_lowercase().to_string());
        let rising = parts.get(2).map(|s| s.to_lowercase().to_string());
        (sun, moon, rising)
    };

    if sun.is_none() && moon.is_none() && rising.is_none() {
        return None;
    }
    Some(UserChart { sun, moon, rising })
}

#[derive(Debug, Default)]
struct UserChart {
    sun: Option<String>,
    moon: Option<String>,
    rising: Option<String>,
}

/// Lightweight "current Mars" position: deterministic by day-of-year so we don't require an API.
/// Mars spends ~2 months per sign; we approximate with (day_of_year * 12 / 365) % 12.
fn simulated_mars_sign_index(now: chrono::DateTime<Utc>) -> u8 {
    let doy = now.ordinal() as u32;
    ((doy as u16).wrapping_mul(12) / 365) as u8 % 12
}

/// Check astro-weather: compare simulated transits to user chart and return state.
pub fn check_astro_weather(store: &KnowledgeStore) -> AstroWeatherState {
    let now = Utc::now();
    let updated_at_ms = now
        .timestamp_millis()
        .try_into()
        .unwrap_or(0_u64);

    let user = match user_chart_from_store(store) {
        Some(u) => u,
        None => {
            return AstroWeatherState {
                risk: TransitRiskLevel::Stable,
                transit_summary: String::from("No birth chart in KB-01"),
                advice: String::from("Standard boundary awareness. Add Sun/Moon/Rising to KB-01 for transit alerts."),
                updated_at_ms,
            };
        }
    };

    let mars_idx = simulated_mars_sign_index(now);

    let mut harsh_any = false;
    let mut summary_parts: Vec<String> = Vec::new();

    for (place, sign_opt) in [
        ("Sun", user.sun.as_deref()),
        ("Moon", user.moon.as_deref()),
        ("Rising", user.rising.as_deref()),
    ] {
        if let Some(sign) = sign_opt.and_then(|s| if s.is_empty() { None } else { Some(s) }) {
            if let Some(user_idx) = sign_index(sign) {
                let dist = aspect_distance(mars_idx, user_idx);
                if is_harsh_aspect(dist) {
                    harsh_any = true;
                    let aspect = match dist {
                        0 => "conjunct",
                        3 => "square",
                        6 => "opposition",
                        _ => "aspect",
                    };
                    summary_parts.push(format!("Mars {} {}", aspect, place));
                }
            }
        }
    }

    let (risk, transit_summary, advice) = if harsh_any {
        let summary = if summary_parts.is_empty() {
            "Mars in harsh aspect to personal points".to_string()
        } else {
            summary_parts.join("; ")
        };
        (
            TransitRiskLevel::HighRisk,
            summary,
            "Risk: High irritability and sovereignty leaks. Lean into Gray Rock protocols; defer non-essential boundary tests.",
        )
    } else {
        (
            TransitRiskLevel::Stable,
            "No harsh transits today".to_string(),
            "Standard boundary awareness. Proceed with usual protocols.",
        )
    };

    AstroWeatherState {
        risk,
        transit_summary,
        advice: advice.to_string(),
        updated_at_ms,
    }
}

/// SystemAlert-equivalent: one-line message for logging when high-risk transit is active.
pub fn system_alert_if_high_risk(state: &AstroWeatherState) -> Option<String> {
    if state.risk.is_high_risk() {
        Some(format!(
            "Astro-Weather High Risk: {}. {}",
            state.transit_summary, state.advice
        ))
    } else {
        None
    }
}

/// Format block for injection into SYSTEM_PROMPT.
pub fn system_prompt_block(state: &AstroWeatherState) -> String {
    format!(
        "Today's Transit: {}. Risk: {}. Advice: {}",
        state.transit_summary,
        match state.risk {
            TransitRiskLevel::Stable => "Stable",
            TransitRiskLevel::Elevated => "Elevated",
            TransitRiskLevel::HighRisk => "High (irritability and sovereignty leaks more likely)",
        },
        state.advice
    )
}

/// Records a "transit correlation" entry in KB-08 when today is high-risk and a sovereignty
/// event is logged (success metric or absurdity). Used to refine predictive accuracy.
pub fn record_transit_correlation_if_high_risk(
    store: &KnowledgeStore,
    state: &AstroWeatherState,
    event_kind: &str,
) -> Result<(), sled::Error> {
    if !state.risk.is_high_risk() {
        return Ok(());
    }
    const SOMA_SLOT: u8 = 8;
    let date = Utc::now().format("%Y-%m-%d").to_string();
    let key = format!("transit_correlation/{}", date);
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let value = serde_json::json!({
        "date": date,
        "transit_high_risk": true,
        "event_kind": event_kind,
        "transit_summary": state.transit_summary,
        "timestamp_ms": timestamp_ms,
    });
    let bytes = value.to_string().into_bytes();
    store.insert(SOMA_SLOT, &key, &bytes)?;
    tracing::info!(
        target: "pagi::astro",
        date = %date,
        event = event_kind,
        "KB-08 transit correlation logged (High Risk + sovereignty event)"
    );
    Ok(())
}

/// Stale threshold: refresh if older than 6 hours.
pub const STALE_MS: u64 = 6 * 60 * 60 * 1000;

/// Returns true if state should be refreshed (missing or stale).
pub fn should_refresh(state: &AstroWeatherState, now_ms: u64) -> bool {
    state.updated_at_ms == 0 || now_ms.saturating_sub(state.updated_at_ms) > STALE_MS
}

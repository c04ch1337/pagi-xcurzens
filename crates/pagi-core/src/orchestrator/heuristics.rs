//! HeuristicProcessor: identify External Resource Drains and High-Input/Low-ROI requests.
//!
//! When a request offers Low-ROI, the orchestrator MUST provide a "Sovereign Override" counsel.
//! Domain-neutral: uses SovereignDomain and SovereignAttributes (capacity, load, status).

use super::traits::{Heuristic, ManeuverOutcome, Protector, RoiResult, ThreatContext, VitalityLevel};
use crate::shared::SovereignAttributes;
use serde::{Deserialize, Serialize};

/// Sovereign Domain: the conceptual boundary the Protector defends (portable across locations).
/// Uses SovereignAttributes from config (capacity, load, status); domain-neutral.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignDomain {
    #[serde(default)]
    pub attributes: SovereignAttributes,
    /// Optional label (e.g. "Home", "Work") for logging only—no hardcoded names.
    #[serde(default)]
    pub label: Option<String>,
}

impl Default for SovereignDomain {
    fn default() -> Self {
        Self {
            attributes: SovereignAttributes::default(),
            label: None,
        }
    }
}

/// HeuristicProcessor: identifies external resource drains and low-ROI requests;
/// produces Sovereign Override counsel when appropriate.
pub struct HeuristicProcessor {
    domain: SovereignDomain,
}

impl HeuristicProcessor {
    pub fn new(domain: SovereignDomain) -> Self {
        Self { domain }
    }

    /// Identify if the context suggests an external resource drain or high-input/low-ROI.
    pub fn process(&self, context: &ThreatContext) -> HeuristicResult {
        let roi = self.calculate_roi(context);
        let threat = self.analyze_threat(context);
        let sovereign_override_counsel = if roi.is_low_roi {
            Some(format!(
                "Sovereign Override: {} Prioritize system stability over external accommodation.",
                roi.reason
            ))
        } else {
            None
        };
        HeuristicResult {
            roi,
            threat_analysis: threat,
            sovereign_override_counsel,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicResult {
    pub roi: RoiResult,
    pub threat_analysis: Option<String>,
    pub sovereign_override_counsel: Option<String>,
}

impl Protector for HeuristicProcessor {
    fn analyze_threat(&self, context: &ThreatContext) -> Option<String> {
        if context.situation.is_empty() {
            return None;
        }
        let s = context.situation.to_lowercase();
        if s.contains("drain") || s.contains("demand") || s.contains("guilt") || s.contains("obligation") {
            return Some("Potential external resource drain or guilt-driven demand.".to_string());
        }
        if context.emotional_valence.as_deref().map(|v| v.eq_ignore_ascii_case("guilt")).unwrap_or(false)
            || context.emotional_valence.as_deref().map(|v| v.eq_ignore_ascii_case("grief")).unwrap_or(false)
        {
            return Some("Emotional state (guilt/grief) may compromise sovereign boundaries.".to_string());
        }
        None
    }

    fn calculate_roi(&self, input: &ThreatContext) -> RoiResult {
        let s = input.situation.to_lowercase();
        let mut score = 0.5_f64;
        if s.contains("reciproc") || s.contains("mutual") {
            score += 0.2;
        }
        if s.contains("one-way") || s.contains("again") || s.contains("recurring") {
            score -= 0.3;
        }
        if input.emotional_valence.as_deref().map(|v| v.eq_ignore_ascii_case("guilt")).unwrap_or(false) {
            score -= 0.2;
        }
        let score = score.clamp(0.0, 1.0);
        let is_low_roi = score < 0.4;
        let reason = if is_low_roi {
            "High input, low return for the sovereign system."
        } else {
            "ROI within acceptable range."
        };
        RoiResult {
            score,
            is_low_roi,
            reason: reason.to_string(),
        }
    }

    fn execute_maneuver(&self, heuristic: &Heuristic) -> ManeuverOutcome {
        match heuristic.id.as_str() {
            "sovereign_override" => ManeuverOutcome {
                applied: true,
                message: "Sovereign Override counsel applied: protect system stability.".to_string(),
            },
            "boundary_hold" => ManeuverOutcome {
                applied: true,
                message: "Boundary hold: no accommodation beyond current limits.".to_string(),
            },
            _ => ManeuverOutcome {
                applied: false,
                message: format!("Unknown heuristic: {}", heuristic.id),
            },
        }
    }

    fn evaluate_vitality(&self, attributes: &SovereignAttributes) -> Option<VitalityLevel> {
        if let Some(ref s) = attributes.status {
            let lower = s.to_lowercase();
            return Some(if lower.contains("critical") {
                VitalityLevel::Critical
            } else if lower.contains("draining") {
                VitalityLevel::Draining
            } else {
                VitalityLevel::Stable
            });
        }
        let cap = attributes.capacity?;
        let load = attributes.load.unwrap_or(0.0);
        if cap <= 0.0 {
            return Some(VitalityLevel::Stable);
        }
        let ratio = load / cap;
        Some(if ratio >= 1.0 || ratio > 0.9 {
            VitalityLevel::Critical
        } else if ratio > 0.6 {
            VitalityLevel::Draining
        } else {
            VitalityLevel::Stable
        })
    }
}

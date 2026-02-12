//! Universal Protector trait: domain-agnostic logic for Sovereign AGI Orchestrator (SAO).
//!
//! Prioritizes **System Stability** over External Emotional Comfort.

use crate::shared::SovereignAttributes;
use serde::{Deserialize, Serialize};

/// Context passed to threat analysis (domain-neutral: no specific locations or names).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreatContext {
    /// Free-form description of the situation (e.g. "Incoming request for time/attention").
    pub situation: String,
    /// Optional subject identifier (e.g. hashed or role, not personal name).
    pub subject_id: Option<String>,
    /// Recent emotional valence if known (e.g. "guilt", "grief", "neutral").
    pub emotional_valence: Option<String>,
}

/// Result of ROI calculation: whether the request is worth system resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoiResult {
    /// Score in [0.0, 1.0]; higher = better return for the sovereign system.
    pub score: f64,
    /// True if the request is high-input and low-ROI (candidate for Sovereign Override).
    pub is_low_roi: bool,
    /// Short reason (e.g. "Recurring drain with no reciprocity").
    pub reason: String,
}

/// Heuristic identifier for execute_maneuver (e.g. "sovereign_override", "boundary_hold").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heuristic {
    pub id: String,
    pub params: Option<serde_json::Value>,
}

/// Outcome of executing a protector maneuver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManeuverOutcome {
    pub applied: bool,
    pub message: String,
}

/// System vitality status (generic: cognitive load, energy, or vertical-specific).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VitalityLevel {
    /// Capacity healthy relative to load.
    Stable,
    /// Load elevated; conserve energy and escalate protocols (e.g. Gray Rock for rank 0–4).
    Draining,
    /// Critical: minimal capacity or overload; full defensive posture.
    Critical,
}

/// Universal Protector trait: analyze threats, calculate ROI, execute maneuvers.
/// Implementations prioritize **System Stability** over External Emotional Comfort.
pub trait Protector {
    /// Analyze the context for threats to sovereign stability (resource drain, boundary erosion).
    fn analyze_threat(&self, context: &ThreatContext) -> Option<String>;

    /// Calculate return-on-investment for the given input (time, attention, resource).
    /// Low ROI should trigger Sovereign Override counsel.
    fn calculate_roi(&self, input: &ThreatContext) -> RoiResult;

    /// Execute a maneuver (e.g. sovereign override counsel, boundary hold).
    fn execute_maneuver(&self, heuristic: &Heuristic) -> ManeuverOutcome;

    /// Boundary scan: evaluate vitality from sovereign attributes (capacity, load, status).
    /// When returning `Draining` or `Critical`, SAO may escalate Social Protocols (KB-05) for low-rank subjects to Gray Rock.
    fn evaluate_vitality(&self, attributes: &SovereignAttributes) -> Option<VitalityLevel> {
        let _ = attributes;
        None
    }
}

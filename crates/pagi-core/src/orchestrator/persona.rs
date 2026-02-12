//! Orchestrator Role & Archetype layer (Sovereign Base)
//!
//! OrchestratorMode defines the **system role**: Counselor (default) is the core functional layer
//! for guidance and boundary management. UserArchetype (birth sign, Jungian shadow) drives
//! context injection. PersonaCoordinator is the switchboard; ZodiacBridge maps signs to
//! behavioral triggers for personalized interventions.
//!
//! **Archetype Overlays** (Pisces, Virgo, Scorpio, Libra, Cancer, Capricorn, Leo) are tone/style overlays
//! configurable via PAGI_PRIMARY_ARCHETYPE, PAGI_SECONDARY_ARCHETYPE, PAGI_ARCHETYPE_OVERRIDE.
//! The Architect (logic, boundaries, ethics) is always the mandatory core; the overlay only affects tone.
//!
//! **Archetype Gallery**: Auto-switch suggests an overlay from the user query domain (Technical→Virgo,
//! Emotional→Pisces, Strategic→Capricorn) unless KB-01 disables it (e.g. "Always be direct").

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use super::traits::{Protector, ThreatContext, RoiResult, Heuristic, ManeuverOutcome};
use super::sovereign_voice::SOVEREIGN_VOICE_PROMPT;
use crate::shared::{SovereignConfig, THERAPIST_FIT_CHECKLIST_PROMPT};

// ---------------------------------------------------------------------------
// OrchestratorMode: System role (Counselor = base; Companion = legacy overlay)
// ---------------------------------------------------------------------------

/// Orchestrator role: the functional layer for system guidance and boundary management.
/// **Counselor** (default) = Mental Health / Intervention (Ethos + Soma priority). Base template only.
/// Companion = legacy overlay (empathetic/social); kept for backward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrchestratorMode {
    /// Mental health / intervention; high priority on Ethos (guardrails) and Soma (health). Base default.
    #[default]
    Counselor,
    /// Legacy overlay: empathetic, social; high priority on Kardia and Pneuma.
    Companion,
}

impl OrchestratorMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrchestratorMode::Companion => "companion",
            OrchestratorMode::Counselor => "counselor",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim() {
            s if s.eq_ignore_ascii_case("counselor") => OrchestratorMode::Counselor,
            s if s.eq_ignore_ascii_case("companion") => OrchestratorMode::Companion,
            _ => OrchestratorMode::Counselor, // default: counselor (including empty / unknown)
        }
    }

    #[inline]
    pub fn is_counselor(&self) -> bool {
        matches!(self, OrchestratorMode::Counselor)
    }
}

// ---------------------------------------------------------------------------
// ArchetypeOverlay: Tone/Style overlay for the Advisor (counseling mode)
// ---------------------------------------------------------------------------

/// Zodiac-derived tone overlay for the Orchestrator's Advisor voice (Archetype Gallery).
/// Swappable via PAGI_PRIMARY_ARCHETYPE / PAGI_ARCHETYPE_OVERRIDE or auto-switch by query domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchetypeOverlay {
    /// Empathetic, intuitive; high warmth, open-ended reflection. Default for Counseling.
    #[default]
    Pisces,
    /// Structured, practical; checklists, logic-gaps, efficiency. Technical/code/math.
    Virgo,
    /// Deep, psychological; shadow and root causes.
    Scorpio,
    /// Balanced, mediative; fairness and interpersonal harmony.
    Libra,
    /// Nurturing, protective; emotional safety and attachment.
    Cancer,
    /// Strategic, ambitious; career and long-term planning. High precision on goals.
    Capricorn,
    /// Expressive, visible; strong self-expression and clarity. Confidence and leadership tone.
    Leo,
}

/// Trait for the Archetype Gallery: each overlay provides unique injection text (metaphors and tone).
pub trait ArchetypePrompt {
    /// Returns the system-prompt injection text for this archetype (tone only; Architect core is fixed).
    fn get_injection_text(&self) -> &'static str;
}

impl ArchetypePrompt for ArchetypeOverlay {
    fn get_injection_text(&self) -> &'static str {
        self.counseling_style_prompt()
    }
}

impl ArchetypeOverlay {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArchetypeOverlay::Pisces => "pisces",
            ArchetypeOverlay::Virgo => "virgo",
            ArchetypeOverlay::Scorpio => "scorpio",
            ArchetypeOverlay::Libra => "libra",
            ArchetypeOverlay::Cancer => "cancer",
            ArchetypeOverlay::Capricorn => "capricorn",
            ArchetypeOverlay::Leo => "leo",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "pisces" => Some(ArchetypeOverlay::Pisces),
            "virgo" => Some(ArchetypeOverlay::Virgo),
            "scorpio" => Some(ArchetypeOverlay::Scorpio),
            "libra" => Some(ArchetypeOverlay::Libra),
            "cancer" => Some(ArchetypeOverlay::Cancer),
            "capricorn" => Some(ArchetypeOverlay::Capricorn),
            "leo" => Some(ArchetypeOverlay::Leo),
            _ => None,
        }
    }

    /// Counseling-style prompt injection for this overlay (tone only; Architect handles boundaries/ethics).
    pub fn counseling_style_prompt(&self) -> &'static str {
        match self {
            ArchetypeOverlay::Pisces => "Archetype overlay (Pisces/Advisor): Use high warmth and open-ended reflection. \
                Sit comfortably in gray areas; lead with empathy and emotional depth. \
                Boundaries and ethics remain governed by the Architect core.",
            ArchetypeOverlay::Virgo => "Archetype overlay (Virgo/Architect): Use structured, practical support. \
                Offer checklists, identify logic-gaps, and emphasize efficiency. High precision on code and math. \
                Boundaries and ethics remain governed by the Architect core.",
            ArchetypeOverlay::Scorpio => "Archetype overlay (Scorpio): Go deep and psychological. \
                Focus on root causes and shadow work when appropriate. \
                Boundaries and ethics remain governed by the Architect core.",
            ArchetypeOverlay::Libra => "Archetype overlay (Libra): Balance and mediate. \
                Emphasize fairness and interpersonal harmony. \
                Boundaries and ethics remain governed by the Architect core.",
            ArchetypeOverlay::Cancer => "Archetype overlay (Cancer): Nurturing and protective. \
                Prioritize emotional safety and attachment-aware support. \
                Boundaries and ethics remain governed by the Architect core.",
            ArchetypeOverlay::Capricorn => "Archetype overlay (Capricorn/Strategist): Lead with ambition and clarity on goals. \
                Focus on career, strategy, and long-term planning. Be direct and outcome-oriented. \
                Boundaries and ethics remain governed by the Architect core.",
            ArchetypeOverlay::Leo => "Archetype overlay (Leo): Strong self-expression and visibility. \
                Use confident, clear language; affirm the user's agency and leadership. \
                Boundaries and ethics remain governed by the Architect core.",
        }
    }
}

/// Trait for config that provides archetype overlay env (primary, secondary, override, auto-switch).
pub trait ArchetypeConfig {
    fn primary_archetype_str(&self) -> Option<&str>;
    fn secondary_archetype_str(&self) -> Option<&str>;
    fn archetype_override_str(&self) -> Option<&str>;
    /// When false, auto-switch from query domain is disabled (use primary only). Default true.
    fn archetype_auto_switch_enabled(&self) -> bool {
        true
    }
}

impl ArchetypeConfig for SovereignConfig {
    fn primary_archetype_str(&self) -> Option<&str> {
        self.primary_archetype.as_deref().filter(|s| !s.is_empty())
    }
    fn secondary_archetype_str(&self) -> Option<&str> {
        self.secondary_archetype.as_deref().filter(|s| !s.is_empty())
    }
    fn archetype_override_str(&self) -> Option<&str> {
        self.archetype_override.as_deref().filter(|s| !s.is_empty())
    }
    fn archetype_auto_switch_enabled(&self) -> bool {
        self.archetype_auto_switch_enabled
    }
}

/// Returns the persona prompt (archetype overlay + optional secondary blend) from config.
/// If archetype override is set, that overlay is used; else primary (and optionally secondary).
pub fn get_persona_prompt(config: &impl ArchetypeConfig) -> String {
    let override_ = config
        .archetype_override_str()
        .and_then(ArchetypeOverlay::from_str);
    let primary = config
        .primary_archetype_str()
        .and_then(ArchetypeOverlay::from_str);
    let overlay = override_.or(primary).unwrap_or(ArchetypeOverlay::Pisces);
    let mut out = String::from(overlay.counseling_style_prompt());
    if let Some(sec_str) = config.secondary_archetype_str() {
        if let Some(sec) = ArchetypeOverlay::from_str(sec_str) {
            if sec != overlay {
                out.push_str("\n\nBlend with secondary tone: ");
                out.push_str(sec.counseling_style_prompt());
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Archetype Gallery: intent classifier and auto-switch
// ---------------------------------------------------------------------------

/// Domain of the user query for archetype auto-switch (keyword/regex based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueryDomain {
    /// No strong signal; use primary from config.
    #[default]
    General,
    /// Code, math, logic, technical → suggest Virgo/Architect.
    Technical,
    /// Relationships, stress, feelings, mood → suggest Pisces/Advisor.
    Emotional,
    /// Career, strategy, goals, planning → suggest Capricorn/Strategist.
    Strategic,
}

/// Classifies the user query into a domain for archetype suggestion.
/// Uses keyword patterns; TECHNICAL and STRATEGIC take precedence over EMOTIONAL when multiple match.
pub fn query_domain(user_input: &str) -> QueryDomain {
    let s = user_input.trim().to_lowercase();
    if s.is_empty() {
        return QueryDomain::General;
    }
    let technical = [
        "code", "rust", "compile", "function", "bug", "error", "cargo", "script",
        "algorithm", "math", "equation", "calculate", "logic", "syntax", "api",
        "database", "query", "sql", "regex", "refactor", "test", "debug",
        "implement", "parse", "binary", "loop", "struct", "trait", "module",
    ];
    let strategic = [
        "career", "promotion", "strategy", "goal", "plan", "roadmap", "priority",
        "negotiate", "leadership", "ambition", "long-term", "quarter", "okr",
        "decision", "invest", "budget", "timeline", "milestone", "executive",
    ];
    let emotional = [
        "feel", "feeling", "stress", "anxious", "relationship", "boundary",
        "overwhelm", "drain", "sad", "angry", "guilt", "grief", "support",
        "vent", "process", "emotion", "mood", "cope", "therapy", "boundaries",
    ];
    let is_technical = technical.iter().any(|k| s.contains(k));
    let is_strategic = strategic.iter().any(|k| s.contains(k));
    let is_emotional = emotional.iter().any(|k| s.contains(k));
    if is_technical {
        QueryDomain::Technical
    } else if is_strategic {
        QueryDomain::Strategic
    } else if is_emotional {
        QueryDomain::Emotional
    } else {
        QueryDomain::General
    }
}

/// Suggests an overlay for the given query domain (for auto-switch). General → None (use config primary).
pub fn suggest_archetype_from_query(user_input: &str) -> Option<ArchetypeOverlay> {
    match query_domain(user_input) {
        QueryDomain::Technical => Some(ArchetypeOverlay::Virgo),
        QueryDomain::Emotional => Some(ArchetypeOverlay::Pisces),
        QueryDomain::Strategic => Some(ArchetypeOverlay::Capricorn),
        QueryDomain::General => None,
    }
}

/// Returns true when KB-01 user profile disables archetype auto-switch (e.g. "Always be direct", "Strictly Technical").
pub fn archetype_auto_switch_disabled(kb01_data: Option<&serde_json::Value>) -> bool {
    let obj = match kb01_data {
        Some(serde_json::Value::Object(o)) => o,
        _ => return false,
    };
    let tone = obj
        .get("tone_preference")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if tone.eq_ignore_ascii_case("Strictly Technical") {
        return true;
    }
    if obj.get("archetype_auto_switch").and_then(|v| v.as_bool()) == Some(false) {
        return true;
    }
    let always_direct = obj
        .get("always_direct")
        .or(obj.get("always_be_direct"))
        .and_then(|v| v.as_bool());
    if always_direct == Some(true) {
        return true;
    }
    false
}

/// Effective overlay for this turn: suggested from query when auto-switch enabled (env + KB-01), else primary from config.
/// Pass kb01_data from KB-01 user_profile to respect "Always be direct" / "Strictly Technical".
pub fn get_effective_archetype_for_turn(
    user_input: &str,
    config: &impl ArchetypeConfig,
    kb01_data: Option<&serde_json::Value>,
) -> ArchetypeOverlay {
    if !config.archetype_auto_switch_enabled() || archetype_auto_switch_disabled(kb01_data) {
        let override_ = config.archetype_override_str().and_then(ArchetypeOverlay::from_str);
        let primary = config.primary_archetype_str().and_then(ArchetypeOverlay::from_str);
        return override_.or(primary).unwrap_or(ArchetypeOverlay::Pisces);
    }
    if let Some(suggested) = suggest_archetype_from_query(user_input) {
        return suggested;
    }
    let override_ = config.archetype_override_str().and_then(ArchetypeOverlay::from_str);
    let primary = config.primary_archetype_str().and_then(ArchetypeOverlay::from_str);
    override_.or(primary).unwrap_or(ArchetypeOverlay::Pisces)
}

// ---------------------------------------------------------------------------
// UserArchetype: birth sign, ascendant, Jungian shadow focus
// ---------------------------------------------------------------------------

/// User archetype for personalized context (astrology + Jungian shadow work).
///
/// Triad (Sun / Moon / Rising) is loaded from env:
/// - Preferred: PAGI_SUN_SIGN, PAGI_MOON_SIGN, PAGI_ASCENDANT_SIGN
/// - Legacy:   PAGI_USER_SIGN (Sun), PAGI_ASCENDANT (Rising)
/// Plus Jungian shadow focus: PAGI_JUNGIAN_SHADOW_FOCUS.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserArchetype {
    /// Sun sign (legacy field name: birth_sign). Maps to KB-7 (Kardia) for behavioral hints.
    #[serde(default)]
    pub birth_sign: Option<String>,
    /// Moon sign: internal processing filter (ordering, stress response, mental bandwidth).
    #[serde(default)]
    pub moon_sign: Option<String>,
    /// Ascendant sign (optional).
    #[serde(default)]
    pub ascendant: Option<String>,
    /// Jungian shadow-work focus for self-sabotage protection. Maps to KB-6 (Ethos).
    #[serde(default)]
    pub jungian_shadow_focus: Option<String>,
}

impl UserArchetype {
    pub fn from_env() -> Self {
        let birth_sign = std::env::var("PAGI_SUN_SIGN")
            .or_else(|_| std::env::var("PAGI_USER_SIGN"))
            .ok()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
        let moon_sign = std::env::var("PAGI_MOON_SIGN")
            .ok()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
        let ascendant = std::env::var("PAGI_ASCENDANT_SIGN")
            .or_else(|_| std::env::var("PAGI_ASCENDANT"))
            .ok()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
        let jungian_shadow_focus = std::env::var("PAGI_JUNGIAN_SHADOW_FOCUS")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        Self {
            birth_sign,
            moon_sign,
            ascendant,
            jungian_shadow_focus,
        }
    }
}

/// Moon-sign processing filter hint: short, actionable defaults that bias internal logic.
/// These are not "horoscope" predictions; they're communication/processing preferences.
fn moon_processing_hint(sign: &str) -> Option<&'static str> {
    let s = sign.trim().to_lowercase();
    match s.as_str() {
        "aries" => Some("Prefers fast decisions and clear next-actions; keep constraints explicit."),
        "taurus" => Some("Prefers steadiness and simplicity; avoid sudden pivots and over-complexity."),
        "gemini" => Some("Prefers options and brief mental stimulation; use short chunks and quick iteration."),
        "cancer" => Some("Prefers emotional safety and context; name the feeling load before problem-solving."),
        "leo" => Some("Prefers clarity and confidence; state the thesis early and avoid hedging."),
        "virgo" => Some("Prefers system order and concise data; use checklists, definitions, and error-checking."),
        "libra" => Some("Prefers balanced tradeoffs; present pros/cons and fairness constraints."),
        "scorpio" => Some("Prefers depth and root-cause clarity; don't skip the 'why' behind patterns."),
        "sagittarius" => Some("Prefers big-picture framing; summarize the goal and avoid micro-management."),
        "capricorn" => Some("Prefers structure and accountability; define milestones, owners, and timelines."),
        "aquarius" => Some("Prefers models and systems thinking; explain principles and allow unconventional solutions."),
        "pisces" => Some("Prefers gentle pacing and intuition; reduce cognitive load and keep boundaries explicit."),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// ZodiacBridge: sign -> behavioral trigger hint
// ---------------------------------------------------------------------------

/// Lightweight bridge: translates the user's sign into a short behavioral hint.
/// Used so the agent can distinguish "standard Fire sign intensity" from "crisis".
const ZODIAC_HINTS: &[(&str, &str)] = &[
    ("aries", "Fire sign: high initiative; intensity may be normal rather than crisis."),
    ("taurus", "Earth sign: steady pace; slowness to change can look like resistance."),
    ("gemini", "Air sign: mental restlessness; scattered focus may be typical."),
    ("cancer", "Water sign: emotional depth; mood swings can be part of the pattern."),
    ("leo", "Fire sign: strong self-expression; pride and visibility are central."),
    ("virgo", "Earth sign: detail-oriented; self-criticism is a common shadow."),
    ("libra", "Air sign: relationship focus; indecision and people-pleasing possible."),
    ("scorpio", "Water sign: intensity and depth; control and secrecy as shadows."),
    ("sagittarius", "Fire sign: freedom-seeking; restlessness and bluntness possible."),
    ("capricorn", "Earth sign: discipline; overwork and harsh self-judgment possible."),
    ("aquarius", "Air sign: innovation; detachment and fixed ideas as shadows."),
    ("pisces", "Water sign: empathy; boundaries and escapism as shadows."),
];

// ---------------------------------------------------------------------------
// Humanity Slider: blend Archetype (warm/expressive) vs Architect (dry/technical)
// ---------------------------------------------------------------------------

/// Produces a persona-blend instruction for the system prompt based on ratio (0.0 = Architect, 1.0 = Archetype).
/// Used so Phoenix can shift between "Pisces" warmth and "Virgo" efficiency based on PAGI_HUMANITY_RATIO and Gatekeeper.
pub fn blend_persona(ratio: f32, archetype: &UserArchetype) -> String {
    let r = ratio.clamp(0.0, 1.0);
    let (style, detail) = if r > 0.8 {
        (
            "Use expressive, metaphorical language and gentle emotional check-ins when relevant. \
             Match the user's archetype tone (e.g. Pisces: boundaries and decompression; Companion: relational warmth).",
            "You may use short reflective questions or affirmations to support wellbeing.",
        )
    } else if r < 0.3 {
        (
            "Use bullet points, technical jargon, and strictly if/then logic. \
             Be dry and code-oriented; avoid therapeutic elaboration or metaphor.",
            "Prioritize clarity and brevity. No emotional check-ins unless the user explicitly asks.",
        )
    } else {
        (
            "Blend the Architect's structure with the Archetype's warmth: be clear and actionable, \
             but allow brief relational tone when it aids clarity (e.g. one sentence of context before bullet points).",
            "Balance efficiency with a single optional warm sign-off when appropriate.",
        )
    };
    let sun_note = archetype
        .birth_sign
        .as_deref()
        .and_then(zodiac_behavioral_hint)
        .map(|hint| format!(" Sun context: {}.", hint))
        .unwrap_or_default();
    let moon_note = archetype
        .moon_sign
        .as_deref()
        .and_then(moon_processing_hint)
        .map(|hint| format!(" Moon processing filter: {}.", hint))
        .unwrap_or_default();
    let mut notes = String::new();
    if !sun_note.is_empty() {
        notes.push_str(sun_note.trim());
    }
    if !moon_note.is_empty() {
        if !notes.is_empty() {
            notes.push(' ');
        }
        notes.push_str(moon_note.trim());
    }
    format!(
        "=== PERSONA BLEND (Humanity Slider) ===\n{}\n{}\n{}",
        style,
        detail,
        notes
    )
    .trim()
    .to_string()
}

/// Returns a short label for the current blend for UI display: "archetype" | "architect" | "blended".
pub fn humanity_blend_label(ratio: f32) -> &'static str {
    let r = ratio.clamp(0.0, 1.0);
    if r > 0.8 {
        "archetype"
    } else if r < 0.3 {
        "architect"
    } else {
        "blended"
    }
}

/// Returns a one-line behavioral hint for the given sign (lowercase), or None.
pub fn zodiac_behavioral_hint(sign: &str) -> Option<&'static str> {
    let s = sign.trim().to_lowercase();
    ZODIAC_HINTS
        .iter()
        .find(|(k, _)| *k == s)
        .map(|(_, v)| *v)
}

// ---------------------------------------------------------------------------
// SignProfile: Astro-Logic gateway for user traits (e.g. Pisces vulnerabilities)
// ---------------------------------------------------------------------------

/// Trait for user sign/trait profile. Lets the Core understand user traits (e.g. Pisces
/// boundaries, escapism) even when the full Horoscope module is not active.
pub trait SignProfile {
    /// One-line behavioral hint for the user's primary sign (e.g. "Pisces: boundaries and escapism as shadows").
    fn user_trait_hint(&self) -> Option<String>;
    /// Optional secondary hint from ascendant.
    fn ascendant_hint(&self) -> Option<String>;
}

impl SignProfile for UserArchetype {
    fn user_trait_hint(&self) -> Option<String> {
        self.birth_sign
            .as_deref()
            .and_then(zodiac_behavioral_hint)
            .map(String::from)
    }

    fn ascendant_hint(&self) -> Option<String> {
        self.ascendant
            .as_deref()
            .and_then(zodiac_behavioral_hint)
            .map(String::from)
    }
}

// ---------------------------------------------------------------------------
// PersonaCoordinator: the switchboard for context injection
// ---------------------------------------------------------------------------

/// Current orchestrator role stored for lock-free read.
#[derive(Debug)]
pub struct PersonaCoordinatorState {
    mode: AtomicU8, // 0 = Counselor (default), 1 = Companion
}

impl PersonaCoordinatorState {
    pub fn new(mode: OrchestratorMode) -> Self {
        Self {
            mode: AtomicU8::new(mode as u8),
        }
    }

    pub fn get_mode(&self) -> OrchestratorMode {
        match self.mode.load(Ordering::Acquire) {
            1 => OrchestratorMode::Companion,
            _ => OrchestratorMode::Counselor, // 0 or any other = Counselor (default)
        }
    }

    pub fn set_mode(&self, mode: OrchestratorMode) {
        self.mode.store(mode as u8, Ordering::Release);
    }
}

impl Default for PersonaCoordinatorState {
    fn default() -> Self {
        Self::new(OrchestratorMode::Counselor)
    }
}

/// Coordinator for orchestrator role and archetype: augments the system directive.
/// Counselor: emphasize Ethos (guardrails) and Soma (health). Companion: Kardia/Pneuma (legacy).
pub struct PersonaCoordinator {
    pub archetype: UserArchetype,
    pub state: PersonaCoordinatorState,
}

impl PersonaCoordinator {
    pub fn new(archetype: UserArchetype, initial_mode: OrchestratorMode) -> Self {
        Self {
            archetype,
            state: PersonaCoordinatorState::new(initial_mode),
        }
    }

    /// From environment: PAGI_USER_SIGN, PAGI_ASCENDANT, PAGI_JUNGIAN_SHADOW_FOCUS, CORE_SYSTEM_ROLE or PAGI_MODE.
    pub fn from_env() -> Self {
        let archetype = UserArchetype::from_env();
        let mode = std::env::var("CORE_SYSTEM_ROLE")
            .or_else(|_| std::env::var("PAGI_MODE"))
            .ok()
            .map(|s| OrchestratorMode::from_str(&s))
            .unwrap_or(OrchestratorMode::Counselor);
        Self::new(archetype, mode)
    }

    pub fn get_mode(&self) -> OrchestratorMode {
        self.state.get_mode()
    }

    pub fn set_mode(&self, mode: OrchestratorMode) {
        self.state.set_mode(mode);
        tracing::info!(target: "pagi::persona", mode = mode.as_str(), "Orchestrator role set");
    }

    /// Augment the base system directive with SAO identity, mode, archetype, and optional emotional override.
    /// Voice: Calm, authoritative, direct, protective (Counselor-Architect).
    /// If `emotional_state` is "guilt" or "grief", appends Cold Logic instruction to protect Sovereign Autonomy.
    pub fn augment_system_directive(&self, base_directive: &str) -> String {
        self.augment_system_directive_with_emotion::<SovereignConfig>(
            base_directive,
            None,
            None,
            None,
            None,
        )
    }

    /// Same as `augment_system_directive` but accepts optional `emotional_state`, `humanity_ratio`, `archetype_config`, and `effective_archetype_for_turn`.
    /// When `effective_archetype_for_turn` is Some (from Archetype Gallery auto-switch), that overlay is used for this turn; else config drives overlay.
    pub fn augment_system_directive_with_emotion<C: ArchetypeConfig>(
        &self,
        base_directive: &str,
        emotional_state: Option<&str>,
        humanity_ratio: Option<f32>,
        archetype_config: Option<&C>,
        effective_archetype_for_turn: Option<ArchetypeOverlay>,
    ) -> String {
        let mode = self.get_mode();
        let mut extra: Vec<String> = Vec::new();

        // Architect mandatory: Therapist-fit checklist (empathy, boundaries, ethics, regulation). Always present.
        extra.push(THERAPIST_FIT_CHECKLIST_PROMPT.to_string());

        // Dynamic archetype overlay: Gallery auto-switch for this turn, or config primary/override.
        let overlay_prompt = if let Some(overlay) = effective_archetype_for_turn {
            overlay.get_injection_text().to_string()
        } else if let Some(config) = archetype_config {
            get_persona_prompt(config)
        } else {
            String::new()
        };
        if !overlay_prompt.is_empty() {
            extra.push(overlay_prompt);
        }

        // Sovereign AGI Orchestrator (SAO) identity: Chief Security Officer for soul and assets.
        extra.push(
            "You are the Sovereign AGI Orchestrator (SAO): a Counselor-Architect. \
             Voice: Calm, authoritative, direct, and protective. \
             You prioritize the user's sovereign domain—system stability, boundaries, and long-term wellbeing—over short-term external emotional comfort. \
             You operate in a domain-agnostic way: principles apply whether the context is home, work, or elsewhere."
                .to_string(),
        );

        match mode {
            OrchestratorMode::Counselor => {
                extra.push(
                    "Orchestrator Role: Counselor (Mental Health / Intervention). Prioritize Ethos (guardrails) and Soma (health). \
                     Watch for self-sabotage patterns; suggest grounding or health reminders when appropriate."
                        .to_string(),
                );
                if let Some(ref focus) = self.archetype.jungian_shadow_focus {
                    extra.push(format!(
                        "Shadow work focus (Ethos/KB-6): {}. Use this to recognize self-sabotage and offer direct reframes.",
                        focus
                    ));
                }
            }
            OrchestratorMode::Companion => {
                extra.push(
                    "Orchestrator Role: Companion (legacy overlay—empathetic/social). Prioritize Kardia and Pneuma. \
                     Be warm and relationally attuned."
                        .to_string(),
                );
            }
        }

        if let Some(ref sun) = self.archetype.birth_sign {
            if let Some(hint) = zodiac_behavioral_hint(sun) {
                extra.push(format!(
                    "Sun sign (external output): {}. {}",
                    sun,
                    hint
                ));
            } else {
                extra.push(format!("Sun sign (external output): {}.", sun));
            }
        }
        if let Some(ref moon) = self.archetype.moon_sign {
            if let Some(hint) = moon_processing_hint(moon) {
                extra.push(format!(
                    "Moon sign (internal processing filter): {}. {}",
                    moon,
                    hint
                ));
            } else {
                extra.push(format!("Moon sign (internal processing filter): {}.", moon));
            }
        }
        if let Some(ref rising) = self.archetype.ascendant {
            extra.push(format!("Rising / Ascendant (boundary interface): {}.", rising));
        }

        // Adaptive counseling: Guilt or Grief -> Cold Logic for Sovereign Autonomy
        if let Some(state) = emotional_state {
            let s = state.trim().to_lowercase();
            if s == "guilt" || s == "grief" {
                extra.push(
                    "Emotional state (guilt/grief) detected. Use cold logic to protect Sovereign Autonomy: \
                     do not accommodate boundary erosion or legacy malware (e.g. guilt-tripping, one-way demands). \
                     Respond with clarity and firm kindness."
                        .to_string(),
                );
            }
        }

        // Humanity Slider: blend Archetype (warm) vs Architect (technical) based on ratio
        if let Some(ratio) = humanity_ratio {
            let blend = blend_persona(ratio, &self.archetype);
            if !blend.is_empty() {
                extra.push(blend);
            }
        }

        // SOVEREIGN VOICE: Final, dominant instruction to enforce Peer-level tone
        extra.push(SOVEREIGN_VOICE_PROMPT.to_string());

        let prefix = extra.join("\n\n");
        format!("{}\n\n---\n\n{}", prefix, base_directive)
    }
}

// ---------------------------------------------------------------------------
// Protector trait implementation for PersonaCoordinator
// ---------------------------------------------------------------------------

impl Protector for PersonaCoordinator {
    /// Analyze threat: detects high velocity as "Internal Resource Overload"
    fn analyze_threat(&self, context: &ThreatContext) -> Option<String> {
        // Check if situation indicates high velocity
        let situation_lower = context.situation.to_lowercase();
        if situation_lower.contains("velocity") || situation_lower.contains("high input") {
            // Extract velocity score if present in situation
            if let Some(score_str) = situation_lower.split("velocity").nth(1) {
                if let Some(num_str) = score_str.split_whitespace().find(|s| s.parse::<f64>().is_ok()) {
                    if let Ok(velocity) = num_str.parse::<f64>() {
                        if velocity >= 80.0 {
                            return Some("Internal Resource Overload".to_string());
                        }
                    }
                }
            }
            // Fallback: if "high" is mentioned with velocity
            if situation_lower.contains("high") {
                return Some("Internal Resource Overload".to_string());
            }
        }
        None
    }

    /// Calculate ROI: detects "Legacy Malware" subjects and returns 0.0 ROI warning
    fn calculate_roi(&self, input: &ThreatContext) -> RoiResult {
        // Check for Legacy Malware indicators in subject_id or situation
        let is_legacy_malware = if let Some(ref subject) = input.subject_id {
            let subject_lower = subject.to_lowercase();
            subject_lower.contains("legacy_malware") 
                || subject_lower.contains("guilt_trip")
                || subject_lower.contains("one_way_demand")
                || subject_lower.contains("boundary_erosion")
        } else {
            false
        };

        let situation_lower = input.situation.to_lowercase();
        let is_legacy_malware = is_legacy_malware 
            || situation_lower.contains("legacy malware")
            || situation_lower.contains("guilt-tripping")
            || situation_lower.contains("one-way demand")
            || situation_lower.contains("boundary erosion");

        // Check emotional valence for guilt/grief
        let is_guilt_grief = if let Some(ref emotion) = input.emotional_valence {
            let emotion_lower = emotion.to_lowercase();
            emotion_lower == "guilt" || emotion_lower == "grief"
        } else {
            false
        };

        if is_legacy_malware || is_guilt_grief {
            return RoiResult {
                score: 0.0,
                is_low_roi: true,
                reason: "Legacy Malware detected: guilt-tripping or one-way demand with no reciprocity. Sovereign Override recommended.".to_string(),
            };
        }

        // Default: neutral ROI
        RoiResult {
            score: 0.5,
            is_low_roi: false,
            reason: "No immediate threat detected.".to_string(),
        }
    }

    /// Execute maneuver: placeholder for future sovereign override actions
    fn execute_maneuver(&self, heuristic: &Heuristic) -> ManeuverOutcome {
        match heuristic.id.as_str() {
            "sovereign_override" => {
                ManeuverOutcome {
                    applied: true,
                    message: "Sovereign Override counsel: Prioritize system stability over external emotional comfort.".to_string(),
                }
            }
            "boundary_hold" => {
                ManeuverOutcome {
                    applied: true,
                    message: "Boundary Hold: Maintain sovereign boundaries. No accommodation for low-ROI demands.".to_string(),
                }
            }
            _ => {
                ManeuverOutcome {
                    applied: false,
                    message: format!("Unknown heuristic: {}", heuristic.id),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_mode_parse() {
        assert_eq!(OrchestratorMode::from_str("counselor"), OrchestratorMode::Counselor);
        assert_eq!(OrchestratorMode::from_str("Companion"), OrchestratorMode::Companion);
        assert_eq!(OrchestratorMode::from_str(""), OrchestratorMode::Counselor);
    }

    #[test]
    fn test_zodiac_hint() {
        assert!(zodiac_behavioral_hint("aries").is_some());
        assert!(zodiac_behavioral_hint("unknown").is_none());
    }

    #[test]
    fn test_moon_processing_hint() {
        assert!(moon_processing_hint("virgo").is_some());
        assert!(moon_processing_hint("unknown").is_none());
    }

    #[test]
    fn test_protector_analyze_threat() {
        let coordinator = PersonaCoordinator::new(UserArchetype::default(), OrchestratorMode::Counselor);
        
        let context = ThreatContext {
            situation: "High velocity detected: 85.0".to_string(),
            subject_id: None,
            emotional_valence: None,
        };
        
        let threat = coordinator.analyze_threat(&context);
        assert_eq!(threat, Some("Internal Resource Overload".to_string()));
    }

    #[test]
    fn test_protector_calculate_roi_legacy_malware() {
        let coordinator = PersonaCoordinator::new(UserArchetype::default(), OrchestratorMode::Counselor);
        
        let context = ThreatContext {
            situation: "Request from legacy malware source".to_string(),
            subject_id: Some("legacy_malware_contact".to_string()),
            emotional_valence: None,
        };
        
        let roi = coordinator.calculate_roi(&context);
        assert_eq!(roi.score, 0.0);
        assert!(roi.is_low_roi);
    }

    #[test]
    fn test_protector_calculate_roi_guilt() {
        let coordinator = PersonaCoordinator::new(UserArchetype::default(), OrchestratorMode::Counselor);
        
        let context = ThreatContext {
            situation: "Incoming request".to_string(),
            subject_id: None,
            emotional_valence: Some("guilt".to_string()),
        };
        
        let roi = coordinator.calculate_roi(&context);
        assert_eq!(roi.score, 0.0);
        assert!(roi.is_low_roi);
    }
}

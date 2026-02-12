//! PHOENIX MARIE Onboarding Protocol: First Run handshake and Domain Audit.
//!
//! Establishes the "Therapeutic Peer" tone and verifies SAO access to the 8 Knowledge Bases.
//! Used by the Studio UI to show the onboarding overlay when the Sovereign Domain is uninitialized.
//! Includes KB-01 Discovery: PII status and archetype-agnostic profiling questions.

use crate::knowledge::KnowledgeStore;
use serde::{Deserialize, Serialize};

/// Key in KB-01 (Pneuma) for the user profile object. When absent, onboarding_status is Incomplete.
pub const KB01_USER_PROFILE_KEY: &str = "user_profile";

/// Key in KB-06 (Ethos) that marks onboarding as complete. When absent, the UI shows the protocol.
pub const ONBOARDING_COMPLETE_KEY: &str = "phoenix_marie_onboarding_complete";

/// Status of one KB slot for the Domain Audit (Phase 2) display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingKbSlot {
    pub slot_id: u8,
    pub label: String,
    pub entry_count: usize,
    pub connected: bool,
}

/// PII/Discovery status derived from KB-01. When Incomplete, UI should show discovery fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingPiiStatus {
    /// KB-01 contains a user_profile object; discovery can be considered complete.
    Complete,
    /// KB-01 has no user_profile; gentle information gathering is the priority.
    Incomplete,
}

/// Full state for the PHOENIX MARIE onboarding flow. Returned by `onboarding_sequence()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnboardingState {
    /// When true, the UI should show the onboarding overlay (Recognition → Domain Audit → CTA).
    pub needs_onboarding: bool,
    /// "Complete" when KB-01 has user_profile; "Incomplete" otherwise. Drives discovery loop.
    pub onboarding_status: String,
    /// Phase 1: Recognition & Persona Handshake (single block of text).
    pub phase1_greeting: String,
    /// Phase 2: Domain Audit lines (KB-01 through KB-08 status) for terminal-typing effect.
    pub phase2_audit_lines: Vec<String>,
    /// Phase 3: Call-to-action message.
    pub phase3_cta: String,
    /// KB slot statuses for overlay (slots 1–8).
    pub kb_status: Vec<OnboardingKbSlot>,
    /// Vitality string for Phase 2 footer: e.g. "stable", "draining", "critical".
    pub vitality: String,
    /// Profiling questions for the Discovery loop (archetype-agnostic).
    pub profiling_questions: Vec<String>,
}

/// Discovery module: pool of profiling questions for archetype-agnostic information gathering.
/// Used when KB-01 has no user_profile; Phoenix Marie uses these to align internal logic without guessing.
pub struct DiscoveryModule;

impl DiscoveryModule {
    /// Returns the ordered list of profiling questions for the discovery loop (human-centric).
    pub fn profiling_questions() -> Vec<String> {
        vec![
            "Who are you? (e.g. your birthday or Sun sign, or personality type like MBTI.)".to_string(),
            "What drains you? (Recurring situations that leave you feeling drained or taken advantage of.)".to_string(),
            "How should we talk? (Do you prefer me to be blunt, gentle, or purely logic-driven?)".to_string(),
        ]
    }
}

/// Phase 1: Recognition & Persona Handshake. Human-centric; no internal jargon.
pub const PHASE1_RECOGNITION: &str = "I'm **PHOENIX MARIE**. I'm ready to help. To give you the best support, I need to understand a few things about you first. Think of this as setting the foundation for our partnership:\n\n1. **Your Nature:** Tell me about your personality (like your birthday or personality type). This helps me understand your natural strengths.\n2. **Your Energy:** What are the recurring situations that leave you feeling drained or taken advantage of?\n3. **Our Connection:** Do you prefer me to be direct and logical, or more supportive and gentle?\n\nEverything you share stays right here on your machine in your private memory. Once I have this, my advice will be tailored specifically to your life and patterns. Where should we start?";

/// Phase 3: The Call to Action. Human-centric.
pub const PHASE3_CTA: &str = "We're starting with a clean slate. Would you like to set your **strategic priorities** next, or run a quick check on how you're feeling right now?";

/// KB-01 (Pneuma) slot ID for identity/user profile.
const KB01_SLOT: u8 = 1;

/// Checks KB-01 for a `user_profile` object. If absent, PII discovery is incomplete.
/// Used to set `onboarding_status` and drive the discovery loop in the UI.
pub fn check_pii_status(store: &KnowledgeStore) -> OnboardingPiiStatus {
    store
        .get(KB01_SLOT, KB01_USER_PROFILE_KEY)
        .ok()
        .and_then(|v| v)
        .filter(|b| !b.is_empty())
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.as_object().map(|_| ()))
        .map(|_| OnboardingPiiStatus::Complete)
        .unwrap_or(OnboardingPiiStatus::Incomplete)
}

/// Returns true if the Sovereign Domain has not yet completed the PHOENIX MARIE onboarding.
/// Checks KB-06 (Ethos) for the presence of `ONBOARDING_COMPLETE_KEY`.
pub fn needs_onboarding(store: &KnowledgeStore) -> bool {
    const ETHOS_SLOT: u8 = 6;
    store
        .get(ETHOS_SLOT, ONBOARDING_COMPLETE_KEY)
        .ok()
        .and_then(|v| v)
        .is_none()
}

/// Builds the full onboarding sequence: greeting, Domain Audit lines (KBs 1–8), and CTA.
/// If `needs_onboarding` is false (e.g. skipped via PAGI_SKIP_ONBOARDING or already completed),
/// the UI can still use `phase1_greeting` as the first chat message when appropriate.
/// Sets `onboarding_status` from `check_pii_status()` (Incomplete when KB-01 has no user_profile).
pub fn onboarding_sequence(store: &KnowledgeStore) -> OnboardingState {
    let needs = needs_onboarding(store);
    let pii_status = check_pii_status(store);
    let onboarding_status = match pii_status {
        OnboardingPiiStatus::Complete => "Complete".to_string(),
        OnboardingPiiStatus::Incomplete => "Incomplete".to_string(),
    };
    let kb_statuses = store.get_all_status();
    let kb_slots: Vec<OnboardingKbSlot> = kb_statuses
        .into_iter()
        .filter(|s| (1..=8).contains(&s.slot_id))
        .map(|s| OnboardingKbSlot {
            slot_id: s.slot_id,
            label: s.name,
            entry_count: s.entry_count,
            connected: s.connected,
        })
        .collect();

    // Phase 2: Domain Audit lines (terminal-typing style). Map slots to the spec labels.
    let phase2_audit_lines = build_phase2_audit_lines(&kb_slots);
    let vitality = "stable".to_string();
    let profiling_questions = DiscoveryModule::profiling_questions();

    OnboardingState {
        needs_onboarding: needs,
        onboarding_status,
        phase1_greeting: PHASE1_RECOGNITION.to_string(),
        phase2_audit_lines,
        phase3_cta: PHASE3_CTA.to_string(),
        kb_status: kb_slots,
        vitality,
        profiling_questions,
    }
}

fn build_phase2_audit_lines(kb_slots: &[OnboardingKbSlot]) -> Vec<String> {
    let mut lines = vec![
        "Initializing your Sovereign Domain...".to_string(),
    ];
    // Map slot_id to the spec's Phase 2 labels (KB-01 Ethos/Pisces, KB-05 Protocols, KB-08 Absurdity).
    for slot in kb_slots.iter().filter(|s| (1..=8).contains(&s.slot_id)) {
        let line = match slot.slot_id {
            1 => format!("• **Your profile:** Personality and identity (private memory)."),
            2 => format!("• **Context:** Workspace and governance."),
            3 => format!("• **Knowledge:** Research and information."),
            4 => format!("• **Memory:** Conversation and timeline."),
            5 => format!("• **Shielding:** Social protocols active and ready."),
            6 => format!("• **Guardrails:** Your preferences and boundaries."),
            7 => format!("• **Relationships:** Affective and relational context."),
            8 => format!("• **Health:** Monitoring and logic consistency."),
            _ => continue,
        };
        lines.push(line);
    }
    lines.push(String::new());
    lines.push("My vitality is stable. We are ready for full-control orchestration.".to_string());
    lines
}

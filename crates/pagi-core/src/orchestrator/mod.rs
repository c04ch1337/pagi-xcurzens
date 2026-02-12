//! Master Brain: task delegation and reasoning.
//!
//! When MoE (Mixture of Experts) mode is enabled, the gateway uses `route_to_experts`
//! to send inputs to the appropriate expert: OpenRouter (creative/complex), LanceDB (knowledge),
//! or SystemTool (commands). Unmatched tasks fall back to OpenRouter or can trigger pagi-evolution.

mod archetype_logic;
mod astro_weather;
mod blueprint;
mod health_report;
mod control;
pub mod heuristics;
pub mod init;
pub mod maintenance;
mod persona;
mod planner;
pub mod protocols;
pub mod skills;
pub mod sovereign_voice;
pub mod traits;

pub use astro_weather::{
    check_astro_weather, record_transit_correlation_if_high_risk, system_alert_if_high_risk,
    system_prompt_block, should_refresh, AstroWeatherState, TransitRiskLevel, STALE_MS,
};
pub use health_report::{
    generate_weekly_report, generate_weekly_sovereignty_report, record_archetype_usage,
    HealthReport, LeakStats, RestVsOutputEntry, ShieldedEvent, TransitCorrelationEntry, ArchetypeUsageBreakdown,
};
pub use blueprint::{BlueprintRegistry, Plan};
pub use control::ControlPanelMessage;
pub use archetype_logic::{
    active_archetype_label, get_sovereignty_leak_triggers, process_archetype_triggers,
    ArchetypeTriggerResult,
};
pub use init::{
    needs_onboarding, onboarding_sequence,
    OnboardingKbSlot, OnboardingState,
    KB01_USER_PROFILE_KEY, ONBOARDING_COMPLETE_KEY, PHASE1_RECOGNITION, PHASE3_CTA,
};
pub use heuristics::{HeuristicProcessor, HeuristicResult, SovereignDomain};
pub use persona::{
    humanity_blend_label, OrchestratorMode, PersonaCoordinator,
    PersonaCoordinatorState, SignProfile, UserArchetype, zodiac_behavioral_hint,
    ArchetypeOverlay, ArchetypePrompt,
    query_domain, QueryDomain,
    suggest_archetype_from_query, archetype_auto_switch_disabled, get_effective_archetype_for_turn,
};
pub use protocols::{
    matched_sovereignty_triggers, rank_subject_from_sovereignty_triggers, ProtocolEngine,
};
pub use skills::{
    validate_skill_permissions, SkillInventoryEntry, SkillManifestEntry, SkillManifestRegistry,
    SovereigntyViolation, TierManifest, TrustTier,
};
pub use traits::{Heuristic, ManeuverOutcome, Protector, RoiResult, ThreatContext, VitalityLevel};
pub use maintenance::{
    init_maintenance_loop, IdleTracker, MaintenanceConfig, TelemetryPulse,
    MaintenancePulseEvent, PendingApproval, ApprovalBridgeHandle, new_approval_bridge,
    PerformanceDelta, ValidationResult, SmokeTestRunner,
    // Evolutionary Versioning & Genetic Memory
    compute_patch_dna, check_genetic_dead_end, record_genetic_dead_end,
};
pub use sovereign_voice::{
    SOVEREIGN_VOICE_PROMPT, FORMATTING_GUIDELINES, FORBIDDEN_PHRASES,
    detect_tone_drift, has_call_to_action, generate_default_cta,
};

use crate::shared::{Goal, TenantContext};
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// MoE (Mixture of Experts) mode and gating
// ---------------------------------------------------------------------------

/// MoE toggle: Dense (one LLM call) vs Sparse (expert routing). Persisted to Sovereign Config (KB-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MoEMode {
    /// Standard: single OpenRouter call (no expert gating).
    #[default]
    Dense,
    /// Expert routing: Gater analyzes local_ctx and user input; route to OpenRouter, LanceDB, or SystemTool.
    Sparse,
}

impl MoEMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            MoEMode::Dense => "dense",
            MoEMode::Sparse => "sparse",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s.trim().eq_ignore_ascii_case("sparse") {
            true => MoEMode::Sparse,
            false => MoEMode::Dense,
        }
    }
}

/// Expert route for toggle-based MoE. When MoE is Sparse, the gateway branches on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoEExpert {
    /// Creative, complex reasoning, or general chat → OpenRouter (LLM).
    OpenRouter,
    /// Factual query, "what do we know", internal memory → LanceDB / KnowledgeQuery.
    LanceDB,
    /// System command, file move, hardware stats → local SystemTool (e.g. shell).
    SystemTool,
    /// No expert matched; fall back to OpenRouter or trigger pagi-evolution to synthesize one.
    Unmatched,
}

/// Lightweight gating: classify user input to choose which expert handles it.
/// Used only when MoE mode is enabled. Heuristics: system-ish keywords → SystemTool;
/// knowledge-ish → LanceDB; else OpenRouter. Unmatched reserved for evolution hook.
pub fn route_to_experts(input: &str) -> MoEExpert {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return MoEExpert::OpenRouter;
    }

    // System / ops: file moves, shell, hardware, process, env
    let system_patterns = [
        "move file", "copy file", "delete file", "run command", "execute ", "powershell",
        "cmd ", "terminal", "list dir", "directory", "cpu ", "memory usage", "disk space",
        "process list", "system info", "env ", "environment variable", "cargo build",
        "cargo run", "cargo test", ".rs ", "rust file", "git status", "git ",
    ];
    if system_patterns.iter().any(|p| s.contains(p)) {
        return MoEExpert::SystemTool;
    }

    // Knowledge / internal memory: what we know, recall, search, lookup, docs
    let knowledge_patterns = [
        "what do we know", "recall", "remember", "search knowledge", "look up",
        "internal memory", "knowledge base", "kb-", "slot ", "documentation",
        "according to our", "in our docs", "from the vault", "from memory",
        "find in", "query knowledge", "lookup ", "retrieve ",
    ];
    if knowledge_patterns.iter().any(|p| s.contains(p)) {
        return MoEExpert::LanceDB;
    }

    // Creative / complex / coding (non-system): open-ended, write, explain, reason
    let creative_patterns = [
        "explain", "why ", "how does", "write a", "draft", "summarize", "translate",
        "idea", "brainstorm", "design", "refactor", "review code", "improve ",
    ];
    if creative_patterns.iter().any(|p| s.contains(p)) {
        return MoEExpert::OpenRouter;
    }

    // Short queries that look like questions → prefer knowledge first if very short
    if s.len() < 60 && (s.starts_with("what ") || s.starts_with("where ") || s.starts_with("when ")) {
        return MoEExpert::LanceDB;
    }

    // Default: send to OpenRouter (general assistant)
    MoEExpert::OpenRouter
}

// ---------------------------------------------------------------------------
// Strategic Timing (Phase 2): Thinking Latency — prevent "Instant-Reply" syndrome
// ---------------------------------------------------------------------------

/// Computes minimum time the system should "think" before releasing a response.
/// Used so the Master Orchestrator feels like a peer-level Architect auditing infrastructure,
/// not a reactive chatbot.
///
/// **Logic**:
/// - Base delay: 800 ms.
/// - Complexity: +200 ms per 100 words in the transcript segment (user prompt / context).
/// - Skill modifier: +1500 ms when SynthesizeMeetingContext or BridgeCopilotSkill is in play.
#[inline]
pub fn calculate_thinking_latency(
    word_count: usize,
    heavy_skill_context: bool,
) -> std::time::Duration {
    const BASE_MS: u64 = 800;
    const PER_100_WORDS_MS: u64 = 200;
    const HEAVY_SKILL_MS: u64 = 1500;

    let complexity_ms = (word_count / 100) as u64 * PER_100_WORDS_MS;
    let skill_ms = if heavy_skill_context { HEAVY_SKILL_MS } else { 0 };
    let total_ms = BASE_MS.saturating_add(complexity_ms).saturating_add(skill_ms);
    std::time::Duration::from_millis(total_ms)
}

/// Gater: analyzes local_ctx and user input to choose expert. Use when MoEMode::Sparse.
/// High "System/Command" entropy in local_ctx (e.g. recent file/shell events) routes to SystemTool without OpenRouter.
pub struct Gater;

impl Gater {
    /// Route based on user input only (same as route_to_experts).
    pub fn route_input(input: &str) -> MoEExpert {
        route_to_experts(input)
    }

    /// Route considering both local context (from 8 KBs) and user input.
    /// If local_ctx contains system/command-like content, route to SystemTool to avoid unnecessary LLM call.
    pub fn route_with_context(local_ctx: &str, user_input: &str) -> MoEExpert {
        let ctx_lower = local_ctx.to_lowercase();
        let system_in_ctx = [
            "executed skill:", "powershell", "cmd", "terminal", "cargo build", "cargo run",
            "file", "directory", "list dir", "run command", "process", "system info",
        ];
        if system_in_ctx.iter().any(|p| ctx_lower.contains(p)) && user_input.trim().len() < 200 {
            return MoEExpert::SystemTool;
        }
        route_to_experts(user_input)
    }
}

#[derive(Debug)]
struct UnknownSkill(String);

impl fmt::Display for UnknownSkill {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown skill: {}", self.0)
    }
}

impl std::error::Error for UnknownSkill {}

/// Trait implemented by all agent capabilities (skills).
#[async_trait::async_trait]
pub trait AgentSkill: Send + Sync {
    /// Unique skill name for routing.
    fn name(&self) -> &str;

    /// Executes the skill with the given context and optional payload.
    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}

/// Registry of agent skills that can be dispatched by name.
pub struct SkillRegistry {
    skills: Vec<Arc<dyn AgentSkill>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
        }
    }

    pub fn register(&mut self, skill: Arc<dyn AgentSkill>) {
        self.skills.push(skill);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn AgentSkill>> {
        self.skills.iter().find(|s| s.name() == name).cloned()
    }

    /// Returns the names of all registered skills (for discovery and planning).
    pub fn skill_names(&self) -> Vec<String> {
        self.skills.iter().map(|s| s.name().to_string()).collect()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Receiver for control-panel messages (hold on orchestrator side).
pub type ControlPanelReceiver = mpsc::Receiver<ControlPanelMessage>;

/// Orchestrator dispatches goals to skills and coordinates execution.
/// Holds control state (active KBs, skills enabled, memory weights, MoE mode) updated by the control panel.
/// When `skill_manifest_registry` is set, ExecuteSkill/QueryKnowledge/UpdateKnowledgeSlot are gated by Sovereignty Firewall.
pub struct Orchestrator {
    registry: Arc<SkillRegistry>,
    blueprint: Arc<BlueprintRegistry>,
    /// Bitmask: bit i (0..7) = KB-(i+1) active. All 8 bits set = all active.
    active_kbs: AtomicU8,
    /// When false, dispatch returns "Skills Disabled" without calling skills.
    skills_enabled: AtomicBool,
    /// (short_term, long_term) weights for memory retrieval scoring.
    memory_weights: RwLock<(f32, f32)>,
    /// MoE mode: 0 = Dense, 1 = Sparse. Persisted to Sovereign Config (KB-6 key sovereign/moe_mode).
    moe_mode: AtomicU8,
    /// When Some, validates skill access to KB layers (1..=9) before execution. Tier 3 cannot touch KB-01 or KB-09.
    skill_manifest_registry: Option<Arc<SkillManifestRegistry>>,
    /// When true (PAGI_FIREWALL_STRICT_MODE), only Core (Tier 1) skills may touch any KB layer.
    firewall_strict_mode: bool,
}

impl Orchestrator {
    pub fn new(registry: Arc<SkillRegistry>) -> Self {
        Self {
            registry: Arc::clone(&registry),
            blueprint: Arc::new(BlueprintRegistry::default_blueprint()),
            active_kbs: AtomicU8::new(0xFF),
            skills_enabled: AtomicBool::new(true),
            memory_weights: RwLock::new((0.7, 0.3)),
            moe_mode: AtomicU8::new(0), // Dense
            skill_manifest_registry: None,
            firewall_strict_mode: false,
        }
    }

    pub fn with_blueprint(registry: Arc<SkillRegistry>, blueprint: Arc<BlueprintRegistry>) -> Self {
        Self {
            registry,
            blueprint,
            active_kbs: AtomicU8::new(0xFF),
            skills_enabled: AtomicBool::new(true),
            memory_weights: RwLock::new((0.7, 0.3)),
            moe_mode: AtomicU8::new(0), // Dense
            skill_manifest_registry: None,
            firewall_strict_mode: false,
        }
    }

    /// Same as with_blueprint but enables the Sovereignty Firewall: skill–KB permission checks before execution.
    /// When `firewall_strict_mode` is true, only Core (Tier 1) skills may touch any KB layer.
    pub fn with_blueprint_and_permissions(
        registry: Arc<SkillRegistry>,
        blueprint: Arc<BlueprintRegistry>,
        skill_manifest_registry: Arc<SkillManifestRegistry>,
        firewall_strict_mode: bool,
    ) -> Self {
        Self {
            registry,
            blueprint,
            active_kbs: AtomicU8::new(0xFF),
            skills_enabled: AtomicBool::new(true),
            memory_weights: RwLock::new((0.7, 0.3)),
            moe_mode: AtomicU8::new(0),
            skill_manifest_registry: Some(skill_manifest_registry),
            firewall_strict_mode,
        }
    }

    /// Set MoE mode (Dense = standard LLM, Sparse = expert routing). Caller should persist to KB via KnowledgeStore::set_sovereign_moe_mode.
    pub fn set_moe_mode(&self, mode: MoEMode) {
        self.moe_mode.store(mode as u8, Ordering::SeqCst);
    }

    /// Current MoE mode. When Sparse, gateway should use Gater::route_with_context before calling OpenRouter.
    pub fn get_moe_mode(&self) -> MoEMode {
        match self.moe_mode.load(Ordering::Acquire) {
            1 => MoEMode::Sparse,
            _ => MoEMode::Dense,
        }
    }

    /// Applies a control-panel message to the orchestrator state (lock-free where possible).
    pub fn pagi_apply_control_signal(&self, msg: ControlPanelMessage) {
        use ControlPanelMessage::*;
        match msg {
            KbState { index, active } => {
                if index < 8 {
                    let mask = 1u8 << index;
                    self.active_kbs.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                        Some(if active { v | mask } else { v & !mask })
                    })
                    .ok();
                }
            }
            SkillsEnabled(enabled) => {
                self.skills_enabled.store(enabled, Ordering::SeqCst);
            }
            MemoryWeights { short_term, long_term } => {
                if let Ok(mut w) = self.memory_weights.write() {
                    *w = (short_term, long_term);
                }
            }
            FullState {
                kb_states,
                skills_enabled: se,
                short_term_memory_weight: st,
                long_term_memory_weight: lt,
            } => {
                let mut mask = 0u8;
                for (i, &on) in kb_states.iter().enumerate().take(8) {
                    if on {
                        mask |= 1u8 << i;
                    }
                }
                self.active_kbs.store(mask, Ordering::SeqCst);
                self.skills_enabled.store(se, Ordering::SeqCst);
                if let Ok(mut w) = self.memory_weights.write() {
                    *w = (st, lt);
                }
            }
            ControlPanelMessage::MoEMode(mode) => {
                self.set_moe_mode(mode);
            }
        }
    }

    /// Returns whether the given KB slot (1..=8) is active.
    #[inline]
    pub fn pagi_kb_active(&self, slot_id: u8) -> bool {
        if !(1..=8).contains(&slot_id) {
            return false;
        }
        let index = (slot_id - 1) as usize;
        let mask = 1u8 << index;
        self.active_kbs.load(Ordering::Acquire) & mask != 0
    }

    /// Returns current memory weights (short_term, long_term).
    pub fn pagi_memory_weights(&self) -> (f32, f32) {
        self.memory_weights.read().map(|g| *g).unwrap_or((0.7, 0.3))
    }

    /// Returns whether the skills execution engine is enabled (control-panel state).
    #[inline]
    pub fn pagi_skills_enabled(&self) -> bool {
        self.skills_enabled.load(Ordering::Acquire)
    }

    /// Spawns a background tokio task that receives control messages and applies them to this orchestrator.
    /// Call with `Arc::clone(&orchestrator)` and the receiver half of the control-panel channel.
    pub fn spawn_control_listener(self: Arc<Self>, mut receiver: ControlPanelReceiver) {
        tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                self.pagi_apply_control_signal(msg);
            }
        });
    }

    /// Dispatches a goal; ExecuteSkill is routed to the registered skill and executed.
    /// Respects control-panel state: skills disabled and inactive KBs are gated.
    pub async fn dispatch(
        &self,
        ctx: &TenantContext,
        goal: Goal,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        if !self.skills_enabled.load(Ordering::Acquire) {
            return Ok(serde_json::json!({
                "status": "skills_disabled",
                "message": "Skills execution is disabled by the control panel.",
                "goal": serde_json::to_string(&goal).unwrap_or_default()
            }));
        }

        match goal {
            Goal::ExecuteSkill { name, payload } => {
                if let Some(ref reg) = self.skill_manifest_registry {
                    if let Some(kb_layer) = extract_kb_layer_from_payload(payload.as_ref()) {
                        if !validate_skill_permissions(reg, &name, kb_layer, self.firewall_strict_mode) {
                            return Err(SovereigntyViolation {
                                skill_id: name.clone(),
                                kb_layer,
                            }
                            .into());
                        }
                    }
                }
                let skill = self
                    .registry
                    .get(&name)
                    .ok_or_else(|| UnknownSkill(name.clone()))?;
                skill.execute(ctx, payload).await
            }
            Goal::QueryKnowledge { slot_id, query } => {
                if !self.pagi_kb_active(slot_id) {
                    return Ok(serde_json::json!({
                        "status": "kb_disabled",
                        "message": format!("KB-{} is disabled by the control panel.", slot_id),
                        "slot_id": slot_id,
                        "query": query
                    }));
                }
                if let Some(ref reg) = self.skill_manifest_registry {
                    if !validate_skill_permissions(reg, "KnowledgeQuery", slot_id, self.firewall_strict_mode) {
                        return Err(SovereigntyViolation {
                            skill_id: "KnowledgeQuery".to_string(),
                            kb_layer: slot_id,
                        }
                        .into());
                    }
                }
                let payload = serde_json::json!({ "slot_id": slot_id, "query_key": query });
                let skill = self
                    .registry
                    .get("KnowledgeQuery")
                    .ok_or_else(|| UnknownSkill("KnowledgeQuery".into()))?;
                skill.execute(ctx, Some(payload)).await
            }
            Goal::IngestData { payload } => {
                let skill = self
                    .registry
                    .get("LeadCapture")
                    .ok_or_else(|| UnknownSkill("LeadCapture".into()))?;
                skill.execute(ctx, payload).await
            }
            Goal::AssembleContext { context_id } => {
                let payload = serde_json::json!({ "lead_id": context_id });
                let skill = self
                    .registry
                    .get("DraftResponse")
                    .ok_or_else(|| UnknownSkill("DraftResponse".into()))?;
                skill.execute(ctx, Some(payload)).await
            }
            Goal::GenerateFinalResponse { context_id } => {
                let draft_skill = self
                    .registry
                    .get("DraftResponse")
                    .ok_or_else(|| UnknownSkill("DraftResponse".into()))?;
                let draft_payload = serde_json::json!({ "lead_id": context_id });
                let draft_result = draft_skill.execute(ctx, Some(draft_payload)).await?;
                let prompt = draft_result
                    .get("draft")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let router_skill = self
                    .registry
                    .get("ModelRouter")
                    .ok_or_else(|| UnknownSkill("ModelRouter".into()))?;
                let router_payload = serde_json::json!({ "prompt": prompt });
                let router_result = router_skill.execute(ctx, Some(router_payload)).await?;
                let mut map = match router_result {
                    serde_json::Value::Object(m) => m,
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "ModelRouter did not return object",
                        )
                        .into())
                    }
                };
                map.insert("goal".to_string(), serde_json::json!("GenerateFinalResponse"));
                map.insert("context_id".to_string(), serde_json::json!(context_id));
                Ok(serde_json::Value::Object(map))
            }
            Goal::AutonomousGoal { intent, context } => {
                let plan = self.blueprint.plan_for_intent(&intent).ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("unknown intent: {}", intent),
                    )
                })?;
                let initial_context = context.clone().unwrap_or(serde_json::json!({}));
                let mut payload = initial_context.clone();
                let mut previous_result = serde_json::Value::Null;
                let mut previous_skill: Option<String> = None;
                let mut steps_trace: Vec<serde_json::Value> = Vec::new();

                for skill_name in &plan.steps {
                    let skill = self
                        .registry
                        .get(skill_name)
                        .ok_or_else(|| UnknownSkill(skill_name.clone()))?;
                    let step_input = chain_payload(previous_skill.as_deref(), skill_name, &previous_result, payload.clone());
                    previous_result = skill.execute(ctx, step_input.clone()).await?;
                    previous_skill = Some(skill_name.clone());
                    payload = previous_result.clone();

                    steps_trace.push(serde_json::json!({
                        "skill": skill_name,
                        "input": step_input,
                        "output": previous_result
                    }));
                }

                let final_result = previous_result.clone();
                let thought_log = serde_json::json!({
                    "intent": intent,
                    "context": initial_context,
                    "plan_steps": plan.steps,
                    "steps": steps_trace,
                    "final_result": final_result
                });

                if let Some(audit_skill) = self.registry.get("ResearchAudit") {
                    let audit_payload = serde_json::json!({ "trace": thought_log });
                    if let Ok(audit_result) = audit_skill.execute(ctx, Some(audit_payload)).await {
                        if let Some(trace_id) = audit_result.get("trace_id").and_then(|v| v.as_str()) {
                            let mut out = match final_result {
                                serde_json::Value::Object(m) => m,
                                _ => {
                                    let mut m = serde_json::Map::new();
                                    m.insert("result".to_string(), final_result);
                                    m
                                }
                            };
                            out.insert("goal".to_string(), serde_json::json!("AutonomousGoal"));
                            out.insert("intent".to_string(), serde_json::json!(intent));
                            out.insert("plan_steps".to_string(), serde_json::json!(plan.steps));
                            out.insert("trace_id".to_string(), serde_json::json!(trace_id));
                            return Ok(serde_json::Value::Object(out));
                        }
                    }
                }

                let mut out = match final_result {
                    serde_json::Value::Object(m) => m,
                    _ => return Ok(final_result),
                };
                out.insert("goal".to_string(), serde_json::json!("AutonomousGoal"));
                out.insert("intent".to_string(), serde_json::json!(intent));
                out.insert("plan_steps".to_string(), serde_json::json!(plan.steps));
                Ok(serde_json::Value::Object(out))
            }
            Goal::UpdateKnowledgeSlot {
                slot_id,
                source_url,
                source_html,
            } => {
                if !self.pagi_kb_active(slot_id) {
                    return Ok(serde_json::json!({
                        "status": "kb_disabled",
                        "message": format!("KB-{} is disabled by the control panel.", slot_id),
                        "slot_id": slot_id
                    }));
                }
                if let Some(ref reg) = self.skill_manifest_registry {
                    if !validate_skill_permissions(reg, "CommunityScraper", slot_id, self.firewall_strict_mode) {
                        return Err(SovereigntyViolation {
                            skill_id: "CommunityScraper".to_string(),
                            kb_layer: slot_id,
                        }
                        .into());
                    }
                }
                let mut payload = serde_json::json!({ "slot_id": slot_id });
                if let Some(url) = source_url {
                    payload["url"] = serde_json::Value::String(url);
                }
                if let Some(html) = source_html {
                    payload["html"] = serde_json::Value::String(html);
                }
                let skill = self
                    .registry
                    .get("CommunityScraper")
                    .ok_or_else(|| UnknownSkill("CommunityScraper".into()))?;
                skill.execute(ctx, Some(payload)).await
            }
            Goal::MemoryOp { path, value } => {
                Ok(serde_json::json!({ "path": path, "value": value, "status": "dispatched" }))
            }
            Goal::Custom(s) => Ok(serde_json::json!({ "custom": s, "status": "dispatched" })),
        }
    }
}

/// Extracts KB layer (1..=9) from a skill payload when present. Used by the Sovereignty Firewall.
fn extract_kb_layer_from_payload(payload: Option<&serde_json::Value>) -> Option<u8> {
    let p = payload.as_ref()?;
    let n = p
        .get("slot_id")
        .or_else(|| p.get("kb_layer"))
        .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|i| i as u64)))?;
    let layer = n.clamp(1, 9) as u8;
    Some(layer)
}

/// Derives the next skill's payload from the previous skill's result (output chaining).
fn chain_payload(
    previous_skill: Option<&str>,
    next_skill: &str,
    previous_result: &serde_json::Value,
    fallback: serde_json::Value,
) -> Option<serde_json::Value> {
    match (previous_skill, next_skill) {
        (Some("DraftResponse"), "SalesCloser") => {
            let draft = previous_result
                .get("draft")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(serde_json::json!({ "draft": draft }))
        }
        (Some("SalesCloser"), "ModelRouter") => {
            let prompt = previous_result
                .get("draft")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(serde_json::json!({ "prompt": prompt }))
        }
        (Some("DraftResponse"), "ModelRouter") => {
            let prompt = previous_result
                .get("draft")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(serde_json::json!({ "prompt": prompt }))
        }
        (Some("CommunityScraper"), "ModelRouter") => {
            let prompt = previous_result
                .get("event")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(serde_json::json!({ "prompt": prompt }))
        }
        _ if previous_result.is_null() => Some(fallback),
        _ => Some(fallback),
    }
}

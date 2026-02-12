//! pagi-core: AGI core library (shared types, orchestrator, memory, 8-slot knowledge base).
//!
//! Re-exports the former pagi-shared, pagi-orchestrator, pagi-memory, and pagi-knowledge
//! so add-ons and the gateway keep a consistent public API.

mod config;
mod env_sync;
mod hot_reload;
mod knowledge;
mod openrouter_service;
mod memory;
mod orchestrator;
mod project_vault;
mod secure_memory;
mod security;
mod shadow_store;
mod shared;
pub mod social_intelligence;
pub mod prompts;
pub mod skills;
pub mod updater;

// Qdrant sidecar management (vector feature)
#[cfg(feature = "vector")]
pub mod qdrant_sidecar;

// Shared (former pagi-shared) + Emotional Context Layer + Task Governance
pub use shared::{
    BiometricState, CoreConfig, EthosPolicy, Goal, MentalState, MENTAL_STATE_KEY, SovereignAttributes,
    SovereignConfig,
    PersonRecord, SomaState, TenantContext, KARDIA_PEOPLE_PREFIX, DEFAULT_AGENT_ID, ETHOS_POLICY_KEY,
    // Dynamic Task Governance (Oikos)
    GovernanceAction, GovernedTask, TaskDifficulty, TaskGovernor,
    OIKOS_TASK_PREFIX, OIKOS_GOVERNANCE_SUMMARY_KEY,
};

// Configuration (Beta Distribution - User Config Manager)
pub use config::{SovereignConfig as SovereignConfigStruct, UserConfig};
pub use shadow_store::{DecryptedEntry, PersonalHistoryEntry, ShadowStore, ShadowStoreHandle};

// Memory (former pagi-memory)
pub use memory::MemoryManager;

// Knowledge (former pagi-knowledge) - L2 Memory System + Shadow Vault
pub use knowledge::{
    initialize_core_identity, initialize_core_skills, initialize_ethos_policy, initialize_therapist_fit_checklist, pagi_kb_slot_label, verify_identity, IdentityStatus, AgentMessage, AlignmentResult, EventRecord, Kb1, Kb2, Kb3,
    Kb4, Kb5, Kb6, Kb7, Kb8, KbRecord, KbStatus, KbType, KnowledgeSource, KnowledgeStore,
    PolicyRecord, RelationRecord, SelfAuditReport, SovereignState, UserPersona, ABSURDITY_LOG_PREFIX, ETHOS_DEFAULT_POLICY_KEY, SkillRecord, SLOT_LABELS, SOVEREIGN_IDENTITY_KEY, kardia_relation_key,
    EmotionalAnchor, SecretVault, VaultError,
    // Plugin Architecture
    ModuleData, ModuleError, ModuleRegistry, SkillPlugin, SkillPluginRegistry,
    SovereignModule, ThreatContext as ModuleThreatContext, ThreatSignal,
};

// Vector Store (Production Semantic Memory Layer)
#[cfg(feature = "vector")]
pub use knowledge::vector_store::{
    create_vector_store, VectorError, VectorResult, VectorSearchResult,
    VectorStore, VectorStoreStatus, QdrantVectorStore,
};

// Social Intelligence Layer (KB-07 Kardia Enhancement)
pub use social_intelligence::{
    AstralContext, ContactReminder, StrategicImportance, StrategicValue,
    SubjectProfile, ZodiacSign,
};

// Orchestrator (former pagi-orchestrator) + MoE gating + Autonomous Maintenance
pub use orchestrator::{
    AgentSkill, BlueprintRegistry, ControlPanelMessage, ControlPanelReceiver, Gater,
    HeuristicProcessor, HeuristicResult, MoEMode, MoEExpert, Orchestrator, OrchestratorMode,
    Plan, PersonaCoordinator, PersonaCoordinatorState, route_to_experts, SignProfile, SkillRegistry,
    SkillInventoryEntry, SkillManifestEntry, SkillManifestRegistry, SovereigntyViolation, TierManifest, TrustTier, validate_skill_permissions,
    SovereignDomain, UserArchetype, zodiac_behavioral_hint, humanity_blend_label,
    get_effective_archetype_for_turn, query_domain, QueryDomain, suggest_archetype_from_query,
    archetype_auto_switch_disabled, ArchetypeOverlay, ArchetypePrompt,
    needs_onboarding, onboarding_sequence, OnboardingKbSlot, OnboardingState,
    ONBOARDING_COMPLETE_KEY, PHASE1_RECOGNITION, PHASE3_CTA,
    KB01_USER_PROFILE_KEY,
    process_archetype_triggers, active_archetype_label, get_sovereignty_leak_triggers,
    ArchetypeTriggerResult,
    Heuristic, ManeuverOutcome, Protector, RoiResult, ThreatContext, VitalityLevel,
    // Sovereign Security Protocols (KB-05) + Astro-Logic subject ranking
    ProtocolEngine, rank_subject_from_sovereignty_triggers, matched_sovereignty_triggers,
    // Autonomous Maintenance & Reflexion Loop
    init_maintenance_loop, IdleTracker, MaintenanceConfig, TelemetryPulse,
    // Maintenance Dashboard (SSE pulse events + UI approval bridge)
    MaintenancePulseEvent, PendingApproval, ApprovalBridgeHandle, new_approval_bridge,
    // Phase 4.5: Validation Benchmarks
    PerformanceDelta, ValidationResult, SmokeTestRunner,
    // Evolutionary Versioning & Genetic Memory
    compute_patch_dna, check_genetic_dead_end, record_genetic_dead_end,
    // Astro-Weather (Transit vs KB-01, SYSTEM_PROMPT + KB-08 correlation)
    check_astro_weather, record_transit_correlation_if_high_risk, system_alert_if_high_risk,
    system_prompt_block, should_refresh, AstroWeatherState, TransitRiskLevel, STALE_MS,
    // Sovereign Health Report (KB-08 Analytics + Archetype usage)
    generate_weekly_report, generate_weekly_sovereignty_report, record_archetype_usage,
    HealthReport, LeakStats, RestVsOutputEntry, ShieldedEvent, TransitCorrelationEntry, ArchetypeUsageBreakdown,
    // Strategic Timing (Phase 2): thinking latency for Sovereign Peer cadence
    calculate_thinking_latency,
    // Sovereign Voice + Tone Firewall
    detect_tone_drift,
};

// OpenRouter Sovereign Bridge (high-level reasoning only; actions and memory stay local)
pub use openrouter_service::{BridgePlan, OpenRouterBridge};

// Live Skills System (priority-based execution with KB-05 security validation)
pub use skills::{
    AuditSkill, EnergyCost, FileSystemSkill, FolderSkill, LiveSkill, LiveSkillRegistry, RefactorSkill,
    ShellExecutorSkill, SkillExecutionRequest, SkillExecutionResult, SkillPriority, WebSearchSkill,
};

// Project Vault: folder summary and sovereign write (Master Analysis + Document Session)
pub use project_vault::{
    summarize_folder_for_context, summarize_folder_for_context_sync, write_document_under_root,
};

// SAO Redaction: protected terms in meeting transcripts/minutes
pub use security::{SAORedactor, PROTECTED_PLACEHOLDER};

// Hot Reload System: Dynamic skill loading for The Forge
pub use hot_reload::{
    HotReloadConfig, HotReloadManager, HotReloadResult, HotReloadedSkillMeta,
    SoftReloadSignal, disable_hot_reload, enable_hot_reload, get_hot_reload_manager,
    hot_reload_skill, init_hot_reload, is_hot_reload_enabled, list_hot_reloaded_skills,
};

// -----------------------------------------------------------------------------
// Self-Audit (KB-08): Logic inconsistencies for the self-improving SAO
// -----------------------------------------------------------------------------

/// Sovereign Sync: add missing keys from `.env.example` to `.env` without overwriting existing values.
pub use env_sync::sync_env_files;

/// Periodically scan KB-08 (Absurdity Log) and summarize logic inconsistencies for the user.
/// Reinforces the SAO's self-improving nature; call from gateway or dashboard to expose "where the AGI is making mistakes."
pub fn self_audit(store: &KnowledgeStore) -> Result<SelfAuditReport, String> {
    store.get_absurdity_log_summary(10).map_err(|e| e.to_string())
}

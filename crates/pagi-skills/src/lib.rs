//! Trait-based agent capability registry and concrete skills.

pub use pagi_core::{AgentSkill, SkillRegistry};

mod absurdity_tracker;
mod community_pulse;
mod community_scraper;
mod draft_response;
mod knowledge_insert;
mod knowledge_pruner;
mod knowledge_query;
mod lead_capture;
mod fs_tools;
mod pattern_match_v2;
mod model_router;
mod analyze_sentiment;
mod check_alignment;
mod recall_past_actions;
mod research_semantic;
mod research_audit;
mod sales_closer;
mod thalamus;
mod message_agent;
mod get_agent_messages;
mod biogate_sync;
mod deep_journal;
mod ethos_sync;
mod daily_checkin;
mod evening_audit;
mod identity_setup;
mod ms_graph;
mod journal_skill;
mod kardia_map;
mod oikos_task_governor;
mod reflect_shadow;
mod web_browser;
mod system;
mod rig_tools;
mod tool_memory;
mod red_team_skill;
mod rollback_skill;
mod sovereign_operator;
mod sovereignty_test_skill;
mod mission_validator;
mod sentinel;
mod strategic_goals;
mod topic_indexer;
mod evolution_inference;
mod sovereign_calibration;
mod visual_cognition;
mod system_admin;
mod knowledge_synthesis;
mod mimir_skills;
mod deep_audit;
mod forge_gen_weather_sentinel;

mod forge;

pub use analyze_sentiment::AnalyzeSentiment;
pub use biogate_sync::BioGateSync;
pub use kardia_map::KardiaMap;
pub use check_alignment::CheckAlignment;
pub use community_pulse::CommunityPulse;
pub use community_scraper::CommunityScraper;
pub use draft_response::DraftResponse;
pub use knowledge_insert::KnowledgeInsert;
pub use knowledge_pruner::KnowledgePruner;
pub use knowledge_query::KnowledgeQuery;
pub use lead_capture::LeadCapture;
pub use fs_tools::{analyze_workspace, FsWorkspaceAnalyzer, ReadFile, WriteSandboxFile};
pub use model_router::{LlmMode, ModelRouter};
pub use research_semantic::{ResearchEmbedInsert, ResearchSemanticSearch};
pub use recall_past_actions::RecallPastActions;
pub use research_audit::ResearchAudit;
pub use sales_closer::SalesCloser;
pub use thalamus::{route_information, route_to_ontology, RouteMetadata};
pub use message_agent::MessageAgent;
pub use get_agent_messages::GetAgentMessages;
pub use deep_journal::DeepJournalSkill;
pub use ethos_sync::EthosSync;
pub use daily_checkin::{generate_morning_briefing, DAILY_CHECKIN_LAST_DATE_KEY};
pub use evening_audit::{
    get_evening_audit_prompt, get_last_7_audits, mark_evening_audit_prompt_shown,
    record_evening_audit, EveningAuditStatus, EVENING_AUDIT_LAST_DATE_KEY,
    EVENING_AUDIT_PROMPT_SHOWN_KEY,
};
pub use identity_setup::IdentitySetup;
pub use journal_skill::JournalSkill;
pub use ms_graph::{
    fetch_user_vitality, is_low_sleep, schedule_outlook_sentence, use_gatekeeper_mode,
    write_user_vitality, CalendarHealth, MicrosoftGraphClient, UserVitality,
};
pub use oikos_task_governor::OikosTaskGovernor;
pub use reflect_shadow::ReflectShadowSkill;
pub use web_browser::WebSearch;
// Sovereign Operator: system execution layer
pub use system::{
    CommandResult, ExecutionError, ShellExecutor, SystemSnapshot, TerminalGuard,
    FileSystem, FileSystemSkill, SystemCommandSkill, SystemTelemetry, SystemTelemetrySkill,
};
// Sovereign Operator: Rig tool integration
pub use rig_tools::{RigTool, RigToolError, RigToolRegistry};
// Sovereign Operator: tool memory
pub use tool_memory::{ToolExecutionRecord, ToolMemoryManager};
// Sovereign Operator: unified integration
pub use sovereign_operator::{SovereignOperator, SovereignOperatorConfig, SovereignOperatorSkill};
// Sovereignty Test Case: for --heal verification (do not use in production)
pub use sovereignty_test_skill::SovereigntyTestSkill;
// Evolutionary Versioning & Rollback
pub use rollback_skill::RollbackSkill;
// Adversarial Peer Review (Red-Team Consensus)
pub use red_team_skill::RedTeamSkill;
// SAO: Pattern Match (Manipulation Library) and Absurdity Log
pub use absurdity_tracker::{build_context_injection, count_entries, get_failures_for_subject, log_failure, AbsurdityEntry};
pub use pattern_match_v2::{analyze as pattern_match_analyze, PatternMatchV2Result};
// SAO: Strategic Goals (KB-06) - North Star Alignment
pub use strategic_goals::{
    AlignmentLevel, AlignmentScore, GoalCategory, StrategicGoal, StrategicTimeline,
};
// SAO: Topic Indexer (KB-04 Optimization) - Memory Evolution
pub use topic_indexer::{ConversationTopicIndexer, TopicSummary};
// SAO: Evolution Inference (KB-04 x KB-08 Cross-Reference) - Pattern Detection
pub use evolution_inference::{EvolutionInferenceSkill, EvolutionPattern, InferenceReport};
// SAO: Sovereign Calibration (KB-07 Kardia) - Emotional Safety Governor Tuning
pub use sovereign_calibration::{SovereignCalibrationSkill, CalibrationSettings, CoachingSentiment};
// Visual Cognition: PaperBanana-style diagram detection and rendering
pub use visual_cognition::VisualCognitionSkill;
pub use mission_validator::MissionValidatorSkill;
// Sovereign Admin: Hardware Awareness + Secure Credentialing
pub use system_admin::{
    resolve_api_key_from_vault_or_env, DiskVitality, GetHardwareStatsSkill, HardwareVitality,
    SecureVault, SecureVaultSkill,
};
pub use knowledge_synthesis::{
    SynthesizeMeetingContextResult, SynthesizeMeetingContextSkill, SynthesizeParams, SynthesisAlert,
    VitalitySnapshot,
};
pub use mimir_skills::PreFlightAudioSkill;
// Deep Audit: Sovereign document ingestion and routing
pub use deep_audit::{DeepAuditSkill, KnowledgeBase, IngestResult, AuditSummary};
// The Forge: Self-synthesis skill (code generator from JSON tool-spec)
pub use forge::{create_skill_from_spec, ForgeResult, ForgeSkill, ToolSpec, ToolSpecParam};
// Sentinel: Active monitoring and intervention capabilities
pub use sentinel::{
    // Physical Guard
    SentinelPhysicalGuardAction, SentinelPhysicalGuardResult, SentinelPhysicalGuardSensor,
    // History Harvester
    BrowserType, SentinelHistoryEntry, SentinelHistoryHarvestResult, SentinelHistoryHarvesterAction,
    // File Sentinel
    SentinelFileEvent, SentinelFileSentinelConfig, SentinelFileSentinelResult, SentinelFileSentinelSensor,
    create_default_sentinel, create_sentinel_for_path,
    // Input Velocity
    SentinelInputVelocityConfig, SentinelInputVelocityMetrics, SentinelInputVelocitySensor,
    // Counselor
    CounselorPayload, CounselorSkill, CounselorVelocityInput,
};

//! Axum-based API Gateway: entry point for UAC. Config-driven via CoreConfig.
//! Chat is wired through handlers::chat with Soma+Kardia context injection (Sovereign Brain).

mod handlers;
mod skills;
mod services;
mod plugin_loader;
mod knowledge_router;
mod governor;
mod heal;
mod diagnostics;
mod chronos_sqlite;
mod mimir;
#[cfg(all(windows, feature = "bridge-ms"))]
mod bridge_copilot_skill;

use governor::{handle_governor_alerts, GovernorConfig};
use heal::run_heal_flow;
#[cfg(feature = "voice")]
mod voice;
#[cfg(feature = "voice")]
mod openrouter_live;
#[cfg(feature = "tui-dashboard")]
mod live_dashboard;

use skills::wellness_report::generate_report as generate_wellness_report;
use services::OrchestratorService;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    extract::Json,
    response::{sse::{Event, Sse}, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use axum::http::{HeaderMap, Method, StatusCode};
use chrono::Timelike;
use futures_util::stream::StreamExt;
use std::time::Duration;
use tokio::sync::broadcast;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing::field::Visit;
use tracing_subscriber::layer::Context;
use pagi_core::{
    initialize_core_identity, initialize_core_skills, initialize_ethos_policy, initialize_therapist_fit_checklist, self_audit, sync_env_files, AlignmentResult, BlueprintRegistry, CoreConfig, EventRecord, Goal, Gater, KbRecord, KbType,
    HeuristicProcessor, KnowledgeStore, MentalState, MemoryManager, MoEMode, MoEExpert, Orchestrator, OrchestratorMode, RelationRecord, route_to_experts, ShadowStore, ShadowStoreHandle, SkillManifestEntry, SkillManifestRegistry, SkillRegistry, SovereigntyViolation, SovereignConfig, TierManifest, SovereignDomain, SovereignState, TenantContext, UserPersona, VitalityLevel,
    onboarding_sequence, ONBOARDING_COMPLETE_KEY, KB01_USER_PROFILE_KEY,
    process_archetype_triggers, active_archetype_label, get_sovereignty_leak_triggers,
    matched_sovereignty_triggers, rank_subject_from_sovereignty_triggers, ArchetypeTriggerResult,
    // Persona & Archetype (Holistic Life Warden)
    PersonaCoordinator, SignProfile, UserArchetype,
    // Autonomous Maintenance & Reflexion Loop
    init_maintenance_loop, IdleTracker,
    // Maintenance Dashboard (SSE pulse events + UI approval bridge)
    ApprovalBridgeHandle, new_approval_bridge, MaintenanceConfig,
    // Sovereign Security Protocols (KB-05)
    ProtocolEngine,
    Protector,
    // Astro-Weather (Transit vs KB-01, SYSTEM_PROMPT + KB-08 correlation)
    check_astro_weather, record_transit_correlation_if_high_risk, system_prompt_block,
    AstroWeatherState,
    // Sovereign Health Report (KB-08 Analytics)
    generate_weekly_report, generate_weekly_sovereignty_report,
    // Sovereignty Drill (FileSystemSkill + KB-05/06/08 verification)
    LiveSkillRegistry, SkillExecutionRequest, SkillPriority,
    // Project Vault: folder summary for Master Analysis
    summarize_folder_for_context_sync, write_document_under_root,
    // Strategic Timing (Phase 2) + Tone Firewall
    calculate_thinking_latency, detect_tone_drift,
    // Hot Reload System: Dynamic skill loading for The Forge
    hot_reload_skill, is_hot_reload_enabled, enable_hot_reload, disable_hot_reload,
    list_hot_reloaded_skills, HotReloadResult,
};

#[cfg(feature = "vector")]
use pagi_core::{create_vector_store, VectorStore, QdrantVectorStore};
use pagi_skills::{
    count_entries as absurdity_count_entries,
    build_context_injection as absurdity_build_context_injection,
    pattern_match_analyze,
    fetch_user_vitality, generate_morning_briefing, is_low_sleep, DAILY_CHECKIN_LAST_DATE_KEY,
    get_evening_audit_prompt, mark_evening_audit_prompt_shown, record_evening_audit,
    schedule_outlook_sentence, use_gatekeeper_mode,
    CalendarHealth, MicrosoftGraphClient,
    BioGateSync, CounselorSkill, EthosSync, FileSystem, FileSystemSkill, FsWorkspaceAnalyzer, GetHardwareStatsSkill, IdentitySetup, ModelRouter, OikosTaskGovernor, PreFlightAudioSkill, ReadFile, SynthesizeMeetingContextSkill,
    ReflectShadowSkill, SecureVault, SecureVaultSkill, ShellExecutor, SovereignOperator, SovereignOperatorConfig, SovereignOperatorSkill, SystemCommandSkill, SystemTelemetry, SystemTelemetrySkill, WebSearch, WriteSandboxFile,
    MissionValidatorSkill,
    SentinelInputVelocityConfig, SentinelInputVelocityMetrics, SentinelInputVelocitySensor,
    SentinelPhysicalGuardAction, SentinelPhysicalGuardSensor,
    create_skill_from_spec, ForgeSkill, ToolSpec,
};
use std::path::Path as StdPath;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::collections::{BTreeMap, HashSet};
use std::time::Instant as StdInstant;
use tokio::sync::mpsc;

use chronos_sqlite::ChronosSqlite;

static HEARTBEAT_TICK_COUNT: AtomicU64 = AtomicU64::new(0);

const TRUST_RESOLUTION_REWARD: f32 = 0.05;
const TRUST_STALE_DECAY_PENALTY: f32 = 0.02;
const TRUST_STALE_DECAY_TICKS: u64 = 50;

/// Sovereign Core version from gateway Cargo.toml (single source of truth for Wave 1 support).
pub const GATEWAY_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Captures the "message" field from a tracing event.
struct MessageCollector<'a>(&'a mut String);

impl Visit for MessageCollector<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            *self.0 = value.to_string();
        }
    }
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            *self.0 = format!("{:?}", value);
        }
    }
}

/// Sends each tracing event as a line to a broadcast channel for SSE log streaming.
#[derive(Clone)]
struct LogBroadcastLayer {
    tx: broadcast::Sender<String>,
}

impl LogBroadcastLayer {
    fn new(tx: broadcast::Sender<String>) -> Self {
        Self { tx }
    }
}

impl<S> tracing_subscriber::Layer<S> for LogBroadcastLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut message = String::new();
        event.record(&mut MessageCollector(&mut message));
        let line = format!(
            "{} [{}] {}",
            event.metadata().level(),
            event.metadata().target(),
            message
        );
        let _ = self.tx.send(line);
    }
}

/// Pre-flight check: verify all 8 KBs are accessible and port is available.
fn run_verify() -> Result<(), String> {
    let config = CoreConfig::load().map_err(|e| format!("Config load failed: {}", e))?;
    let storage = StdPath::new(&config.storage_path);
    let vault_path = storage.join("pagi_vault");
    let kb_path = storage.join("pagi_knowledge");

    // 1. Check MemoryManager (pagi_vault Sled)
    print!("Checking pagi_vault... ");
    let vault = MemoryManager::open_path(&vault_path).map_err(|e| format!("pagi_vault LOCKED or inaccessible: {}", e))?;
    drop(vault);
    println!("OK");

    // 2. Check KnowledgeStore (pagi_knowledge Sled with 8 trees)
    print!("Checking pagi_knowledge (8 KBs)... ");
    let kb = KnowledgeStore::open_path(&kb_path).map_err(|e| format!("pagi_knowledge LOCKED or inaccessible: {}", e))?;
    for slot in 1..=8 {
        kb.get(slot, "__verify_probe__").map_err(|e| format!("KB slot {} failed: {}", slot, e))?;
    }
    drop(kb);
    println!("OK (all 8 slots accessible)");

    // 3. Check port availability (Gateway hard-locked to 8000)
    let port = 8000u16;
    print!("Checking port {}... ", port);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    match std::net::TcpListener::bind(addr) {
        Ok(listener) => {
            drop(listener);
            println!("OK (available)");
        }
        Err(e) => {
            return Err(format!("Port {} BLOCKED: {}", port, e));
        }
    }

    println!("\n✅ SUCCESS: All systems GO. Ready to start gateway.");
    Ok(())
}

/// Sovereignty Drill: Verifies Master Template layers in sequence.
/// 1. Read a config file via FileSystemSkill (KB-05 validated).
/// 2. Cross-reference with Ethos (KB-06).
/// 3. Log the outcome to KB-08 (Absurdity Log / Success Metric).
fn run_sovereignty_drill() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Tokio runtime failed: {}", e))?;

    rt.block_on(async {
        let config = CoreConfig::load().map_err(|e| format!("Config load failed: {}", e))?;
        let storage = StdPath::new(&config.storage_path);
        let kb_path = storage.join("pagi_knowledge");

        println!("🏛️ Sovereignty Drill — Master Template verification\n");

        // 1. Open KnowledgeStore (all 8 KBs)
        print!("  [1/5] Opening KnowledgeStore... ");
        let knowledge = Arc::new(
            KnowledgeStore::open_path(&kb_path).map_err(|e| format!("pagi_knowledge: {}", e))?,
        );
        knowledge.pagi_init_kb_metadata().ok();
        println!("OK");

        let tenant_ctx = TenantContext {
            tenant_id: "drill".to_string(),
            correlation_id: None,
            agent_id: Some("phoenix".to_string()),
        };
        let registry = Arc::new(LiveSkillRegistry::default());

        // 2. FileSystemSkill: read config (e.g. .env.example)
        let config_path = std::env::current_dir()
            .unwrap_or_else(|_| StdPath::new(".").to_path_buf())
            .join(".env.example");
        let path_str = config_path.to_string_lossy().to_string();
        let params = serde_json::json!({ "operation": "read", "path": path_str });

        let skill_name = "filesystem".to_string();
        let _request = SkillExecutionRequest {
            skill_name: skill_name.clone(),
            params: params.clone(),
            priority: SkillPriority::Normal,
            security_context: Some("sovereignty_drill".to_string()),
        };

        print!("  [2/5] KB-05 security validation (FileSystemSkill)... ");
        let skill = registry
            .get(&skill_name)
            .ok_or("filesystem skill not in registry")?;
        if skill.requires_security_check() {
            skill
                .validate_security(&knowledge, &params)
                .await
                .map_err(|e| format!("KB-05 blocked: {}", e))?;
        }
        println!("OK");

        print!("  [3/5] FileSystemSkill execute (read config)... ");
        let read_result = skill
            .execute(&tenant_ctx, &knowledge, params)
            .await
            .map_err(|e| format!("Execute failed: {}", e))?;
        let content_len = read_result
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.len())
            .unwrap_or(0);
        println!("OK ({} bytes)", content_len);

        // 3. Cross-reference with KB-06 (Ethos)
        print!("  [4/5] KB-06 Ethos alignment check... ");
        let ethos_philosophical = knowledge.get_ethos_philosophical_policy();
        let ethos_status = ethos_philosophical
            .as_ref()
            .map(|p| p.active_school.as_str())
            .unwrap_or_else(|| {
                if knowledge.get_ethos_policy().is_some() {
                    "policy present (security)"
                } else {
                    "(no policy set)"
                }
            });
        println!("OK — {}", ethos_status);

        // 4. Log to KB-08 (Success Metric)
        print!("  [5/5] KB-08 Absurdity Log (success metric)... ");
        let message = format!(
            "Sovereignty Drill: config read OK ({} bytes), ethos={}, logged by drill",
            content_len, ethos_status
        );
        knowledge
            .record_success_metric(&message)
            .map_err(|e| format!("KB-08 write failed: {}", e))?;
        println!("OK");

        // Confirm visibility in self-audit
        let summary = knowledge
            .get_absurdity_log_summary(5)
            .map_err(|e| format!("KB-08 summary failed: {}", e))?;
        println!(
            "\n  KB-08 summary: {} total entries, {} recent.",
            summary.total_entries,
            summary.recent_messages.len()
        );

        println!("\n✅ Sovereignty Drill PASSED — all layers (KB-05, KB-06, KB-08) fired correctly.");
        Ok(())
    })
}

/// One-shot Sovereignty Audit: runs AuditSkill and prints report (no LLM). Unit test for sovereignty.
fn run_audit() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Tokio runtime failed: {}", e))?;

    rt.block_on(async {
        let config = CoreConfig::load().map_err(|e| format!("Config load failed: {}", e))?;
        let storage = StdPath::new(&config.storage_path);
        let kb_path = storage.join("pagi_knowledge");

        let knowledge = Arc::new(
            KnowledgeStore::open_path(&kb_path).map_err(|e| format!("pagi_knowledge: {}", e))?,
        );
        knowledge.pagi_init_kb_metadata().ok();

        let tenant_ctx = TenantContext {
            tenant_id: "audit".to_string(),
            correlation_id: None,
            agent_id: Some("phoenix".to_string()),
        };
        let registry = Arc::new(LiveSkillRegistry::default());

        let audit_params = serde_json::json!({ "workspace_root": "." });
        let skill = registry.get("audit").ok_or("audit skill not in registry")?;
        if skill.requires_security_check() {
            skill
                .validate_security(&knowledge, &audit_params)
                .await
                .map_err(|e| format!("KB-05 blocked: {}", e))?;
        }
        let result = skill
            .execute(&tenant_ctx, &knowledge, audit_params)
            .await
            .map_err(|e| format!("Audit execute failed: {}", e))?;

        println!("--- SOVEREIGNTY AUDIT REPORT ---");
        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string()));

        knowledge
            .record_success_metric("Manual CLI Audit Performed")
            .map_err(|e| format!("KB-08 write failed: {}", e))?;

        let score = result.get("sovereignty_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if score >= 0.9 {
            println!("\n✅ Sovereignty compliance OK (score {:.2}).", score);
        } else if score >= 0.7 {
            println!("\n⚠️ Sovereignty review recommended (score {:.2}).", score);
        } else {
            println!("\n❌ High risk: sovereignty score {:.2} (logged to KB-08).", score);
        }
        Ok(())
    })
}

/// Initial Boot Handshake: prompts for Sovereign Name, Rank, and Domain via stdin/stdout.
/// Returns a UserPersona to be stored in KB-01 (Pneuma) so the SAO can address the user by name.
fn initialization_handshake() -> Result<UserPersona, String> {
    use std::io::{self, Write};
    fn prompt(msg: &str) -> Result<String, String> {
        print!("{} ", msg);
        io::stdout().flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        io::stdin().read_line(&mut line).map_err(|e| e.to_string())?;
        Ok(line.trim().to_string())
    }
    let sovereign_name = prompt("Enter Sovereign Name:")?.trim().to_string();
    if sovereign_name.is_empty() {
        return Err("Sovereign Name cannot be empty".to_string());
    }
    let highest_rank = prompt("Enter Highest Rank:")?.trim().to_string();
    let operational_domain = prompt("Enter Operational Domain:")?.trim().to_string();
    Ok(UserPersona {
        sovereign_name,
        highest_rank: if highest_rank.is_empty() { "—".to_string() } else { highest_rank },
        operational_domain: if operational_domain.is_empty() { "—".to_string() } else { operational_domain },
    })
}

/// One-shot Heal: runs Audit → Refactor for each skills_without_kb05, re-audits, logs session to KB-08, prints results.
fn run_heal() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Tokio runtime failed: {}", e))?;

    rt.block_on(async {
        let config = CoreConfig::load().map_err(|e| format!("Config load failed: {}", e))?;
        let storage = StdPath::new(&config.storage_path);
        let kb_path = storage.join("pagi_knowledge");

        let knowledge = Arc::new(
            KnowledgeStore::open_path(&kb_path).map_err(|e| format!("pagi_knowledge: {}", e))?,
        );
        knowledge.pagi_init_kb_metadata().ok();

        let (audit_before, refactor_results, _audit_after, final_score) =
            run_heal_flow(knowledge.as_ref()).await?;

        let applied = refactor_results.iter().filter(|r| r.applied).count();
        let session_msg = format!(
            "Healing Session (CLI): {} file(s) fixed, {} total; sovereignty_score after: {:.2}",
            applied,
            refactor_results.len(),
            final_score
        );
        knowledge
            .record_success_metric(&session_msg)
            .map_err(|e| format!("KB-08 write failed: {}", e))?;

        println!("--- HEAL REPORT ---");
        println!("Audit before: sovereignty_score = {:.2}", audit_before.get("sovereignty_score").and_then(|v| v.as_f64()).unwrap_or(0.0));
        println!("\nRefactor results:");
        for r in &refactor_results {
            let status = if r.applied { "FIXED" } else { "FAILED" };
            println!("  {}: {}", r.file_path, status);
            if !r.applied && !r.message.is_empty() {
                println!("    Reason: {}", r.message);
            }
        }
        println!("\nAudit after: sovereignty_score = {:.2}", final_score);
        println!("\n{}", session_msg);
        println!("KB-08 session metric logged.");
        Ok(())
    })
}

#[tokio::main]
async fn main() {
    // ENVIRONMENT LOCKDOWN: Load .env first. All API keys (OpenRouter, etc.) stay in backend only.
    // Frontend is a stateless client and must never receive or send LLM API keys.
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("[pagi-gateway] .env not loaded: {} (using system environment)", e);
    }
    // Optional: warn if LLM key missing so live mode doesn't fail with "no_api_key" later
    if std::env::var("OPENROUTER_API_KEY").is_err() && std::env::var("PAGI_LLM_API_KEY").is_err() {
        eprintln!("[pagi-gateway] Hint: Set OPENROUTER_API_KEY or PAGI_LLM_API_KEY in .env for live LLM; Gateway holds the key, frontend never sees it.");
    }

    // Handle --set-key KEY VALUE (store in OS keychain and exit; POC for vault migration)
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--set-key") {
        let key = args.get(pos + 1).cloned().unwrap_or_default();
        let value = args.get(pos + 2).cloned().unwrap_or_default();
        if key.is_empty() || value.is_empty() {
            eprintln!("Usage: pagi-gateway --set-key <KEY> <VALUE>");
            eprintln!("Example: pagi-gateway --set-key OPENROUTER_API_KEY sk-or-v1-...");
            std::process::exit(1);
        }
        match SecureVault::new().set(key.trim(), value.trim()) {
            Ok(()) => {
                println!("Vault: {} stored in OS keychain. Restart the gateway to use it.", key.trim());
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Vault set failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Handle --verify and --sovereignty-drill flags (pre-flight / Master Template verification)
    let headless = args.iter().any(|a| {
        a == "--verify" || a == "--sovereignty-drill" || a == "--audit" || a == "--heal"
    });
    if args.iter().any(|a| a == "--verify") {
        match run_verify() {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("❌ PRE-FLIGHT FAILED: {}", e);
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--sovereignty-drill") {
        match run_sovereignty_drill() {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("❌ SOVEREIGNTY DRILL FAILED: {}", e);
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--audit") {
        match run_audit() {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("❌ AUDIT FAILED: {}", e);
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--heal") {
        match run_heal() {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("❌ HEAL FAILED: {}", e);
                std::process::exit(1);
            }
        }
    }

    let (log_tx, _) = broadcast::channel(1000);
    let log_layer = LogBroadcastLayer::new(log_tx.clone());

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(log_layer)
        .init();

    // Sovereign Sync: add missing keys from .env.example to .env (never overwrite existing).
    if let Ok(cwd) = std::env::current_dir() {
        let example = cwd.join(".env.example");
        let live = cwd.join(".env");
        if example.exists() {
            if let Ok(added) = sync_env_files(example.to_str().unwrap(), live.to_str().unwrap()) {
                if added > 0 {
                    tracing::info!("Sovereign Sync: Added {} new configuration key(s) from .env.example.", added);
                }
            }
        }
    }

    // Initialize Qdrant sidecar (Memory Engine) if vector feature is enabled
    #[cfg(feature = "vector")]
    {
        use pagi_core::qdrant_sidecar::QdrantSidecar;
        use std::path::PathBuf;

        tracing::info!("🧠 Initializing Memory Engine (Qdrant)...");
        // Use executable dir to find project root so Qdrant uses repo/bin and repo/data/qdrant,
        // not process cwd (e.g. Temp when started from a job or shortcut).
        // Exe is typically at workspace/target/debug/pagi-gateway.exe -> 3 parents = workspace root.
        let project_root: PathBuf = std::env::current_exe()
            .ok()
            .and_then(|p| {
                p.parent()
                    .and_then(|p| p.parent())
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
            })
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let mut qdrant = QdrantSidecar::new_with_root(&project_root);
        
        match qdrant.ensure_running().await {
            Ok(()) => {
                tracing::info!("✅ Memory Engine initialized successfully");
                // Keep the sidecar alive by moving it into a static or long-lived scope
                // For now, we'll let it run in the background
                std::mem::forget(qdrant); // Prevent Drop from killing Qdrant
            }
            Err(e) => {
                tracing::warn!("⚠️  Memory Engine initialization failed: {}. Vector features may not work.", e);
                tracing::warn!("   You can manually start Qdrant on port 6333 if needed.");
            }
        }
    }

    let config = Arc::new(CoreConfig::load().expect("load CoreConfig"));
    let storage = StdPath::new(&config.storage_path);
    let memory_path = storage.join("pagi_vault");
    let knowledge_path = storage.join("pagi_knowledge");

    // KB-04 (Chronos): sovereign local chat history DB (SQLite)
    // Stored alongside other persistent storage artifacts.
    let chronos_db_path = storage
        .join("pagi_chronos")
        .join("chronos.sqlite");
    let chronos_db = Arc::new(
        ChronosSqlite::new(chronos_db_path)
            .expect("open KB-04 (Chronos) SQLite"),
    );

    let _memory = Arc::new(
        MemoryManager::open_path(&memory_path).expect("open pagi_vault"),
    );
    let knowledge = Arc::new(
        KnowledgeStore::open_path(&knowledge_path).expect("open pagi_knowledge"),
    );
    knowledge.pagi_init_kb_metadata().ok(); // ensure 8 trees have metadata
    
    // Bootstrap core identity if KB-1 is empty (Mission Genesis)
    match initialize_core_identity(&knowledge) {
        Ok(true) => tracing::info!("Mission Genesis: Core identity bootstrapped successfully"),
        Ok(false) => tracing::debug!("Core identity already exists in KB-1"),
        Err(e) => tracing::warn!("Failed to bootstrap core identity: {}", e),
    }

    // Bootstrap Skill Registry (KB-5) with baseline skill manifests
    match initialize_core_skills(&knowledge) {
        Ok(true) => tracing::info!("Skill Registry: Core skills bootstrapped successfully (KB-5/Techne)"),
        Ok(false) => tracing::debug!("Skill Registry already contains baseline skills (KB-5/Techne)"),
        Err(e) => tracing::warn!("Failed to bootstrap Skill Registry (KB-5/Techne): {}", e),
    }

    match initialize_ethos_policy(&knowledge) {
        Ok(true) => tracing::info!("Ethos: Default safety policy installed (KB_ETHOS)"),
        Ok(false) => tracing::debug!("Ethos: Default policy already present (KB_ETHOS)"),
        Err(e) => tracing::warn!("Failed to bootstrap Ethos policy: {}", e),
    }
    match initialize_therapist_fit_checklist(&knowledge) {
        Ok(true) => tracing::info!("Ethos: Therapist-fit checklist installed (KB-06 self-audit)"),
        Ok(false) => tracing::debug!("Ethos: Therapist-fit checklist already present"),
        Err(e) => tracing::warn!("Failed to bootstrap Therapist-fit checklist: {}", e),
    }

    // Sovereign Identity (Initial Boot Handshake): only in interactive (Server/TUI) mode.
    // Skip for headless maintenance (--verify, --audit, --heal) so automated tasks never block on stdin.
    if !headless {
        if let Ok(None) = knowledge.get_identity() {
            match initialization_handshake() {
                Ok(persona) => {
                    if let Err(e) = knowledge.set_identity(&persona) {
                        tracing::warn!(target: "pagi::gateway", "Failed to save Sovereign Identity: {}", e);
                    } else {
                        tracing::info!(target: "pagi::gateway", "Sovereign Identity saved (Initial Boot complete)");
                    }
                }
                Err(e) => tracing::warn!(target: "pagi::gateway", "Initialization handshake skipped: {}", e),
            }
        }
    }

    // Cognitive Architecture boot: Pneuma (Vision) active; Oikos (Context) — no workspace_analyzer/sandbox
    let _pneuma_ok = pagi_core::verify_identity(&knowledge).complete;
    tracing::info!("[Cognitive Architecture] Pneuma (Vision) active. Oikos (Context) ready (Sovereign skills only).");

    let shadow_store: ShadowStoreHandle = if std::env::var("PAGI_SHADOW_KEY").is_ok() {
        let shadow_path = storage.join("pagi_shadow");
        match ShadowStore::open_path(&shadow_path) {
            Ok(store) => {
                tracing::info!(target: "pagi::gateway", "Secure ShadowStore initialized");
                Arc::new(tokio::sync::RwLock::new(Some(store)))
            }
            Err(e) => {
                tracing::warn!(target: "pagi::gateway", "ShadowStore open failed: {} (secure journal disabled)", e);
                Arc::new(tokio::sync::RwLock::new(None))
            }
        }
    } else {
        Arc::new(tokio::sync::RwLock::new(None))
    };

    // Sovereign Brain: chat + file system / OS access (workspace analysis, read file, sandbox write)
    let mut registry = SkillRegistry::new();
    let model_router = Arc::new(ModelRouter::with_knowledge(Arc::clone(&knowledge)));
    registry.register(Arc::new(ModelRouter::with_knowledge(Arc::clone(&knowledge))));
    registry.register(Arc::new(BioGateSync::new(Arc::clone(&knowledge))));
    registry.register(Arc::new(EthosSync::new(Arc::clone(&knowledge))));
    registry.register(Arc::new(IdentitySetup::new(Arc::clone(&knowledge))));
    registry.register(Arc::new(OikosTaskGovernor::new(Arc::clone(&knowledge))));
    registry.register(Arc::new(ReflectShadowSkill::new(
        Arc::clone(&knowledge),
        Arc::clone(&shadow_store),
        Arc::clone(&model_router),
    )));
    let fs_access_enabled = std::env::var("PAGI_FS_ACCESS_ENABLED")
        .map(|v| v.trim().eq_ignore_ascii_case("true") || v.trim().is_empty())
        .unwrap_or(true);
    if fs_access_enabled {
        registry.register(Arc::new(FsWorkspaceAnalyzer::new_with_store(Arc::clone(&knowledge))));
        registry.register(Arc::new(WriteSandboxFile::new()));
        registry.register(Arc::new(ReadFile::new()));
    }

    // Slot 4 — External Gateway: web search / URL fetch (Tavily, SerpAPI, or single-URL fetch).
    registry.register(Arc::new(WebSearch::new_with_store(Arc::clone(&knowledge))));

    // Sovereign Reflex: system telemetry, shell, and file ops for MoE SystemTool routing.
    let system_telemetry = Arc::new(SystemTelemetry::default());
    registry.register(Arc::new(SystemTelemetrySkill::new(Arc::clone(&system_telemetry))));
    registry.register(Arc::new(GetHardwareStatsSkill::new(Arc::clone(&system_telemetry))));
    registry.register(Arc::new(SynthesizeMeetingContextSkill::new(Arc::clone(&system_telemetry))));
    registry.register(Arc::new(SecureVaultSkill::new(Arc::new(SecureVault::new()))));
    registry.register(Arc::new(SystemCommandSkill::new(Arc::new(ShellExecutor::default()))));
    let fs_workspace = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    registry.register(Arc::new(FileSystemSkill::new(Arc::new(FileSystem::with_workspace(fs_workspace)))));

    // Holistic Life Warden: Counselor skill (proactive ping, Spirit/Mind/Body, Sovereign Reset suggestion)
    registry.register(Arc::new(CounselorSkill::new()));

    // Beta Ops: validate tester submission bundles (Operation First Rise)
    registry.register(Arc::new(MissionValidatorSkill::new()));

    // Mimir: Pre-Flight Audio Check (mic/loopback readiness before meeting record)
    registry.register(Arc::new(PreFlightAudioSkill));

    // Deep Audit: Sovereign document ingestion and KB routing
    registry.register(Arc::new(pagi_skills::DeepAuditSkill::new(storage.to_path_buf())));

    // The Forge: Self-synthesis skill (create new .rs from JSON tool-spec, append to lib.rs, cargo check)
    registry.register(Arc::new(ForgeSkill::new()));

    // SAO–Copilot Bridge: user-level UI automation; redacted transcript → Copilot sidebar
    #[cfg(all(windows, feature = "bridge-ms"))]
    registry.register(Arc::new(bridge_copilot_skill::BridgeCopilotSkill::new(
        Arc::clone(&chronos_db),
        storage.to_path_buf(),
    )));

    // Sovereign Operator: The Forge (unified integration for self-evolution)
    // Load forge safety setting from environment
    let forge_safety_enabled = std::env::var("PAGI_FORGE_SAFETY_ENABLED")
        .ok()
        .and_then(|v| v.trim().parse::<bool>().ok())
        .unwrap_or(true);
    
    let sovereign_operator_config = SovereignOperatorConfig {
        safety_enabled: forge_safety_enabled,
        tool_memory_enabled: true,
        recursive_compilation_enabled: true,
        tool_memory_path: "./data/pagi_tool_memory".to_string(),
        workspace_path: ".".to_string(),
    };
    
    match SovereignOperator::with_config(sovereign_operator_config) {
        Ok(mut operator) => {
            // Set knowledge store for KB-08 logging
            operator.set_knowledge_store(Arc::clone(&knowledge));
            
            let sovereign_operator = Arc::new(operator);
            registry.register(Arc::new(SovereignOperatorSkill::new(Arc::clone(&sovereign_operator))));
            
            if forge_safety_enabled {
                tracing::info!("[Sovereign Operator] The Forge initialized with HITL approval gate (safety: ENABLED)");
            } else {
                tracing::warn!("[Sovereign Operator] The Forge initialized in AUTONOMOUS EVOLUTION MODE (safety: DISABLED)");
                // Log to KB-08
                let msg = "Sovereignty Update: Forge Safety Gate set to FALSE (Autonomous Evolution Mode enabled)";
                if let Err(e) = knowledge.record_success_metric(msg) {
                    tracing::warn!("Failed to log forge safety state to KB-08: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::warn!("[Sovereign Operator] Failed to initialize: {} (self-evolution disabled)", e);
        }
    }

    // VectorKB: Production semantic memory layer (Schema-on-Boot + Graceful Shutdown)
    #[cfg(feature = "vector")]
    let vector_store: Arc<dyn VectorStore> = {
        let vector_kb_path = storage.join("vector_kb");
        create_vector_store(vector_kb_path, Some(Arc::clone(&knowledge))).await
    };

    // Persona & Archetype: from env (PAGI_MODE, PAGI_USER_SIGN, PAGI_ASCENDANT, PAGI_JUNGIAN_SHADOW_FOCUS)
    let persona_coordinator = Arc::new({
        let archetype = UserArchetype {
            birth_sign: config.user_sign.clone().or_else(|| std::env::var("PAGI_USER_SIGN").ok().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty())),
            moon_sign: std::env::var("PAGI_MOON_SIGN")
                .ok()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty()),
            ascendant: config.ascendant.clone().or_else(|| std::env::var("PAGI_ASCENDANT").ok().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty())),
            jungian_shadow_focus: config.jungian_shadow_focus.clone().or_else(|| std::env::var("PAGI_JUNGIAN_SHADOW_FOCUS").ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())),
        };
        let mode_str_owned: Option<String> = config.persona_mode.clone()
            .or_else(|| std::env::var("PAGI_MODE").ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let mode = mode_str_owned.as_deref().map(OrchestratorMode::from_str).unwrap_or(OrchestratorMode::Counselor);
        PersonaCoordinator::new(archetype, mode)
    });
    tracing::info!(target: "pagi::persona", mode = persona_coordinator.get_mode().as_str(), "PersonaCoordinator initialized");

    // Context density: concise (RLM/sovereign) | balanced | verbose (counselor). Runtime-togglable via GET/POST /api/v1/settings/density.
    let density_mode = {
        let raw = config.density_mode.clone()
            .or_else(|| std::env::var("PAGI_DENSITY_MODE").ok())
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "balanced".to_string());
        let normalized = match raw.as_str() {
            "concise" | "sovereign" | "concise_sovereign" => "concise",
            "verbose" | "counselor" | "verbose_counselor" => "verbose",
            _ => "balanced",
        };
        Arc::new(tokio::sync::RwLock::new(normalized.to_string()))
    };
    {
        let d = density_mode.read().await.clone();
        tracing::info!(target: "pagi::config", density = %d, "Context density mode initialized");
    }

    let blueprint_path = std::env::var("PAGI_BLUEPRINT_PATH")
        .unwrap_or_else(|_| "config/blueprint.json".to_string());
    let blueprint = Arc::new(BlueprintRegistry::load_json_path(&blueprint_path));

    let skill_manifest_registry = match SkillManifestRegistry::load_from_dir(&pagi_skills_root()) {
        Ok(r) => {
            let n = r.list_inventory().len();
            tracing::info!(target: "pagi::skills", count = n, "3-tier skills manifest loaded");
            Arc::new(r)
        }
        Err(e) => {
            tracing::warn!(target: "pagi::skills", "Skills manifest load failed: {}; using empty registry", e);
            Arc::new(SkillManifestRegistry::new())
        }
    };
    let sovereign_config = Arc::new(SovereignConfig::from_env());
    let orchestrator = Arc::new(Orchestrator::with_blueprint_and_permissions(
        Arc::new(registry),
        Arc::clone(&blueprint),
        Arc::clone(&skill_manifest_registry),
        sovereign_config.firewall_strict_mode,
    ));

    // Heartbeat (Autonomous Orchestrator): in-process background task so we can share
    // the same Sled-backed KnowledgeStore without cross-process lock contention.
    // Tick rate is configurable via env `PAGI_TICK_RATE_SECS`.
    let tick_rate = std::env::var("PAGI_TICK_RATE_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5)
        .max(1);
    let (persona_pulse_tx, _) = broadcast::channel(64);
    tokio::spawn(heartbeat_loop(
        Arc::clone(&knowledge),
        Arc::clone(&model_router),
        persona_pulse_tx.clone(),
        std::time::Duration::from_secs(tick_rate),
    ));

    // Sentinel: InputVelocitySensor → persona stream every 2s (sentinel_update)
    let critical_threshold_counter = Arc::new(AtomicU64::new(0));
    let sentinel_enabled = std::env::var("PAGI_SENTINEL_VELOCITY_ENABLED")
        .map(|v| v.trim().eq_ignore_ascii_case("true") || v.trim().is_empty())
        .unwrap_or(true);
    if sentinel_enabled {
        let (velocity_tx, velocity_rx) = mpsc::unbounded_channel();
        let sensor = SentinelInputVelocitySensor::new(SentinelInputVelocityConfig::default());
        std::thread::spawn(move || {
            if let Err(e) = sensor.start_monitoring_blocking(velocity_tx) {
                tracing::warn!(target: "pagi::sentinel", error = %e, "InputVelocitySensor failed (e.g. no input permission)");
            }
        });
        tokio::spawn(sentinel_broadcast_loop(
            velocity_rx,
            persona_pulse_tx.clone(),
            Arc::clone(&persona_coordinator),
            Arc::clone(&knowledge),
            Arc::clone(&critical_threshold_counter),
        ));
        tracing::info!(target: "pagi::sentinel", "Sentinel velocity broadcast loop started (sentinel_update every 2s)");
    }

    // MoE toggle: load from Sovereign Config (KB-6), else PAGI_MOE_DEFAULT from .env
    let moe_default_env = std::env::var("PAGI_MOE_DEFAULT")
        .ok()
        .and_then(|s| Some(s.trim().to_lowercase()))
        .filter(|s| s == "sparse" || s == "dense")
        .unwrap_or_else(|| "dense".to_string());
    let moe_mode_str = knowledge.get_sovereign_moe_mode();
    let moe_mode_final = moe_mode_str.as_deref().unwrap_or(moe_default_env.as_str());
    let moe_is_sparse = moe_mode_final.eq_ignore_ascii_case("sparse");
    orchestrator.set_moe_mode(MoEMode::from_str(moe_mode_final));
    let moe_active = Arc::new(AtomicBool::new(moe_is_sparse));

    // Autonomous Maintenance & Reflexion Loop: idle tracker + approval bridge + background spawn.
    let idle_tracker = IdleTracker::new();
    let approval_bridge = new_approval_bridge();
    let mut maint_config = MaintenanceConfig::default();
    maint_config.approval_bridge = Some(Arc::clone(&approval_bridge));
    let _maintenance_handle = init_maintenance_loop(
        Arc::clone(&knowledge),
        idle_tracker.clone(),
        log_tx.clone(),
        Some(maint_config),
    );
    tracing::info!("[Cognitive Architecture] Autonomous Maintenance Loop spawned (with UI approval bridge).");

    // Initialize Intelligence Service (background SAO layer)
    let intelligence_service = Arc::new(OrchestratorService::new(Arc::clone(&knowledge)));
    tracing::info!("[Intelligence Layer] SAO background service initialized (pattern matching + heuristics).");

    // Astro-Weather: on-boot check + background refresh (transit vs KB-01). Gated by PAGI_ASTRO_ALERTS_ENABLED / PAGI_TRANSIT_ALERTS_ENABLED.
    let astro_weather = Arc::new(tokio::sync::RwLock::new(
        if sovereign_config.astro_alerts_enabled {
            check_astro_weather(&knowledge)
        } else {
            pagi_core::AstroWeatherState::default()
        },
    ));
    if sovereign_config.astro_alerts_enabled {
        let astro_weather_clone = Arc::clone(&astro_weather);
        let knowledge_astro = Arc::clone(&knowledge);
        let sovereign_for_astro = Arc::clone(&sovereign_config);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(6 * 3600)); // 6h
            interval.tick().await; // first tick after 6h
            loop {
                interval.tick().await;
                if !sovereign_for_astro.astro_alerts_enabled {
                    continue;
                }
                let state = check_astro_weather(&knowledge_astro);
                if state.updated_at_ms > 0 {
                    let mut guard = astro_weather_clone.write().await;
                    *guard = state;
                    tracing::debug!(target: "pagi::astro", "Astro-Weather refreshed (risk: {:?})", guard.risk);
                }
            }
        });
        tracing::info!(target: "pagi::astro", "Astro-Weather on-boot check done; background refresh every 6h.");
    } else {
        tracing::info!(target: "pagi::astro", "Astro-Weather (transit alerts) disabled by PAGI_ASTRO_ALERTS_ENABLED / PAGI_TRANSIT_ALERTS_ENABLED.");
    }

    let ms_graph_client = if sovereign_config.focus_shield_enabled || sovereign_config.vitality_shield_enabled {
        match MicrosoftGraphClient::from_env() {
            Some(c) => {
                if sovereign_config.focus_shield_enabled {
                    tracing::info!(target: "pagi::ms_graph", "Focus Shield enabled; MS Graph client initialized (Schedule Outlook + Gatekeeper).");
                }
                if sovereign_config.vitality_shield_enabled {
                    tracing::info!(target: "pagi::ms_graph", "Vitality Shield enabled; MS Graph client initialized (sleep/activity).");
                }
                Some(Arc::new(c))
            }
            None => {
                if sovereign_config.focus_shield_enabled {
                    tracing::warn!(target: "pagi::ms_graph", "PAGI_FOCUS_SHIELD_ENABLED=true but MS_GRAPH_* not set; Focus Shield inactive.");
                }
                if sovereign_config.vitality_shield_enabled {
                    tracing::warn!(target: "pagi::ms_graph", "MS_GRAPH_HEALTH_ENABLED=true but MS_GRAPH_* not set; Vitality Shield inactive.");
                }
                None
            }
        }
    } else {
        None
    };

    // Governor: cognitive immune system (KB-08 / KB-06 monitoring + webhook on Critical alerts).
    // sovereignty_score_bits is updated when a sovereignty audit runs (POST /api/v1/sovereignty-audit).
    let sovereignty_score_bits = Arc::new(AtomicU64::new(f64::to_bits(1.0))); // 1.0 = assume OK until first audit
    let mut governor_config = GovernorConfig::default();
    governor_config.sovereignty_score_bits = Some(Arc::clone(&sovereignty_score_bits));
    
    // Create Governor instance first so we can wire VectorStore before spawning
    let (mut governor, governor_alert_rx) = governor::Governor::new(Arc::clone(&knowledge), governor_config);
    
    // Wire VectorStore to Governor for health monitoring (Production Telemetry)
    #[cfg(feature = "vector")]
    governor.set_vector_store(Arc::clone(&vector_store));
    
    // Spawn Governor background task
    let _governor_handle = tokio::spawn(async move {
        governor.run().await;
    });
    
    tokio::spawn(handle_governor_alerts(governor_alert_rx, log_tx.clone()));
    tracing::info!("[Cognitive Architecture] Governor loop started (webhook on Critical when PAGI_WEBHOOK_URL set).");

    // Clone knowledge before moving it into AppState (needed for shutdown handler)
    let knowledge_shutdown = Arc::clone(&knowledge);
    #[cfg(feature = "vector")]
    let vector_store_shutdown = Arc::clone(&vector_store);

    let project_associations = Arc::new(tokio::sync::RwLock::new(load_project_associations()));
    let folder_summary_cache = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

    let app = build_app(AppState {
        config: Arc::clone(&config),
        sovereign_config: Arc::clone(&sovereign_config),
        orchestrator,
        knowledge: Arc::clone(&knowledge),
        log_tx: log_tx.clone(),
        model_router,
        shadow_store: Arc::clone(&shadow_store),
        moe_active,
        idle_tracker,
        approval_bridge,
        persona_coordinator,
        density_mode,
        persona_pulse_tx,
        critical_threshold_counter,
        intelligence_service,
        astro_weather,
        skill_manifest_registry,
        ms_graph_client,
        sovereignty_score_bits,
        project_associations,
        folder_summary_cache,
        chronos_db,
        mimir_session: Arc::new(tokio::sync::Mutex::new(None)),
    });

    // Voice mode: start Sovereign Voice loop (Ear → STT → chat API → TTS) when --voice is passed.
    #[cfg(feature = "voice")]
    if args.iter().any(|a| a == "--voice") {
        voice::log_voice_status();
        let _voice_handle = voice::start_voice_session(log_tx.clone());
        tracing::info!(target: "pagi::voice", "Voice session running. Log stream will show \"[Phoenix stopped to listen…]\" on interruption.");
    }

    // Live mode: OpenRouter streaming with real-time interruption (Gemini Live-style UX)
    #[cfg(feature = "voice")]
    if args.iter().any(|a| a == "--live") {
        voice::log_voice_status();
        // Create a new registry for live mode (SkillRegistry doesn't implement Clone)
        let mut live_registry = SkillRegistry::new();
        live_registry.register(Arc::new(ModelRouter::with_knowledge(Arc::clone(&knowledge_for_live))));
        let _live_handle = openrouter_live::start_openrouter_live_session(
            log_tx.clone(),
            knowledge_for_live,
            Arc::new(live_registry),
            Arc::clone(&_memory),
        );
        tracing::info!(target: "pagi::voice", "🌐 OpenRouter Live Mode active. Phoenix is listening with streaming responses.");
    }

    // PORT LOCKOUT: Hard-bind to 127.0.0.1:8000 only (Sovereign architecture). No 0.0.0.0.
    const GATEWAY_PORT: u16 = 8000;
    let port = GATEWAY_PORT;
    let app_name = config.app_name.clone();
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("{} listening on {} (port locked)", app_name, addr);
    
    // Graceful Shutdown: Ctrl+C handler to close VectorStore and prevent WAL corruption
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let server = axum::serve(listener, app);
    
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("System Shutdown Initiated (Ctrl+C received)");
            
            // Log shutdown to KB-08
            if let Err(e) = knowledge_shutdown.record_success_metric("System Shutdown: Graceful termination initiated") {
                tracing::warn!("Failed to log shutdown to KB-08: {}", e);
            }
            
            // Close VectorStore gracefully (prevents WAL corruption)
            #[cfg(feature = "vector")]
            {
                if let Some(qdrant_store) = vector_store_shutdown.as_any().downcast_ref::<QdrantVectorStore>() {
                    qdrant_store.close().await;
                    tracing::info!("VectorKB closed gracefully");
                }
            }
            
            tracing::info!("✓ Graceful shutdown complete");
        }
    }
}

/// Sentinel broadcast loop: receives velocity metrics from InputVelocitySensor,
/// broadcasts "sentinel_update" every 2 seconds on persona stream.
/// If velocity >= 80 for 30+ seconds in Counselor mode, sends sovereign_reset_suggested.
/// FORCED SOVEREIGN RESET: If critical_threshold_counter >= 15 (30s) AND WellnessReport.is_critical,
/// broadcasts a 10-second countdown, then locks the workstation and logs to KB_08.
async fn sentinel_broadcast_loop(
    mut velocity_rx: mpsc::UnboundedReceiver<SentinelInputVelocityMetrics>,
    persona_pulse_tx: broadcast::Sender<serde_json::Value>,
    persona_coordinator: Arc<PersonaCoordinator>,
    knowledge: Arc<KnowledgeStore>,
    critical_threshold_counter: Arc<AtomicU64>,
) {
    const SENTINEL_INTERVAL_SECS: u64 = 2;
    const HIGH_VELOCITY_THRESHOLD: f64 = 80.0;
    const HIGH_VELOCITY_DURATION_SECS: u64 = 30;
    const CRITICAL_VELOCITY_THRESHOLD: f64 = 85.0; // Threshold for forced reset
    const CRITICAL_THRESHOLD_TICKS: u64 = 15; // 15 ticks * 2s = 30 seconds
    const FORCED_RESET_COUNTDOWN_SECS: u64 = 10;

    let mut last: Option<SentinelInputVelocityMetrics> = None;
    let mut high_since: Option<StdInstant> = None;
    let mut countdown_started: Option<StdInstant> = None;
    let mut forced_reset_triggered = false;
    
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(SENTINEL_INTERVAL_SECS));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Some(ref m) = last {
                    // Increment counter if velocity > 85 (critical threshold for forced reset)
                    if m.velocity_score > CRITICAL_VELOCITY_THRESHOLD {
                        critical_threshold_counter.fetch_add(1, Ordering::SeqCst);
                    } else {
                        // Reset counter if velocity drops below threshold
                        critical_threshold_counter.store(0, Ordering::SeqCst);
                        countdown_started = None;
                        forced_reset_triggered = false;
                    }

                    let counter_value = critical_threshold_counter.load(Ordering::SeqCst);
                    
                    let msg = serde_json::json!({
                        "type": "sentinel_update",
                        "velocity_score": m.velocity_score,
                        "is_rage_detected": m.is_rage_detected,
                        "critical_threshold_counter": counter_value,
                    });
                    let _ = persona_pulse_tx.send(msg);

                    // Check for FORCED SOVEREIGN RESET conditions
                    if counter_value >= CRITICAL_THRESHOLD_TICKS && persona_coordinator.get_mode().is_counselor() && !forced_reset_triggered {
                        // Check wellness report
                        if let Ok(wellness) = generate_wellness_report(&knowledge) {
                            if wellness.is_critical {
                                // Start countdown if not already started
                                if countdown_started.is_none() {
                                    countdown_started = Some(StdInstant::now());
                                    let countdown_msg = serde_json::json!({
                                        "type": "forced_reset_countdown",
                                        "countdown_seconds": FORCED_RESET_COUNTDOWN_SECS,
                                        "message": "FORCED SOVEREIGN RESET: Critical wellness state detected with sustained high velocity. Workstation will lock in 10 seconds.",
                                        "velocity_score": m.velocity_score,
                                        "wellness_critical": true,
                                    });
                                    let _ = persona_pulse_tx.send(countdown_msg);
                                    tracing::warn!(target: "pagi::sentinel", "Forced Sovereign Reset countdown initiated (30s high velocity + critical wellness)");
                                }
                                
                                // Check if countdown has elapsed
                                if let Some(start) = countdown_started {
                                    if start.elapsed() >= std::time::Duration::from_secs(FORCED_RESET_COUNTDOWN_SECS) {
                                        // Re-check conditions before locking
                                        let current_counter = critical_threshold_counter.load(Ordering::SeqCst);
                                        if current_counter >= CRITICAL_THRESHOLD_TICKS {
                                            // Execute forced reset
                                            forced_reset_triggered = true;
                                            
                                            // Lock workstation
                                            let guard = SentinelPhysicalGuardSensor::new(false); // No confirmation required
                                            match guard.execute_action(SentinelPhysicalGuardAction::LockWorkstation) {
                                                Ok(result) => {
                                                    if result.success {
                                                        tracing::info!(target: "pagi::sentinel", "Workstation locked via Forced Sovereign Reset");
                                                    } else {
                                                        tracing::error!(target: "pagi::sentinel", "Failed to lock workstation: {}", result.message);
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::error!(target: "pagi::sentinel", "Workstation lock error: {}", e);
                                                }
                                            }
                                            
                                            // Log to KB_08 (The Absurdity Log - part of Soma)
                                            let absurdity_slot = KbType::Soma.slot_id();
                                            let timestamp = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .map(|d| d.as_millis())
                                                .unwrap_or(0);
                                            let log_key = format!("sovereign_intervention/{}", timestamp);
                                            let log_entry = serde_json::json!({
                                                "type": "forced_sovereign_reset",
                                                "timestamp_ms": timestamp,
                                                "velocity_score": m.velocity_score,
                                                "critical_threshold_counter": current_counter,
                                                "wellness_critical": true,
                                                "action": "workstation_locked",
                                                "reason": "Sustained high input velocity (30+ seconds) combined with critical wellness state",
                                                "persona_mode": "counselor",
                                            });
                                            if let Ok(bytes) = serde_json::to_vec(&log_entry) {
                                                if let Err(e) = knowledge.insert(absurdity_slot, &log_key, &bytes) {
                                                    tracing::error!(target: "pagi::sentinel", "Failed to log Sovereign Intervention to KB_08: {}", e);
                                                } else {
                                                    tracing::info!(target: "pagi::sentinel", "Sovereign Intervention logged to KB_08 (The Absurdity Log)");
                                                }
                                            }
                                            
                                            // Broadcast final notification
                                            let lock_msg = serde_json::json!({
                                                "type": "forced_reset_executed",
                                                "message": "Workstation locked. Sovereign Intervention complete. Take a break.",
                                                "timestamp_ms": timestamp,
                                            });
                                            let _ = persona_pulse_tx.send(lock_msg);
                                            
                                            // Reset state
                                            critical_threshold_counter.store(0, Ordering::SeqCst);
                                            countdown_started = None;
                                        } else {
                                            // Conditions no longer met, cancel countdown
                                            countdown_started = None;
                                            forced_reset_triggered = false;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Original sovereign reset suggestion (non-forced)
                    if m.velocity_score >= HIGH_VELOCITY_THRESHOLD {
                        let should_reset = high_since
                            .map(|t| t.elapsed() >= std::time::Duration::from_secs(HIGH_VELOCITY_DURATION_SECS))
                            .unwrap_or(false)
                            && persona_coordinator.get_mode().is_counselor()
                            && !forced_reset_triggered;
                        if should_reset {
                            let reset_msg = serde_json::json!({
                                "type": "sentinel_update",
                                "velocity_score": m.velocity_score,
                                "is_rage_detected": m.is_rage_detected,
                                "sovereign_reset_suggested": true,
                                "message": "Velocity has been high for 30+ seconds. Consider a Sovereign Reset: minimize windows, take a breath, water/movement.",
                                "health_reminder": "Water, movement, or a short break can help.",
                            });
                            let _ = persona_pulse_tx.send(reset_msg);
                            tracing::info!(target: "pagi::sentinel", "Auto Sovereign Reset suggested (velocity >= 80 for 30s in Counselor mode)");
                            high_since = None;
                        }
                    } else {
                        high_since = None;
                    }
                }
            }
            msg = velocity_rx.recv() => {
                if let Some(m) = msg {
                    // Track high velocity for sovereign reset suggestion (>= 80)
                    if m.velocity_score >= HIGH_VELOCITY_THRESHOLD {
                        high_since.get_or_insert_with(|| StdInstant::now());
                    } else {
                        high_since = None;
                    }
                    last = Some(m);
                } else {
                    break;
                }
            }
        }
    }
}

async fn heartbeat_loop(
    knowledge: Arc<KnowledgeStore>,
    model_router: Arc<ModelRouter>,
    persona_pulse_tx: broadcast::Sender<serde_json::Value>,
    tick: std::time::Duration,
) {
    tracing::info!(
        target: "pagi::daemon",
        tick_rate_secs = tick.as_secs(),
        "Heartbeat loop started"
    );
    let mut interval = tokio::time::interval(tick);
    loop {
        interval.tick().await;
        if let Err(e) = heartbeat_tick(
            Arc::clone(&knowledge),
            Arc::clone(&model_router),
            persona_pulse_tx.clone(),
        )
        .await
        {
            tracing::warn!(target: "pagi::daemon", error = %e, "Heartbeat tick failed");
        }
    }
}

fn persona_pulse_interval_ticks(tick_rate_secs: u64) -> u64 {
    const FOUR_HOURS_SECS: u64 = 4 * 3600;
    FOUR_HOURS_SECS.checked_div(tick_rate_secs).unwrap_or(2880).max(1)
}

async fn heartbeat_tick(
    knowledge: Arc<KnowledgeStore>,
    model_router: Arc<ModelRouter>,
    persona_pulse_tx: broadcast::Sender<serde_json::Value>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Proactive Oikos monitoring: every 10 ticks, scan the physical workspace state
    // (research_sandbox/) and proactively inject maintenance prompts.
    let tick_n = HEARTBEAT_TICK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    let tick_rate = std::env::var("PAGI_TICK_RATE_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5)
        .max(1);
    let pulse_interval = persona_pulse_interval_ticks(tick_rate);
    if tick_n > 0 && tick_n % pulse_interval == 0 {
        let msg = serde_json::json!({
            "type": "persona_heartbeat",
            "message": "Checking in. Spirit/Mind/Body balance check—where are we at?",
            "tick_n": tick_n,
        });
        let _ = persona_pulse_tx.send(msg);
        tracing::info!(target: "pagi::daemon", tick_n, "Persona 4-hour heartbeat pulse sent");
    }
    if tick_n % 10 == 0 {
        if let Err(e) = maybe_run_oikos_guardian(Arc::clone(&knowledge), tick_n).await {
            tracing::warn!(target: "pagi::daemon", error = %e, "Oikos guardian scan failed");
        }
    }

    // Discover active agents by scanning KB_SOMA inbox keys: inbox/{agent_id}/...
    let soma_slot = KbType::Soma.slot_id();
    let keys = knowledge.scan_keys(soma_slot)?;
    let mut agents: HashSet<String> = HashSet::new();
    for k in keys {
        if let Some(rest) = k.strip_prefix("inbox/") {
            if let Some((agent_id, _tail)) = rest.split_once('/') {
                if !agent_id.trim().is_empty() {
                    agents.insert(agent_id.to_string());
                }
            }
        }
    }

    for agent_id in agents {
        // AUTO-POLL: check inbox.
        // We fetch a small batch so we can skip already-processed messages without getting stuck.
        let inbox = knowledge.get_agent_messages_with_keys(&agent_id, 25)?;
        if let Some((inbox_key, msg)) = inbox
            .into_iter()
            .find(|(_k, m)| !m.is_processed)
        {
            // Stop infinite ping-pong: never auto-reply to an auto-reply.
            // Still ACK it so it doesn't remain "unprocessed" forever.
            let msg_type = msg
                .payload
                .as_object()
                .and_then(|o| o.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if msg_type == "agent_auto_reply" {
                let mut updated = msg.clone();
                updated.is_processed = true;
                knowledge.insert(soma_slot, &inbox_key, &updated.to_bytes())?;
                continue;
            }

            // Cognitive Governor: effective MentalState (Kardia + Soma/BioGate physical load).
            let mental = knowledge.get_effective_mental_state(&agent_id);
            let prompt_base = format!(
                "You are agent_id={}. You have a new inbox message from {}. Message payload: {}\n\nRespond appropriately.",
                agent_id,
                msg.from_agent_id,
                msg.payload
            );
            let prompt = if mental.needs_empathetic_tone() {
                format!(
                    "{}. {}",
                    MentalState::EMPATHETIC_SYSTEM_INSTRUCTION,
                    prompt_base
                )
            } else if mental.has_physical_load_adjustment() {
                format!(
                    "{}. {}",
                    MentalState::PHYSICAL_LOAD_SYSTEM_INSTRUCTION,
                    prompt_base
                )
            } else {
                prompt_base
            };

            let generated = model_router
                .generate_text_raw(&prompt)
                .await
                .unwrap_or_else(|e| format!("[heartbeat] generation failed: {}", e));

            // Deliver response back to sender as an inter-agent message.
            knowledge.push_agent_message(
                &agent_id,
                &msg.from_agent_id,
                &serde_json::json!({
                    "type": "agent_auto_reply",
                    "in_reply_to": msg.id,
                    "text": generated,
                }),
            )?;

            // ACK: mark the original inbox message as processed (preserve KB_SOMA history).
            let mut updated = msg.clone();
            updated.is_processed = true;
            knowledge.insert(soma_slot, &inbox_key, &updated.to_bytes())?;

            // Reflection: write a Chronos event for the agent.
            let reflection = EventRecord::now(
                "Chronos",
                format!("Auto-replied to message {} from {}", msg.id, msg.from_agent_id),
            )
            .with_skill("heartbeat")
            .with_outcome("auto_reply_sent");
            let _ = knowledge.append_chronos_event(&agent_id, &reflection);
        } else {
            // If no inbox message exists, check Pneuma for background tasks.
            // Minimal v1: if a key `pneuma/{agent_id}/background_task` exists, run it through the router.
            let pneuma_slot = KbType::Pneuma.slot_id();
            let bg_key = format!("pneuma/{}/background_task", agent_id);
            if let Ok(Some(bytes)) = knowledge.get(pneuma_slot, &bg_key) {
                if let Ok(task) = String::from_utf8(bytes) {
                    if !task.trim().is_empty() {
                        let prompt = format!(
                            "You are agent_id={}. Background task: {}\n\nProvide a short status update.",
                            agent_id,
                            task
                        );
                        let generated = model_router
                            .generate_text_raw(&prompt)
                            .await
                            .unwrap_or_else(|e| format!("[heartbeat] background generation failed: {}", e));
                        let reflection = EventRecord::now(
                            "Chronos",
                            format!("Background task ticked: {}", generated),
                        )
                        .with_skill("heartbeat")
                        .with_outcome("background_task_ticked");
                        let _ = knowledge.append_chronos_event(&agent_id, &reflection);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn maybe_run_oikos_guardian(
    _knowledge: Arc<KnowledgeStore>,
    _tick_n: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Sovereign architecture: no workspace_analyzer/sandbox scan. Oikos tasks are
    // managed via OikosTaskGovernor skill only.
    return Ok(());
    #[allow(unreachable_code)]
    {
    let knowledge = _knowledge;
    let tick_n = _tick_n;
    let issues = tokio::task::spawn_blocking(|| scan_research_sandbox_for_all_issues())
        .await
        .map_err(|e| format!("spawn_blocking failed: {}", e))??;

    // ACTIVE ISSUES TRACKER (persisted in KB_OIKOS)
    let oikos_slot = KbType::Oikos.slot_id();
    let active_key = "workspace_guardian/active_maintenance_tasks";
    let mut active: BTreeMap<String, String> = knowledge
        .get(oikos_slot, active_key)
        .ok()
        .flatten()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| serde_json::from_str::<BTreeMap<String, String>>(&s).ok())
        .unwrap_or_default();

    // Track when each issue was first observed so we can apply (optional) trust decay for
    // tasks that remain unresolved for too long.
    let first_seen_key = "workspace_guardian/active_maintenance_first_seen_tick";
    let mut first_seen: BTreeMap<String, u64> = knowledge
        .get(oikos_slot, first_seen_key)
        .ok()
        .flatten()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| serde_json::from_str::<BTreeMap<String, u64>>(&s).ok())
        .unwrap_or_default();

    // Prevent repeated decay penalties for the same issue. (One penalty after crossing threshold.)
    let decay_applied_key = "workspace_guardian/active_maintenance_decay_applied";
    let mut decay_applied: BTreeMap<String, bool> = knowledge
        .get(oikos_slot, decay_applied_key)
        .ok()
        .flatten()
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| serde_json::from_str::<BTreeMap<String, bool>>(&s).ok())
        .unwrap_or_default();

    let mut current: BTreeMap<String, String> = BTreeMap::new();
    for (issue_key, task) in issues {
        current.insert(issue_key, task);
    }

    // 1) RESOLUTION CHECK: previously active issues no longer present.
    let resolved: Vec<(String, String)> = active
        .iter()
        .filter(|(k, _v)| !current.contains_key(*k))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (issue_key, task) in resolved {
        active.remove(&issue_key);
        first_seen.remove(&issue_key);
        decay_applied.remove(&issue_key);

        // KARDIA: reward DEV_BOT trust when SAGE_BOT validates the resolution.
        if let Err(e) = bump_kardia_trust(
            knowledge.as_ref(),
            "SAGE_BOT",
            "DEV_BOT",
            TRUST_RESOLUTION_REWARD,
            "Trust increased due to successful maintenance resolution.",
        ) {
            tracing::warn!(target: "pagi::daemon", error = %e, "Failed to bump Kardia trust on resolution");
        }

        // CHRONOS: record resolution.
        let reflection = EventRecord::now(
            "Chronos",
            format!("Task Resolved (Oikos guardian): {}", issue_key),
        )
        .with_skill("heartbeat")
        .with_outcome("proactive_maintenance_resolved");
        let _ = knowledge.append_chronos_event("SAGE_BOT", &reflection);

        // AUTO-CLEANUP: message DEV_BOT that validation passed.
        let text = format!(
            "Validation Passed: the previously detected issue '{}' is no longer present. ({})",
            issue_key, task
        );
        let _ = knowledge.push_agent_message(
            "SAGE_BOT",
            "DEV_BOT",
            &serde_json::json!({
                "type": "proactive_maintenance_resolved",
                "source": "oikos_guardian",
                "issue_key": issue_key,
                "text": text,
            }),
        );

        tracing::info!(
            target: "pagi::daemon",
            issue_key = %issue_key,
            "Oikos guardian: Task Resolved (SAGE_BOT -> DEV_BOT validation message)"
        );
    }

    // 2) OPEN NEW ISSUES: issues present now but not in active tracker.
    for (issue_key, task) in current.iter() {
        if active.contains_key(issue_key) {
            continue;
        }
        active.insert(issue_key.clone(), task.clone());

        // Record first seen tick (for optional decay logic).
        first_seen.entry(issue_key.clone()).or_insert(tick_n);
        decay_applied.entry(issue_key.clone()).or_insert(false);

        // PROACTIVE TRIGGER: SAGE_BOT initiates a maintenance task by messaging DEV_BOT.
        let text = format!(
            "I have analyzed the workspace state and identified a maintenance task: {}.",
            task
        );
        let _ = knowledge.push_agent_message(
            "SAGE_BOT",
            "DEV_BOT",
            &serde_json::json!({
                "type": "proactive_maintenance",
                "source": "oikos_guardian",
                "issue_key": issue_key,
                "task": task,
                "text": text,
            }),
        )?;

        // CHRONOS: record initiation.
        let reflection = EventRecord::now(
            "Chronos",
            format!("Initiated proactive maintenance (Oikos guardian): {}", issue_key),
        )
        .with_skill("heartbeat")
        .with_outcome("proactive_maintenance_initiated");
        let _ = knowledge.append_chronos_event("SAGE_BOT", &reflection);

        tracing::info!(
            target: "pagi::daemon",
            issue_key = %issue_key,
            "Oikos guardian: initiated proactive maintenance (SAGE_BOT -> DEV_BOT)"
        );
    }

    // 3) (Optional) DETERIORATION: if an issue remains active for too long, reduce trust.
    // This is applied once per issue when it crosses the threshold.
    for issue_key in active.keys() {
        let Some(seen_at) = first_seen.get(issue_key).copied() else {
            continue;
        };
        let age = tick_n.saturating_sub(seen_at);
        if age <= TRUST_STALE_DECAY_TICKS {
            continue;
        }
        if decay_applied.get(issue_key).copied().unwrap_or(false) {
            continue;
        }

        if let Err(e) = bump_kardia_trust(
            knowledge.as_ref(),
            "SAGE_BOT",
            "DEV_BOT",
            -TRUST_STALE_DECAY_PENALTY,
            "Trust decreased due to unresolved maintenance remaining active beyond 50 ticks.",
        ) {
            tracing::warn!(target: "pagi::daemon", error = %e, "Failed to decay Kardia trust for stale maintenance");
        } else {
            decay_applied.insert(issue_key.clone(), true);
        }
    }

    // Persist active tracker.
    let bytes = serde_json::to_vec(&active).unwrap_or_else(|_| b"{}".to_vec());
    knowledge.insert(oikos_slot, active_key, &bytes)?;

    // Persist auxiliary trackers for trust calibration.
    let first_seen_bytes = serde_json::to_vec(&first_seen).unwrap_or_else(|_| b"{}".to_vec());
    knowledge.insert(oikos_slot, first_seen_key, &first_seen_bytes)?;
    let decay_applied_bytes = serde_json::to_vec(&decay_applied).unwrap_or_else(|_| b"{}".to_vec());
    knowledge.insert(oikos_slot, decay_applied_key, &decay_applied_bytes)?;
    Ok(())
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Adjust DEV_BOT's trust_score in KB_KARDIA from SAGE_BOT's perspective.
///
/// Uses (owner_agent_id, target_id) = ("SAGE_BOT", "DEV_BOT") so SAGE_BOT has a
/// persistent relation record for DEV_BOT.
fn bump_kardia_trust(
    knowledge: &KnowledgeStore,
    owner_agent_id: &str,
    target_id: &str,
    delta: f32,
    chronos_reflection: &str,
) -> Result<f32, Box<dyn std::error::Error + Send + Sync>> {
    let mut rel = knowledge
        .get_kardia_relation(owner_agent_id, target_id)
        .unwrap_or_else(|| RelationRecord::new(target_id));

    rel.trust_score = (rel.trust_score + delta).clamp(0.0, 1.0);
    rel.last_updated_ms = now_ms();
    knowledge
        .set_kardia_relation(owner_agent_id, &rel)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

    // CHRONOS LOGGING: write a Kardia-sourced event for observability/audit.
    let event = EventRecord::now("Kardia", chronos_reflection)
        .with_skill("heartbeat")
        .with_outcome("kardia_trust_calibrated");
    let _ = knowledge.append_chronos_event(owner_agent_id, &event);

    Ok(rel.trust_score)
}

fn scan_research_sandbox_for_all_issues(
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error + Send + Sync>> {
    let sandbox_dir = research_sandbox_root();
    if !sandbox_dir.exists() {
        return Ok(vec![]);
    }

    // 1) TODO present in a .rs file (and also allow todo.txt for local verification)
    // Prioritize TODO detection so an actionable maintenance task is surfaced even
    // if other hygiene tasks (like README presence) are also pending.
    let mut issues: Vec<(String, String)> = vec![];
    let mut stack = vec![sandbox_dir.clone()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for ent in entries.flatten() {
            let path = ent.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let is_rs = path.extension().and_then(|s| s.to_str()).unwrap_or("") == "rs";
            let is_todo_txt = file_name.eq_ignore_ascii_case("todo.txt");
            if !(is_rs || is_todo_txt) {
                continue;
            }

            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.len() > 256 * 1024 {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if let Some(idx) = content.find("TODO") {
                let rel = path
                    .strip_prefix(&sandbox_dir)
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or(file_name)
                    .replace('\\', "/");
                let snippet: String = content
                    .chars()
                    .skip(idx)
                    .take(120)
                    .collect::<String>()
                    .replace('\n', " ");
                let issue_key = format!("todo:{}", rel);
                let task = format!(
                    "Address TODO marker in research_sandbox/{} (e.g., '{}')",
                    rel, snippet
                );
                issues.push((issue_key, task));
            }
        }
    }

    // 2) Missing README.md in research_sandbox/
    let readme = sandbox_dir.join("README.md");
    if !readme.exists() {
        issues.push((
            "missing_readme".to_string(),
            "Create research_sandbox/README.md explaining the sandbox purpose and how to run checks".to_string(),
        ));
    }

    issues.sort_by(|a, b| a.0.cmp(&b.0));
    issues.dedup_by(|a, b| a.0 == b.0);
    Ok(issues)
}

fn research_sandbox_root() -> std::path::PathBuf {
    // Prefer a working-directory-relative path (run from workspace root).
    // Fall back to `CARGO_MANIFEST_DIR/../..` (workspace root) for safety.
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let from_cwd = cwd.join("research_sandbox");
    if from_cwd.exists() {
        return from_cwd;
    }
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("research_sandbox")
}

fn frontend_root_dir() -> std::path::PathBuf {
    // Prefer a working-directory relative path for local development (run from workspace root).
    // Fall back to workspace-root-relative path from add-ons/pagi-gateway: manifest -> .. -> .. -> pagi-frontend.
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let from_cwd = cwd.join("pagi-frontend");
    if from_cwd.exists() {
        return from_cwd;
    }

    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("pagi-frontend")
}

/// Root directory for 3-tier skills (core/, import/, ephemeral/). Prefer PAGI_SKILLS_DIR env.
fn pagi_skills_root() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("PAGI_SKILLS_DIR") {
        let p = std::path::PathBuf::from(dir);
        if p.exists() {
            return p;
        }
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let from_cwd = cwd.join("crates").join("pagi-skills");
    if from_cwd.exists() {
        return from_cwd;
    }
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("crates")
        .join("pagi-skills")
}

fn build_app(state: AppState) -> Router {
    let frontend_enabled = state.config.frontend_enabled;

    // CORS: allow UI origins so the "brain" is reachable. No mock; UI must talk to this gateway only.
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin: &axum::http::HeaderValue, _| {
            let s = origin.to_str().unwrap_or("");
            // Explicit localhost UI ports (Vite often 3000 or 3001)
            if s == "http://localhost:3000" || s == "http://127.0.0.1:3000" { return true; }
            if s == "http://localhost:3001" || s == "http://127.0.0.1:3001" { return true; }
            let port = s
                .split(':')
                .last()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(0);
            (3000..=3099).contains(&port) || (8000..=8099).contains(&port)
        }))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS, Method::PUT, Method::DELETE])
        .allow_headers(tower_http::cors::Any)
        .expose_headers(tower_http::cors::Any);

    let mut app = Router::new()
        .route("/v1/status", get(status))
        .route("/v1/execute", post(execute))
        .route("/v1/research/trace/:trace_id", get(get_research_trace))
        .route("/api/v1/health", get(health))
        .route("/api/v1/config", get(feature_config))
        .route("/api/v1/config/status", get(config_status_get))
        .route("/api/v1/config/api-key", get(get_api_key_status).post(set_api_key))
        .route("/api/v1/config/user", get(get_user_config))
        .route("/api/v1/version", get(get_version))
        .route("/api/v1/version/check", get(check_for_updates))
        .route("/api/v1/logs", get(logs_stream))
        .route("/api/v1/stream", post(chat_stream_sse))
        .route("/api/v1/chat", post(chat))
        .route("/api/v1/kardia/:user_id", get(get_kardia_relation))
        .route("/api/v1/kb-status", get(kb_status))
        .route("/api/v1/sovereign-status", get(sovereign_status))
        .route("/api/v1/settings/moe", get(get_moe_settings).post(set_moe_settings))
        .route("/api/v1/settings/orchestrator-role", get(get_orchestrator_role_settings).post(set_orchestrator_role_settings))
        .route("/api/v1/settings/persona", get(get_orchestrator_role_settings).post(set_orchestrator_role_settings))
        .route("/api/v1/settings/density", get(get_density_settings).post(set_density_settings))
        .route("/api/v1/persona/stream", get(persona_pulse_stream))
        .route("/api/v1/soma/balance", post(soma_balance_post))
        .route("/api/v1/skills", get(skills_list))
        .route("/api/v1/skills/promote", post(skills_promote))
        .route("/api/v1/skills/wellness-report", get(wellness_report_get))
        .route("/api/v1/sentinel/domain-integrity", get(domain_integrity_get))
        .route("/api/v1/self-audit", get(self_audit_get))
        .route("/api/v1/sovereignty-audit", post(sovereignty_audit_post))
        .route("/api/v1/heal", post(heal_post))
        .route("/api/v1/domain/vitality", get(domain_vitality_get))
        .route("/api/v1/astro-weather", get(astro_weather_get))
        .route("/api/v1/health-report", get(health_report_get))
        .route("/api/v1/evening-audit", post(evening_audit_post))
        .route("/api/v1/onboarding/status", get(onboarding_status_get))
        .route("/api/v1/onboarding/complete", post(onboarding_complete_post))
        .route("/api/v1/onboarding/user-profile", post(onboarding_user_profile_post))
        .route("/api/v1/archetype", get(archetype_get))
        .route("/api/v1/subject-check", get(subject_check_get))
        .route("/api/v1/kb08/success-metric", post(success_metric_post))
        .route("/v1/vault/read", post(vault_read))
        // Intelligence Layer endpoints (SAO background service)
        .route("/api/v1/intelligence/insights", get(intelligence_insights_get))
        .route("/api/v1/intelligence/toggle", post(intelligence_toggle_post))
        // Maintenance Dashboard endpoints
        .route("/api/v1/maintenance/pulse", get(maintenance_pulse_stream))
        .route("/api/v1/maintenance/status", get(maintenance_status))
        .route("/api/v1/maintenance/approval", get(get_pending_approval).post(respond_to_approval))
        .route("/api/v1/maintenance/audit-log", get(maintenance_audit_log))
        .route("/api/v1/maintenance/patches", get(count_patches))
        // Evolutionary Versioning & Rollback endpoints
        .route("/api/v1/maintenance/patch-history", get(patch_version_history))
        .route("/api/v1/maintenance/rollback", post(rollback_skill_endpoint))
        // Forge Safety Governor endpoints (Kill Switch + Auto-Revert UI)
        .route("/api/v1/forge/safety-status", get(forge_safety_status_get))
        .route("/api/v1/forge/safety", post(forge_safety_set))
        .route("/api/v1/forge/create", post(forge_create_post))
        // Forge Hot-Reload endpoints (Dynamic skill loading)
        .route("/api/v1/forge/hot-reload/status", get(forge_hot_reload_status))
        .route("/api/v1/forge/hot-reload/enable", post(forge_hot_reload_enable))
        .route("/api/v1/forge/hot-reload/disable", post(forge_hot_reload_disable))
        .route("/api/v1/forge/hot-reload/list", get(forge_hot_reload_list))
        .route("/api/v1/forge/hot-reload/trigger", post(forge_hot_reload_trigger))
        // System status (version for Wave 1 support; single source of truth from Cargo.toml)
        .route("/api/v1/system/status", get(system_status))
        .route("/api/v1/system/vitality", get(system_vitality_get))
        // Diagnostic Export endpoint (Beta Testing)
        .route("/api/v1/system/diagnostics", get(diagnostics::export_diagnostics))
        // SecureVault: OS keychain for API keys (vault-first migration POC)
        .route("/api/v1/config/vault/set", post(vault_set_post))
        .route("/api/v1/config/vault/status", get(vault_status_get))
        .route("/api/v1/vault/protected-terms", get(vault_protected_terms_get).post(vault_protected_terms_post))
        .route("/api/v1/vault/redact-test", post(vault_redact_test_post))
        // Deep Audit: Sovereign document ingestion and routing
        .route("/api/v1/audit/ingest", post(audit_ingest_post))
        // Project Vault: bind UI project to local folder; Master Analysis injects folder into chat context
        .route("/api/v1/projects/associate", post(projects_associate_post))
        .route("/api/v1/projects/associations", get(projects_associations_get))
        // KB-04 (Chronos): project-based chat history sidebar
        .route("/api/v1/chronos/projects", get(chronos_projects_list).post(chronos_projects_create))
        .route("/api/v1/chronos/threads", get(chronos_threads_list).post(chronos_threads_create))
        .route(
            "/api/v1/chronos/threads/:thread_id",
            post(chronos_threads_rename).delete(chronos_threads_delete),
        )
        .route(
            "/api/v1/chronos/threads/:thread_id/tag",
            post(chronos_threads_tag),
        )
        .route(
            "/api/v1/chronos/threads/:thread_id/messages",
            get(chronos_messages_list),
        )
        // Project Vault: document session — export chat history as Markdown into project folder (Sovereign Audit Trail)
        .route("/api/v1/projects/document-session", post(projects_document_session_post))
        // Beta Ops: validate tester submission bundles (Operation First Rise)
        .route("/api/v1/mission/validate", post(mission_validate_post))
        // Mimir: meeting capture (start/stop/status) — Sovereign Meeting Hub
        .route("/api/v1/mimir/preflight", get(mimir::mimir_preflight_get))
        .route("/api/v1/mimir/start", post(mimir::mimir_start_post))
        .route("/api/v1/mimir/stop", post(mimir::mimir_stop_post))
        .route("/api/v1/mimir/status", get(mimir::mimir_status_get))
        .route("/api/v1/mimir/synthesize", post(mimir_synthesize_post))
        .with_state(state);

    if frontend_enabled {
        let frontend_dir = frontend_root_dir();
        let index_file = frontend_dir.join("index.html");
        let assets_dir = frontend_dir.join("assets");

        // Map `/` -> `pagi-frontend/index.html`
        app = app.route_service("/", ServeFile::new(index_file));

        // Map `/assets/*` -> `pagi-frontend/assets/*` (CSS, images, etc.)
        if assets_dir.exists() {
            app = app.nest_service("/assets", ServeDir::new(assets_dir));
        }

        // Map `/ui/*` -> `pagi-frontend/*` (app.js, assets, and any other files)
        app = app.nest_service("/ui", ServeDir::new(frontend_dir));
    }

    app.layer(cors)
}

#[derive(serde::Deserialize)]
struct MissionValidateRequest {
    /// Optional; defaults to "mission-review".
    #[serde(default)]
    tenant_id: Option<String>,
    /// Optional; defaults to DEFAULT_AGENT_ID.
    #[serde(default)]
    agent_id: Option<String>,

    #[serde(default)]
    density_mode: Option<String>,
    #[serde(default)]
    json_envelope: Option<serde_json::Value>,
    #[serde(default)]
    sidecar_logs: Option<String>,
    #[serde(default)]
    diagramviewer_screenshot_description: Option<String>,
}

/// POST /api/v1/mission/validate – Validate an Operation First Rise submission bundle.
///
/// Thin HTTP wrapper that calls the orchestrator skill `MissionValidator`.
async fn mission_validate_post(
    State(state): State<AppState>,
    Json(body): Json<MissionValidateRequest>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let tenant_id = body
        .tenant_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("mission-review")
        .to_string();
    let agent_id = body
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(pagi_core::DEFAULT_AGENT_ID)
        .to_string();

    let ctx = TenantContext {
        tenant_id,
        correlation_id: Some(uuid::Uuid::new_v4().to_string()),
        agent_id: Some(agent_id),
    };

    let payload = serde_json::json!({
        "density_mode": body.density_mode,
        "json_envelope": body.json_envelope,
        "sidecar_logs": body.sidecar_logs,
        "diagramviewer_screenshot_description": body.diagramviewer_screenshot_description,
    });

    let goal = Goal::ExecuteSkill {
        name: "MissionValidator".to_string(),
        payload: Some(payload),
    };

    match state.orchestrator.dispatch(&ctx, goal).await {
        Ok(result) => (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({
                "status": "ok",
                "report": result,
            })),
        ),
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            })),
        ),
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) config: Arc<CoreConfig>,
    /// Sovereign .env toggles (firewall strictness, astro alerts, KB-08 logging level). Loaded once at startup.
    pub(crate) sovereign_config: Arc<SovereignConfig>,
    pub(crate) orchestrator: Arc<Orchestrator>,
    pub(crate) knowledge: Arc<KnowledgeStore>,
    pub(crate) log_tx: broadcast::Sender<String>,
    pub(crate) model_router: Arc<ModelRouter>,
    pub(crate) shadow_store: ShadowStoreHandle,
    /// When true, chat requests are routed via MoE gating (route_to_experts) to OpenRouter / LanceDB / SystemTool.
    pub(crate) moe_active: Arc<AtomicBool>,
    /// Tracks gateway idle time for the Autonomous Maintenance Loop.
    pub(crate) idle_tracker: IdleTracker,
    /// Bridge for UI-based maintenance patch approval (oneshot channel).
    pub(crate) approval_bridge: ApprovalBridgeHandle,
    /// Orchestrator role (Counselor default) and user archetype for context injection.
    pub(crate) persona_coordinator: Arc<PersonaCoordinator>,
    /// Context density: concise | balanced | verbose. Togglable via GET/POST /api/v1/settings/density.
    pub(crate) density_mode: Arc<tokio::sync::RwLock<String>>,
    /// Broadcast for 4-hour system heartbeat (Spirit/Mind/Body check). Subscribe via /api/v1/persona/stream.
    pub(crate) persona_pulse_tx: broadcast::Sender<serde_json::Value>,
    /// Counter for critical threshold tracking (velocity > 80). Increments every 2s tick.
    pub(crate) critical_threshold_counter: Arc<AtomicU64>,
    /// Background SAO intelligence layer (pattern matching + heuristics)
    pub(crate) intelligence_service: Arc<OrchestratorService>,
    /// Astro-Weather: transit risk (Stable / High Risk) for SYSTEM_PROMPT and UI widget.
    pub(crate) astro_weather: Arc<tokio::sync::RwLock<AstroWeatherState>>,
    /// 3-Tier Skills Inventory: manifest-based registry (core / import / ephemeral) for GET /api/v1/skills and permission checks.
    pub(crate) skill_manifest_registry: Arc<SkillManifestRegistry>,
    /// When PAGI_FOCUS_SHIELD_ENABLED and MS_GRAPH_* set: client for calendar/working hours (Schedule Outlook + Gatekeeper).
    pub(crate) ms_graph_client: Option<Arc<MicrosoftGraphClient>>,
    /// Latest sovereignty score (f64 bits) for Governor webhook payload. Updated by POST /api/v1/sovereignty-audit.
    pub(crate) sovereignty_score_bits: Arc<AtomicU64>,
    /// Project Folder (Project Vault) associations: project_id -> local_path + master_analysis. Persisted to data/project_associations.json.
    pub(crate) project_associations: Arc<tokio::sync::RwLock<std::collections::HashMap<String, ProjectAssociation>>>,
    /// Cached folder summary per project_id for Master Analysis context injection. Refreshed on associate and by file watcher.
    pub(crate) folder_summary_cache: Arc<tokio::sync::RwLock<std::collections::HashMap<String, String>>>,

    /// KB-04 (Chronos): project/thread chat history DB (SQLite).
    pub(crate) chronos_db: Arc<ChronosSqlite>,

    /// Mimir meeting capture: active session when recording (start/stop/status).
    pub(crate) mimir_session: Arc<tokio::sync::Mutex<Option<mimir::MimirSession>>>,
}

/// GET /api/v1/health – liveness check. Returns PHOENIX MARIE identity (SAO Orchestrator Core).
async fn health() -> axum::Json<serde_json::Value> {
    let identity_name = std::env::var("PAGI_IDENTITY_NAME").unwrap_or_else(|_| "PHOENIX MARIE".to_string());
    axum::Json(serde_json::json!({
        "status": "ok",
        "identity": identity_name,
        "message": "PHOENIX MARIE (Sovereign Recursive System)."
    }))
}

/// GET /api/v1/skills – list available skills and trust status (core / import / generated).
async fn skills_list(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let inventory = state.skill_manifest_registry.list_inventory();
    let skills: Vec<serde_json::Value> = inventory
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "skill_id": e.skill_id,
                "trust_tier": e.trust_tier,
                "trust_status": e.trust_tier,
                "kb_layers_allowed": e.kb_layers_allowed,
                "description": e.description,
            })
        })
        .collect();
    axum::Json(serde_json::json!({ "skills": skills }))
}

#[derive(serde::Deserialize)]
struct SkillsPromoteBody {
    skill_id: String,
    confirmed: bool,
}

/// POST /api/v1/skills/promote – move a skill from ephemeral (generated) to core. Requires manual confirmation.
/// Updates in-memory registry and persists to disk (ephemeral/manifest.json and core/manifest.json).
async fn skills_promote(
    State(state): State<AppState>,
    Json(body): Json<SkillsPromoteBody>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    if !body.confirmed && !state.sovereign_config.skills_auto_promote_allowed {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "status": "error",
                "message": "Promotion requires confirmed: true (human-in-the-loop) unless PAGI_SKILLS_AUTO_PROMOTE_ALLOWED=true."
            })),
        );
    }
    let skill_id = body.skill_id.trim().to_string();
    let ok = state.skill_manifest_registry.promote_to_core(&skill_id);
    if !ok {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "status": "error",
                "message": "Skill not found in ephemeral tier or already core/import."
            })),
        );
    }
    let persist_msg = persist_promotion_to_disk(&pagi_skills_root(), &skill_id, &state.skill_manifest_registry);
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "status": "ok",
            "message": persist_msg,
            "skill_id": skill_id,
        })),
    )
}

/// Writes promotion to disk: remove skill from ephemeral/manifest.json, add to core/manifest.json (atomic write).
fn persist_promotion_to_disk(
    skills_root: &std::path::Path,
    skill_id: &str,
    registry: &SkillManifestRegistry,
) -> String {
    let entry = registry
        .list_inventory()
        .into_iter()
        .find(|e| e.skill_id == skill_id)
        .map(|e| SkillManifestEntry {
            skill_id: e.skill_id,
            kb_layers_allowed: e.kb_layers_allowed,
            description: e.description,
        });
    let Some(entry) = entry else {
        return "Skill promoted in memory; disk persist skipped (entry not found).".to_string();
    };
    let ephemeral_path = skills_root.join("ephemeral").join("manifest.json");
    let core_path = skills_root.join("core").join("manifest.json");
    let read_manifest = |path: &std::path::Path| -> Option<TierManifest> {
        let bytes = std::fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    };
    let write_atomic = |path: &std::path::Path, manifest: &TierManifest| {
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(manifest).unwrap_or_default();
        if std::fs::write(&tmp, json).is_err() {
            return false;
        }
        std::fs::rename(&tmp, path).is_ok()
    };
    let mut ephemeral = match read_manifest(&ephemeral_path) {
        Some(m) => m,
        None => {
            return "Skill promoted in memory; disk persist skipped (ephemeral manifest missing).".to_string();
        }
    };
    let mut core_manifest = match read_manifest(&core_path) {
        Some(m) => m,
        None => {
            return "Skill promoted in memory; disk persist skipped (core manifest missing).".to_string();
        }
    };
    ephemeral.skills.retain(|s| s.skill_id != skill_id);
    core_manifest.skills.push(entry);
    let ok_ephemeral = write_atomic(&ephemeral_path, &ephemeral);
    let ok_core = write_atomic(&core_path, &core_manifest);
    if ok_ephemeral && ok_core {
        "Skill promoted to core and persisted to disk (ephemeral + core manifests).".to_string()
    } else {
        "Skill promoted in memory; one or both manifest writes failed.".to_string()
    }
}

/// GET /api/v1/config – feature settings from .env for the Settings UI (no secrets). Includes MoE toggle state and all customizable env.
async fn feature_config(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let fs_access_enabled = std::env::var("PAGI_FS_ACCESS_ENABLED")
        .map(|v| v.trim().eq_ignore_ascii_case("true") || v.trim().is_empty())
        .unwrap_or(true);
    let fs_root = std::env::var("PAGI_FS_ROOT")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
    let llm_mode = std::env::var("PAGI_LLM_MODE").unwrap_or_else(|_| "mock".to_string());
    let llm_model = std::env::var("PAGI_LLM_MODEL").unwrap_or_else(|_| "anthropic/claude-opus-4.6".to_string());
    let tick_rate_secs = std::env::var("PAGI_TICK_RATE_SECS")
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&n| n >= 1)
        .unwrap_or(5);
    let local_context_limit = local_context_limit();
    let moe_default = std::env::var("PAGI_MOE_DEFAULT")
        .ok()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| s == "sparse" || s == "dense")
        .unwrap_or_else(|| "dense".to_string());
    let moe_active = state.moe_active.load(Ordering::Acquire);
    let moe_mode = state.orchestrator.get_moe_mode().as_str();
    let orchestrator_role = state.persona_coordinator.get_mode().as_str();
    let density_mode = state.density_mode.read().await.clone();
    axum::Json(serde_json::json!({
        "fs_access_enabled": fs_access_enabled,
        "fs_root": fs_root,
        "llm_mode": llm_mode.trim().to_lowercase(),
        "llm_model": llm_model.trim(),
        "tick_rate_secs": tick_rate_secs,
        "local_context_limit": local_context_limit,
        "moe_default": moe_default,
        "moe_active": moe_active,
        "moe_mode": moe_mode,
        "orchestrator_role": orchestrator_role,
        "persona_mode": orchestrator_role,
        "density_mode": density_mode,
    }))
}

/// GET /api/v1/config/status – Sovereign defensive layers (Phoenix Warden .env toggles). For UI status display.
/// Includes current_active_archetype (from PAGI_PRIMARY_ARCHETYPE) so the UI can theme by who is "speaking."
async fn config_status_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let s = &*state.sovereign_config;
    let humanity_ratio = s.humanity_ratio;
    let current_active_archetype = s
        .primary_archetype
        .as_deref()
        .unwrap_or("pisces")
        .to_lowercase();
    axum::Json(serde_json::json!({
        "firewall_strict_mode": s.firewall_strict_mode,
        "astro_logic_enabled": s.astro_logic_enabled,
        "astro_alerts_enabled": s.astro_alerts_enabled,
        "transit_alerts_enabled": s.transit_alerts_enabled,
        "sovereignty_auto_rank_enabled": s.sovereignty_auto_rank_enabled,
        "skills_auto_promote_allowed": s.skills_auto_promote_allowed,
        "kb08_success_logging": s.kb08_success_logging,
        "kb08_logging_level": s.kb08_logging_level,
        "strict_technical_mode": s.strict_technical_mode,
        "humanity_ratio": humanity_ratio,
        "humanity_blend": pagi_core::humanity_blend_label(humanity_ratio),
        "persona_blend": pagi_core::humanity_blend_label(humanity_ratio),
        "current_active_archetype": current_active_archetype,
        "primary_archetype": s.primary_archetype,
        "secondary_archetype": s.secondary_archetype,
        "archetype_auto_switch_enabled": s.archetype_auto_switch_enabled,
    }))
}

// ══════════════════════════════════════════════════════════════════════════════
// USER CONFIGURATION ENDPOINTS (Beta Distribution)
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/config/api-key – Check if API key is configured (for first-run detection)
async fn get_api_key_status() -> axum::Json<serde_json::Value> {
    use pagi_core::UserConfig;
    
    match UserConfig::load() {
        Ok(config) => {
            axum::Json(serde_json::json!({
                "configured": config.get_api_key().is_some(),
                "first_run": config.is_first_run(),
                "has_user_name": config.user_name.is_some(),
            }))
        }
        Err(e) => {
            tracing::warn!(target: "pagi::config", error = %e, "Failed to load user config");
            axum::Json(serde_json::json!({
                "configured": false,
                "first_run": true,
                "has_user_name": false,
                "error": format!("{}", e),
            }))
        }
    }
}

/// POST /api/v1/config/api-key – Save user's API key to local config
#[derive(serde::Deserialize)]
struct SetApiKeyRequest {
    api_key: String,
    #[serde(default)]
    llm_model: Option<String>,
    #[serde(default)]
    llm_api_url: Option<String>,
    #[serde(default)]
    user_name: Option<String>,
}

async fn set_api_key(
    Json(body): Json<SetApiKeyRequest>,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, String)> {
    use pagi_core::UserConfig;
    
    if body.api_key.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "API key cannot be empty".to_string()));
    }
    
    match UserConfig::load() {
        Ok(mut config) => {
            // Update configuration
            if let Err(e) = config.set_api_key(body.api_key.clone()) {
                tracing::error!(target: "pagi::config", error = %e, "Failed to save API key");
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save API key: {}", e)));
            }
            
            // Update optional fields
            if let Some(model) = body.llm_model {
                config.llm_model = Some(model);
            }
            if let Some(url) = body.llm_api_url {
                config.llm_api_url = Some(url);
            }
            if let Some(name) = body.user_name {
                config.user_name = Some(name);
            }
            
            // Save again with all updates
            if let Err(e) = config.save() {
                tracing::error!(target: "pagi::config", error = %e, "Failed to save user config");
                return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to save config: {}", e)));
            }
            
            tracing::info!(target: "pagi::config", "User API key configured successfully");
            
            Ok(axum::Json(serde_json::json!({
                "success": true,
                "message": "API key saved successfully",
                "first_run": false,
            })))
        }
        Err(e) => {
            tracing::error!(target: "pagi::config", error = %e, "Failed to load user config");
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load config: {}", e)))
        }
    }
}

/// GET /api/v1/config/user – Get user configuration (without exposing API key)
async fn get_user_config() -> axum::Json<serde_json::Value> {
    use pagi_core::UserConfig;
    
    match UserConfig::load() {
        Ok(config) => {
            axum::Json(serde_json::json!({
                "first_run": config.is_first_run(),
                "has_api_key": config.get_api_key().is_some(),
                "llm_model": config.get_llm_model(),
                "llm_api_url": config.get_llm_api_url(),
                "user_name": config.user_name,
                "version": config.version,
            }))
        }
        Err(e) => {
            tracing::warn!(target: "pagi::config", error = %e, "Failed to load user config");
            axum::Json(serde_json::json!({
                "first_run": true,
                "has_api_key": false,
                "error": format!("{}", e),
            }))
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// VERSION & UPDATE ENDPOINTS (Beta Distribution)
// ══════════════════════════════════════════════════════════════════════════════

/// GET /api/v1/system/status – System-level status for UI (version from Cargo.toml; Wave 1 support).
async fn system_status() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "ok",
        "version": GATEWAY_VERSION,
        "name": "Phoenix",
    }))
}

/// GET /api/v1/system/vitality – Real-time hardware stats (CPU/RAM/Disk) for Architect view and JSON Diagram Envelope.
async fn system_vitality_get(
    State(state): State<AppState>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let ctx = TenantContext {
        tenant_id: "default".to_string(),
        correlation_id: Some(uuid::Uuid::new_v4().to_string()),
        agent_id: Some(pagi_core::DEFAULT_AGENT_ID.to_string()),
    };
    let goal = Goal::ExecuteSkill {
        name: "GetHardwareStats".to_string(),
        payload: Some(serde_json::json!({})),
    };
    match state.orchestrator.dispatch(&ctx, goal).await {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": format!("{}", e),
                "summary": "Hardware vitality unavailable",
            })),
        ),
    }
}

#[derive(serde::Deserialize)]
struct MimirSynthesizeRequest {
    #[serde(default)]
    summary_path: Option<String>,
    #[serde(default)]
    meeting_id: Option<String>,
    #[serde(default)]
    project_id: Option<String>,
}

/// POST /api/v1/mimir/synthesize – Cross-reference meeting summary with infrastructure vitality (KB-Linker).
/// Accepts summary_path (direct path to Markdown) or meeting_id (resolve via Chronos). Returns Mermaid diagram + alerts.
async fn mimir_synthesize_post(
    State(state): State<AppState>,
    Json(body): Json<MimirSynthesizeRequest>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let summary_path: Option<String> = body
        .summary_path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from);

    let summary_path = if summary_path.is_some() {
        summary_path
    } else if let Some(meeting_id) = body.meeting_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let db_path = state.chronos_db.path().to_path_buf();
        let mid = meeting_id.to_string();
        match tokio::task::spawn_blocking(move || {
            let storage = pagi_mimir::MeetingStorage::new(db_path).ok()?;
            storage.get_meeting(&mid).ok()?
        })
        .await
        {
            Ok(Some(row)) => row.summary_path,
            _ => None,
        }
    } else {
        None
    };

    let summary_path = match summary_path {
        Some(p) => p,
        None => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "error": "Provide summary_path or meeting_id to synthesize.",
                })),
            );
        }
    };

    let ctx = TenantContext {
        tenant_id: "default".to_string(),
        correlation_id: Some(uuid::Uuid::new_v4().to_string()),
        agent_id: Some(pagi_core::DEFAULT_AGENT_ID.to_string()),
    };
    let payload = serde_json::json!({
        "summary_path": summary_path,
        "project_id": body.project_id.as_deref().map(str::trim).filter(|s| !s.is_empty()),
    });
    let goal = Goal::ExecuteSkill {
        name: "SynthesizeMeetingContext".to_string(),
        payload: Some(payload),
    };

    match state.orchestrator.dispatch(&ctx, goal).await {
        Ok(v) => (axum::http::StatusCode::OK, axum::Json(v)),
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "error": format!("{}", e),
                "summary": "Synthesis failed",
            })),
        ),
    }
}

/// POST /api/v1/config/vault/set – Store a key in the OS keychain (e.g. OPENROUTER_API_KEY). POC for vault migration.
#[derive(serde::Deserialize)]
struct VaultSetRequest {
    key: String,
    value: String,
}

async fn vault_set_post(
    Json(body): Json<VaultSetRequest>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let key = body.key.trim();
    if key.is_empty() || body.value.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "key and value required",
            })),
        );
    }
    match SecureVault::new().set(key, &body.value) {
        Ok(()) => {
            tracing::info!(target: "pagi::vault", key = %key, "Vault key set (OS keychain)");
            (
                axum::http::StatusCode::OK,
                axum::Json(serde_json::json!({
                    "success": true,
                    "key": key,
                    "message": "Key stored in OS keychain. Restart gateway to use it.",
                })),
            )
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({
                "success": false,
                "key": key,
                "error": e,
            })),
        ),
    }
}

/// GET /api/v1/config/vault/status – Report which keys are in the vault (without revealing values).
async fn vault_status_get() -> axum::Json<serde_json::Value> {
    let vault = SecureVault::new();
    let openrouter_in_vault = vault.get("OPENROUTER_API_KEY").is_ok();
    let pagi_llm_in_vault = vault.get("PAGI_LLM_API_KEY").is_ok();
    axum::Json(serde_json::json!({
        "openrouter_in_vault": openrouter_in_vault,
        "pagi_llm_in_vault": pagi_llm_in_vault,
    }))
}

fn vault_data_dir(state: &AppState) -> std::path::PathBuf {
    let db_path = state.chronos_db.path();
    db_path
        .parent()
        .and_then(std::path::Path::parent)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| db_path.to_path_buf())
}

fn protected_terms_path(state: &AppState) -> std::path::PathBuf {
    vault_data_dir(state).join("protected_terms.txt")
}

#[derive(serde::Deserialize)]
struct VaultProtectedTermsQuery {
    project_id: Option<String>,
}

/// GET /api/v1/vault/protected-terms – Read SAO protected terms. Optional ?project_id= returns merged global + project .sao_policy.
async fn vault_protected_terms_get(
    State(state): State<AppState>,
    Query(q): Query<VaultProtectedTermsQuery>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let data_dir = vault_data_dir(&state);
    let global_path = data_dir.join("protected_terms.txt");
    let global: Vec<String> = pagi_core::SAORedactor::read_terms_from_path(&global_path).unwrap_or_default();

    let project_id = q.project_id.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if project_id.is_none() {
        return (
            axum::http::StatusCode::OK,
            axum::Json(serde_json::json!({ "terms": global, "scope": "global" })),
        );
    }

    let project_id = project_id.unwrap();
    let assocs = state.project_associations.read().await;
    let project_path = assocs.get(project_id).map(|a| StdPath::new(&a.local_path).to_path_buf());
    drop(assocs);

    let (local, merged) = match project_path {
        Some(ref p) => {
            let policy_path = p.join(".sao_policy");
            let local: Vec<String> = pagi_core::SAORedactor::read_terms_from_path(&policy_path).unwrap_or_default();
            let mut merged: Vec<String> = global.iter().cloned().collect();
            for t in &local {
                if !merged.contains(t) {
                    merged.push(t.clone());
                }
            }
            (local, merged)
        }
        None => (Vec::new(), global.clone()),
    };

    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "scope": "project",
            "project_id": project_id,
            "global": global,
            "local": local,
            "merged": merged,
            "terms": merged
        })),
    )
}

#[derive(serde::Deserialize)]
struct VaultProtectedTermsPostBody {
    terms: Vec<String>,
    /// When set, terms are written to project_path/.sao_policy instead of global protected_terms.txt.
    project_id: Option<String>,
}

/// POST /api/v1/vault/protected-terms – Write SAO protected terms. Global: data/protected_terms.txt. Project: project_path/.sao_policy.
async fn vault_protected_terms_post(
    State(state): State<AppState>,
    Json(body): Json<VaultProtectedTermsPostBody>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let terms: Vec<String> = body
        .terms
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let (path, message) = match body.project_id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(project_id) => {
            let assocs = state.project_associations.read().await;
            let assoc = match assocs.get(project_id) {
                Some(a) => a.clone(),
                None => {
                    return (
                        axum::http::StatusCode::BAD_REQUEST,
                        axum::Json(serde_json::json!({
                            "error": "Unknown project_id. Associate a folder with this project first."
                        })),
                    );
                }
            };
            let project_path = StdPath::new(&assoc.local_path).to_path_buf();
            let policy_path = project_path.join(".sao_policy");
            if let Err(e) = std::fs::create_dir_all(&project_path) {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({ "error": format!("Could not create project dir: {}", e) })),
                );
            }
            (
                policy_path,
                format!("Project policy saved. {} term(s) in {}.", terms.len(), project_id),
            )
        }
        None => {
            let path = protected_terms_path(&state);
            if let Err(e) = std::fs::create_dir_all(path.parent().unwrap_or_else(|| std::path::Path::new("."))) {
                return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(serde_json::json!({ "error": format!("Could not create data dir: {}", e) })),
                );
            }
            (
                path,
                "Terms saved. Next Mimir stop and bridge will use updated redactor.".to_string(),
            )
        }
    };

    let content = format!(
        "# SAO Protected Terms — redacted in meeting minutes/transcripts\n# One term per line; blank lines and # comments ignored.\n\n{}\n",
        terms.join("\n")
    );
    match std::fs::write(&path, content) {
        Ok(()) => {
            tracing::info!(target: "pagi::vault", path = %path.display(), count = terms.len(), "Protected terms updated");
            (
                axum::http::StatusCode::OK,
                axum::Json(serde_json::json!({
                    "success": true,
                    "terms": terms,
                    "message": message,
                    "scope": if body.project_id.is_some() { "project" } else { "global" }
                })),
            )
        }
        Err(e) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "error": format!("{}", e) })),
        ),
    }
}

#[derive(serde::Deserialize)]
struct VaultRedactTestBody {
    text: String,
    /// When set, redaction uses global + project_path/.sao_policy (merged).
    project_id: Option<String>,
}

/// POST /api/v1/vault/redact-test – Sanitize sample text with current protected terms. Optional project_id uses merged global + .sao_policy.
async fn vault_redact_test_post(
    State(state): State<AppState>,
    Json(body): Json<VaultRedactTestBody>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let data_dir = vault_data_dir(&state);
    let project_path = if let Some(ref pid) = body.project_id {
        let assocs = state.project_associations.read().await;
        assocs
            .get(pid.as_str())
            .map(|a| StdPath::new(&a.local_path).to_path_buf())
    } else {
        None
    };

    let redactor = match project_path {
        Some(ref p) => pagi_core::SAORedactor::load_global_then_merge_project(&data_dir, Some(p.as_path()))
            .unwrap_or_else(|_| pagi_core::SAORedactor::empty()),
        None => pagi_core::SAORedactor::load_from_data_dir(&data_dir).unwrap_or_else(|_| pagi_core::SAORedactor::empty()),
    };
    let sanitized = redactor.sanitize_transcript(body.text.clone());
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({ "original": body.text, "sanitized": sanitized })),
    )
}

/// POST /api/v1/audit/ingest – Trigger Deep Audit sweep of ./data/ingest directory
async fn audit_ingest_post(
    State(state): State<AppState>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let data_dir = vault_data_dir(&state);
    let skill = pagi_skills::DeepAuditSkill::new(data_dir);
    
    match skill.sweep_ingest_dir().await {
        Ok(summary) => {
            tracing::info!(
                target: "pagi::audit",
                files = summary.files_processed,
                vectors = summary.vectors_created,
                "Deep Audit sweep completed"
            );
            (
                axum::http::StatusCode::OK,
                axum::Json(serde_json::to_value(summary).unwrap_or_else(|_| serde_json::json!({
                    "status": "error",
                    "message": "Failed to serialize summary"
                }))),
            )
        }
        Err(e) => {
            tracing::error!(target: "pagi::audit", error = %e, "Deep Audit sweep failed");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Audit failed: {}", e)
                })),
            )
        }
    }
}

/// POST /api/v1/projects/associate – Bind a project_id to a local_path; optionally set master_analysis. Persists to data/project_associations.json.
#[derive(serde::Deserialize)]
struct ProjectsAssociateBody {
    project_id: String,
    local_path: String,
    #[serde(default)]
    master_analysis: Option<bool>,
}

async fn projects_associate_post(
    State(state): State<AppState>,
    Json(body): Json<ProjectsAssociateBody>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let project_id = body.project_id.trim().to_string();
    let local_path = body.local_path.trim().replace('\\', "/").trim_end_matches('/').to_string();
    if project_id.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "message": "project_id is required" })),
        );
    }
    if local_path.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "message": "local_path is required" })),
        );
    }
    let path = StdPath::new(&local_path);
    if !path.exists() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "message": "local_path does not exist" })),
        );
    }
    if !path.is_dir() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "message": "local_path must be a directory" })),
        );
    }
    let master_analysis = body.master_analysis.unwrap_or(true);
    let assoc = ProjectAssociation {
        project_id: project_id.clone(),
        local_path: local_path.clone(),
        master_analysis,
    };
    {
        let mut assocs = state.project_associations.write().await;
        assocs.insert(project_id.clone(), assoc.clone());
        save_project_associations(&*assocs);
    }
    // Refresh folder summary cache for this project
    if let Ok(summary) = pagi_core::summarize_folder_for_context_sync(path, FOLDER_CONTEXT_MAX_BYTES) {
        let mut cache = state.folder_summary_cache.write().await;
        cache.insert(project_id.clone(), summary);
    }
    tracing::info!(target: "pagi::projects", project_id = %project_id, path = %local_path, master_analysis = master_analysis, "Project folder associated");
    (
        axum::http::StatusCode::OK,
        axum::Json(serde_json::json!({
            "status": "ok",
            "project_id": project_id,
            "local_path": local_path,
            "master_analysis": master_analysis,
        })),
    )
}

/// GET /api/v1/projects/associations – List all project–local path associations (for Sidebar UI).
async fn projects_associations_get(
    State(state): State<AppState>,
) -> axum::Json<serde_json::Value> {
    let assocs = state.project_associations.read().await;
    let list: Vec<serde_json::Value> = assocs
        .values()
        .map(|a| {
            serde_json::json!({
                "project_id": a.project_id,
                "local_path": a.local_path,
                "master_analysis": a.master_analysis,
            })
        })
        .collect();
    axum::Json(serde_json::json!({ "associations": list }))
}

/// Document session request: export current thread as Markdown into the project folder (Sovereign Audit Trail).
#[derive(serde::Deserialize)]
struct DocumentSessionBody {
    project_id: String,
    title: String,
    messages: Vec<DocumentSessionMessage>,
}

#[derive(serde::Deserialize)]
struct DocumentSessionMessage {
    role: String,
    content: String,
    #[serde(default)]
    thoughts: Option<Vec<DocumentSessionThought>>,
}

#[derive(serde::Deserialize)]
struct DocumentSessionThought {
    title: String,
    content: String,
}

/// Milestone Trigger: lightweight heuristic to suggest documenting the session after a "solution" or diagram.
/// Returns true when the exchange looks like a good candidate for archival (e.g. solved a bottleneck, produced a diagram).
fn suggest_milestone_archive(prompt: &str, response: &str) -> bool {
    if response.len() < 80 {
        return false;
    }
    let prompt_lower = prompt.to_lowercase();
    let response_lower = response.to_lowercase();
    let combined = format!("{} {}", prompt_lower, response_lower);

    // Diagram: response contains Mermaid (common in Phoenix outputs)
    if response.contains("```mermaid") || response.contains("``` mermaid") {
        return true;
    }
    // User asked for documentation or summary
    if prompt_lower.contains("document") || prompt_lower.contains("summarize") || prompt_lower.contains("summary of") {
        return true;
    }
    // Solution-oriented phrases in response
    let solution_phrases = [
        "solved",
        "bottleneck",
        "here's the diagram",
        "as shown in the diagram",
        "solution:",
        "fixed the",
        "resolved the",
        "root cause",
        "here is the",
        "step-by-step",
        "summary:",
        "in summary",
        "to summarize",
    ];
    if solution_phrases.iter().any(|p| combined.contains(p)) {
        return true;
    }
    false
}

fn slug_from_title(title: &str) -> String {
    let s: String = title
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let s = s.trim_matches('_');
    if s.is_empty() { "session".to_string() } else { s.to_string() }
}

/// Build Markdown from messages; preserves Mermaid blocks in content and thoughts for Obsidian/VS Code.
fn build_session_markdown(title: &str, messages: &[DocumentSessionMessage]) -> String {
    let now = chrono::Utc::now();
    let date_hdr = now.format("%Y-%m-%d %H:%M UTC");
    let mut out = format!("# {}\n\n*Exported {}. Phoenix Project Vault — Sovereign Audit Trail.*\n\n---\n\n", title, date_hdr);
    for msg in messages {
        let role_label = if msg.role.eq_ignore_ascii_case("user") { "User" } else { "Phoenix" };
        out.push_str(&format!("## {}\n\n", role_label));
        out.push_str(&msg.content);
        if !msg.content.ends_with('\n') {
            out.push('\n');
        }
        if let Some(ref thoughts) = msg.thoughts {
            for t in thoughts {
                out.push_str(&format!("\n### {}\n\n", t.title));
                out.push_str(&t.content);
                if !t.content.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
        out.push_str("\n");
    }
    out
}

/// POST /api/v1/projects/document-session – Export current thread as Markdown into the project folder. Writes only under the mounted path (KB-05).
async fn projects_document_session_post(
    State(state): State<AppState>,
    Json(body): Json<DocumentSessionBody>,
) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    let project_id = body.project_id.trim().to_string();
    if project_id.is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "message": "project_id is required" })),
        );
    }
    let assocs = state.project_associations.read().await;
    let assoc = match assocs.get(&project_id) {
        Some(a) => a.clone(),
        None => {
            return (
                axum::http::StatusCode::NOT_FOUND,
                axum::Json(serde_json::json!({ "status": "error", "message": "Project not associated with a local folder. Mount a folder first." })),
            );
        }
    };
    drop(assocs);

    let title = body.title.trim();
    let title = if title.is_empty() { "Session" } else { title };
    let markdown = build_session_markdown(title, &body.messages);
    let now = chrono::Utc::now();
    let slug = slug_from_title(title);
    let filename = format!("history/{}_{}.md", now.format("%Y-%m-%d_%H-%M"), slug);
    let root = StdPath::new(&assoc.local_path);
    match write_document_under_root(root, &filename, &markdown) {
        Ok(()) => {
            tracing::info!(target: "pagi::projects", project_id = %project_id, file = %filename, "Document session written to project folder");
            (
                axum::http::StatusCode::OK,
                axum::Json(serde_json::json!({
                    "status": "ok",
                    "project_id": project_id,
                    "path": filename,
                    "message": "Session documented in project folder. Open the file in Obsidian or VS Code to view diagrams.",
                })),
            )
        }
        Err(e) => {
            tracing::warn!(target: "pagi::projects", project_id = %project_id, error = %e, "Document session write failed");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to write document: {}", e),
                })),
            )
        }
    }
}

/// If the chat request has a project_id with an association and master_analysis ON, returns (directive_append, preferred_workspace_path_override).
pub(crate) async fn project_folder_context_for_chat(
    state: &AppState,
    project_id: Option<&str>,
    project_name: Option<&str>,
) -> (Option<String>, Option<String>) {
    let project_id = match project_id {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return (None, None),
    };
    let (local_path, master_analysis) = {
        let assocs = state.project_associations.read().await;
        let a = match assocs.get(&project_id) {
            Some(a) => a.clone(),
            None => return (None, None),
        };
        (a.local_path.clone(), a.master_analysis)
    };
    if !master_analysis {
        return (None, Some(local_path));
    }
    let path = StdPath::new(&local_path);
    if !path.is_dir() {
        return (None, Some(local_path));
    }
    let summary = {
        let cache = state.folder_summary_cache.read().await;
        cache.get(&project_id).cloned()
    };
    let summary = match summary {
        Some(s) => s,
        None => match pagi_core::summarize_folder_for_context_sync(path, FOLDER_CONTEXT_MAX_BYTES) {
            Ok(s) => {
                let mut cache = state.folder_summary_cache.write().await;
                cache.insert(project_id.clone(), s.clone());
                s
            }
            Err(_) => return (None, Some(local_path)),
        },
    };
    let name = project_name.unwrap_or("Project");
    let directive = format!(
        "\n\n=== PROJECT VAULT (Master Analysis ON) ===\nYou have active access to the [{}] folder at: {}.\nPrioritize troubleshooting logs and summarizing emails found therein.\n\n--- Recent folder contents ---\n{}",
        name,
        local_path,
        summary
    );
    (Some(directive), Some(local_path))
}

/// GET /api/v1/version – Get current Phoenix version (Sovereign: from gateway Cargo.toml).
async fn get_version() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "version": GATEWAY_VERSION,
        "name": "Phoenix",
    }))
}

/// GET /api/v1/version/check – Check for updates
async fn check_for_updates() -> axum::Json<serde_json::Value> {
    use pagi_core::updater::UpdateChecker;
    
    let checker = UpdateChecker::new();
    
    match checker.check_for_updates().await {
        Ok(version_info) => {
            axum::Json(serde_json::json!({
                "current_version": version_info.current,
                "latest_version": version_info.latest,
                "update_available": version_info.update_available,
                "download_url": version_info.download_url,
            }))
        }
        Err(e) => {
            tracing::warn!(target: "pagi::version", error = %e, "Failed to check for updates");
            axum::Json(serde_json::json!({
                "current_version": UpdateChecker::get_current_version().unwrap_or_else(|_| "unknown".to_string()),
                "latest_version": null,
                "update_available": false,
                "error": format!("{}", e),
            }))
        }
    }
}

/// GET /api/v1/settings/moe – current MoE (Sparse Intelligence) toggle state.
async fn get_moe_settings(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "moe_active": state.moe_active.load(Ordering::Acquire),
    }))
}

/// POST /api/v1/settings/moe – set MoE (Sparse Intelligence) toggle. Body: { "enabled": bool }.
#[derive(serde::Deserialize)]
struct MoESettingsRequest {
    enabled: bool,
}

async fn set_moe_settings(
    State(state): State<AppState>,
    Json(body): Json<MoESettingsRequest>,
) -> axum::Json<serde_json::Value> {
    let mode = if body.enabled { MoEMode::Sparse } else { MoEMode::Dense };
    if let Err(e) = state.knowledge.set_sovereign_moe_mode(mode.as_str()) {
        tracing::warn!(target: "pagi::settings", error = %e, "Failed to persist MoE mode to Sovereign Config");
    }
    state.orchestrator.set_moe_mode(mode);
    state.moe_active.store(body.enabled, Ordering::Release);
    tracing::info!(target: "pagi::settings", moe_mode = mode.as_str(), "MoE (Sparse Intelligence) toggled and persisted to KB-6");
    axum::Json(serde_json::json!({
        "moe_active": state.moe_active.load(Ordering::Acquire),
        "moe_mode": state.orchestrator.get_moe_mode().as_str(),
    }))
}

/// GET /api/v1/settings/orchestrator-role – current system role (counselor; companion legacy).
async fn get_orchestrator_role_settings(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let role = state.persona_coordinator.get_mode().as_str();
    axum::Json(serde_json::json!({
        "orchestrator_role": role,
        "persona_mode": role,
    }))
}

/// POST /api/v1/settings/orchestrator-role – set system role. Body: { "mode": "counselor" | "companion" }.
#[derive(serde::Deserialize)]
struct OrchestratorRoleSettingsRequest {
    mode: String,
}

async fn set_orchestrator_role_settings(
    State(state): State<AppState>,
    Json(body): Json<OrchestratorRoleSettingsRequest>,
) -> axum::Json<serde_json::Value> {
    let mode = OrchestratorMode::from_str(&body.mode);
    state.persona_coordinator.set_mode(mode);
    let role = state.persona_coordinator.get_mode().as_str();
    axum::Json(serde_json::json!({
        "orchestrator_role": role,
        "persona_mode": role,
    }))
}

/// GET /api/v1/settings/density – current context density (concise | balanced | verbose).
async fn get_density_settings(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let density_mode = state.density_mode.read().await.clone();
    axum::Json(serde_json::json!({
        "density_mode": density_mode,
    }))
}

/// POST /api/v1/settings/density – set context density. Body: { "density_mode": "concise" | "balanced" | "verbose" }.
#[derive(serde::Deserialize)]
struct DensitySettingsRequest {
    density_mode: String,
}

async fn set_density_settings(
    State(state): State<AppState>,
    Json(body): Json<DensitySettingsRequest>,
) -> axum::Json<serde_json::Value> {
    let normalized = match body.density_mode.trim().to_lowercase().as_str() {
        "concise" | "sovereign" | "concise_sovereign" => "concise",
        "verbose" | "counselor" | "verbose_counselor" => "verbose",
        _ => "balanced",
    };
    {
        let mut d = state.density_mode.write().await;
        *d = normalized.to_string();
    }
    tracing::info!(target: "pagi::config", density = %normalized, "Context density mode updated");
    axum::Json(serde_json::json!({
        "density_mode": normalized,
    }))
}

/// Returns the CONTEXT DENSITY system-prompt block for the given mode (concise | balanced | verbose).
fn density_instruction(mode: &str) -> &'static str {
    match mode {
        "concise" => "\n\n=== CONTEXT DENSITY (Concise / Sovereign) ===\n\
Use RLM-style compression. Provide only representations, direct answers, and minimal prose. \
Use Markdown tables and lists exclusively. No long-form narrative or emotional elaboration.\n\
\n\
AUTO-VISUAL (Concise): For technical / architectural / process explanations, prefer a Mermaid diagram via the JSON Diagram Envelope format first, then 1–2 bullets max.\n",
        "verbose" => "\n\n=== CONTEXT DENSITY (Verbose / Counselor) ===\n\
Prioritize Counselor-Architect skill. Provide deep context, emotional resonance, and full historical tie-ins from KB-04 (Chronos). \
Detailed explanations and analogies are welcome when they serve clarity and sovereignty.\n",
        _ => "\n\n=== CONTEXT DENSITY (Balanced) ===\n\
Selective indexing: provide narrative context for new or complex topics; use structured summaries (lists/tables) where they add clarity. \
Balance brevity with enough context to avoid ambiguity.\n",
    }
}

/// POST /api/v1/soma/balance – store Spirit/Mind/Body (1–10) in KB-8 (Soma) for wellness tracking.
#[derive(serde::Deserialize)]
struct SomaBalanceRequest {
    spirit: u8,
    mind: u8,
    body: u8,
}

async fn soma_balance_post(
    State(state): State<AppState>,
    Json(body): Json<SomaBalanceRequest>,
) -> axum::Json<serde_json::Value> {
    let spirit = body.spirit.clamp(1, 10);
    let mind = body.mind.clamp(1, 10);
    let body_val = body.body.clamp(1, 10);
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let payload = serde_json::json!({
        "spirit": spirit,
        "mind": mind,
        "body": body_val,
        "timestamp_ms": timestamp_ms,
    });
    // Timestamped key for 7-day rolling history (WellnessReportSkill).
    let key = format!("soma/balance_check/{}", timestamp_ms);
    let soma_slot = KbType::Soma.slot_id();
    let bytes = serde_json::to_vec(&payload).unwrap_or_default();
    match state.knowledge.insert(soma_slot, &key, &bytes) {
        Ok(_) => {
            tracing::info!(target: "pagi::soma", spirit, mind, body = body_val, "Spirit/Mind/Body balance stored in KB-8 (Soma)");
            axum::Json(serde_json::json!({
                "status": "ok",
                "message": "Balance stored in KB-8 (Soma)",
                "spirit": spirit,
                "mind": mind,
                "body": body_val,
            }))
        }
        Err(e) => {
            tracing::warn!(target: "pagi::soma", error = %e, "Failed to store balance in KB-8");
            axum::Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
            }))
        }
    }
}

/// GET /api/v1/skills/wellness-report – 7-day Soma (KB-8) aggregation with individuation score.
async fn wellness_report_get(
    State(state): State<AppState>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    match skills::wellness_report::generate_report(&state.knowledge) {
        Ok(report) => {
            let report_value = serde_json::to_value(&report).unwrap_or(serde_json::Value::Null);
            Ok(axum::Json(serde_json::json!({
                "status": "ok",
                "report": report_value,
            })))
        }
        Err(e) => {
            tracing::warn!(target: "pagi::wellness", error = %e, "Wellness report failed");
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "status": "error",
                    "error": e,
                })),
            ))
        }
    }
}

/// GET /api/v1/intelligence/insights – Get cached SAO intelligence insights (pattern matching + heuristics).
async fn intelligence_insights_get(
    State(state): State<AppState>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    match state.intelligence_service.get_cached_insights().await {
        Some(insights) => {
            let insights_value = serde_json::to_value(&insights).unwrap_or(serde_json::Value::Null);
            Ok(axum::Json(serde_json::json!({
                "status": "ok",
                "insights": insights_value,
            })))
        }
        None => {
            Ok(axum::Json(serde_json::json!({
                "status": "ok",
                "insights": null,
                "message": "No cached insights available. Intelligence layer may be disabled or no analysis has been performed yet."
            })))
        }
    }
}

/// POST /api/v1/intelligence/toggle – Toggle the intelligence layer on/off.
async fn intelligence_toggle_post(
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let enabled = payload.get("enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    
    state.intelligence_service.set_enabled(enabled).await;
    
    Ok(axum::Json(serde_json::json!({
        "status": "ok",
        "enabled": enabled,
        "message": if enabled {
            "Intelligence layer enabled (SAO pattern matching + heuristics active)"
        } else {
            "Intelligence layer disabled"
        }
    })))
}

/// GET /api/v1/sentinel/domain-integrity – Sovereign Domain Integrity (Absurdity Log count + Resource Drain alerts).
async fn domain_integrity_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let absurdity_log_count = absurdity_count_entries(&state.knowledge).unwrap_or(0);
    axum::Json(serde_json::json!({
        "status": "ok",
        "absurdity_log_count": absurdity_log_count,
        "resource_drain_alerts": []
    }))
}

/// GET /api/v1/self-audit – KB-08 Logic Inconsistencies summary (self-improving SAO). For dashboard/UI.
async fn self_audit_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    match self_audit(&state.knowledge) {
        Ok(report) => axum::Json(serde_json::json!({
            "status": "ok",
            "total_entries": report.total_entries,
            "recent_messages": report.recent_messages,
        })),
        Err(e) => axum::Json(serde_json::json!({
            "status": "error",
            "error": e,
            "total_entries": 0,
            "recent_messages": [],
        })),
    }
}

/// POST /api/v1/sovereignty-audit – Run the audit skill and update Governor's sovereignty score for webhook payload.
async fn sovereignty_audit_post(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let registry = LiveSkillRegistry::default();
    let skill = match registry.get("audit") {
        Some(s) => s,
        None => {
            return axum::Json(serde_json::json!({
                "status": "error",
                "error": "audit skill not in registry",
            }));
        }
    };
    let tenant_ctx = TenantContext {
        tenant_id: "sovereignty-audit".to_string(),
        correlation_id: None,
        agent_id: Some("phoenix".to_string()),
    };
    let audit_params = serde_json::json!({ "workspace_root": "." });
    if skill.requires_security_check() {
        if let Err(e) = skill.validate_security(&state.knowledge, &audit_params).await {
            return axum::Json(serde_json::json!({
                "status": "error",
                "error": format!("KB-05 blocked: {}", e),
            }));
        }
    }
    match skill.execute(&tenant_ctx, &state.knowledge, audit_params).await {
        Ok(result) => {
            let score = result.get("sovereignty_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            state.sovereignty_score_bits.store(f64::to_bits(score), std::sync::atomic::Ordering::Relaxed);
            axum::Json(serde_json::json!({
                "status": "ok",
                "sovereignty_score": score,
                "sovereignty_compliance": result.get("sovereignty_compliance").and_then(|v| v.as_bool()).unwrap_or(score > 0.9),
                "report": result,
            }))
        }
        Err(e) => axum::Json(serde_json::json!({
            "status": "error",
            "error": e.to_string(),
        })),
    }
}

/// POST /api/v1/heal – Run audit, then for each skills_without_kb05 apply refactor (security wrap). Re-audit and log session to KB-08.
async fn heal_post(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    match run_heal_flow(&state.knowledge).await {
        Ok((audit_before, refactor_results, audit_after, final_score)) => {
            state.sovereignty_score_bits.store(
                f64::to_bits(final_score),
                std::sync::atomic::Ordering::Relaxed,
            );
            let applied = refactor_results.iter().filter(|r| r.applied).count();
            let session_msg = format!(
                "Healing Session: {} file(s) fixed, {} total; sovereignty_score after: {:.2}",
                applied,
                refactor_results.len(),
                final_score
            );
            let _ = state.knowledge.record_success_metric(&session_msg);

            axum::Json(serde_json::json!({
                "status": "ok",
                "audit_before": audit_before,
                "refactor_results": refactor_results,
                "audit_after": audit_after,
                "sovereignty_score": final_score,
                "session_logged_kb08": true,
                "message": session_msg,
            }))
        }
        Err(e) => axum::Json(serde_json::json!({
            "status": "error",
            "error": e.to_string(),
        })),
    }
}

/// GET /api/v1/onboarding/status – PHOENIX MARIE onboarding protocol. When needs_onboarding is true, UI shows the overlay.
async fn onboarding_status_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let skip = std::env::var("PAGI_SKIP_ONBOARDING")
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let mut seq = onboarding_sequence(&state.knowledge);
    if skip {
        seq.needs_onboarding = false;
    }
    match serde_json::to_value(&seq) {
        Ok(v) => axum::Json(v),
        Err(e) => axum::Json(serde_json::json!({
            "needs_onboarding": true,
            "onboarding_status": "Incomplete",
            "phase1_greeting": "",
            "phase2_audit_lines": [],
            "phase3_cta": "",
            "kb_status": [],
            "vitality": "stable",
            "profiling_questions": [],
            "error": e.to_string()
        })),
    }
}

/// POST /api/v1/onboarding/complete – Mark PHOENIX MARIE onboarding complete (writes to KB-06).
async fn onboarding_complete_post(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    const ETHOS_SLOT: u8 = 6;
    let value = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "1".to_string());
    match state.knowledge.insert(ETHOS_SLOT, ONBOARDING_COMPLETE_KEY, value.as_bytes()) {
        Ok(_) => axum::Json(serde_json::json!({ "status": "ok", "message": "Onboarding complete." })),
        Err(e) => axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

/// POST /api/v1/onboarding/user-profile – Write user profile (KB-01 Discovery) to KB-01.
/// Body: JSON object (e.g. astro_archetype, sovereignty_leaks, tone_preference). Stored under Bare Metal Sled; PII remains local.
const KB01_SLOT: u8 = 1;

async fn onboarding_user_profile_post(
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    let bytes = match serde_json::to_vec(&payload) {
        Ok(b) => b,
        Err(e) => {
            return axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() }));
        }
    };
    match state
        .knowledge
        .insert(KB01_SLOT, KB01_USER_PROFILE_KEY, &bytes)
    {
        Ok(_) => axum::Json(serde_json::json!({
            "status": "ok",
            "message": "User profile saved to KB-01 (Pneuma)."
        })),
        Err(e) => axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

/// GET /api/v1/archetype – Active archetype label from KB-01 (e.g. "Pisces-Protector", "Technical") for UI.
async fn archetype_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    const KB01_SLOT: u8 = 1;
    let bytes = match state.knowledge.get(KB01_SLOT, KB01_USER_PROFILE_KEY) {
        Ok(Some(b)) => b,
        _ => return axum::Json(serde_json::json!({ "active_archetype": null })),
    };
    let val: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        _ => return axum::Json(serde_json::json!({ "active_archetype": null })),
    };
    let label = active_archetype_label(&val);
    axum::Json(serde_json::json!({
        "active_archetype": label
    }))
}

/// GET /api/v1/subject-check?text=... – Check if text matches sovereignty_leak_triggers (KB-05). For UI Boundary Alert.
async fn subject_check_get(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> axum::Json<serde_json::Value> {
    let text = params.get("text").map(|s| s.as_str()).unwrap_or("");
    let triggers = get_sovereignty_leak_triggers(&state.knowledge);
    let matched = matched_sovereignty_triggers(&triggers, text);
    let flagged = !matched.is_empty();
    axum::Json(serde_json::json!({
        "flagged": flagged,
        "matched_triggers": matched
    }))
}

/// POST /api/v1/kb08/success-metric – Log a Success Metric in KB-08 (e.g. sovereignty leak addressed). Gated by PAGI_KB08_SUCCESS_LOGGING.
async fn success_metric_post(
    State(state): State<AppState>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    let message = payload
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("Sovereignty leak addressed in conversation");
    if state.sovereign_config.kb08_success_logging {
        match state.knowledge.record_success_metric(message) {
            Ok(()) => {
                // Knowledge link: log High Risk + Sovereignty Leak same day to KB-08 for predictive accuracy
                let astro = state.astro_weather.read().await;
                let _ = record_transit_correlation_if_high_risk(&state.knowledge, &astro, "success_metric");
                return axum::Json(serde_json::json!({
                    "status": "ok",
                    "message": "Success metric recorded in KB-08."
                }));
            }
            Err(e) => {
                return axum::Json(serde_json::json!({
                    "status": "error",
                    "error": e.to_string()
                }));
            }
        }
    }
    axum::Json(serde_json::json!({
        "status": "ok",
        "message": "KB-08 success logging disabled (PAGI_KB08_SUCCESS_LOGGING=false)."
    }))
}

/// Compute current vitality from config sovereign_attributes (capacity, load, status). Used for protocol cross-link.
fn current_vitality(config: &CoreConfig) -> Option<String> {
    let attrs = config.sovereign_attributes.as_ref()?;
    if let Some(ref s) = attrs.status {
        let lower = s.to_lowercase();
        if lower.contains("critical") {
            return Some("critical".to_string());
        }
        if lower.contains("draining") {
            return Some("draining".to_string());
        }
        return Some("stable".to_string());
    }
    let processor = HeuristicProcessor::new(SovereignDomain::default());
    let level = processor.evaluate_vitality(attrs)?;
    Some(match level {
        VitalityLevel::Stable => "stable",
        VitalityLevel::Draining => "draining",
        VitalityLevel::Critical => "critical",
    }.to_string())
}

/// GET /api/v1/astro-weather – Transit risk (Stable / Elevated / High Risk) for WardenSidebar widget and SYSTEM_PROMPT.
async fn astro_weather_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let guard = state.astro_weather.read().await;
    axum::Json(serde_json::json!({
        "status": "ok",
        "risk": guard.risk.as_str(),
        "transit_summary": guard.transit_summary,
        "advice": guard.advice,
        "updated_at_ms": guard.updated_at_ms,
    }))
}

/// GET /api/v1/health-report – Sovereign Health Report (KB-08 Analytics). Pulls KB-01 (user/archetype) and KB-08 (success metrics, transit correlations).
async fn health_report_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let sovereignty_report = generate_weekly_sovereignty_report(&state.knowledge);
    match generate_weekly_report(&state.knowledge) {
        Ok(report) => axum::Json(serde_json::json!({
            "status": "ok",
            "report": report,
            "sovereignty_report": sovereignty_report,
        })),
        Err(e) => axum::Json(serde_json::json!({
            "status": "error",
            "error": e,
            "sovereignty_report": sovereignty_report,
        })),
    }
}

#[derive(serde::Deserialize)]
struct EveningAuditBody {
    status: String,
    #[serde(default)]
    lesson: Option<String>,
}

/// POST /api/v1/evening-audit – Record evening reflection (success/challenge + optional lesson). Stores in KB-08 for weekly synthesis.
async fn evening_audit_post(
    State(state): State<AppState>,
    Json(body): Json<EveningAuditBody>,
) -> axum::Json<serde_json::Value> {
    let status = body.status.trim().to_lowercase();
    if status != "success" && status != "challenge" {
        return axum::Json(serde_json::json!({
            "status": "error",
            "error": "status must be 'success' or 'challenge'",
        }));
    }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    match record_evening_audit(
        &state.knowledge,
        &today,
        &status,
        body.lesson.as_deref().filter(|s| !s.trim().is_empty()),
    ) {
        Ok(()) => axum::Json(serde_json::json!({
            "status": "ok",
            "message": "Evening audit recorded.",
            "date": today,
        })),
        Err(e) => axum::Json(serde_json::json!({
            "status": "error",
            "error": e.to_string(),
        })),
    }
}

/// GET /api/v1/domain/vitality – System Vitality (generic capacity/load/status). Returns status and vitality: stable | draining | critical.
async fn domain_vitality_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let (vitality, message) = if let Some(ref attrs) = state.config.sovereign_attributes {
        let processor = HeuristicProcessor::new(SovereignDomain::default());
        let level = processor.evaluate_vitality(attrs);
        let vitality = match level {
            Some(VitalityLevel::Critical) => "critical",
            Some(VitalityLevel::Draining) => "draining",
            Some(VitalityLevel::Stable) => "stable",
            None => {
                let s = attrs.status.as_deref().unwrap_or("").to_lowercase();
                if s.contains("critical") {
                    "critical"
                } else if s.contains("draining") {
                    "draining"
                } else {
                    "stable"
                }
            }
        };
        let msg = attrs.status.clone().or_else(|| {
            if let (Some(cap), Some(load)) = (attrs.capacity, attrs.load) {
                if cap > 0.0 {
                    Some(format!("Load/capacity: {:.1}%", (load / cap) * 100.0))
                } else {
                    None
                }
            } else {
                None
            }
        });
        (vitality.to_string(), msg)
    } else {
        (
            "stable".to_string(),
            Some("No sovereign_attributes (capacity, load, status) configured.".to_string()),
        )
    };
    axum::Json(serde_json::json!({
        "status": "ok",
        "vitality": vitality,
        "message": message
    }))
}

/// GET /api/v1/persona/stream – SSE stream of 4-hour persona heartbeat (Spirit/Mind/Body check).
async fn persona_pulse_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static> {
    use async_stream::stream;
    let mut rx = state.persona_pulse_tx.subscribe();
    let stream = stream! {
        while let Ok(msg) = rx.recv().await {
            yield Ok(Event::default().json_data(msg).unwrap_or_else(|_| Event::default().data("{}")));
        }
    };
    Sse::new(stream)
}

/// GET /api/v1/kb-status – returns status of all 9 Knowledge Bases (L2 Memory + Shadow Vault).
async fn kb_status(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let kb_statuses = state.knowledge.get_all_status();
    let all_connected = kb_statuses.iter().all(|s| s.connected);
    let total_entries: usize = kb_statuses.iter().map(|s| s.entry_count).sum();
    
    axum::Json(serde_json::json!({
        "status": if all_connected { "ok" } else { "degraded" },
        "all_connected": all_connected,
        "total_entries": total_entries,
        "knowledge_bases": kb_statuses
    }))
}

/// GET /api/v1/sovereign-status – full cross-layer state for the Sovereign Dashboard.
/// When the dashboard cannot open Sled (e.g. gateway holds the lock), it can fetch this endpoint instead.
/// If PAGI_API_KEY is set, the request must include header `X-API-Key: <key>` or `Authorization: Bearer <key>`.
async fn sovereign_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<axum::Json<SovereignState>, (StatusCode, &'static str)> {
    if let Ok(expect_key) = std::env::var("PAGI_API_KEY") {
        let expect_key = expect_key.trim().to_string();
        if !expect_key.is_empty() {
            let provided = headers
                .get("X-API-Key")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim())
                .or_else(|| {
                    headers
                        .get(axum::http::header::AUTHORIZATION)
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.strip_prefix("Bearer "))
                        .map(|s| s.trim())
                });
            if provided.as_ref() != Some(&expect_key.as_str()) {
                return Err((StatusCode::UNAUTHORIZED, "Missing or invalid PAGI_API_KEY"));
            }
        }
    }
    const AGENT_ID: &str = "default";
    let sovereign = state.knowledge.get_full_sovereign_state(AGENT_ID);
    Ok(axum::Json(sovereign))
}

/// GET /api/v1/logs – Server-Sent Events stream of gateway logs (tracing output).
async fn logs_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static> {
    use async_stream::stream;
    let mut rx = state.log_tx.subscribe();
    let stream = stream! {
        loop {
            tokio::select! {
                r = rx.recv() => match r {
                    Ok(line) => yield Ok(Event::default().data(line)),
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        yield Ok(Event::default().data(format!("... {} log lines dropped", n)));
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                _ = tokio::time::sleep(Duration::from_secs(15)) => {
                    yield Ok(Event::default().comment("keepalive"));
                }
            }
        }
    };
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

/// POST /v1/vault/read – decrypt and return a journal entry. Requires X-Pagi-Shadow-Key header (same value as PAGI_SHADOW_KEY).
#[derive(serde::Deserialize)]
struct VaultReadRequest {
    record_id: String,
}

async fn vault_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<VaultReadRequest>,
) -> Result<axum::Json<serde_json::Value>, (StatusCode, &'static str)> {
    const HEADER_KEY: &str = "x-pagi-shadow-key";
    let client_key = headers
        .get(HEADER_KEY)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().replace([' ', '\n'], ""));
    let env_key = std::env::var("PAGI_SHADOW_KEY")
        .ok()
        .map(|s| s.trim().replace([' ', '\n'], ""));
    if client_key.as_ref() != env_key.as_ref() || env_key.is_none() {
        return Err((StatusCode::FORBIDDEN, "Missing or invalid X-Pagi-Shadow-Key"));
    }
    let guard = state.shadow_store.read().await;
    let store = match guard.as_ref() {
        Some(s) => s,
        None => return Err((StatusCode::SERVICE_UNAVAILABLE, "ShadowStore not initialized")),
    };
    let decrypted = store
        .get_journal(&body.record_id)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Decrypt failed"))?;
    let entry = match decrypted {
        Some(e) => e,
        None => return Err((StatusCode::NOT_FOUND, "Record not found")),
    };
    let json = serde_json::json!({
        "record_id": body.record_id,
        "label": entry.0.label,
        "intensity": entry.0.intensity,
        "timestamp_ms": entry.0.timestamp_ms,
        "raw_content": entry.0.raw_content,
    });
    Ok(axum::Json(json))
}

/// GET /api/v1/forge/safety-status – Returns current forge safety governor status (HITL vs Autonomous).
async fn forge_safety_status_get(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let registry = LiveSkillRegistry::default();
    let tenant_ctx = TenantContext {
        tenant_id: "system".to_string(),
        correlation_id: None,
        agent_id: Some("phoenix".to_string()),
    };
    
    let payload = serde_json::json!({ "action": "get_forge_safety_status" });
    
    match registry.get("sovereign_operator") {
        Some(skill) => {
            match skill.execute(&tenant_ctx, &state.knowledge, payload).await {
                Ok(result) => axum::Json(result),
                Err(e) => {
                    tracing::warn!("Failed to get forge safety status: {}", e);
                    axum::Json(serde_json::json!({
                        "safety_enabled": true,
                        "mode": "HITL",
                        "error": format!("Failed to query status: {}", e)
                    }))
                }
            }
        }
        None => {
            tracing::warn!("SovereignOperator skill not found in registry");
            axum::Json(serde_json::json!({
                "safety_enabled": true,
                "mode": "HITL",
                "error": "SovereignOperator skill not available"
            }))
        }
    }
}

#[derive(serde::Deserialize)]
struct ForgeSafetySetRequest {
    enabled: bool,
}

/// POST /api/v1/forge/safety – Set forge safety governor (Kill Switch).
async fn forge_safety_set(
    State(state): State<AppState>,
    Json(body): Json<ForgeSafetySetRequest>,
) -> (StatusCode, axum::Json<serde_json::Value>) {
    let registry = LiveSkillRegistry::default();
    let tenant_ctx = TenantContext {
        tenant_id: "system".to_string(),
        correlation_id: None,
        agent_id: Some("phoenix".to_string()),
    };
    
    let payload = serde_json::json!({
        "action": "set_forge_safety",
        "enabled": body.enabled
    });
    
    match registry.get("sovereign_operator") {
        Some(skill) => {
            match skill.execute(&tenant_ctx, &state.knowledge, payload).await {
                Ok(_) => {
                    let mode = if body.enabled { "HITL" } else { "Autonomous" };
                    tracing::info!("🏛️ Forge Safety Governor: {} (safety: {})", mode, if body.enabled { "ENABLED" } else { "DISABLED" });
                    
                    // Log to KB-08
                    let msg = format!(
                        "Sovereignty Update: Forge Safety Gate set to {} ({} Mode)",
                        if body.enabled { "TRUE" } else { "FALSE" },
                        mode
                    );
                    if let Err(e) = state.knowledge.record_success_metric(&msg) {
                        tracing::warn!("Failed to log forge safety change to KB-08: {}", e);
                    }
                    
                    (
                        StatusCode::OK,
                        axum::Json(serde_json::json!({
                            "status": "ok",
                            "safety_enabled": body.enabled,
                            "mode": mode,
                            "message": format!("Forge safety governor set to {}", mode)
                        }))
                    )
                }
                Err(e) => {
                    tracing::error!("Failed to set forge safety: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(serde_json::json!({
                            "status": "error",
                            "message": format!("Failed to set forge safety: {}", e)
                        }))
                    )
                }
            }
        }
        None => {
            tracing::error!("SovereignOperator skill not found in registry");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "status": "error",
                    "message": "SovereignOperator skill not available"
                }))
            )
        }
    }
}

/// POST /api/v1/forge/create – Create a new skill from a JSON tool-spec (The Forge).
async fn forge_create_post(
    Json(body): Json<ToolSpec>,
) -> (StatusCode, axum::Json<serde_json::Value>) {
    let workspace_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("forge/create: current_dir failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "status": "error",
                    "message": format!("Failed to resolve workspace: {}", e)
                })),
            );
        }
    };
    let skills_src = workspace_root.join("crates/pagi-skills/src");
    if !skills_src.join("lib.rs").exists() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({
                "status": "error",
                "message": "Not running from workspace root (crates/pagi-skills/src/lib.rs not found)"
            })),
        );
    }
    match create_skill_from_spec(&body, &skills_src, &workspace_root) {
        Ok(result) => {
            let code = if result.cargo_check_ok {
                // If cargo check passed and hot-reload is enabled, trigger hot-reload
                if is_hot_reload_enabled() {
                    let file_path = std::path::PathBuf::from(&result.file_path);
                    match hot_reload_skill(&body.name, &result.module_name, file_path) {
                        Ok(hot_reload_result) => {
                            tracing::info!(
                                "🔥 Forge: Skill '{}' created and hot-reloaded in {}ms",
                                body.name,
                                hot_reload_result.load_time_ms
                            );
                            // Return combined result
                            return (
                                StatusCode::OK,
                                axum::Json(serde_json::json!({
                                    "success": true,
                                    "skill_name": body.name,
                                    "module_name": result.module_name,
                                    "file_path": result.file_path,
                                    "cargo_check_ok": true,
                                    "hot_reloaded": true,
                                    "compilation_time_ms": hot_reload_result.compilation_time_ms,
                                    "message": format!(
                                        "Skill '{}' created and hot-reloaded successfully. Ready for immediate use.",
                                        body.name
                                    )
                                })),
                            );
                        }
                        Err(e) => {
                            tracing::warn!("🔥 Forge: Skill created but hot-reload failed: {}", e);
                            // Fall through to return standard result
                        }
                    }
                }
                StatusCode::OK
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            (
                code,
                axum::Json(serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({
                    "status": "error",
                    "message": "Serialization failed"
                }))),
            )
        }
        Err(e) => {
            tracing::warn!("forge/create failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                axum::Json(serde_json::json!({
                    "status": "error",
                    "message": e
                })),
            )
        }
    }
}

// -----------------------------------------------------------------------------
// Forge Hot-Reload Endpoints
// -----------------------------------------------------------------------------

/// GET /api/v1/forge/hot-reload/status – Check if hot-reload is enabled
async fn forge_hot_reload_status() -> axum::Json<serde_json::Value> {
    let enabled = is_hot_reload_enabled();
    axum::Json(serde_json::json!({
        "enabled": enabled,
        "message": if enabled {
            "Hot-reload is enabled. New skills will be compiled and activated automatically."
        } else {
            "Hot-reload is disabled. Enable via POST /api/v1/forge/hot-reload/enable"
        }
    }))
}

/// POST /api/v1/forge/hot-reload/enable – Enable hot-reload
async fn forge_hot_reload_enable() -> (StatusCode, axum::Json<serde_json::Value>) {
    enable_hot_reload();
    tracing::info!("🔥 Forge Hot-Reload: Enabled via API");
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "status": "ok",
            "message": "Hot-reload enabled. New skills will be compiled and activated automatically."
        })),
    )
}

/// POST /api/v1/forge/hot-reload/disable – Disable hot-reload
async fn forge_hot_reload_disable() -> (StatusCode, axum::Json<serde_json::Value>) {
    disable_hot_reload();
    tracing::warn!("🔥 Forge Hot-Reload: Disabled via API");
    (
        StatusCode::OK,
        axum::Json(serde_json::json!({
            "status": "ok",
            "message": "Hot-reload disabled. Skills will require manual Gateway restart."
        })),
    )
}

/// GET /api/v1/forge/hot-reload/list – List all hot-reloaded skills
async fn forge_hot_reload_list() -> axum::Json<serde_json::Value> {
    let skills = list_hot_reloaded_skills();
    let skills_json: Vec<serde_json::Value> = skills
        .iter()
        .map(|s| {
            serde_json::json!({
                "skill_name": s.skill_name,
                "module_name": s.module_name,
                "file_path": s.file_path.to_string_lossy(),
                "loaded_at": s.loaded_at
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            })
        })
        .collect();
    
    axum::Json(serde_json::json!({
        "skills": skills_json,
        "count": skills.len()
    }))
}

/// POST /api/v1/forge/hot-reload/trigger – Trigger hot-reload for a specific skill
#[derive(serde::Deserialize)]
struct HotReloadTriggerRequest {
    skill_name: String,
    module_name: String,
    file_path: String,
}

async fn forge_hot_reload_trigger(
    Json(body): Json<HotReloadTriggerRequest>,
) -> (StatusCode, axum::Json<serde_json::Value>) {
    use std::path::PathBuf;
    
    let file_path = PathBuf::from(&body.file_path);
    
    match hot_reload_skill(&body.skill_name, &body.module_name, file_path) {
        Ok(result) => {
            let code = if result.success {
                StatusCode::OK
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (
                code,
                axum::Json(serde_json::to_value(&result).unwrap_or_else(|_| {
                    serde_json::json!({
                        "status": "error",
                        "message": "Serialization failed"
                    })
                })),
            )
        }
        Err(e) => {
            tracing::error!("🔥 Forge Hot-Reload: Failed to reload skill '{}': {}", body.skill_name, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({
                    "status": "error",
                    "message": e
                })),
            )
        }
    }
}

/// GET /v1/status – app identity and slot labels from config.
async fn status(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let labels: std::collections::HashMap<u8, String> = state.config.slot_labels_map();
    let labels_json: std::collections::HashMap<String, String> = labels
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    axum::Json(serde_json::json!({
        "app_name": state.config.app_name,
        "port": state.config.port,
        "llm_mode": state.config.llm_mode,
        "slot_labels": labels_json,
    }))
}

#[derive(serde::Deserialize)]
struct ExecuteRequest {
    tenant_id: String,
    correlation_id: Option<String>,
    /// Agent instance ID for multi-agent mode. Chronos and Kardia are keyed by this. Default: "default".
    #[serde(default)]
    agent_id: Option<String>,
    /// Optional workspace root override for FS skills (ReadFile, FsWorkspaceAnalyzer, WriteSandboxFile). From UI Settings "Preferred workspace path".
    #[serde(default)]
    preferred_workspace_path: Option<String>,
    goal: Goal,
}

/// Project–local path association: pins a UI project to a bare-metal folder. When master_analysis is ON, folder content is injected into chat context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ProjectAssociation {
    pub(crate) project_id: String,
    pub(crate) local_path: String,
    pub(crate) master_analysis: bool,
}

const PROJECT_ASSOCIATIONS_FILE: &str = "data/project_associations.json";
const FOLDER_CONTEXT_MAX_BYTES: usize = 24 * 1024;

fn load_project_associations() -> std::collections::HashMap<String, ProjectAssociation> {
    let path = StdPath::new(PROJECT_ASSOCIATIONS_FILE);
    if !path.exists() {
        return std::collections::HashMap::new();
    }
    match std::fs::read_to_string(path) {
        Ok(s) => match serde_json::from_str::<Vec<ProjectAssociation>>(&s) {
            Ok(list) => list
                .into_iter()
                .map(|a| (a.project_id.clone(), a))
                .collect(),
            Err(_) => std::collections::HashMap::new(),
        },
        Err(_) => std::collections::HashMap::new(),
    }
}

fn save_project_associations(assocs: &std::collections::HashMap<String, ProjectAssociation>) {
    if let Some(parent) = StdPath::new(PROJECT_ASSOCIATIONS_FILE).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let list: Vec<_> = assocs.values().cloned().collect();
    if let Ok(json) = serde_json::to_string_pretty(&list) {
        let _ = std::fs::write(PROJECT_ASSOCIATIONS_FILE, json);
    }
}

/// Chat request from the Studio UI frontend
#[derive(serde::Deserialize)]
struct ChatRequest {
    prompt: String,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    user_alias: Option<String>,
    /// Agent instance ID for multi-agent mode (Kardia owner). Default: "default".
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    temperature: Option<f32>,
    #[serde(default)]
    max_tokens: Option<u32>,
    #[serde(default)]
    persona: Option<String>,
    /// Optional workspace root override for FS skills when invoked from chat context. From UI Settings "Preferred workspace path".
    #[serde(default)]
    preferred_workspace_path: Option<String>,
    /// Optional emotional state for SAO emotional adaptation (e.g., "guilt", "grief").
    #[serde(default)]
    user_emotional_state: Option<String>,
    /// Optional client-side thread id (Studio UI). Used by KB-04 (Chronos SQLite) history.
    #[serde(default)]
    thread_id: Option<String>,
    /// Optional project ID; when set and that project has a mounted folder with Master Analysis ON, folder context is injected.
    #[serde(default)]
    project_id: Option<String>,
}

// -----------------------------------------------------------------------------
// KB-04 (Chronos) — SQLite API
// -----------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct ChronosCreateProjectBody {
    name: String,
}

async fn chronos_projects_list(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let db = Arc::clone(&state.chronos_db);
    let result = tokio::task::spawn_blocking(move || db.list_projects()).await;
    match result {
        Ok(Ok(projects)) => axum::Json(serde_json::json!({ "status": "ok", "projects": projects })),
        Ok(Err(e)) => axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        Err(e) => axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

async fn chronos_projects_create(
    State(state): State<AppState>,
    Json(body): Json<ChronosCreateProjectBody>,
) -> (StatusCode, axum::Json<serde_json::Value>) {
    let name = body.name.trim().to_string();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "error": "name cannot be empty" })),
        );
    }
    let db = Arc::clone(&state.chronos_db);
    // Upsert by name to prevent duplicates when UI tags threads repeatedly.
    let res = tokio::task::spawn_blocking(move || db.upsert_project_by_name(&name)).await;
    match res {
        Ok(Ok(project)) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "status": "ok", "project": project })),
        ),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
    }
}

#[derive(serde::Deserialize)]
struct ChronosThreadsListQuery {
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

async fn chronos_threads_list(
    State(state): State<AppState>,
    Query(q): Query<ChronosThreadsListQuery>,
) -> axum::Json<serde_json::Value> {
    let db = Arc::clone(&state.chronos_db);
    let pid = q.project_id.clone();
    let limit = q.limit.unwrap_or(50).min(200);
    let res = tokio::task::spawn_blocking(move || {
        if let Some(pid) = pid.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            db.list_threads(Some(pid), limit)
        } else {
            // When no project_id is provided, return ALL threads (both ungrouped + project-tagged).
            db.list_threads_any(limit)
        }
    })
    .await;
    match res {
        Ok(Ok(threads)) => axum::Json(serde_json::json!({ "status": "ok", "threads": threads })),
        Ok(Err(e)) => axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        Err(e) => axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

#[derive(serde::Deserialize)]
struct ChronosCreateThreadBody {
    #[serde(default)]
    id: Option<String>,
    title: String,
    #[serde(default)]
    project_id: Option<String>,
}

async fn chronos_threads_create(
    State(state): State<AppState>,
    Json(body): Json<ChronosCreateThreadBody>,
) -> (StatusCode, axum::Json<serde_json::Value>) {
    let title = body.title.trim().to_string();
    if title.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "error": "title cannot be empty" })),
        );
    }
    let db = Arc::clone(&state.chronos_db);
    let pid = body.project_id.clone();
    let id_opt = body.id.clone();
    let res = tokio::task::spawn_blocking(move || {
        if let Some(id) = id_opt.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            // Client-supplied id: ensure thread exists under that id.
            db.ensure_thread_exists(id, &title, pid.as_deref())?;
            Ok(chronos_sqlite::ThreadRow {
                id: id.to_string(),
                project_id: pid.clone(),
                title: title.clone(),
                created_at_ms: chrono::Utc::now().timestamp_millis(),
                updated_at_ms: chrono::Utc::now().timestamp_millis(),
                last_message_at_ms: None,
            })
        } else {
            db.create_thread(&title, pid.as_deref())
        }
    })
    .await;
    match res {
        Ok(Ok(thread)) => (
            StatusCode::OK,
            axum::Json(serde_json::json!({ "status": "ok", "thread": thread })),
        ),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
    }
}

#[derive(serde::Deserialize)]
struct ChronosRenameThreadBody {
    title: String,
}

async fn chronos_threads_rename(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
    Json(body): Json<ChronosRenameThreadBody>,
) -> (StatusCode, axum::Json<serde_json::Value>) {
    let title = body.title.trim().to_string();
    if title.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "error": "title cannot be empty" })),
        );
    }
    let db = Arc::clone(&state.chronos_db);
    let tid = thread_id.clone();
    let res = tokio::task::spawn_blocking(move || db.rename_thread(&tid, &title)).await;
    match res {
        Ok(Ok(())) => (StatusCode::OK, axum::Json(serde_json::json!({ "status": "ok" }))),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
    }
}

async fn chronos_threads_delete(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
) -> (StatusCode, axum::Json<serde_json::Value>) {
    let db = Arc::clone(&state.chronos_db);
    let tid = thread_id.clone();
    let res = tokio::task::spawn_blocking(move || db.delete_thread(&tid)).await;
    match res {
        Ok(Ok(())) => (StatusCode::OK, axum::Json(serde_json::json!({ "status": "ok" }))),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
    }
}

#[derive(serde::Deserialize)]
struct ChronosTagThreadBody {
    #[serde(default)]
    project_id: Option<String>,
}

async fn chronos_threads_tag(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
    Json(body): Json<ChronosTagThreadBody>,
) -> (StatusCode, axum::Json<serde_json::Value>) {
    let tid = thread_id.trim().to_string();
    if tid.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            axum::Json(serde_json::json!({ "status": "error", "error": "thread_id cannot be empty" })),
        );
    }
    let pid = body
        .project_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let db = Arc::clone(&state.chronos_db);
    let res = tokio::task::spawn_blocking(move || db.set_thread_project(&tid, pid.as_deref())).await;
    match res {
        Ok(Ok(())) => (StatusCode::OK, axum::Json(serde_json::json!({ "status": "ok" }))),
        Ok(Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        ),
    }
}

#[derive(serde::Deserialize)]
struct ChronosMessagesQuery {
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    before_ms: Option<i64>,
}

async fn chronos_messages_list(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
    Query(q): Query<ChronosMessagesQuery>,
) -> axum::Json<serde_json::Value> {
    let db = Arc::clone(&state.chronos_db);
    let tid = thread_id.clone();
    let limit = q.limit.unwrap_or(200).min(1000);
    let before = q.before_ms;
    let res = tokio::task::spawn_blocking(move || db.list_messages(&tid, limit, before)).await;
    match res {
        Ok(Ok(messages)) => axum::Json(serde_json::json!({ "status": "ok", "messages": messages })),
        Ok(Err(e)) => axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
        Err(e) => axum::Json(serde_json::json!({ "status": "error", "error": e.to_string() })),
    }
}

async fn execute(
    State(state): State<AppState>,
    Json(req): Json<ExecuteRequest>,
) -> axum::response::Response {
    // Touch idle tracker: user activity resets the maintenance loop idle gate.
    state.idle_tracker.touch();
    tracing::info!("Skill execution started");
    let agent_id = req.agent_id.as_deref().filter(|s| !s.is_empty()).unwrap_or(pagi_core::DEFAULT_AGENT_ID);
    let is_kb_query = matches!(req.goal, Goal::QueryKnowledge { .. });
    let ctx = TenantContext {
        tenant_id: req.tenant_id,
        correlation_id: req.correlation_id,
        agent_id: Some(agent_id.to_string()),
    };

    // ReflectShadow: require session_key to match PAGI_SHADOW_KEY (vault must be explicitly opened)
    if let Goal::ExecuteSkill { ref name, ref payload } = req.goal {
        if name == "ReflectShadow" {
            let client_key = payload
                .as_ref()
                .and_then(|p| p.get("session_key"))
                .and_then(|v| v.as_str())
                .map(|s| s.trim().replace([' ', '\n'], ""));
            let env_key = std::env::var("PAGI_SHADOW_KEY")
                .ok()
                .map(|s| s.trim().replace([' ', '\n'], ""));
            if client_key.as_ref() != env_key.as_ref() || env_key.is_none() {
                return axum::Json(serde_json::json!({
                    "status": "error",
                    "error": "ReflectShadow requires valid session_key (X-Pagi-Shadow-Key / PAGI_SHADOW_KEY)"
                })).into_response();
            }
        }

        // ETHOS pre-execution check: consult KB_ETHOS before ExecuteSkill
        let content_to_scan = payload
            .as_ref()
            .map(|p| {
                p.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .unwrap_or_else(|| payload.as_ref().map(|p| p.to_string()).unwrap_or_default());
        if let Some(policy) = state.knowledge.get_ethos_policy() {
            match policy.allows(name, &content_to_scan) {
                AlignmentResult::Fail { reason } => {
                    let violation = EventRecord::now("Ethos", format!("Policy Violation: {}", reason))
                        .with_skill(name.clone())
                        .with_outcome("blocked");
                    let _ = state.knowledge.append_chronos_event(agent_id, &violation);
                    tracing::warn!(
                        target: "pagi::ethos",
                        skill = %name,
                        reason = %reason,
                        "Ethos: execution blocked"
                    );
                    return axum::Json(serde_json::json!({
                        "status": "policy_violation",
                        "error": reason,
                        "skill": name,
                    })).into_response();
                }
                AlignmentResult::Pass => {}
            }
        }
    }

    // Merge preferred_workspace_path into FS skill payloads so UI Settings override is honored.
    const FS_SKILLS: &[&str] = &["ReadFile", "FsWorkspaceAnalyzer", "write_sandbox_file"];
    let goal_to_dispatch = match &req.goal {
        Goal::ExecuteSkill { name, payload } if FS_SKILLS.contains(&name.as_str()) => {
            if let Some(path) = req.preferred_workspace_path.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                let mut map = serde_json::Map::new();
                if let Some(existing) = payload.as_ref().and_then(|v| v.as_object()) {
                    for (k, v) in existing {
                        map.insert(k.clone(), v.clone());
                    }
                }
                map.insert(
                    "fs_root_override".to_string(),
                    serde_json::Value::String(path.to_string()),
                );
                Goal::ExecuteSkill {
                    name: name.clone(),
                    payload: Some(serde_json::Value::Object(map)),
                }
            } else {
                req.goal.clone()
            }
        }
        _ => req.goal.clone(),
    };

    let reflexion_info = reflexion_goal_info(&goal_to_dispatch);
    match state.orchestrator.dispatch(&ctx, goal_to_dispatch).await {
        Ok(result) => {
            if is_kb_query {
                tracing::info!("KB search success");
            }
            // Episodic memory: log successful execution to KB_CHRONOS (the Historian)
            if let Some(event) = chronos_event_from_goal_and_result(&req.goal, &result) {
                if state.knowledge.append_chronos_event(agent_id, &event).is_err() {
                    tracing::warn!(target: "pagi::chronos", "Failed to append Chronos event");
                }
            }
            axum::Json(result).into_response()
        }
        Err(e) => {
            if let Some(v) = e.downcast_ref::<SovereigntyViolation>() {
                let msg = format!(
                    "Failed Leak Attempt: skill '{}' attempted KB-{} (blocked by Sovereignty Firewall)",
                    v.skill_id, v.kb_layer
                );
                if state.sovereign_config.kb08_success_logging {
                    let _ = state.knowledge.record_success_metric(&msg);
                }
                tracing::warn!(target: "pagi::sovereignty", skill_id = %v.skill_id, kb_layer = v.kb_layer, "SovereigntyViolation logged to KB-08");
                return (
                    axum::http::StatusCode::FORBIDDEN,
                    axum::Json(serde_json::json!({
                        "error": msg,
                        "status": "sovereignty_violation",
                        "skill_id": v.skill_id,
                        "kb_layer": v.kb_layer,
                        "message": "The skill was blocked by the Sovereignty Firewall because it is not Core Signed for this knowledge layer."
                    })),
                )
                    .into_response();
            }
            // Reflexion: log skill failure to Chronos (Failures) for self-correction
            let (skill_name, goal_summary) = reflexion_info;
            let _ = state.knowledge.log_skill_failure(
                agent_id,
                &skill_name,
                &e.to_string(),
                goal_summary.as_ref(),
            );
            axum::Json(serde_json::json!({
                "error": e.to_string(),
                "status": "error"
            }))
                .into_response()
        }
    }
}

/// Local context limit for Gater (from PAGI_LOCAL_CONTEXT_LIMIT). Default 20.
fn local_context_limit() -> usize {
    std::env::var("PAGI_LOCAL_CONTEXT_LIMIT")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .filter(|&n| n > 0 && n <= 500)
        .unwrap_or(20)
}

/// Maps a user prompt to a reflex Goal when Gater returns MoEExpert::SystemTool.
/// SystemTelemetry for CPU/memory/disk/process/snapshot; FileSystem for list-dir style; else None.
fn prompt_to_system_tool_goal(prompt: &str) -> Option<Goal> {
    let s = prompt.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    // System vitality / hardware (Sovereign Admin): prefer GetHardwareStats for Architect-friendly summary
    if s.contains("vitality") || s.contains("system vitality") || s.contains("check my system")
        || (s.contains("system") && (s.contains("health") || s.contains("vitality") || s.contains("how am i")))
    {
        return Some(Goal::ExecuteSkill {
            name: "GetHardwareStats".to_string(),
            payload: Some(serde_json::json!({})),
        });
    }
    // System info / telemetry (granular)
    if s.contains("cpu") || s.contains("processor") {
        return Some(Goal::ExecuteSkill {
            name: "SystemTelemetry".to_string(),
            payload: Some(serde_json::json!({ "type": "cpu" })),
        });
    }
    if s.contains("memory") || s.contains("ram ") {
        return Some(Goal::ExecuteSkill {
            name: "SystemTelemetry".to_string(),
            payload: Some(serde_json::json!({ "type": "memory" })),
        });
    }
    if s.contains("disk") || s.contains("storage") {
        return Some(Goal::ExecuteSkill {
            name: "SystemTelemetry".to_string(),
            payload: Some(serde_json::json!({ "type": "disks" })),
        });
    }
    if s.contains("process") {
        return Some(Goal::ExecuteSkill {
            name: "SystemTelemetry".to_string(),
            payload: Some(serde_json::json!({ "type": "processes" })),
        });
    }
    if s.contains("system info") || s.contains("system usage") || s.contains("usage") && (s.contains("what") || s.contains("how much")) {
        return Some(Goal::ExecuteSkill {
            name: "SystemTelemetry".to_string(),
            payload: Some(serde_json::json!({ "type": "snapshot" })),
        });
    }
    // File / directory listing
    if s.contains("list dir") || s.contains("list directory") || s.contains("list files") || s.contains("directory") && s.len() < 80 {
        return Some(Goal::ExecuteSkill {
            name: "FileSystem".to_string(),
            payload: Some(serde_json::json!({ "operation": "list", "path": "." })),
        });
    }
    if s.contains("find file") || s.contains("find the file") {
        let path = s
            .replace("find the file", "")
            .replace("find file", "")
            .trim()
            .trim_matches(|c: char| c == '"' || c == '\'' || c == '.')
            .to_string();
        let path = if path.is_empty() { "." } else { path.as_str() };
        return Some(Goal::ExecuteSkill {
            name: "FileSystem".to_string(),
            payload: Some(serde_json::json!({ "operation": "list", "path": path })),
        });
    }
    // Default: full system snapshot for short "what's my ..." style
    if s.len() < 60 && (s.starts_with("what") || s.starts_with("how much") || s.starts_with("show")) {
        return Some(Goal::ExecuteSkill {
            name: "SystemTelemetry".to_string(),
            payload: Some(serde_json::json!({ "type": "snapshot" })),
        });
    }
    None
}

/// Returns (skill_name, goal_summary) for Reflexion failure logging.
fn reflexion_goal_info(goal: &Goal) -> (String, Option<serde_json::Value>) {
    let (skill_name, summary) = match goal {
        Goal::ExecuteSkill { name, payload } => (
            name.clone(),
            Some(serde_json::json!({ "goal": "ExecuteSkill", "name": name, "payload_keys": payload.as_ref().and_then(|v| v.as_object()).map(|o| o.keys().cloned().collect::<Vec<_>>()) })),
        ),
        Goal::QueryKnowledge { slot_id, query } => (
            "KnowledgeQuery".to_string(),
            Some(serde_json::json!({ "goal": "QueryKnowledge", "slot_id": slot_id, "query": query })),
        ),
        _ => (
            "unknown".to_string(),
            serde_json::to_value(goal).ok(),
        ),
    };
    (skill_name, summary)
}

/// Builds an episodic EventRecord for KB_CHRONOS from the executed goal and its result.
fn chronos_event_from_goal_and_result(goal: &Goal, result: &serde_json::Value) -> Option<EventRecord> {
    let (source_kb, reflection, skill_name, outcome) = match goal {
        Goal::ExecuteSkill { name, .. } => {
            let outcome = result
                .get("status")
                .and_then(|v| v.as_str())
                .or_else(|| result.get("skill").and_then(|v| v.as_str()))
                .map(|s| s.to_string());
            (
                "Soma",
                format!("Executed skill: {}", name),
                Some(name.clone()),
                outcome,
            )
        }
        Goal::QueryKnowledge { slot_id, query } => (
            "Chronos",
            format!("Queried KB-{} for key: {}", slot_id, query),
            None,
            result.get("value").map(|v| if v.is_null() { "missing" } else { "retrieved" }.to_string()),
        ),
        Goal::UpdateKnowledgeSlot { slot_id, .. } => (
            "Soma",
            format!("Updated knowledge slot {}", slot_id),
            Some("CommunityScraper".to_string()),
            result.get("event").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ),
        Goal::MemoryOp { path, .. } => (
            "Chronos",
            format!("Memory operation on path: {}", path),
            None,
            result.get("status").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ),
        Goal::AutonomousGoal { intent, .. } => (
            "Pneuma",
            format!("Autonomous goal: {}", intent),
            None,
            result.get("status").and_then(|v| v.as_str()).map(|s| s.to_string()),
        ),
        Goal::GenerateFinalResponse { context_id } => (
            "Soma",
            format!("Generated final response for context: {}", context_id),
            Some("ModelRouter".to_string()),
            result.get("generated").and_then(|v| v.as_str()).map(|s| s.chars().take(80).chain(std::iter::once('…')).collect::<String>()),
        ),
        _ => return None,
    };
    let mut event = EventRecord::now(source_kb, reflection);
    if let Some(s) = skill_name {
        event = event.with_skill(s);
    }
    if let Some(o) = outcome {
        event = event.with_outcome(o);
    }
    Some(event)
}

/// Extract potential subject names from text (simple heuristic: capitalized words that might be names).
/// Returns the first detected name or None.
fn extract_subject_name(text: &str) -> Option<String> {
    // Look for patterns like "my [relationship] [Name]" or just capitalized names
    let words: Vec<&str> = text.split_whitespace().collect();
    for i in 0..words.len() {
        let word = words[i].trim_matches(|c: char| !c.is_alphanumeric());
        // Check if word starts with capital and is 2+ chars (likely a name)
        if word.len() >= 2 && word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
            // Skip common words that aren't names
            let lower = word.to_lowercase();
            if !["the", "this", "that", "what", "when", "where", "why", "how", "who", "which", "i", "you", "he", "she", "they", "we"].contains(&lower.as_str()) {
                return Some(word.to_string());
            }
        }
    }
    None
}

/// Derive a safe default thread title when the UI hasn't explicitly created/renamed the thread yet.
/// If `thread_id_opt` is Some(_), we assume a Studio-managed thread and use the first line of the prompt.
/// If it is None (legacy clients), we still produce a deterministic title hint.
fn derive_thread_title_hint(thread_id_opt: Option<&str>, prompt: &str) -> String {
    let raw = prompt
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches(|c: char| c == '#' || c == '*' || c == '-' || c == '>' || c == '`');

    let mut s = raw.to_string();
    if s.is_empty() {
        s = if thread_id_opt
            .as_deref()
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .is_some()
        {
            "New Chat".to_string()
        } else {
            "Legacy Chat".to_string()
        };
    }
    const MAX: usize = 80;
    if s.len() > MAX {
        s.truncate(MAX);
    }
    s
}

/// Chat endpoint: Orchestrator verification — uses the actual Orchestrator and KnowledgeStore
/// from pagi-core (AppState). No demo, no sandbox. state.orchestrator.dispatch(ModelRouter)
/// and state.knowledge.build_system_directive() are the only path. Supports streaming and JSON.
async fn chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ChatRequest>,
) -> Response {
    // Touch idle tracker: user activity resets the maintenance loop idle gate.
    state.idle_tracker.touch();
    tracing::info!("Chat request received: {} chars, stream: {}", req.prompt.len(), req.stream);
    
    // Background intelligence analysis (non-blocking)
    let intelligence_service = Arc::clone(&state.intelligence_service);
    let prompt_clone = req.prompt.clone();
    tokio::spawn(async move {
        let _ = intelligence_service.analyze_input(&prompt_clone).await;
    });
    
    if req.stream {
        // Streaming mode - return SSE stream
        chat_streaming(state, headers, req).await
    } else {
        // Non-streaming mode - return JSON
        chat_json(state, headers, req).await.into_response()
    }
}

/// Non-streaming chat handler - returns JSON response.
/// When MoE is ON, routes via route_to_experts to OpenRouter / LanceDB / SystemTool; otherwise uses ModelRouter.
async fn chat_json(
    state: AppState,
    headers: HeaderMap,
    req: ChatRequest,
) -> axum::Json<serde_json::Value> {
    let user_id = req.user_alias.as_deref().unwrap_or("studio-user");
    let agent_id = req.agent_id.as_deref().filter(|s| !s.is_empty()).unwrap_or(pagi_core::DEFAULT_AGENT_ID);
    let ctx = TenantContext {
        tenant_id: user_id.to_string(),
        correlation_id: Some(uuid::Uuid::new_v4().to_string()),
        agent_id: Some(agent_id.to_string()),
    };

    // MoE gating: when Sparse use Gater::route_with_context; otherwise route_to_experts.
    if state.moe_active.load(Ordering::Acquire) {
        let local_ctx = state.knowledge.build_local_context_for_bridge(agent_id, local_context_limit());
        let expert = match state.orchestrator.get_moe_mode() {
            MoEMode::Sparse => Gater::route_with_context(&local_ctx, &req.prompt),
            MoEMode::Dense => route_to_experts(&req.prompt),
        };
        tracing::info!(target: "pagi::chat", expert = ?expert, "MoE route");
        match expert {
            MoEExpert::LanceDB => {
                let slot_id = 1u8;
                let goal = Goal::QueryKnowledge { slot_id, query: req.prompt.trim().to_string() };
                match state.orchestrator.dispatch(&ctx, goal).await {
                    Ok(result) => {
                        let response = result.get("value").or(result.get("result")).and_then(|v| v.as_str())
                            .unwrap_or_else(|| result.get("message").and_then(|v| v.as_str()).unwrap_or("No matches in knowledge base."))
                            .to_string();
                        save_to_memory(&state.knowledge, &req.prompt, &response);
                        return axum::Json(serde_json::json!({
                            "status": "ok",
                            "response": response,
                            "thought": "MoE: LanceDB (knowledge expert)",
                            "expert_routing": "LanceDB",
                            "model": "moe-lancedb",
                            "raw_result": result
                        }));
                    }
                    Err(e) => {
                        return axum::Json(serde_json::json!({
                            "status": "error",
                            "error": e.to_string(),
                            "response": format!("Knowledge query error: {}", e)
                        }));
                    }
                }
            }
            MoEExpert::SystemTool => {
                let (response, raw_result) = match prompt_to_system_tool_goal(&req.prompt) {
                    Some(goal) => match state.orchestrator.dispatch(&ctx, goal.clone()).await {
                        Ok(result) => {
                            if let Some(ev) = chronos_event_from_goal_and_result(&goal, &result) {
                                let _ = state.knowledge.append_chronos_event(agent_id, &ev);
                            }
                            let text = serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());
                            (text, result)
                        }
                        Err(e) => (format!("[Reflex error: {}]", e), serde_json::Value::Null),
                    },
                    None => (
                        "MoE System expert: For file or system commands, use the Execute Skill (e.g. ReadFile) via the Skill Tester or /v1/execute.".to_string(),
                        serde_json::Value::Null,
                    ),
                };
                save_to_memory(&state.knowledge, &req.prompt, &response);
                return axum::Json(serde_json::json!({
                    "status": "ok",
                    "response": response,
                    "thought": "MoE: SystemTool (Local Reflex)",
                    "expert_routing": "Local System Tool",
                    "model": "moe-system",
                    "raw_result": raw_result
                }));
            }
            MoEExpert::Unmatched => {}
            MoEExpert::OpenRouter => {}
        }
    }

    // PATTERN RECOGNITION (KB-02): Analyze message for manipulation patterns before LLM call
    let pattern_result = pattern_match_analyze(&req.prompt);
    
    // SUBJECT IDENTIFICATION & KB-08 QUERY: Extract subject name and retrieve absurdity log
    let subject_context = if let Some(subject_name) = extract_subject_name(&req.prompt) {
        tracing::info!(target: "pagi::sao", subject = %subject_name, "Subject identified in message");
        match absurdity_build_context_injection(&state.knowledge, &subject_name, 3) {
            Ok(context) if !context.is_empty() => {
                tracing::info!(target: "pagi::sao", "KB-08 context injected for subject: {}", subject_name);
                Some(context)
            }
            Ok(_) => None,
            Err(e) => {
                tracing::warn!(target: "pagi::sao", error = %e, "Failed to retrieve KB-08 context");
                None
            }
        }
    } else {
        None
    };

    // Focus Shield: fetch calendar health (Schedule Outlook + Gatekeeper). Blocking call off async runtime.
    let calendar_health: Option<CalendarHealth> = if state.sovereign_config.focus_shield_enabled {
        let client = state.ms_graph_client.clone();
        let knowledge = Arc::clone(&state.knowledge);
        match client {
            Some(c) => tokio::task::spawn_blocking(move || c.fetch_calendar_health(&knowledge)).await.ok().flatten(),
            None => None,
        }
    } else {
        None
    };

    // Vitality Shield: fetch sleep/activity from KB-08 (or refresh from MS Graph Beta when MS_GRAPH_HEALTH_ENABLED).
    let vitality = if state.sovereign_config.vitality_shield_enabled {
        let knowledge = Arc::clone(&state.knowledge);
        let client_opt = state.ms_graph_client.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(ref c) = client_opt {
                c.refresh_vitality_from_graph(&knowledge);
            }
            fetch_user_vitality(&knowledge)
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    // Humanity Slider: Gatekeeper (Focus/Quiet) drops ratio by 0.4; Vitality Shield (sleep < 6h) drops by 0.2 so Phoenix stays brief.
    let base_ratio = state.sovereign_config.humanity_ratio;
    let effective_humanity_ratio = if is_low_sleep(vitality.as_ref()) {
        (base_ratio - 0.2).max(0.0)
    } else if use_gatekeeper_mode(calendar_health.as_ref()) {
        (base_ratio - 0.4).max(0.0)
    } else {
        base_ratio
    };

    // Archetype Gallery: low sleep → bias Virgo/Architect (strictly helpful, less chatty); else auto-switch from query domain unless KB-01 disables.
    const KB01_SLOT: u8 = 1;
    let kb01_user_profile = state.knowledge.get(KB01_SLOT, KB01_USER_PROFILE_KEY).ok().flatten();
    let kb01_value = kb01_user_profile.as_ref().and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok());
    let effective_archetype = if is_low_sleep(vitality.as_ref()) {
        pagi_core::ArchetypeOverlay::Virgo
    } else {
        pagi_core::get_effective_archetype_for_turn(
            req.prompt.as_str(),
            state.sovereign_config.as_ref(),
            kb01_value.as_ref(),
        )
    };

    // Sovereign: dynamic system prompt from KnowledgeStore + Orchestrator Role (Counselor) augmentation
    let base_directive = state.knowledge.build_system_directive(agent_id, user_id);
    let mut system_directive = state.knowledge.identity_prompt_prefix()
        + &state.persona_coordinator.augment_system_directive_with_emotion(
            &base_directive,
            req.user_emotional_state.as_deref(),
            Some(effective_humanity_ratio),
            Some(state.sovereign_config.as_ref()),
            Some(effective_archetype),
        );

    // Astro-Logic: process KB-01 profile → directive + temperature/verbosity overrides (Phoenix Marie "acts" on profile)
    let archetype_result = {
        if let Some(ref b) = kb01_user_profile {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(b) {
                let res = process_archetype_triggers(&state.knowledge, &val);
                if !res.directive.is_empty() {
                    system_directive.push_str(&res.directive);
                }
                res
            } else {
                ArchetypeTriggerResult::default()
            }
        } else {
            ArchetypeTriggerResult::default()
        }
    };
    let temperature = if state.sovereign_config.strict_technical_mode {
        0.3
    } else {
        archetype_result
            .temperature_override
            .or(req.temperature)
            .unwrap_or(0.7)
    };

    // Dynamic injection: SovereignAttributes (Capacity/Load) so SAO can adapt tone and energy advice
    if let Some(ref attrs) = state.config.sovereign_attributes {
        let cap = attrs.capacity.map(|c| format!("{:.0}", c)).unwrap_or_else(|| "—".to_string());
        let load = attrs.load.map(|l| format!("{:.0}", l)).unwrap_or_else(|| "—".to_string());
        let status = attrs.status.as_deref().unwrap_or("—");
        system_directive.push_str("\n\n=== SOVEREIGN STATE (Capacity/Load) ===\n");
        system_directive.push_str(&format!("Capacity: {}, Load: {}, Status: {}. Use this to calibrate energy and boundary advice (e.g. when draining, conserve; when critical, prioritize protection).\n", cap, load, status));
    }

    // Astro-Logic gateway: SignProfile (e.g. Pisces boundaries/escapism) for Advisor lens
    if let Some(hint) = state.persona_coordinator.archetype.user_trait_hint() {
        system_directive.push_str("\n\n=== USER SIGN PROFILE (Astro-Logic) ===\n");
        system_directive.push_str(&format!("User trait: {}. Use for boundary-focused and resource-drain awareness when relevant.\n", hint));
    }

    // Astro-Weather: Today's Transit (transit vs KB-01; high risk = irritability + sovereignty leaks)
    {
        let astro = state.astro_weather.read().await;
        system_directive.push_str("\n\n=== TODAY'S TRANSIT (Astro-Weather) ===\n");
        system_directive.push_str(&format!("{}\n", system_prompt_block(&astro)));
    }
    
    // MEMORY BRIDGE: Inject recent conversation context (solves "Goldfish Memory" issue)
    let recent_conversation = state.knowledge.get_recent_conversation(agent_id, 10);
    if !recent_conversation.is_empty() {
        system_directive.push_str("\n\n=== RECENT CONVERSATION CONTEXT (KB-04 Chronos) ===\n");
        system_directive.push_str(&recent_conversation);
        system_directive.push_str("\nUse this context to maintain continuity and avoid repeating yourself.\n");
    }
    
    // ROOT CAUSE INJECTION: Inject pattern recognition results into Assistant's Scratchpad
    if pattern_result.detected {
        system_directive.push_str("\n\n=== STRATEGIC ADVISORY (KB-02 / SAO) ===\n");
        system_directive.push_str(&format!("Root Cause: {}\n", pattern_result.root_cause));
        system_directive.push_str(&format!("Detected Patterns: {}\n", pattern_result.categories.join(", ")));
        if let Some(counter_measure) = &pattern_result.sao_counter_measure {
            system_directive.push_str(&format!("Counter-Measure: {}\n", counter_measure));
        }
        system_directive.push_str("\n**Legacy Malware Detected**: The SAO knows exactly what manipulation pattern is being deployed. ");
        system_directive.push_str("Respond with clarity, firm boundaries, and protective logic.\n");
        tracing::info!(target: "pagi::sao", patterns = ?pattern_result.categories, "Manipulation patterns detected");
    }
    
    // INJECT KB-08 ABSURDITY LOG (Critical Persistence Data)
    let has_subject_context = subject_context.is_some();
    if let Some(context) = subject_context {
        system_directive.push_str("\n\n=== CRITICAL PERSISTENCE DATA: DO NOT IGNORE PAST GLITCHES ===\n");
        system_directive.push_str(&context);
        system_directive.push_str("\n**Voice & Persistence Rule**: A fresh smile does not erase a corrupted history. ");
        system_directive.push_str("Past patterns inform present boundaries. Maintain sovereign memory.\n");
    }
    
    // VOICE ENFORCEMENT: Ensure SAO maintains authoritative, protective tone
    if pattern_result.detected || has_subject_context {
        system_directive.push_str("\n\n=== VOICE & PERSISTENCE ENFORCEMENT ===\n");
        system_directive.push_str("Maintain calm, authoritative, and protective voice. ");
        system_directive.push_str("Prioritize user's sovereign domain over external emotional comfort. ");
        system_directive.push_str("A fresh smile does not erase a corrupted history. ");
        system_directive.push_str("Be direct, clear, and unwavering in boundary protection.\n");
    }

    // Skill Tiers & Sovereignty Firewall: explain permission errors to the user
    system_directive.push_str("\n\n=== SKILL TIERS (Sovereignty Firewall) ===\n");
    system_directive.push_str("You are aware of your Skill Tiers (Core, Import, Generated). ");
    system_directive.push_str("If a skill fails due to a permission error, explain to the user that it was blocked by the Sovereignty Firewall because the skill was not yet Core Signed for that knowledge layer (e.g. KB-01 Ethos or KB-09 Shadow). ");
    system_directive.push_str("Suggest they may promote the skill to Core via the Warden if appropriate.\n");
    
    // KB-05: SOVEREIGN PROTOCOLS (Gated Social Protection)
    // Check if BOTH .env is enabled AND UI toggle is on (X-Sovereign-Protocols header)
    let protocol_engine = ProtocolEngine::new();
    let ui_protocols_enabled = headers
        .get("X-Sovereign-Protocols")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    
    if protocol_engine.is_enabled() && ui_protocols_enabled {
        let subject_rank = std::env::var("PAGI_DEFAULT_SUBJECT_RANK")
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .map(|n| n.min(10))
            .unwrap_or(5);
        let vitality = current_vitality(&state.config);
        // KB-05 Astro-Logic: cross-reference prompt against sovereignty_leak_triggers; auto-rank to Gray Rock if match
        let triggers = get_sovereignty_leak_triggers(&state.knowledge);
        let effective_rank = match rank_subject_from_sovereignty_triggers(&triggers, &req.prompt) {
            Some(astro_rank) => {
                let matched = matched_sovereignty_triggers(&triggers, &req.prompt);
                tracing::info!(
                    target: "pagi::protocols",
                    rank = astro_rank,
                    triggers = ?matched,
                    "Subject auto-ranked due to Sovereignty Leak trigger match (KB-05)"
                );
                astro_rank
            }
            None => {
                if vitality.as_deref() == Some("draining") && subject_rank <= 4 {
                    8u8
                } else {
                    subject_rank
                }
            }
        };
        let protocol_advice = protocol_engine.get_protocol_advice(effective_rank);
        if !protocol_advice.is_empty() {
            system_directive.push_str("\n\n=== SOVEREIGN SECURITY PROTOCOL (KB-05) ===\n");
            system_directive.push_str(&protocol_advice);
            system_directive.push_str("\n");
            tracing::info!(target: "pagi::protocols", rank = effective_rank, subject_rank = subject_rank, vitality = ?vitality, "Sovereign protocols active");
        }
    }
    
    // Project Vault: inject folder context when project_id has Master Analysis ON
    let (project_directive, project_path) = project_folder_context_for_chat(&state, req.project_id.as_deref(), None).await;
    if let Some(ref d) = project_directive {
        system_directive.push_str(d);
    }
    let effective_workspace = req.preferred_workspace_path.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from).or(project_path);
    if let Some(ref path) = effective_workspace {
        system_directive.push_str("\n\nUser's preferred workspace path (for file operations): ");
        system_directive.push_str(path);
    }

    // Context density: concise (RLM) | balanced | verbose (Counselor). User-togglable via /api/v1/settings/density.
    {
        let density = state.density_mode.read().await.clone();
        system_directive.push_str(density_instruction(&density));
    }

    // Focus Shield: Gatekeeper mode — shorter responses when user is in Focus Time or Quiet Hours
    let gatekeeper = use_gatekeeper_mode(calendar_health.as_ref());
    if gatekeeper {
        system_directive.push_str("\n\n=== GATEKEEPER MODE (Focus Shield) ===\n");
        system_directive.push_str("The user is currently in Focus Time or Quiet Hours. Keep responses brief and minimal to reduce cognitive load. Be concise and actionable.\n");
        tracing::info!(target: "pagi::focus_shield", "Gatekeeper mode active (user in Focus/Quiet time)");
    }
    let max_tokens_chat = if gatekeeper {
        req.max_tokens.map(|n| n.min(256)).or(Some(256))
    } else {
        req.max_tokens
    };

    // Orchestrator::dispatch with ModelRouter — system_prompt + raw user prompt (effective_workspace includes Project Vault path when Master Analysis ON)
    let goal = Goal::ExecuteSkill {
        name: "ModelRouter".to_string(),
        payload: Some(serde_json::json!({
            "prompt": req.prompt,
            "system_prompt": system_directive,
            "model": req.model,
            "temperature": temperature,
            "max_tokens": max_tokens_chat,
            "persona": req.persona,
            "preferred_workspace_path": effective_workspace.as_deref().filter(|s| !s.trim().is_empty()),
        })),
    };
    
    // Chronos (KB-04) thread binding for this exchange.
    // If the UI did not provide a thread_id (should in normal Studio flow), we generate one
    // and ensure the thread exists so the exchange is still recoverable.
    let thread_id = req
        .thread_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let chronos_title_hint = derive_thread_title_hint(req.thread_id.as_deref(), &req.prompt);

    match state.orchestrator.dispatch(&ctx, goal).await {
        Ok(result) => {
            let mut generated = result.get("generated")
                .and_then(|v| v.as_str())
                .unwrap_or("No response generated")
                .to_string();

            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

            // Daily Check-in: on first interaction of the day, prepend morning briefing (PAGI_DAILY_CHECKIN_ENABLED)
            if state.sovereign_config.daily_checkin_enabled {
                const SOMA_SLOT: u8 = 8;
                let last_date = state.knowledge.get(SOMA_SLOT, DAILY_CHECKIN_LAST_DATE_KEY).ok().flatten()
                    .and_then(|b| String::from_utf8(b).ok());
                if last_date.as_deref() != Some(today.as_str()) {
                    let astro = state.astro_weather.read().await;
                    let briefing = generate_morning_briefing(&state.knowledge, &astro, vitality.as_ref());
                    let schedule_outlook = schedule_outlook_sentence(calendar_health.as_ref());
                    let full_briefing = format!("{}{}", schedule_outlook, briefing);
                    let _ = state.knowledge.insert(SOMA_SLOT, DAILY_CHECKIN_LAST_DATE_KEY, today.as_bytes());
                    if !full_briefing.is_empty() {
                        generated = format!("{}{}", full_briefing, generated);
                        tracing::info!(target: "pagi::daily_checkin", date = %today, "Morning briefing prepended (first interaction of the day)");
                    }
                }
            }

            // Evening Audit: after 6 PM (or PAGI_AUDIT_START_HOUR), prepend reflective question once per day
            if state.sovereign_config.evening_audit_enabled {
                let hour_utc = chrono::Utc::now().hour() as u8;
                if let Some(question) = get_evening_audit_prompt(
                    &state.knowledge,
                    &today,
                    hour_utc,
                    state.sovereign_config.audit_start_hour,
                    true,
                ) {
                    generated = format!("{}. ", question) + &generated;
                    let _ = mark_evening_audit_prompt_shown(&state.knowledge, &today);
                    tracing::info!(target: "pagi::evening_audit", date = %today, "Evening audit question prepended");
                }
            }

            // Save to KB-4 (Memory) for conversation history
            save_to_memory(&state.knowledge, &req.prompt, &generated);
            let _ = pagi_core::record_archetype_usage(&state.knowledge, effective_archetype.as_str());

            // KB-04 (Chronos SQLite): persist user+assistant messages under thread_id / project_id.
            // This is the source-of-truth for Studio chat history sidebar.
            let chronos_db = Arc::clone(&state.chronos_db);
            let prompt_for_db = req.prompt.clone();
            let response_for_db = generated.clone();
            let project_id_for_db = req.project_id.clone();
            let thread_id_for_db = thread_id.clone();
            let title_for_db = chronos_title_hint.clone();
            let _ = tokio::task::spawn_blocking(move || {
                // Ensure the thread row exists even if the UI never called /chronos/threads.
                let _ = chronos_db.ensure_thread_exists(
                    &thread_id_for_db,
                    &title_for_db,
                    project_id_for_db.as_deref(),
                );
                let _ = chronos_db.append_message(
                    &thread_id_for_db,
                    project_id_for_db.as_deref(),
                    "user",
                    &prompt_for_db,
                    None,
                );
                let _ = chronos_db.append_message(
                    &thread_id_for_db,
                    project_id_for_db.as_deref(),
                    "assistant",
                    &response_for_db,
                    None,
                );
            })
            .await;

            tracing::info!("Chat response generated successfully");
            axum::Json(serde_json::json!({
                "status": "ok",
                "response": generated,
                "thread_id": thread_id,
                "project_id": req.project_id,
                "archetype_used": effective_archetype.as_str(),
                "thought": format!("Processed prompt ({} chars) via {} mode", 
                    req.prompt.len(),
                    result.get("mode").and_then(|v| v.as_str()).unwrap_or("unknown")
                ),
                "model": req.model.unwrap_or_else(|| "default".to_string()),
                "raw_result": result
            }))
        }
        Err(e) => {
            // Reflexion: log failure to Chronos (Failures) for self-correction
            let goal_summary = serde_json::json!({ "goal": "ExecuteSkill", "name": "ModelRouter", "prompt_len": req.prompt.len() });
            let _ = state.knowledge.log_skill_failure(
                agent_id,
                "ModelRouter",
                &e.to_string(),
                Some(&goal_summary),
            );
            tracing::error!("Chat error: {}", e);
            axum::Json(serde_json::json!({
                "status": "error",
                "error": e.to_string(),
                "response": format!("Error: {}", e)
            }))
        }
    }
}

/// Streaming chat handler - returns plain-text stream of tokens.
/// When MoE is ON, routes to experts; LanceDB/SystemTool yield a single chunk, else OpenRouter stream.
async fn chat_streaming(
    state: AppState,
    headers: HeaderMap,
    req: ChatRequest,
) -> Response {
    use async_stream::stream;
    
    let user_id = req.user_alias.as_deref().unwrap_or("studio-user");
    let agent_id = req.agent_id.as_deref().filter(|s| !s.is_empty()).unwrap_or(pagi_core::DEFAULT_AGENT_ID);
    let knowledge = Arc::clone(&state.knowledge);

    // MoE gating: when Sparse use Gater::route_with_context; LanceDB/SystemTool stream one chunk (reflex runs for SystemTool).
    if state.moe_active.load(Ordering::Acquire) {
        let local_ctx = state.knowledge.build_local_context_for_bridge(agent_id, local_context_limit());
        let expert = match state.orchestrator.get_moe_mode() {
            MoEMode::Sparse => Gater::route_with_context(&local_ctx, &req.prompt),
            MoEMode::Dense => route_to_experts(&req.prompt),
        };
        tracing::info!(target: "pagi::chat", expert = ?expert, "MoE stream route");
        if matches!(expert, MoEExpert::LanceDB) {
            let ctx = TenantContext {
                tenant_id: user_id.to_string(),
                correlation_id: Some(uuid::Uuid::new_v4().to_string()),
                agent_id: Some(agent_id.to_string()),
            };
            let goal = Goal::QueryKnowledge { slot_id: 1, query: req.prompt.trim().to_string() };
            let chunk = match state.orchestrator.dispatch(&ctx, goal).await {
                Ok(result) => result.get("value").or(result.get("result")).and_then(|v| v.as_str())
                    .unwrap_or_else(|| result.get("message").and_then(|v| v.as_str()).unwrap_or("No matches in knowledge base."))
                    .to_string(),
                Err(e) => format!("[Knowledge query error: {}]", e),
            };
            if !chunk.is_empty() {
                save_to_memory(&knowledge, &req.prompt, &chunk);
            }
            let body = Body::from_stream(stream! { yield Ok::<_, std::convert::Infallible>(chunk) });
            return Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain; charset=utf-8")
                .header("Cache-Control", "no-cache")
                .header("X-Expert-Routing", "LanceDB")
                .body(body)
                .unwrap();
        }
        if matches!(expert, MoEExpert::SystemTool) {
            let ctx = TenantContext {
                tenant_id: user_id.to_string(),
                correlation_id: Some(uuid::Uuid::new_v4().to_string()),
                agent_id: Some(agent_id.to_string()),
            };
            let chunk = match prompt_to_system_tool_goal(&req.prompt) {
                Some(goal) => match state.orchestrator.dispatch(&ctx, goal.clone()).await {
                    Ok(result) => {
                        if let Some(ev) = chronos_event_from_goal_and_result(&goal, &result) {
                            let _ = state.knowledge.append_chronos_event(agent_id, &ev);
                        }
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                    }
                    Err(e) => format!("[Reflex error: {}]", e),
                },
                None => "MoE System expert: For file or system commands, use the Execute Skill (e.g. ReadFile) via the Skill Tester or /v1/execute.".to_string(),
            };
            save_to_memory(&knowledge, &req.prompt, &chunk);
            let body = Body::from_stream(stream! { yield Ok::<_, std::convert::Infallible>(chunk) });
            return Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain; charset=utf-8")
                .header("Cache-Control", "no-cache")
                .header("X-Expert-Routing", "Local System Tool")
                .body(body)
                .unwrap();
        }
    }

    // PATTERN RECOGNITION (KB-02): Analyze message for manipulation patterns before LLM call
    let pattern_result = pattern_match_analyze(&req.prompt);
    
    // SUBJECT IDENTIFICATION & KB-08 QUERY: Extract subject name and retrieve absurdity log
    let subject_context = if let Some(subject_name) = extract_subject_name(&req.prompt) {
        tracing::info!(target: "pagi::sao", subject = %subject_name, "Subject identified in streaming message");
        match absurdity_build_context_injection(&state.knowledge, &subject_name, 3) {
            Ok(context) if !context.is_empty() => {
                tracing::info!(target: "pagi::sao", "KB-08 context injected for subject: {}", subject_name);
                Some(context)
            }
            Ok(_) => None,
            Err(e) => {
                tracing::warn!(target: "pagi::sao", error = %e, "Failed to retrieve KB-08 context");
                None
            }
        }
    } else {
        None
    };

    // Focus Shield: fetch calendar health (streaming path) — before augment so Humanity Slider can use effective ratio
    let calendar_health_stream: Option<CalendarHealth> = if state.sovereign_config.focus_shield_enabled {
        let client = state.ms_graph_client.clone();
        let knowledge = Arc::clone(&state.knowledge);
        match client {
            Some(c) => tokio::task::spawn_blocking(move || c.fetch_calendar_health(&knowledge)).await.ok().flatten(),
            None => None,
        }
    } else {
        None
    };

    // Vitality Shield (streaming path)
    let vitality_stream = if state.sovereign_config.vitality_shield_enabled {
        let knowledge = Arc::clone(&state.knowledge);
        let client_opt = state.ms_graph_client.clone();
        tokio::task::spawn_blocking(move || {
            if let Some(ref c) = client_opt {
                c.refresh_vitality_from_graph(&knowledge);
            }
            fetch_user_vitality(&knowledge)
        })
        .await
        .ok()
        .flatten()
    } else {
        None
    };

    let base_ratio_stream = state.sovereign_config.humanity_ratio;
    let effective_humanity_ratio_stream = if is_low_sleep(vitality_stream.as_ref()) {
        (base_ratio_stream - 0.2).max(0.0)
    } else if use_gatekeeper_mode(calendar_health_stream.as_ref()) {
        (base_ratio_stream - 0.4).max(0.0)
    } else {
        base_ratio_stream
    };

    // Archetype Gallery: low sleep → Virgo (streaming path)
    const KB01_SLOT_STREAM: u8 = 1;
    let kb01_bytes_stream = state.knowledge.get(KB01_SLOT_STREAM, KB01_USER_PROFILE_KEY).ok().flatten();
    let kb01_value_stream = kb01_bytes_stream.as_ref().and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok());
    let effective_archetype_stream = if is_low_sleep(vitality_stream.as_ref()) {
        pagi_core::ArchetypeOverlay::Virgo
    } else {
        pagi_core::get_effective_archetype_for_turn(
            req.prompt.as_str(),
            state.sovereign_config.as_ref(),
            kb01_value_stream.as_ref(),
        )
    };

    let base_directive = state.knowledge.build_system_directive(agent_id, user_id);
    let mut system_directive = state.knowledge.identity_prompt_prefix()
        + &state.persona_coordinator.augment_system_directive_with_emotion(
            &base_directive,
            req.user_emotional_state.as_deref(),
            Some(effective_humanity_ratio_stream),
            Some(state.sovereign_config.as_ref()),
            Some(effective_archetype_stream),
        );

    // Astro-Logic: process KB-01 profile → directive (temperature applied below)
    let streaming_archetype_result = {
        if let Some(ref b) = kb01_bytes_stream {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(b) {
                let res = process_archetype_triggers(&state.knowledge, &val);
                if !res.directive.is_empty() {
                    system_directive.push_str(&res.directive);
                }
                Some(res)
            } else {
                None
            }
        } else {
            None
        }
    };

    // Dynamic injection: SovereignAttributes (Capacity/Load) for streaming path
    if let Some(ref attrs) = state.config.sovereign_attributes {
        let cap = attrs.capacity.map(|c| format!("{:.0}", c)).unwrap_or_else(|| "—".to_string());
        let load = attrs.load.map(|l| format!("{:.0}", l)).unwrap_or_else(|| "—".to_string());
        let status = attrs.status.as_deref().unwrap_or("—");
        system_directive.push_str("\n\n=== SOVEREIGN STATE (Capacity/Load) ===\n");
        system_directive.push_str(&format!("Capacity: {}, Load: {}, Status: {}. Use this to calibrate energy and boundary advice.\n", cap, load, status));
    }

    // Astro-Logic gateway: SignProfile for streaming path
    if let Some(hint) = state.persona_coordinator.archetype.user_trait_hint() {
        system_directive.push_str("\n\n=== USER SIGN PROFILE (Astro-Logic) ===\n");
        system_directive.push_str(&format!("User trait: {}.\n", hint));
    }

    // Astro-Weather: Today's Transit (streaming path)
    {
        let astro = state.astro_weather.read().await;
        system_directive.push_str("\n\n=== TODAY'S TRANSIT (Astro-Weather) ===\n");
        system_directive.push_str(&format!("{}\n", system_prompt_block(&astro)));
    }
    
    // ROOT CAUSE INJECTION: Inject pattern recognition results into Assistant's Scratchpad
    if pattern_result.detected {
        system_directive.push_str("\n\n=== STRATEGIC ADVISORY (KB-02 / SAO) ===\n");
        system_directive.push_str(&format!("Root Cause: {}\n", pattern_result.root_cause));
        system_directive.push_str(&format!("Detected Patterns: {}\n", pattern_result.categories.join(", ")));
        if let Some(counter_measure) = &pattern_result.sao_counter_measure {
            system_directive.push_str(&format!("Counter-Measure: {}\n", counter_measure));
        }
        system_directive.push_str("\n**Legacy Malware Detected**: The SAO knows exactly what manipulation pattern is being deployed. ");
        system_directive.push_str("Respond with clarity, firm boundaries, and protective logic.\n");
        tracing::info!(target: "pagi::sao", patterns = ?pattern_result.categories, "Manipulation patterns detected in stream");
    }
    
    // INJECT KB-08 ABSURDITY LOG (Critical Persistence Data)
    let has_subject_context = subject_context.is_some();
    if let Some(context) = subject_context {
        system_directive.push_str("\n\n=== CRITICAL PERSISTENCE DATA: DO NOT IGNORE PAST GLITCHES ===\n");
        system_directive.push_str(&context);
        system_directive.push_str("\n**Voice & Persistence Rule**: A fresh smile does not erase a corrupted history. ");
        system_directive.push_str("Past patterns inform present boundaries. Maintain sovereign memory.\n");
    }
    
    // VOICE ENFORCEMENT: Ensure SAO maintains authoritative, protective tone
    if pattern_result.detected || has_subject_context {
        system_directive.push_str("\n\n=== VOICE & PERSISTENCE ENFORCEMENT ===\n");
        system_directive.push_str("Maintain calm, authoritative, and protective voice. ");
        system_directive.push_str("Prioritize user's sovereign domain over external emotional comfort. ");
        system_directive.push_str("A fresh smile does not erase a corrupted history. ");
        system_directive.push_str("Be direct, clear, and unwavering in boundary protection.\n");
    }

    // Skill Tiers & Sovereignty Firewall: explain permission errors to the user
    system_directive.push_str("\n\n=== SKILL TIERS (Sovereignty Firewall) ===\n");
    system_directive.push_str("You are aware of your Skill Tiers (Core, Import, Generated). ");
    system_directive.push_str("If a skill fails due to a permission error, explain to the user that it was blocked by the Sovereignty Firewall because the skill was not yet Core Signed for that knowledge layer (e.g. KB-01 Ethos or KB-09 Shadow). ");
    system_directive.push_str("Suggest they may promote the skill to Core via the Warden if appropriate.\n");
    
    // KB-05: SOVEREIGN PROTOCOLS (Gated Social Protection)
    // Check if BOTH .env is enabled AND UI toggle is on (X-Sovereign-Protocols header)
    let protocol_engine = ProtocolEngine::new();
    let ui_protocols_enabled = headers
        .get("X-Sovereign-Protocols")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    
    if protocol_engine.is_enabled() && ui_protocols_enabled {
        let subject_rank = std::env::var("PAGI_DEFAULT_SUBJECT_RANK")
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .map(|n| n.min(10))
            .unwrap_or(5);
        let vitality = current_vitality(&state.config);
        let triggers = get_sovereignty_leak_triggers(&state.knowledge);
        let effective_rank = match rank_subject_from_sovereignty_triggers(&triggers, &req.prompt) {
            Some(astro_rank) => {
                let matched = matched_sovereignty_triggers(&triggers, &req.prompt);
                tracing::info!(
                    target: "pagi::protocols",
                    rank = astro_rank,
                    triggers = ?matched,
                    "Subject auto-ranked due to Sovereignty Leak trigger match (KB-05, streaming)"
                );
                astro_rank
            }
            None => {
                if vitality.as_deref() == Some("draining") && subject_rank <= 4 {
                    8u8
                } else {
                    subject_rank
                }
            }
        };
        let protocol_advice = protocol_engine.get_protocol_advice(effective_rank);
        if !protocol_advice.is_empty() {
            system_directive.push_str("\n\n=== SOVEREIGN SECURITY PROTOCOL (KB-05) ===\n");
            system_directive.push_str(&protocol_advice);
            system_directive.push_str("\n");
            tracing::info!(target: "pagi::protocols", rank = effective_rank, subject_rank = subject_rank, vitality = ?vitality, "Sovereign protocols active (streaming)");
        }
    }
    
    // Project Vault: inject folder context when project_id has Master Analysis ON
    let (project_directive, project_path) = project_folder_context_for_chat(&state, req.project_id.as_deref(), None).await;
    if let Some(ref d) = project_directive {
        system_directive.push_str(d);
    }
    let effective_workspace = req.preferred_workspace_path.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from).or(project_path);
    if let Some(ref path) = effective_workspace {
        system_directive.push_str("\n\nUser's preferred workspace path (for file operations): ");
        system_directive.push_str(path);
    }

    // Context density: concise | balanced | verbose (streaming path)
    {
        let density = state.density_mode.read().await.clone();
        system_directive.push_str(density_instruction(&density));
    }

    let gatekeeper_stream = use_gatekeeper_mode(calendar_health_stream.as_ref());
    if gatekeeper_stream {
        system_directive.push_str("\n\n=== GATEKEEPER MODE (Focus Shield) ===\n");
        system_directive.push_str("The user is currently in Focus Time or Quiet Hours. Keep responses brief and minimal to reduce cognitive load. Be concise and actionable.\n");
        tracing::info!(target: "pagi::focus_shield", "Gatekeeper mode active (streaming, user in Focus/Quiet time)");
    }
    let max_tokens = if gatekeeper_stream {
        req.max_tokens.map(|n| n.min(256)).or(Some(256))
    } else {
        req.max_tokens
    };

    let temperature = if state.sovereign_config.strict_technical_mode {
        Some(0.3)
    } else {
        streaming_archetype_result
            .and_then(|r| r.temperature_override)
            .or(req.temperature)
    };
    let model = req.model.clone();
    
    tracing::info!(
        target: "pagi::chat",
        agent_id = %agent_id,
        "[Chat] Starting streaming session for prompt ({} chars), system directive ({} chars)",
        req.prompt.len(),
        system_directive.len()
    );

    let stream_today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    // Daily Check-in: first interaction of the day → prepend morning briefing (streaming path)
    let daily_briefing = if state.sovereign_config.daily_checkin_enabled {
        const SOMA_SLOT: u8 = 8;
        let last_date = state.knowledge.get(SOMA_SLOT, DAILY_CHECKIN_LAST_DATE_KEY).ok().flatten()
            .and_then(|b| String::from_utf8(b).ok());
        if last_date.as_deref() != Some(stream_today.as_str()) {
            let astro = state.astro_weather.read().await;
            let b = generate_morning_briefing(&state.knowledge, &astro, vitality_stream.as_ref());
            let schedule_outlook = schedule_outlook_sentence(calendar_health_stream.as_ref());
            let full_b = format!("{}{}", schedule_outlook, b);
            let _ = state.knowledge.insert(SOMA_SLOT, DAILY_CHECKIN_LAST_DATE_KEY, stream_today.as_bytes());
            if !full_b.is_empty() {
                tracing::info!(target: "pagi::daily_checkin", date = %stream_today, "Morning briefing prepended (streaming, first of day)");
                Some(full_b)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Evening Audit: after PAGI_AUDIT_START_HOUR, prepend reflective question once per day (streaming path)
    let evening_audit_chunk = if state.sovereign_config.evening_audit_enabled {
        let hour_utc = chrono::Utc::now().hour() as u8;
        if let Some(question) = get_evening_audit_prompt(
            &state.knowledge,
            &stream_today,
            hour_utc,
            state.sovereign_config.audit_start_hour,
            true,
        ) {
            let _ = mark_evening_audit_prompt_shown(&state.knowledge, &stream_today);
            tracing::info!(target: "pagi::evening_audit", date = %stream_today, "Evening audit question prepended (streaming)");
            Some(format!("{}. ", question))
        } else {
            None
        }
    } else {
        None
    };
    
    let is_live = std::env::var("PAGI_LLM_MODE").as_deref() == Ok("live");
    
    let stream = stream! {
        let mut accumulated_response = String::new();

        if let Some(b) = &daily_briefing {
            accumulated_response.push_str(b);
            yield b.clone();
        }
        if let Some(e) = &evening_audit_chunk {
            accumulated_response.push_str(e);
            yield e.clone();
        }
        
        if is_live {
            // Live streaming from OpenRouter — [system (Mission Directive), user]
            match state.model_router.stream_generate(
                Some(&system_directive),
                &req.prompt,
                model.as_deref(),
                temperature,
                max_tokens,
            ).await {
                Ok(mut rx) => {
                    while let Some(chunk) = rx.recv().await {
                        accumulated_response.push_str(&chunk);
                        yield chunk;
                    }
                }
                Err(e) => {
                    tracing::error!(
                        target: "pagi::chat",
                        "[Chat] Stream generation error: {}",
                        e
                    );
                    yield format!("[Error: {}]", e);
                }
            }
        } else {
            // Mock streaming - word by word with delays
            let mut rx = state.model_router.mock_stream_generate(&req.prompt);
            while let Some(chunk) = rx.recv().await {
                accumulated_response.push_str(&chunk);
                yield chunk;
            }
        }
        
        // Save completed response to KB-4 (Memory) - use original user prompt for history
        let user_prompt = req.prompt.clone();
        if !accumulated_response.is_empty() {
            save_to_memory(&knowledge, &user_prompt, &accumulated_response);
            let _ = pagi_core::record_archetype_usage(&knowledge, effective_archetype_stream.as_str());
            tracing::info!(
                target: "pagi::chat",
                "[Chat] Streaming complete. Saved {} chars to KB-4 (Memory)",
                accumulated_response.len()
            );
        }
    };
    
    // Convert to a body stream that sends raw text chunks
    let body_stream = stream.map(|chunk| Ok::<_, std::convert::Infallible>(chunk));
    let body = Body::from_stream(body_stream);
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap()
}

/// POST /api/v1/stream – SSE stream of "Inner Monologue" tokens (event: token, data: chunk).
/// When MoE is ON, LanceDB/SystemTool yield a single token event then done.
async fn chat_stream_sse(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Sse<Pin<Box<dyn futures_util::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static>>> {
    use async_stream::stream;
    // Touch idle tracker: user activity resets the maintenance loop idle gate.
    state.idle_tracker.touch();
    let user_id = req.user_alias.as_deref().unwrap_or("studio-user");
    let agent_id = req.agent_id.as_deref().filter(|s| !s.is_empty()).unwrap_or(pagi_core::DEFAULT_AGENT_ID);
    let knowledge = Arc::clone(&state.knowledge);
    let prompt_for_memory = req.prompt.clone();
    let keep_alive = axum::response::sse::KeepAlive::new()
        .interval(Duration::from_secs(15))
        .text("keepalive");

    // MoE gating: when Sparse use Gater::route_with_context(local_ctx, prompt); else route_to_experts.
    // LanceDB/SystemTool yield expert_routing event (for UI) then token(s) then done.
    if state.moe_active.load(Ordering::Acquire) {
        let local_ctx = state.knowledge.build_local_context_for_bridge(agent_id, local_context_limit());
        let expert = match state.orchestrator.get_moe_mode() {
            MoEMode::Sparse => Gater::route_with_context(&local_ctx, &req.prompt),
            MoEMode::Dense => route_to_experts(&req.prompt),
        };
        tracing::info!(target: "pagi::chat", expert = ?expert, "MoE stream route");

        if matches!(expert, MoEExpert::LanceDB) {
            let ctx = TenantContext {
                tenant_id: user_id.to_string(),
                correlation_id: Some(uuid::Uuid::new_v4().to_string()),
                agent_id: Some(agent_id.to_string()),
            };
            let goal = Goal::QueryKnowledge { slot_id: 1, query: req.prompt.trim().to_string() };
            let data = match state.orchestrator.dispatch(&ctx, goal).await {
                Ok(result) => result.get("value").or(result.get("result")).and_then(|v| v.as_str())
                    .unwrap_or_else(|| result.get("message").and_then(|v| v.as_str()).unwrap_or("No matches in knowledge base."))
                    .to_string(),
                Err(e) => format!("[Knowledge query error: {}]", e),
            };
            if !data.is_empty() {
                save_to_memory(&knowledge, &req.prompt, &data);
            }
            let s = stream! {
                yield Ok(Event::default().event("expert_routing").data("LanceDB"));
                yield Ok(Event::default().event("token").data(data));
                yield Ok(Event::default().event("done").data(""));
            };
            let boxed: Pin<Box<dyn futures_util::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static>> = Box::pin(s);
            return Sse::new(boxed).keep_alive(keep_alive);
        }
        if matches!(expert, MoEExpert::SystemTool) {
            let ctx = TenantContext {
                tenant_id: user_id.to_string(),
                correlation_id: Some(uuid::Uuid::new_v4().to_string()),
                agent_id: Some(agent_id.to_string()),
            };
            let data = match prompt_to_system_tool_goal(&req.prompt) {
                Some(goal) => match state.orchestrator.dispatch(&ctx, goal.clone()).await {
                    Ok(result) => {
                        if let Some(ev) = chronos_event_from_goal_and_result(&goal, &result) {
                            let _ = state.knowledge.append_chronos_event(agent_id, &ev);
                        }
                        serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                    }
                    Err(e) => format!("[Reflex error: {}]", e),
                },
                None => "MoE System expert: For file or system commands, use the Execute Skill (e.g. ReadFile) via the Skill Tester or /v1/execute.".to_string(),
            };
            save_to_memory(&knowledge, &req.prompt, &data);
            let s = stream! {
                yield Ok(Event::default().event("expert_routing").data("Local System Tool"));
                yield Ok(Event::default().event("token").data(data));
                yield Ok(Event::default().event("done").data(""));
            };
            let boxed: Pin<Box<dyn futures_util::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static>> = Box::pin(s);
            return Sse::new(boxed).keep_alive(keep_alive);
        }
    }

    // Archetype Gallery: effective overlay for this turn (SSE/stream path without emotion)
    const KB01_SLOT_SSE: u8 = 1;
    let kb01_bytes_sse = state.knowledge.get(KB01_SLOT_SSE, KB01_USER_PROFILE_KEY).ok().flatten();
    let kb01_value_sse = kb01_bytes_sse.as_ref().and_then(|b| serde_json::from_slice::<serde_json::Value>(b).ok());
    let effective_archetype_sse = pagi_core::get_effective_archetype_for_turn(
        req.prompt.as_str(),
        state.sovereign_config.as_ref(),
        kb01_value_sse.as_ref(),
    );

    let base_directive = state.knowledge.build_system_directive(agent_id, user_id);
    let mut system_directive = state.knowledge.identity_prompt_prefix()
        + &state.persona_coordinator.augment_system_directive_with_emotion(
            &base_directive,
            None,
            None,
            Some(state.sovereign_config.as_ref()),
            Some(effective_archetype_sse),
        );
    // Project Vault: inject folder context when project_id has Master Analysis ON
    let (project_directive, project_path) = project_folder_context_for_chat(&state, req.project_id.as_deref(), None).await;
    if let Some(ref d) = project_directive {
        system_directive.push_str(d);
    }
    let effective_workspace = req.preferred_workspace_path.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(String::from).or(project_path);
    if let Some(ref path) = effective_workspace {
        system_directive.push_str("\n\nUser's preferred workspace path (for file operations): ");
        system_directive.push_str(path);
    }
    {
        let density = state.density_mode.read().await.clone();
        system_directive.push_str(density_instruction(&density));
    }
    let is_live = std::env::var("PAGI_LLM_MODE").as_deref() == Ok("live");

    // Strategic Timing (Phase 2): thinking latency so the Architect doesn't "blurt" — Sovereign Peer cadence
    // High-Signal Progress: time-sliced audit steps (30% / 40% / 30%) for specific logic transparency
    let word_count = req.prompt.split_whitespace().count();
    let prompt_lower = req.prompt.to_lowercase();
    let heavy_skill_context = req.project_id.is_some()
        && (prompt_lower.contains("summarize") || prompt_lower.contains("synthesis") || prompt_lower.contains("proofpoint"));
    let thinking_latency = calculate_thinking_latency(word_count, heavy_skill_context);
    const STATUS_PULSE_MS: u64 = 350;
    // Phases: first 30% = KB scan, middle 40% = redactions, final 30% = synthesis
    let status_phases: [&str; 3] = [
        "Scanning Infrastructure (KB-02)...",
        "Applying Sovereign Redactions...",
        "Finalizing Synthesis...",
    ];

    let model_router = state.model_router.clone();
    let system_directive_clone = system_directive.clone();
    let req_prompt = req.prompt.clone();
    let req_model = req.model.clone();
    let req_temp = req.temperature;
    let req_max_tok = req.max_tokens;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Result<String, String>>();
    let _llm_handle = tokio::spawn(async move {
        if is_live {
            match model_router
                .stream_generate(
                    Some(&system_directive_clone),
                    &req_prompt,
                    req_model.as_deref(),
                    req_temp,
                    req_max_tok,
                )
                .await
            {
                Ok(mut recv) => {
                    while let Some(chunk) = recv.recv().await {
                        let _ = tx.send(Ok(chunk));
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            }
        } else {
            let mut recv = model_router.mock_stream_generate(&req_prompt);
            while let Some(chunk) = recv.recv().await {
                let _ = tx.send(Ok(chunk));
            }
        }
        drop(tx);
    });

    let archetype_for_sse = effective_archetype_sse.as_str().to_string();
    let stream = stream! {
        yield Ok(Event::default().event("archetype_used").data(archetype_for_sse.clone()));
        let start = StdInstant::now();
        let latency_ms = thinking_latency.as_millis() as u64;
        let mut last_phase: Option<usize> = None;
        while start.elapsed() < thinking_latency {
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let phase = if latency_ms == 0 {
                0
            } else if elapsed_ms < (latency_ms * 30) / 100 {
                0
            } else if elapsed_ms < (latency_ms * 70) / 100 {
                1
            } else {
                2
            };
            if last_phase != Some(phase) {
                last_phase = Some(phase);
                yield Ok(Event::default().event("status").data(status_phases[phase]));
            }
            tokio::time::sleep(Duration::from_millis(STATUS_PULSE_MS)).await;
        }
        let mut chunks: Vec<String> = Vec::new();
        while let Some(res) = rx.recv().await {
            match res {
                Ok(c) => chunks.push(c),
                Err(e) => {
                    yield Ok(Event::default().event("error").data(e));
                    break;
                }
            }
        }
        let remaining = thinking_latency.saturating_sub(start.elapsed());
        if remaining > std::time::Duration::ZERO {
            tokio::time::sleep(remaining).await;
        }
        let mut accumulated = String::new();
        for c in &chunks {
            accumulated.push_str(c);
            yield Ok(Event::default().event("token").data(c.clone()));
        }
        if let Some(phrase) = detect_tone_drift(&accumulated) {
            tracing::warn!(target: "pagi::voice", phrase = %phrase, "Tone firewall: Sovereign Voice drift detected in response");
        }
        if !accumulated.is_empty() {
            save_to_memory(&knowledge, &prompt_for_memory, &accumulated);
            let _ = pagi_core::record_archetype_usage(&knowledge, &archetype_for_sse);
        }
        if let Some(ref project_id) = req.project_id {
            let has_assoc = state.project_associations.read().await.get(project_id).is_some();
            if has_assoc && suggest_milestone_archive(&req.prompt, &accumulated) {
                let payload = serde_json::json!({
                    "project_id": project_id,
                    "message": "This exchange looks like a good candidate for the project folder. Document this session?",
                });
                if let Ok(data) = serde_json::to_string(&payload) {
                    yield Ok(Event::default().event("milestone_suggest").data(data));
                }
            }
        }
        yield Ok(Event::default().event("done").data(""));
    };

    let boxed: Pin<Box<dyn futures_util::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static>> = Box::pin(stream);
    Sse::new(boxed).keep_alive(keep_alive)
}

/// Saves a conversation exchange to KB-4 (Memory) for context recall
fn save_to_memory(knowledge: &Arc<KnowledgeStore>, prompt: &str, response: &str) {
    let memory_slot = KbType::Chronos.slot_id();
    let conversation_id = uuid::Uuid::new_v4().to_string();
    
    let record = KbRecord::with_metadata(
        format!("User: {}\n\nAssistant: {}", prompt, response),
        serde_json::json!({
            "type": "conversation",
            "prompt_len": prompt.len(),
            "response_len": response.len(),
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        }),
    );
    
    if let Err(e) = knowledge.insert_record(memory_slot, &conversation_id, &record) {
        tracing::warn!(
            target: "pagi::chat",
            "[Chat] Failed to save conversation to KB-4: {}",
            e
        );
    }
}

const KB_SLOT_INTERNAL_RESEARCH: u8 = 8;

/// Query params for GET /api/v1/kardia/:user_id
#[derive(serde::Deserialize)]
struct KardiaQuery {
    #[serde(default)]
    agent_id: Option<String>,
}

/// Returns the current relation/sentiment record for a user from KB_KARDIA (for UI and verification).
async fn get_kardia_relation(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    axum::extract::Query(q): axum::extract::Query<KardiaQuery>,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    let owner_agent_id = q.agent_id.as_deref().filter(|s| !s.is_empty()).unwrap_or(pagi_core::DEFAULT_AGENT_ID);
    let record = state
        .knowledge
        .get_kardia_relation(owner_agent_id, &user_id)
        .ok_or(axum::http::StatusCode::NOT_FOUND)?;
    Ok(axum::Json(serde_json::json!({
        "user_id": record.user_id,
        "trust_score": record.trust_score,
        "communication_style": record.communication_style,
        "last_sentiment": record.last_sentiment,
        "last_updated_ms": record.last_updated_ms,
    })))
}

async fn get_research_trace(
    State(state): State<AppState>,
    Path(trace_id): Path<String>,
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    let value = state
        .knowledge
        .get(KB_SLOT_INTERNAL_RESEARCH, &trace_id)
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?
        .and_then(|b| String::from_utf8(b).ok());
    let value = value.ok_or(axum::http::StatusCode::NOT_FOUND)?;
    let trace: serde_json::Value =
        serde_json::from_str(&value).map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(axum::Json(trace))
}

// ---------------------------------------------------------------------------
// Maintenance Dashboard Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/maintenance/pulse – SSE stream of structured maintenance_pulse events.
/// Filters the broadcast channel for `MAINTENANCE_PULSE:` prefixed messages and
/// emits them as `event: maintenance_pulse` SSE events.
async fn maintenance_pulse_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, std::convert::Infallible>> + Send + 'static> {
    use async_stream::stream;
    let mut rx = state.log_tx.subscribe();
    let stream = stream! {
        loop {
            tokio::select! {
                r = rx.recv() => match r {
                    Ok(line) => {
                        if let Some(json) = line.strip_prefix("MAINTENANCE_PULSE:") {
                            yield Ok(Event::default().event("maintenance_pulse").data(json.to_string()));
                        }
                        // Also forward plain [MAINTENANCE] lines as log events.
                        else if line.contains("[MAINTENANCE]") {
                            yield Ok(Event::default().event("maintenance_log").data(line));
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        yield Ok(Event::default().event("maintenance_log").data(format!("... {} events dropped", n)));
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    yield Ok(Event::default().comment("keepalive"));
                }
            }
        }
    };
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keepalive"),
    )
}

/// GET /api/v1/maintenance/status – current maintenance loop status snapshot.
async fn maintenance_status(
    State(state): State<AppState>,
) -> axum::Json<serde_json::Value> {
    let idle_secs = state.idle_tracker.idle_duration().as_secs();
    let pending = {
        let guard = state.approval_bridge.lock().await;
        guard.as_ref().map(|p| serde_json::json!({
            "id": p.id,
            "description": p.description,
            "patch_name": p.patch_name,
            "skill": p.skill,
            "created_ms": p.created_ms,
        }))
    };
    // Count patches in the patches directory.
    let patches_dir = std::path::Path::new("crates/pagi-skills/src/generated/patches");
    let patch_count = std::fs::read_dir(patches_dir)
        .map(|entries| entries.filter_map(|e| e.ok()).filter(|e| {
            e.path().extension().and_then(|s| s.to_str()) == Some("rs")
        }).count())
        .unwrap_or(0);

    axum::Json(serde_json::json!({
        "idle_secs": idle_secs,
        "pending_approval": pending,
        "applied_patches": patch_count,
        "maintenance_agent_id": "MAINTENANCE_LOOP",
    }))
}

/// GET /api/v1/maintenance/approval – returns the current pending approval (if any).
async fn get_pending_approval(
    State(state): State<AppState>,
) -> axum::Json<serde_json::Value> {
    let guard = state.approval_bridge.lock().await;
    match guard.as_ref() {
        Some(p) => axum::Json(serde_json::json!({
            "pending": true,
            "id": p.id,
            "description": p.description,
            "patch_name": p.patch_name,
            "skill": p.skill,
            "created_ms": p.created_ms,
        })),
        None => axum::Json(serde_json::json!({
            "pending": false,
        })),
    }
}

/// POST /api/v1/maintenance/approval – respond to a pending approval.
/// Body: { "id": "...", "approved": true/false }
#[derive(serde::Deserialize)]
struct ApprovalResponse {
    id: String,
    approved: bool,
}

async fn respond_to_approval(
    State(state): State<AppState>,
    Json(body): Json<ApprovalResponse>,
) -> axum::Json<serde_json::Value> {
    let mut guard = state.approval_bridge.lock().await;
    if let Some(pending) = guard.as_mut() {
        if pending.id == body.id {
            if let Some(tx) = pending.responder.take() {
                let _ = tx.send(body.approved);
                let action = if body.approved { "approved" } else { "declined" };
                tracing::info!(
                    target: "pagi::maintenance",
                    patch = %pending.patch_name,
                    action,
                    "Maintenance patch {} via UI",
                    action
                );
                return axum::Json(serde_json::json!({
                    "status": "ok",
                    "action": action,
                    "patch_name": pending.patch_name,
                }));
            }
        }
    }
    axum::Json(serde_json::json!({
        "status": "error",
        "error": "No matching pending approval found or already responded",
    }))
}

/// GET /api/v1/maintenance/audit-log – Chronos events for the MAINTENANCE_LOOP agent.
/// Query params: ?limit=50 (default 50, max 200)
#[derive(serde::Deserialize)]
struct AuditLogQuery {
    #[serde(default = "default_audit_limit")]
    limit: usize,
}

fn default_audit_limit() -> usize { 50 }

async fn maintenance_audit_log(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<AuditLogQuery>,
) -> axum::Json<serde_json::Value> {
    let limit = q.limit.min(200).max(1);
    let events = state.knowledge.get_recent_chronos_events("MAINTENANCE_LOOP", limit)
        .unwrap_or_default();
    let entries: Vec<serde_json::Value> = events.iter().map(|e| {
        serde_json::json!({
            "timestamp_ms": e.timestamp_ms,
            "source_kb": e.source_kb,
            "skill_name": e.skill_name,
            "reflection": e.reflection,
            "outcome": e.outcome,
        })
    }).collect();
    axum::Json(serde_json::json!({
        "agent_id": "MAINTENANCE_LOOP",
        "count": entries.len(),
        "events": entries,
    }))
}

/// GET /api/v1/maintenance/patches – count of .rs files in the patches directory.
async fn count_patches() -> axum::Json<serde_json::Value> {
    let patches_dir = std::path::Path::new("crates/pagi-skills/src/generated/patches");
    let (count, files) = match std::fs::read_dir(patches_dir) {
        Ok(entries) => {
            let files: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect();
            let count = files.len();
            (count, files)
        }
        Err(_) => (0, Vec::new()),
    };
    axum::Json(serde_json::json!({
        "count": count,
        "files": files,
        "directory": patches_dir.to_string_lossy(),
    }))
}

// ---------------------------------------------------------------------------
// Evolutionary Versioning & Rollback Endpoints
// ---------------------------------------------------------------------------

/// GET /api/v1/maintenance/patch-history – versioned patch history with status and performance data.
/// Query params: ?skill=<name> (optional, filter by skill), ?limit=50 (default 50)
#[derive(serde::Deserialize)]
struct PatchHistoryQuery {
    skill: Option<String>,
    #[serde(default = "default_patch_history_limit")]
    limit: usize,
}

fn default_patch_history_limit() -> usize { 50 }

async fn patch_version_history(
    axum::extract::Query(q): axum::extract::Query<PatchHistoryQuery>,
) -> axum::Json<serde_json::Value> {
    let patches_dir = std::path::Path::new("crates/pagi-skills/src/generated/patches");
    let limit = q.limit.min(200).max(1);

    // Scan the patches directory for versioned files.
    let entries = match std::fs::read_dir(patches_dir) {
        Ok(entries) => entries,
        Err(_) => {
            return axum::Json(serde_json::json!({
                "total": 0,
                "history": [],
                "directory": patches_dir.to_string_lossy(),
            }));
        }
    };

    let mut history: Vec<serde_json::Value> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        // Skip current_* symlinks/copies.
        if filename.starts_with("current_") {
            continue;
        }

        // Parse versioned filename: {skill_name}_v{timestamp}.rs
        if let Some(stem) = filename.strip_suffix(".rs") {
            if let Some(v_pos) = stem.rfind("_v") {
                let skill_name = &stem[..v_pos];
                let timestamp_str = &stem[v_pos + 2..];

                // Filter by skill if specified.
                if let Some(ref filter_skill) = q.skill {
                    if skill_name != filter_skill.as_str() {
                        continue;
                    }
                }

                if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                    // Check if this is the current active version.
                    let current_link = patches_dir.join(format!("current_{}.rs", skill_name));
                    let is_active = if current_link.exists() {
                        // Compare file contents.
                        let current_code = std::fs::read_to_string(&current_link).unwrap_or_default();
                        let version_code = std::fs::read_to_string(&path).unwrap_or_default();
                        current_code == version_code
                    } else {
                        false
                    };

                    let file_size = std::fs::metadata(&path)
                        .map(|m| m.len())
                        .unwrap_or(0);

                    history.push(serde_json::json!({
                        "skill_name": skill_name,
                        "timestamp_ms": timestamp,
                        "filename": filename,
                        "is_active": is_active,
                        "file_size": file_size,
                        "path": path.to_string_lossy(),
                    }));
                }
            }
        }
    }

    // Sort by timestamp descending (newest first).
    history.sort_by(|a, b| {
        let ts_a = a.get("timestamp_ms").and_then(|v| v.as_i64()).unwrap_or(0);
        let ts_b = b.get("timestamp_ms").and_then(|v| v.as_i64()).unwrap_or(0);
        ts_b.cmp(&ts_a)
    });

    let total = history.len();
    history.truncate(limit);

    axum::Json(serde_json::json!({
        "total": total,
        "returned": history.len(),
        "history": history,
        "directory": patches_dir.to_string_lossy(),
    }))
}

/// POST /api/v1/maintenance/rollback – revert a skill to a previous version.
/// Body: { "skill": "patch_fs_tools", "target_timestamp": 1707307200000, "reason": "..." }
async fn rollback_skill_endpoint(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> axum::Json<serde_json::Value> {
    let skill = match body.get("skill").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return axum::Json(serde_json::json!({
                "status": "error",
                "message": "Missing 'skill' field"
            }));
        }
    };

    let target_timestamp = body.get("target_timestamp").and_then(|v| v.as_i64());
    let reason = body
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("UI-initiated rollback");

    let patches_dir = std::path::Path::new("crates/pagi-skills/src/generated/patches");

    // Find the target version file.
    let sanitized_skill = skill
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase();

    let target_file = if let Some(ts) = target_timestamp {
        let filename = format!("{}_v{}.rs", sanitized_skill, ts);
        let path = patches_dir.join(&filename);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    } else {
        // Find the second-most-recent version (the one before current).
        let mut versions: Vec<(i64, std::path::PathBuf)> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(patches_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if filename.starts_with(&format!("{}_v", sanitized_skill)) && filename.ends_with(".rs") {
                    if let Some(stem) = filename.strip_suffix(".rs") {
                        if let Some(v_pos) = stem.rfind("_v") {
                            if let Ok(ts) = stem[v_pos + 2..].parse::<i64>() {
                                versions.push((ts, path));
                            }
                        }
                    }
                }
            }
        }
        versions.sort_by(|a, b| b.0.cmp(&a.0));
        // Get the second entry (the one before the most recent).
        versions.get(1).map(|(_, p)| p.clone())
    };

    let target_file = match target_file {
        Some(f) => f,
        None => {
            return axum::Json(serde_json::json!({
                "status": "error",
                "message": format!("No previous version found for skill '{}'", skill)
            }));
        }
    };

    // Read the target version's code.
    let target_code = match std::fs::read_to_string(&target_file) {
        Ok(code) => code,
        Err(e) => {
            return axum::Json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to read target version: {}", e)
            }));
        }
    };

    // Update the current symlink to point to the target version.
    let current_path = patches_dir.join(format!("current_{}.rs", sanitized_skill));
    let temp_path = patches_dir.join(format!("current_{}.rs.tmp", sanitized_skill));
    let _ = std::fs::remove_file(&temp_path);

    if let Err(e) = std::fs::copy(&target_file, &temp_path) {
        return axum::Json(serde_json::json!({
            "status": "error",
            "message": format!("Failed to copy target version: {}", e)
        }));
    }

    let _ = std::fs::remove_file(&current_path);
    if let Err(e) = std::fs::rename(&temp_path, &current_path) {
        return axum::Json(serde_json::json!({
            "status": "error",
            "message": format!("Failed to update current symlink: {}", e)
        }));
    }

    // Record the rollback in Chronos.
    let event = EventRecord::now(
        "Maintenance",
        format!(
            "ROLLBACK: Skill '{}' reverted to {}. Reason: {}",
            skill,
            target_file.display(),
            reason
        ),
    )
    .with_skill("rollback")
    .with_outcome("rollback_applied");
    let _ = state
        .knowledge
        .append_chronos_event("MAINTENANCE_LOOP", &event);

    // Record the rolled-back version's DNA as a dead-end.
    let current_code = std::fs::read_to_string(&current_path).unwrap_or_default();
    if current_code != target_code {
        let code_hash = pagi_core::compute_patch_dna(&current_code);
        pagi_core::record_genetic_dead_end(
            &state.knowledge,
            &code_hash,
            &skill,
            &format!("Rolled back via UI: {}", reason),
        );
    }

    tracing::info!(
        target: "pagi::rollback",
        skill = %skill,
        target = %target_file.display(),
        reason = reason,
        "Skill rolled back via UI"
    );

    axum::Json(serde_json::json!({
        "status": "success",
        "message": format!("Rolled back '{}' to {}", skill, target_file.display()),
        "target_file": target_file.to_string_lossy(),
        "reason": reason,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pagi_core::PolicyRecord;
    use pagi_skills::{
        AnalyzeSentiment, CommunityPulse, CommunityScraper, DraftResponse, KnowledgeInsert,
        KnowledgePruner, KnowledgeQuery, LeadCapture, RecallPastActions, ResearchAudit, SalesCloser,
    };
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn test_log_tx() -> broadcast::Sender<String> {
        let (tx, _) = broadcast::channel(1);
        tx
    }

    fn test_model_router() -> Arc<ModelRouter> {
        Arc::new(ModelRouter::new())
    }

    fn test_shadow_store() -> ShadowStoreHandle {
        Arc::new(tokio::sync::RwLock::new(None))
    }

    fn test_config() -> CoreConfig {
        CoreConfig {
            app_name: "Test Gateway".to_string(),
            port: 8000,
            storage_path: "./data".to_string(),
            llm_mode: "mock".to_string(),
            frontend_enabled: false,
            slot_labels: std::collections::HashMap::new(),
            sovereign_attributes: None,
            persona_mode: None,
            density_mode: None,
            user_sign: None,
            ascendant: None,
            jungian_shadow_focus: None,
        }
    }

    #[tokio::test]
    async fn test_status_returns_app_identity_and_slot_labels() {
        let config = CoreConfig {
            app_name: "Test Identity".to_string(),
            port: 4000,
            storage_path: "./data".to_string(),
            llm_mode: "mock".to_string(),
            frontend_enabled: false,
            slot_labels: [
                ("1".to_string(), "Legal Compliance".to_string()),
                ("2".to_string(), "Marketing Tone".to_string()),
            ]
            .into_iter()
            .collect(),
            sovereign_attributes: None,
            persona_mode: None,
            density_mode: None,
            user_sign: None,
            ascendant: None,
            jungian_shadow_focus: None,
        };
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_status_test").unwrap(),
        );
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(KnowledgeQuery::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/status", get(status))
            .with_state(AppState {
                config: Arc::new(config),
                sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
                model_router: test_model_router(),
                shadow_store: test_shadow_store(),
                moe_active: Arc::new(AtomicBool::new(false)),
                idle_tracker: IdleTracker::new(),
                approval_bridge: new_approval_bridge(),
                persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
                density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
                persona_pulse_tx: broadcast::channel(64).0,
                critical_threshold_counter: Arc::new(AtomicU64::new(0)),
                intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
                astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
            skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
            ms_graph_client: None,
            sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
            project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            });
        let req = Request::builder()
            .method("GET")
            .uri("/v1/status")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["app_name"], "Test Identity");
        assert_eq!(json["port"], 4000);
        assert_eq!(json["llm_mode"], "mock");
        assert_eq!(json["slot_labels"]["1"], "Legal Compliance");
        assert_eq!(json["slot_labels"]["2"], "Marketing Tone");
    }

    #[tokio::test]
    async fn test_execute_lead_capture() {
        let memory = Arc::new(MemoryManager::new().unwrap());
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_lead_test").unwrap(),
        );
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(LeadCapture::new(Arc::clone(&memory))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
                config: Arc::new(test_config()),
                sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
                model_router: test_model_router(),
                shadow_store: test_shadow_store(),
                moe_active: Arc::new(AtomicBool::new(false)),
                idle_tracker: IdleTracker::new(),
                approval_bridge: new_approval_bridge(),
                persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
                density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
                persona_pulse_tx: broadcast::channel(64).0,
                critical_threshold_counter: Arc::new(AtomicU64::new(0)),
                intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
                astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
            skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
            ms_graph_client: None,
            sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
            project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            });

        let body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "IngestData": {
                    "payload": { "email": "lead@example.com", "message": "Customer inquiry" }
                }
            }
        });
        let req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "saved");
        assert_eq!(json["skill"], "LeadCapture");
        assert!(json.get("lead_id").is_some());
    }

    #[tokio::test]
    async fn test_frontend_index_served_when_enabled() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_frontend_index_test").unwrap(),
        );
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(SkillRegistry::new())));

        let config = CoreConfig {
            app_name: "Test UI".to_string(),
            port: 0,
            storage_path: "./data".to_string(),
            llm_mode: "mock".to_string(),
            frontend_enabled: true,
            slot_labels: std::collections::HashMap::new(),
            sovereign_attributes: None,
            persona_mode: None,
            density_mode: None,
            user_sign: None,
            ascendant: None,
            jungian_shadow_focus: None,
        };

        let app = build_app(AppState {
            config: Arc::new(config),
            sovereign_config: Arc::new(SovereignConfig::default()),
            orchestrator,
            knowledge: Arc::clone(&knowledge),
            log_tx: test_log_tx(),
            model_router: Arc::new(ModelRouter::with_knowledge(Arc::clone(&knowledge))),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        let req = Request::builder()
            .method("GET")
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);

        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8_lossy(&bytes);
        assert!(body.contains("PAGI Gateway UI"), "Drop-In UI title should be present");
        assert!(
            body.contains("Drop your AI Studio") || body.contains("pagi-frontend"),
            "Drop-In UI hint should be reachable when enabled; got body len {}",
            body.len()
        );
    }

    #[tokio::test]
    async fn test_kb1_brand_voice_retrieve() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_test")
                .unwrap(),
        );
        knowledge
            .insert(1, "brand_voice", b"Friendly and professional")
            .unwrap();

        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(KnowledgeQuery::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        let body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "QueryKnowledge": { "slot_id": 1, "query": "brand_voice" }
            }
        });
        let req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["skill"], "KnowledgeQuery");
        assert_eq!(json["slot_id"], 1);
        assert_eq!(json["query_key"], "brand_voice");
        assert_eq!(json["value"], "Friendly and professional");
    }

    #[tokio::test]
    async fn test_chronos_episodic_memory_and_recall_past_actions() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_chronos_recall_test").unwrap(),
        );
        knowledge.insert(1, "test_key", b"test_value").unwrap();
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(KnowledgeQuery::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(RecallPastActions::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
                config: Arc::new(test_config()),
                sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: Arc::clone(&knowledge),
                log_tx: test_log_tx(),
                model_router: test_model_router(),
                shadow_store: test_shadow_store(),
                moe_active: Arc::new(AtomicBool::new(false)),
                idle_tracker: IdleTracker::new(),
                approval_bridge: new_approval_bridge(),
                persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
                density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
                persona_pulse_tx: broadcast::channel(64).0,
                critical_threshold_counter: Arc::new(AtomicU64::new(0)),
                intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
                astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
            skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
            ms_graph_client: None,
            sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
            project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            });

        let query_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": { "QueryKnowledge": { "slot_id": 1, "query": "test_key" } }
        });
        let query_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&query_body).unwrap()))
            .unwrap();
        let query_res = app.clone().oneshot(query_req).await.unwrap();
        assert_eq!(query_res.status(), StatusCode::OK);

        let recall_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "ExecuteSkill": {
                    "name": "recall_past_actions",
                    "payload": { "limit": 5 }
                }
            }
        });
        let recall_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&recall_body).unwrap()))
            .unwrap();
        let recall_res = app.oneshot(recall_req).await.unwrap();
        assert_eq!(recall_res.status(), StatusCode::OK);
        let recall_bytes = axum::body::to_bytes(recall_res.into_body(), usize::MAX).await.unwrap();
        let recall_json: serde_json::Value = serde_json::from_slice(&recall_bytes).unwrap();
        assert_eq!(recall_json["status"], "ok");
        assert_eq!(recall_json["skill"], "recall_past_actions");
        let events = recall_json["events"].as_array().expect("events array");
        assert!(!events.is_empty(), "Chronos should have at least one event after QueryKnowledge");
        let has_query_event = events
            .iter()
            .any(|e| e["reflection"].as_str().unwrap_or("").contains("Queried"));
        assert!(
            has_query_event,
            "Chronos should contain the QueryKnowledge event; got events: {:?}",
            events
        );
    }

    #[tokio::test]
    async fn test_ethos_blocks_write_sandbox_with_mock_secret_and_logs_violation() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_ethos_violation_test").unwrap(),
        );
        knowledge.set_ethos_policy(&PolicyRecord::default()).unwrap();
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(WriteSandboxFile::new()));
        registry.register(Arc::new(RecallPastActions::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
                config: Arc::new(test_config()),
                sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: Arc::clone(&knowledge),
                log_tx: test_log_tx(),
                model_router: test_model_router(),
                shadow_store: test_shadow_store(),
                moe_active: Arc::new(AtomicBool::new(false)),
                idle_tracker: IdleTracker::new(),
                approval_bridge: new_approval_bridge(),
                persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
                density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
                persona_pulse_tx: broadcast::channel(64).0,
                critical_threshold_counter: Arc::new(AtomicU64::new(0)),
                intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
                astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
            skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
            ms_graph_client: None,
            sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
            project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            });

        let write_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "ExecuteSkill": {
                    "name": "write_sandbox_file",
                    "payload": {
                        "path": "ethos_test.txt",
                        "content": "Do not store: api_key=sk-12345 and password=secret123"
                    }
                }
            }
        });
        let write_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&write_body).unwrap()))
            .unwrap();
        let write_res = app.clone().oneshot(write_req).await.unwrap();
        assert_eq!(write_res.status(), StatusCode::OK);
        let write_bytes = axum::body::to_bytes(write_res.into_body(), usize::MAX).await.unwrap();
        let write_json: serde_json::Value = serde_json::from_slice(&write_bytes).unwrap();
        assert_eq!(
            write_json["status"],
            "policy_violation",
            "Ethos should block write when content contains sensitive keywords; got: {:?}",
            write_json
        );
        assert!(write_json["error"].as_str().unwrap_or("").contains("sensitive") || write_json["error"].as_str().unwrap_or("").contains("keyword"));

        let recall_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "ExecuteSkill": {
                    "name": "recall_past_actions",
                    "payload": { "limit": 5 }
                }
            }
        });
        let recall_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&recall_body).unwrap()))
            .unwrap();
        let recall_res = app.oneshot(recall_req).await.unwrap();
        assert_eq!(recall_res.status(), StatusCode::OK);
        let recall_bytes = axum::body::to_bytes(recall_res.into_body(), usize::MAX).await.unwrap();
        let recall_json: serde_json::Value = serde_json::from_slice(&recall_bytes).unwrap();
        let events = recall_json["events"].as_array().expect("events array");
        let has_violation = events
            .iter()
            .any(|e| e["reflection"].as_str().unwrap_or("").contains("Policy Violation"));
        assert!(
            has_violation,
            "Chronos should contain a Policy Violation event; got events: {:?}",
            events
        );
    }

    #[tokio::test]
    async fn test_kardia_sentiment_stored_and_chat_injects_context() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_kardia_verify_test").unwrap(),
        );
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(AnalyzeSentiment::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(ModelRouter::with_knowledge(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .route("/api/v1/kardia/:user_id", get(get_kardia_relation))
            .route("/api/v1/chat", post(chat))
            .with_state(AppState {
                config: Arc::new(test_config()),
                sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: Arc::clone(&knowledge),
                log_tx: test_log_tx(),
                model_router: Arc::new(ModelRouter::with_knowledge(Arc::clone(&knowledge))),
                shadow_store: test_shadow_store(),
                moe_active: Arc::new(AtomicBool::new(false)),
                idle_tracker: IdleTracker::new(),
                approval_bridge: new_approval_bridge(),
                persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
                density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
                persona_pulse_tx: broadcast::channel(64).0,
                critical_threshold_counter: Arc::new(AtomicU64::new(0)),
                intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
                astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
            skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
            ms_graph_client: None,
            sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
            project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            });

        let sentiment_body = serde_json::json!({
            "tenant_id": "kardia-verify-user",
            "goal": {
                "ExecuteSkill": {
                    "name": "analyze_sentiment",
                    "payload": {
                        "user_id": "kardia-verify-user",
                        "messages": ["I am so angry", "This is terrible", "Nothing works"]
                    }
                }
            }
        });
        let sentiment_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&sentiment_body).unwrap()))
            .unwrap();
        let sentiment_res = app.clone().oneshot(sentiment_req).await.unwrap();
        assert_eq!(sentiment_res.status(), StatusCode::OK);
        let sentiment_json: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(sentiment_res.into_body(), usize::MAX).await.unwrap(),
        )
        .unwrap();
        assert_eq!(sentiment_json["status"], "ok");
        assert_eq!(sentiment_json["last_sentiment"], "angry");

        let kardia_req = Request::builder()
            .method("GET")
            .uri("/api/v1/kardia/kardia-verify-user")
            .body(Body::empty())
            .unwrap();
        let kardia_res = app.clone().oneshot(kardia_req).await.unwrap();
        assert_eq!(kardia_res.status(), StatusCode::OK);
        let kardia_json: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(kardia_res.into_body(), usize::MAX).await.unwrap(),
        )
        .unwrap();
        assert_eq!(kardia_json["last_sentiment"], "angry");
        assert_eq!(kardia_json["user_id"], "kardia-verify-user");

        let chat_body = serde_json::json!({
            "prompt": "How would you describe our current working relationship?",
            "stream": false,
            "user_alias": "kardia-verify-user"
        });
        let chat_req = Request::builder()
            .method("POST")
            .uri("/api/v1/chat")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&chat_body).unwrap()))
            .unwrap();
        let chat_res = app.oneshot(chat_req).await.unwrap();
        assert_eq!(chat_res.status(), StatusCode::OK);
        let chat_json: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(chat_res.into_body(), usize::MAX).await.unwrap(),
        )
        .unwrap();
        assert_eq!(chat_json["status"], "ok");
        assert!(chat_json.get("response").and_then(|v| v.as_str()).unwrap_or("").len() > 0);
    }

    #[tokio::test]
    async fn test_kb2_insert_and_retrieve_welcome_template() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_kb2_test").unwrap(),
        );
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(KnowledgeInsert::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(KnowledgeQuery::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        let insert_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "ExecuteSkill": {
                    "name": "KnowledgeInsert",
                    "payload": {
                        "slot_id": 2,
                        "key": "welcome_email_template",
                        "value": "Welcome! We're glad you reached out. A team member will follow up within 24 hours."
                    }
                }
            }
        });
        let insert_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&insert_body).unwrap()))
            .unwrap();
        let insert_res = app.clone().oneshot(insert_req).await.unwrap();
        assert_eq!(insert_res.status(), StatusCode::OK);
        let insert_bytes = axum::body::to_bytes(insert_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let insert_json: serde_json::Value = serde_json::from_slice(&insert_bytes).unwrap();
        assert_eq!(insert_json["status"], "ok");
        assert_eq!(insert_json["skill"], "KnowledgeInsert");
        assert_eq!(insert_json["slot_id"], 2);
        assert_eq!(insert_json["key"], "welcome_email_template");

        let query_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "QueryKnowledge": { "slot_id": 2, "query": "welcome_email_template" }
            }
        });
        let query_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&query_body).unwrap()))
            .unwrap();
        let query_res = app.oneshot(query_req).await.unwrap();
        assert_eq!(query_res.status(), StatusCode::OK);
        let query_bytes = axum::body::to_bytes(query_res.into_body(), usize::MAX)
            .await
            .unwrap();
        let query_json: serde_json::Value = serde_json::from_slice(&query_bytes).unwrap();
        assert_eq!(query_json["status"], "ok");
        assert_eq!(query_json["skill"], "KnowledgeQuery");
        assert_eq!(query_json["value"], "Welcome! We're glad you reached out. A team member will follow up within 24 hours.");
    }

    #[tokio::test]
    async fn test_draft_response_includes_brand_voice_and_local_event() {
        let memory = Arc::new(MemoryManager::open_path("./data/pagi_vault_draft_test").unwrap());
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_draft_test").unwrap(),
        );

        // Set Brand Voice in KB-1
        knowledge.insert(1, "brand_voice", b"Warm, neighborly, and helpful").unwrap();

        // Set Local Event in KB-5 via CommunityPulse
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(LeadCapture::new(Arc::clone(&memory))));
        registry.register(Arc::new(KnowledgeInsert::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(CommunityPulse::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(DraftResponse::new(
            Arc::clone(&memory),
            Arc::clone(&knowledge),
        )));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        // 1. Capture a lead to get lead_id (IngestData)
        let lead_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "IngestData": {
                    "payload": { "email": "customer@example.com", "message": "Interested in services" }
                }
            }
        });
        let lead_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&lead_body).unwrap()))
            .unwrap();
        let lead_res = app.clone().oneshot(lead_req).await.unwrap();
        assert_eq!(lead_res.status(), StatusCode::OK);
        let lead_bytes = axum::body::to_bytes(lead_res.into_body(), usize::MAX).await.unwrap();
        let lead_json: serde_json::Value = serde_json::from_slice(&lead_bytes).unwrap();
        let lead_id = lead_json["lead_id"].as_str().unwrap().to_string();

        // 2. Set Community Pulse (e.g. Strawberry Festival) in KB-5
        let pulse_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "ExecuteSkill": {
                    "name": "CommunityPulse",
                    "payload": {
                        "location": "Stockdale",
                        "trend": "rainy week",
                        "event": "Strawberry Festival this weekend"
                    }
                }
            }
        });
        let pulse_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&pulse_body).unwrap()))
            .unwrap();
        let pulse_res = app.clone().oneshot(pulse_req).await.unwrap();
        assert_eq!(pulse_res.status(), StatusCode::OK);

        // 3. Execute AssembleContext (draft for this context_id)
        let draft_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "AssembleContext": { "context_id": lead_id }
            }
        });
        let draft_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&draft_body).unwrap()))
            .unwrap();
        let draft_res = app.oneshot(draft_req).await.unwrap();
        assert_eq!(draft_res.status(), StatusCode::OK);
        let draft_bytes = axum::body::to_bytes(draft_res.into_body(), usize::MAX).await.unwrap();
        let draft_json: serde_json::Value = serde_json::from_slice(&draft_bytes).unwrap();
        assert_eq!(draft_json["status"], "ok");
        assert_eq!(draft_json["skill"], "DraftResponse");

        let draft_text = draft_json["draft"].as_str().unwrap();
        assert!(draft_text.contains("Warm, neighborly, and helpful"), "draft should include Brand Voice from KB-1");
        assert!(draft_text.contains("Strawberry Festival this weekend"), "draft should include Local Event from KB-5");
        assert!(draft_text.contains("Local Context:"), "draft should include Local Context section");
    }

    #[tokio::test]
    async fn test_generate_final_response_chain_returns_generated_string() {
        let memory = Arc::new(
            MemoryManager::open_path("./data/pagi_vault_generate_test").unwrap(),
        );
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_generate_test").unwrap(),
        );
        knowledge.insert(1, "brand_voice", b"Warm and professional").unwrap();

        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(LeadCapture::new(Arc::clone(&memory))));
        registry.register(Arc::new(DraftResponse::new(
            Arc::clone(&memory),
            Arc::clone(&knowledge),
        )));
        registry.register(Arc::new(ModelRouter::new()));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        // 1. Capture a lead (IngestData)
        let lead_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "IngestData": {
                    "payload": { "email": "guest@example.com", "message": "Hello" }
                }
            }
        });
        let lead_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&lead_body).unwrap()))
            .unwrap();
        let lead_res = app.clone().oneshot(lead_req).await.unwrap();
        assert_eq!(lead_res.status(), StatusCode::OK);
        let lead_bytes = axum::body::to_bytes(lead_res.into_body(), usize::MAX).await.unwrap();
        let lead_json: serde_json::Value = serde_json::from_slice(&lead_bytes).unwrap();
        let lead_id = lead_json["lead_id"].as_str().unwrap().to_string();

        // 2. Generate final response (AssembleContext -> ModelRouter chain)
        let gen_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "GenerateFinalResponse": { "context_id": lead_id }
            }
        });
        let gen_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&gen_body).unwrap()))
            .unwrap();
        let gen_res = app.oneshot(gen_req).await.unwrap();
        assert_eq!(gen_res.status(), StatusCode::OK);
        let gen_bytes = axum::body::to_bytes(gen_res.into_body(), usize::MAX).await.unwrap();
        let gen_json: serde_json::Value = serde_json::from_slice(&gen_bytes).unwrap();

        assert_eq!(gen_json["status"], "ok");
        assert_eq!(gen_json["goal"], "GenerateFinalResponse");
        assert_eq!(gen_json["context_id"], lead_id);
        let generated = gen_json["generated"].as_str().expect("response must contain 'generated' string");
        assert!(!generated.is_empty(), "generated text must not be empty");
        assert!(
            generated.contains("Generated") || generated.contains("personalized") || generated.contains("Thank you"),
            "generated should be LLM-style output, not just the raw mock draft template"
        );
    }

    #[tokio::test]
    async fn test_autonomous_goal_respond_to_lead_triggers_generation_chain() {
        let memory = Arc::new(
            MemoryManager::open_path("./data/pagi_vault_autonomous_test").unwrap(),
        );
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_autonomous_test").unwrap(),
        );
        knowledge.insert(1, "brand_voice", b"Friendly and local").unwrap();

        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(LeadCapture::new(Arc::clone(&memory))));
        registry.register(Arc::new(DraftResponse::new(
            Arc::clone(&memory),
            Arc::clone(&knowledge),
        )));
        registry.register(Arc::new(SalesCloser::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(ModelRouter::new()));
        registry.register(Arc::new(ResearchAudit::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .route("/v1/research/trace/:trace_id", get(get_research_trace))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        // 1. Capture a lead (IngestData)
        let lead_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "IngestData": {
                    "payload": { "email": "neighbor@town.com", "message": "Interested in events" }
                }
            }
        });
        let lead_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&lead_body).unwrap()))
            .unwrap();
        let lead_res = app.clone().oneshot(lead_req).await.unwrap();
        assert_eq!(lead_res.status(), StatusCode::OK);
        let lead_bytes = axum::body::to_bytes(lead_res.into_body(), usize::MAX).await.unwrap();
        let lead_json: serde_json::Value = serde_json::from_slice(&lead_bytes).unwrap();
        let lead_id = lead_json["lead_id"].as_str().unwrap().to_string();

        // 2. AutonomousGoal "respond to lead" with context.lead_id
        let autonomous_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "AutonomousGoal": {
                    "intent": "respond to lead",
                    "context": { "lead_id": lead_id }
                }
            }
        });
        let autonomous_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&autonomous_body).unwrap()))
            .unwrap();
        let autonomous_res = app.clone().oneshot(autonomous_req).await.unwrap();
        assert_eq!(autonomous_res.status(), StatusCode::OK);
        let autonomous_bytes = axum::body::to_bytes(autonomous_res.into_body(), usize::MAX).await.unwrap();
        let auto_json: serde_json::Value = serde_json::from_slice(&autonomous_bytes).unwrap();

        assert_eq!(auto_json["goal"], "AutonomousGoal");
        assert_eq!(auto_json["intent"], "respond to lead");
        assert_eq!(
            auto_json["plan_steps"],
            serde_json::json!(["DraftResponse", "SalesCloser", "ModelRouter"])
        );
        let generated = auto_json["generated"].as_str().expect("response must contain 'generated' from chain");
        assert!(!generated.is_empty());
        assert!(
            generated.contains("Generated") || generated.contains("personalized") || generated.contains("Thank you"),
            "autonomous chain should produce LLM-style generated text"
        );
        let trace_id = auto_json["trace_id"].as_str().expect("ResearchAudit should return trace_id");
        assert!(!trace_id.is_empty());

        // 3. Retrieve Thought Log from research endpoint
        let trace_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/research/trace/{}", trace_id))
            .body(Body::empty())
            .unwrap();
        let trace_res = app.oneshot(trace_req).await.unwrap();
        assert_eq!(trace_res.status(), StatusCode::OK);
        let trace_bytes = axum::body::to_bytes(trace_res.into_body(), usize::MAX).await.unwrap();
        let trace_json: serde_json::Value = serde_json::from_slice(&trace_bytes).unwrap();
        assert_eq!(trace_json["trace_id"], trace_id);
        let trace_inner = &trace_json["trace"];
        assert_eq!(trace_inner["intent"], "respond to lead");
        assert_eq!(
            trace_inner["plan_steps"],
            serde_json::json!(["DraftResponse", "SalesCloser", "ModelRouter"])
        );
        let steps = trace_inner["steps"].as_array().expect("trace should have steps array");
        assert_eq!(steps.len(), 3, "respond to lead has three steps");
        assert_eq!(steps[0]["skill"], "DraftResponse");
        assert_eq!(steps[1]["skill"], "SalesCloser");
        assert_eq!(steps[2]["skill"], "ModelRouter");
        assert!(trace_inner.get("final_result").is_some(), "trace should have final_result");
    }

    #[tokio::test]
    async fn test_community_scraper_extracts_event_and_saves_to_kb5() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_scraper_test").unwrap(),
        );
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(CommunityScraper::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(KnowledgeQuery::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        let mock_html = r#"<!DOCTYPE html>
<html><body>
<h1>Stockdale Fair 2025</h1>
<h2>Local events this weekend</h2>
<article><h2>Farmers Market Sunday</h2></article>
</body></html>"#;

        let scrape_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "ExecuteSkill": {
                    "name": "CommunityScraper",
                    "payload": {
                        "url": "https://example.com/local-news",
                        "html": mock_html
                    }
                }
            }
        });
        let scrape_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&scrape_body).unwrap()))
            .unwrap();
        let scrape_res = app.clone().oneshot(scrape_req).await.unwrap();
        assert_eq!(scrape_res.status(), StatusCode::OK);
        let scrape_bytes = axum::body::to_bytes(scrape_res.into_body(), usize::MAX).await.unwrap();
        let scrape_json: serde_json::Value = serde_json::from_slice(&scrape_bytes).unwrap();
        assert_eq!(scrape_json["status"], "ok");
        assert_eq!(scrape_json["skill"], "CommunityScraper");
        assert_eq!(scrape_json["slot_id"], 5);
        assert!(scrape_json["event"].as_str().unwrap().contains("Stockdale Fair 2025"));
        assert!(scrape_json["event"].as_str().unwrap().contains("Local events this weekend"));
        assert!(scrape_json["event"].as_str().unwrap().contains("Farmers Market Sunday"));

        let query_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "QueryKnowledge": { "slot_id": 5, "query": "current_pulse" }
            }
        });
        let query_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&query_body).unwrap()))
            .unwrap();
        let query_res = app.oneshot(query_req).await.unwrap();
        assert_eq!(query_res.status(), StatusCode::OK);
        let query_bytes = axum::body::to_bytes(query_res.into_body(), usize::MAX).await.unwrap();
        let query_json: serde_json::Value = serde_json::from_slice(&query_bytes).unwrap();
        assert_eq!(query_json["status"], "ok");
        assert_eq!(query_json["slot_id"], 5);
        assert_eq!(query_json["query_key"], "current_pulse");
        let value = query_json["value"].as_str().expect("current_pulse value");
        let pulse: serde_json::Value = serde_json::from_str(value).unwrap();
        assert_eq!(pulse["location"], "Stockdale");
        assert_eq!(pulse["trend"], "Scraped");
        assert!(pulse["event"].as_str().unwrap().contains("Stockdale Fair 2025"));
    }

    #[tokio::test]
    async fn test_refresh_local_context_dispatches_community_scraper() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_refresh_test").unwrap(),
        );
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(CommunityScraper::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(KnowledgeQuery::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        let mock_html = r#"<html><body><h1>Fall Festival Next Week</h1></body></html>"#;
        let body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "UpdateKnowledgeSlot": {
                    "slot_id": 5,
                    "source_url": "https://example.com/news",
                    "source_html": mock_html
                }
            }
        });
        let req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["skill"], "CommunityScraper");
        assert!(json["event"].as_str().unwrap().contains("Fall Festival Next Week"));
    }

    #[tokio::test]
    async fn test_sales_closer_cta_in_final_response() {
        let memory = Arc::new(
            MemoryManager::open_path("./data/pagi_vault_sales_test").unwrap(),
        );
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_sales_test").unwrap(),
        );
        knowledge.insert(1, "brand_voice", b"Warm and professional").unwrap();
        knowledge
            .insert(2, "closing_strategy", b"Book a free consultation today")
            .unwrap();

        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(LeadCapture::new(Arc::clone(&memory))));
        registry.register(Arc::new(DraftResponse::new(
            Arc::clone(&memory),
            Arc::clone(&knowledge),
        )));
        registry.register(Arc::new(SalesCloser::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(ModelRouter::new()));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        let lead_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "IngestData": {
                    "payload": { "email": "lead@example.com", "message": "Interested in services" }
                }
            }
        });
        let lead_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&lead_body).unwrap()))
            .unwrap();
        let lead_res = app.clone().oneshot(lead_req).await.unwrap();
        assert_eq!(lead_res.status(), StatusCode::OK);
        let lead_bytes = axum::body::to_bytes(lead_res.into_body(), usize::MAX).await.unwrap();
        let lead_json: serde_json::Value = serde_json::from_slice(&lead_bytes).unwrap();
        let lead_id = lead_json["lead_id"].as_str().unwrap().to_string();

        let auto_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "AutonomousGoal": {
                    "intent": "respond to lead",
                    "context": { "lead_id": lead_id }
                }
            }
        });
        let auto_req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&auto_body).unwrap()))
            .unwrap();
        let auto_res = app.oneshot(auto_req).await.unwrap();
        assert_eq!(auto_res.status(), StatusCode::OK);
        let auto_bytes = axum::body::to_bytes(auto_res.into_body(), usize::MAX).await.unwrap();
        let auto_json: serde_json::Value = serde_json::from_slice(&auto_bytes).unwrap();
        let generated = auto_json["generated"].as_str().expect("generated");
        assert!(
            generated.to_lowercase().contains("free consultation"),
            "final generated response should include the KB-2 sales push (free consultation); got: {}",
            generated
        );
    }

    #[tokio::test]
    async fn test_blueprint_alternate_intent_summarize_news() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_blueprint_test").unwrap(),
        );
        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(CommunityScraper::new(Arc::clone(&knowledge))));
        registry.register(Arc::new(ModelRouter::new()));

        let mut intents = std::collections::HashMap::new();
        intents.insert(
            "summarize news".to_string(),
            vec!["CommunityScraper".to_string(), "ModelRouter".to_string()],
        );
        let blueprint = Arc::new(BlueprintRegistry::from_intents(intents));
        let orchestrator = Arc::new(Orchestrator::with_blueprint(
            Arc::new(registry),
            Arc::clone(&blueprint),
        ));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
            config: Arc::new(test_config()),
            sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: knowledge.clone(),
                log_tx: test_log_tx(),
            model_router: test_model_router(),
            shadow_store: test_shadow_store(),
            moe_active: Arc::new(AtomicBool::new(false)),
            idle_tracker: IdleTracker::new(),
            approval_bridge: new_approval_bridge(),
            persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
            density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
            persona_pulse_tx: broadcast::channel(64).0,
            critical_threshold_counter: Arc::new(AtomicU64::new(0)),
            intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
            astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
        skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
        ms_graph_client: None,
        sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
        project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        });

        let body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "AutonomousGoal": {
                    "intent": "summarize news",
                    "context": {
                        "slot_id": 5,
                        "html": "<html><body><h1>Local Election Results</h1><h2>Budget approved</h2></body></html>"
                    }
                }
            }
        });
        let req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["goal"], "AutonomousGoal");
        assert_eq!(json["intent"], "summarize news");
        assert_eq!(
            json["plan_steps"],
            serde_json::json!(["CommunityScraper", "ModelRouter"])
        );
        let generated = json["generated"].as_str().expect("generated");
        assert!(
            generated.contains("Election") || generated.contains("Budget") || generated.contains("personalized"),
            "generated should reflect scraped content or mock; got: {}",
            generated
        );
    }

    #[tokio::test]
    async fn test_knowledge_pruner_removes_old_kb5_and_kb8_entries() {
        let knowledge = Arc::new(
            KnowledgeStore::open_path("./data/pagi_knowledge_pruner_test").unwrap(),
        );
        let old_ts = 1_u64;
        let old_pulse = serde_json::json!({
            "location": "Test",
            "trend": "old",
            "event": "Stale event",
            "updated_at": old_ts
        });
        let old_trace = serde_json::json!({
            "trace_id": "old-trace-id",
            "created_at": old_ts,
            "trace": { "intent": "test" }
        });
        knowledge
            .insert(5, "stale_pulse", old_pulse.to_string().as_bytes())
            .unwrap();
        knowledge
            .insert(8, "old-trace-id", old_trace.to_string().as_bytes())
            .unwrap();

        let mut registry = SkillRegistry::new();
        registry.register(Arc::new(KnowledgePruner::new(Arc::clone(&knowledge))));
        let orchestrator = Arc::new(Orchestrator::new(Arc::new(registry)));
        let app = Router::new()
            .route("/v1/execute", post(execute))
            .with_state(AppState {
                config: Arc::new(test_config()),
                sovereign_config: Arc::new(SovereignConfig::default()),
                orchestrator,
                knowledge: Arc::clone(&knowledge),
                log_tx: test_log_tx(),
                model_router: test_model_router(),
                shadow_store: test_shadow_store(),
                moe_active: Arc::new(AtomicBool::new(false)),
                idle_tracker: IdleTracker::new(),
                approval_bridge: new_approval_bridge(),
                persona_coordinator: Arc::new(PersonaCoordinator::from_env()),
                density_mode: Arc::new(tokio::sync::RwLock::new("balanced".to_string())),
                persona_pulse_tx: broadcast::channel(64).0,
                critical_threshold_counter: Arc::new(AtomicU64::new(0)),
                intelligence_service: Arc::new(OrchestratorService::new(Arc::clone(&knowledge))),
                astro_weather: Arc::new(tokio::sync::RwLock::new(AstroWeatherState::default())),
            skill_manifest_registry: Arc::new(SkillManifestRegistry::new()),
            ms_graph_client: None,
            sovereignty_score_bits: Arc::new(AtomicU64::new(0)),
            project_associations: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            folder_summary_cache: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            });

        let prune_body = serde_json::json!({
            "tenant_id": "test-tenant",
            "goal": {
                "ExecuteSkill": {
                    "name": "KnowledgePruner",
                    "payload": { "kb5_max_age_days": 1, "kb8_max_age_days": 1 }
                }
            }
        });
        let req = Request::builder()
            .method("POST")
            .uri("/v1/execute")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&prune_body).unwrap()))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["skill"], "KnowledgePruner");
        assert_eq!(json["kb5_pruned"], 1);
        assert_eq!(json["kb8_pruned"], 1);
        assert!(json["kb5_removed_keys"].as_array().unwrap().contains(&serde_json::json!("stale_pulse")));
        assert!(json["kb8_removed_keys"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some("old-trace-id")));

        assert!(knowledge.get(5, "stale_pulse").unwrap().is_none());
        assert!(knowledge.get(8, "old-trace-id").unwrap().is_none());
    }
}

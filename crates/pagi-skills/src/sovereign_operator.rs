//! Sovereign Operator: Unified Integration Layer
//!
//! This module provides the unified integration of the Sovereign Operator with
//! Slot 1 (Gateway/Interface) and Slot 2 (Oikos/Workspace).
//!
//! ## Architecture
//!
//! The Sovereign Operator consolidates the "Shell" and "Native" approaches
//! into a single Action Layer:
//!
//! | Component | Logic | Responsibility |
//! | --- | --- | --- |
//! | Native Hand (sysinfo) | `sysinfo` / `walkdir` | Silent, fast telemetry (CPU, RAM, File Trees). |
//! | Shell Hand (PowerShell) | `std::process::Command` | High-level system actions (Git, Cargo, Networking). |
//! | Safety Interlock | `tokio::sync::mpsc` | The "Y/N" confirmation prompt in your terminal. |
//!
//! ## Integration Points
//!
//! 1. **Slot 1 (Gateway)**: The Sovereign Operator is registered as skills
//!    in the SkillRegistry, allowing the Orchestrator to dispatch system commands.
//! 2. **Slot 2 (Oikos)**: Tool outputs are indexed into LanceDB for
//!    semantic memory and "Reflexion" - the ability to reflect on past actions.
//! 3. **Recursive Compiler**: When the Agent identifies a missing skill, it can
//!    generate Rust code, compile it, and hot-swap the new .dll into the SkillRegistry.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use pagi_core::{AgentSkill, KnowledgeStore, SkillRegistry, TenantContext};
use pagi_evolution::{
    ApprovalGate, ChangeSeverity, Compiler, ProposedChange,
    RollbackConfig, RollbackManager, SkillError, SkillLoader,
};

use crate::system::{
    CommandResult, ExecutionError, FileSystem, ShellExecutor, SystemSnapshot,
    SystemTelemetry, TerminalGuard,
};
use crate::rig_tools::{RigTool, RigToolRegistry};
use crate::tool_memory::{ToolExecutionRecord, ToolMemoryManager};

// ---------------------------------------------------------------------------
// Sovereign Operator Configuration
// ---------------------------------------------------------------------------

/// Configuration for the Sovereign Operator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignOperatorConfig {
    /// Whether safety interlock is enabled
    pub safety_enabled: bool,
    /// Whether tool memory is enabled (LanceDB)
    pub tool_memory_enabled: bool,
    /// Whether recursive compilation is enabled
    pub recursive_compilation_enabled: bool,
    /// Path to LanceDB database for tool memory
    pub tool_memory_path: String,
    /// Path to workspace directory
    pub workspace_path: String,
}

impl Default for SovereignOperatorConfig {
    fn default() -> Self {
        Self {
            safety_enabled: true,
            tool_memory_enabled: true,
            recursive_compilation_enabled: true,
            tool_memory_path: "./data/pagi_tool_memory".to_string(),
            workspace_path: ".".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Sovereign Operator
// ---------------------------------------------------------------------------

/// The Sovereign Operator: Unified system execution layer
///
/// This is the main entry point for all Sovereign Operator functionality.
/// It integrates:
/// - Cross-platform shell execution with safety interlocks
/// - Native system telemetry
/// - File system operations
/// - Tool memory for semantic search
/// - Recursive compilation for self-evolution
pub struct SovereignOperator {
    /// Configuration
    config: SovereignOperatorConfig,
    /// Terminal guard for safety interlocks
    terminal_guard: Arc<TerminalGuard>,
    /// Shell executor for command execution
    shell_executor: Arc<ShellExecutor>,
    /// System telemetry for native OS information
    system_telemetry: Arc<SystemTelemetry>,
    /// File system for workspace operations
    file_system: Arc<FileSystem>,
    /// Rig tool registry for LLM tool calling
    rig_tool_registry: Arc<RigToolRegistry>,
    /// Tool memory manager for semantic indexing
    tool_memory: Option<Arc<ToolMemoryManager>>,
    /// Skill loader for dynamic skill hot-reload
    skill_loader: Arc<SkillLoader>,
    /// Rollback manager for evolutionary versioning
    rollback_manager: Arc<RollbackManager>,
    /// Approval gate for human-in-the-loop authorization
    approval_gate: Arc<ApprovalGate>,
    /// Knowledge store for KB-08 logging (optional)
    knowledge_store: Option<Arc<KnowledgeStore>>,
    /// Thread-safe runtime toggle for Forge safety (can be flipped by kill switch)
    forge_safety_atomic: Arc<AtomicBool>,
}

impl SovereignOperator {
    /// Create a new Sovereign Operator with default configuration
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::with_config(SovereignOperatorConfig::default())
    }

    /// Create a new Sovereign Operator with custom configuration
    pub fn with_config(
        config: SovereignOperatorConfig,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let terminal_guard = if config.safety_enabled {
            Arc::new(TerminalGuard::new())
        } else {
            Arc::new(TerminalGuard::unsafe_mode())
        };

        let shell_executor = Arc::new(ShellExecutor::new());
        let system_telemetry = Arc::new(SystemTelemetry::new());
        let file_system = Arc::new(FileSystem::with_workspace(&config.workspace_path));
        let rig_tool_registry = Arc::new(RigToolRegistry::new(
            Arc::clone(&system_telemetry),
            Arc::clone(&shell_executor),
            Arc::clone(&file_system),
        ));

        let tool_memory = if config.tool_memory_enabled {
            Some(Arc::new(ToolMemoryManager::new(&config.tool_memory_path)))
        } else {
            None
        };

        let skill_loader = Arc::new(SkillLoader::new());

        let rollback_config = RollbackConfig {
            patches_dir: std::path::PathBuf::from("crates/pagi-skills/src/generated/patches"),
            artifacts_dir: std::path::PathBuf::from("data/pagi_evolution"),
            max_versions_per_skill: 50,
        };
        let rollback_manager = Arc::new(RollbackManager::new(
            rollback_config,
            Arc::clone(&skill_loader),
        ));

        // Initialize approval gate (enabled by default for safety)
        let approval_gate = Arc::new(ApprovalGate::new(config.safety_enabled));

        // Initialize thread-safe forge safety atomic
        let forge_safety_atomic = Arc::new(AtomicBool::new(config.safety_enabled));

        Ok(Self {
            config,
            terminal_guard,
            shell_executor,
            system_telemetry,
            file_system,
            rig_tool_registry,
            tool_memory,
            skill_loader,
            rollback_manager,
            approval_gate,
            knowledge_store: None, // Will be set via set_knowledge_store()
            forge_safety_atomic,
        })
    }

    /// Initialize the Sovereign Operator
    ///
    /// This should be called after creation to set up any necessary resources.
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Initializing Sovereign Operator");

        // Initialize tool memory if enabled
        if let Some(ref tool_memory) = self.tool_memory {
            tool_memory.initialize().await?;
            info!("Tool memory initialized at: {}", self.config.tool_memory_path);
        }

        info!("Sovereign Operator initialized successfully");
        Ok(())
    }

    /// Register all Sovereign Operator skills with the SkillRegistry
    ///
    /// This integrates the Sovereign Operator with Slot 1 (Gateway).
    pub fn register_skills(&self, registry: &mut SkillRegistry) {
        info!("Registering Sovereign Operator skills");

        // Register system telemetry skill
        registry.register(Arc::new(crate::system::SystemTelemetrySkill::new(
            Arc::clone(&self.system_telemetry),
        )));

        // Register system command skill
        registry.register(Arc::new(crate::system::SystemCommandSkill::new(
            Arc::clone(&self.shell_executor),
        )));

        // Register file system skill
        registry.register(Arc::new(crate::system::FileSystemSkill::new(
            Arc::clone(&self.file_system),
        )));

        info!("Sovereign Operator skills registered");
    }

    /// Execute a shell command with tool memory indexing
    ///
    /// This is the main entry point for command execution. It:
    /// 1. Executes the command with safety approval
    /// 2. Records the execution in tool memory (if enabled)
    /// 3. Returns the result
    pub async fn execute_command(
        &self,
        command: &str,
        reason: Option<&str>,
    ) -> Result<CommandResult, ExecutionError> {
        let start = Instant::now();

        let result = if let Some(r) = reason {
            self.shell_executor.execute_with_reason(command, r).await?
        } else {
            self.shell_executor.execute(command).await?
        };

        let duration = start.elapsed();

        // Index into tool memory if enabled
        if let Some(ref tool_memory) = self.tool_memory {
            let record = ToolExecutionRecord::new(
                "execute_shell_command".to_string(),
                serde_json::json!({ "command": command }),
                result.success,
                result.stdout.clone(),
                if result.stderr.is_empty() {
                    None
                } else {
                    Some(result.stderr.clone())
                },
                duration.as_millis() as u64,
            );

            if let Err(e) = tool_memory.store_execution(&record).await {
                warn!("Failed to store tool execution in memory: {}", e);
            }
        }

        Ok(result)
    }

    /// Get system snapshot with tool memory indexing
    pub async fn get_system_snapshot(&self) -> SystemSnapshot {
        let snapshot = self.system_telemetry.get_system_snapshot().await;

        // Index into tool memory if enabled
        if let Some(ref tool_memory) = self.tool_memory {
            let record = ToolExecutionRecord::new(
                "get_system_snapshot".to_string(),
                serde_json::json!({}),
                true,
                serde_json::to_string(&snapshot).unwrap_or_default(),
                None,
                0,
            );

            if let Err(e) = tool_memory.store_execution(&record).await {
                warn!("Failed to store tool execution in memory: {}", e);
            }
        }

        snapshot
    }

    /// Search tool memory for similar executions
    pub async fn search_tool_memory(
        &self,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<ToolExecutionRecord>, String> {
        if let Some(ref tool_memory) = self.tool_memory {
            tool_memory.search_similar(embedding, limit).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Get recent executions for a specific tool
    pub async fn get_recent_executions(
        &self,
        tool_name: &str,
        limit: usize,
    ) -> Result<Vec<ToolExecutionRecord>, String> {
        if let Some(ref tool_memory) = self.tool_memory {
            tool_memory.get_recent_executions(tool_name, limit).await
        } else {
            Ok(Vec::new())
        }
    }

    /// Compile and load a new skill dynamically with versioned storage.
    ///
    /// This is the "Recursive Compiler" capability with HITL approval gate.
    /// When the Agent identifies a missing skill, it can:
    /// 1. Check genetic memory for evolutionary dead-ends
    /// 2. Request human approval via the Approval Gate (or auto-approve if safety disabled)
    /// 3. Save the versioned patch source
    /// 4. Generate the Rust code and compile it
    /// 5. Use SkillLoader to hot-swap the new .dll into the SkillRegistry
    /// 6. Register the compiled artifact with the RollbackManager
    /// 7. AUTO-REVERT: If compilation fails in autonomous mode, re-enable safety
    pub async fn compile_and_load_skill(
        &self,
        code: &str,
        name: &str,
    ) -> Result<(), SkillError> {
        if !self.config.recursive_compilation_enabled {
            return Err(SkillError::Load(
                "Recursive compilation is disabled".to_string(),
            ));
        }

        info!("🔥 Forge activation requested for skill: {}", name);

        // Check the runtime safety status (thread-safe)
        let safety_enabled = self.is_forge_safety_enabled();

        // Step 0: Check genetic memory for dead-ends
        if let Some(dead_end) = self.rollback_manager.check_dead_end(code) {
            warn!(
                "Evolutionary Dead-End detected for '{}': {} (occurrences: {})",
                name, dead_end.reason, dead_end.occurrence_count
            );
            return Err(SkillError::Load(format!(
                "Evolutionary Dead-End: code hash {} was previously rejected for '{}'. Reason: {}",
                &dead_end.code_hash[..12],
                dead_end.skill_name,
                dead_end.reason
            )));
        }

        // Step 1: Request approval via the Approval Gate (respects runtime safety status)
        let file_path = format!("crates/pagi-skills/src/generated/{}.rs", name);
        let rationale = format!(
            "Phoenix has identified a need for a new skill: '{}'. \
            This skill will be compiled and hot-loaded into the Sovereign Core.",
            name
        );
        
        // Create a simple diff preview (first 20 lines)
        let diff = code
            .lines()
            .take(20)
            .enumerate()
            .map(|(i, line)| format!("+{:4} | {}", i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        
        let diff_preview = if code.lines().count() > 20 {
            format!("{}\n... ({} more lines)", diff, code.lines().count() - 20)
        } else {
            diff
        };

        // Determine severity based on code analysis
        let severity = if code.contains("unsafe") || code.contains("std::process::Command") {
            ChangeSeverity::Critical
        } else if code.contains("File::") || code.contains("std::fs") {
            ChangeSeverity::Warning
        } else {
            ChangeSeverity::Info
        };

        // Create a temporary approval gate that respects the runtime safety status
        let runtime_gate = ApprovalGate::new(safety_enabled);
        let proposed_change = runtime_gate.propose_and_review(
            file_path,
            rationale,
            diff_preview,
            severity,
        ).map_err(|e| SkillError::Load(format!("Approval gate error: {}", e)))?;

        // Log the approval decision to KB-08
        self.log_forge_approval(&proposed_change);

        // Check if the change was authorized
        if proposed_change.status != pagi_evolution::ApprovalStatus::Authorized {
            warn!("❌ Compilation DENIED by Coach Jamey for skill: {}", name);
            return Err(SkillError::Load(format!(
                "Compilation denied by operator for skill: {}",
                name
            )));
        }

        if safety_enabled {
            info!("✅ Compilation AUTHORIZED by Coach Jamey for skill: {}", name);
        } else {
            info!("⚡ Autonomous compilation proceeding for skill: {}", name);
        }

        // Step 2: Save versioned patch source
        let version = self.rollback_manager.save_versioned_patch(
            name,
            code,
            &format!("Compiled skill: {}", name),
            None,
        )?;

        // Step 3: Compile the code (with auto-revert on failure)
        let lib_path = match Compiler::compile_from_string(code, name, None) {
            Ok(path) => path,
            Err(e) => {
                // AUTO-REVERT: If we're in autonomous mode and compilation fails, re-enable safety
                if !safety_enabled {
                    error!("❌ Autonomous compilation FAILED for '{}': {}", name, e);
                    error!("🛡️  AUTO-REVERT: Re-enabling Forge safety governor");
                    self.set_forge_safety(true);
                    
                    return Err(SkillError::Load(format!(
                        "Autonomous compilation failed for '{}'. Safety governor re-engaged. Error: {}",
                        name, e
                    )));
                }
                return Err(e);
            }
        };

        // Step 4: Register the artifact with the RollbackManager
        if let Err(e) = self.rollback_manager.register_artifact(
            name,
            version.timestamp_ms,
            lib_path.clone(),
        ) {
            warn!("Failed to register artifact with RollbackManager: {}", e);
        }

        // Step 5: Load the compiled library
        self.skill_loader.load(&lib_path, name.to_string())?;

        info!("🎉 Successfully compiled and loaded skill: {}", name);
        Ok(())
    }

    /// Get the Rig tool registry
    pub fn rig_tool_registry(&self) -> &Arc<RigToolRegistry> {
        &self.rig_tool_registry
    }

    /// Get the terminal guard
    pub fn terminal_guard(&self) -> &Arc<TerminalGuard> {
        &self.terminal_guard
    }

    /// Get the shell executor
    pub fn shell_executor(&self) -> &Arc<ShellExecutor> {
        &self.shell_executor
    }

    /// Get the system telemetry
    pub fn system_telemetry(&self) -> &Arc<SystemTelemetry> {
        &self.system_telemetry
    }

    /// Get the file system
    pub fn file_system(&self) -> &Arc<FileSystem> {
        &self.file_system
    }

    /// Get the tool memory manager
    pub fn tool_memory(&self) -> &Option<Arc<ToolMemoryManager>> {
        &self.tool_memory
    }

    /// Get the skill loader
    pub fn skill_loader(&self) -> &Arc<SkillLoader> {
        &self.skill_loader
    }

    /// Get the rollback manager
    pub fn rollback_manager(&self) -> &Arc<RollbackManager> {
        &self.rollback_manager
    }

    /// Get the approval gate
    pub fn approval_gate(&self) -> &Arc<ApprovalGate> {
        &self.approval_gate
    }

    /// Set the knowledge store for KB-08 logging
    pub fn set_knowledge_store(&mut self, knowledge_store: Arc<KnowledgeStore>) {
        self.knowledge_store = Some(knowledge_store);
    }

    /// Get the current forge safety status (thread-safe)
    pub fn is_forge_safety_enabled(&self) -> bool {
        self.forge_safety_atomic.load(Ordering::SeqCst)
    }

    /// Set the forge safety status (thread-safe, for kill switch)
    pub fn set_forge_safety(&self, enabled: bool) {
        self.forge_safety_atomic.store(enabled, Ordering::SeqCst);
        info!("🛡️  Forge safety {} by runtime control", if enabled { "ENABLED" } else { "DISABLED" });
        
        // Log the sovereignty change to KB-08
        if let Some(ref knowledge_store) = self.knowledge_store {
            let key = format!("forge_sovereignty_change/{}", chrono::Utc::now().timestamp_millis());
            let value = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "safety_enabled": enabled,
                "trigger": "runtime_control",
                "message": if enabled {
                    "Forge safety re-enabled via kill switch or UI control"
                } else {
                    "Autonomous Evolution Mode activated"
                }
            });
            let value_bytes = serde_json::to_vec(&value).unwrap_or_default();
            
            const SOMA_SLOT: u8 = 8;
            if let Err(e) = knowledge_store.insert(SOMA_SLOT, &key, &value_bytes) {
                warn!("Failed to log sovereignty change to KB-08: {}", e);
            }
        }
    }

    /// Get the forge safety atomic for external monitoring
    pub fn forge_safety_atomic(&self) -> &Arc<AtomicBool> {
        &self.forge_safety_atomic
    }

    /// Log a Forge approval event to KB-08 (Soma)
    fn log_forge_approval(&self, change: &ProposedChange) {
        if let Some(ref knowledge_store) = self.knowledge_store {
            let (key, value) = pagi_evolution::create_kb08_log_entry(change);
            let value_bytes = serde_json::to_vec(&value).unwrap_or_default();
            
            const SOMA_SLOT: u8 = 8;
            if let Err(e) = knowledge_store.insert(SOMA_SLOT, &key, &value_bytes) {
                warn!("Failed to log Forge approval to KB-08: {}", e);
            } else {
                info!("📝 Forge approval logged to KB-08: {} ({})", change.file_path, change.status);
            }
        }
    }
}

impl Default for SovereignOperator {
    fn default() -> Self {
        Self::new().expect("Failed to create SovereignOperator")
    }
}

// ---------------------------------------------------------------------------
// Sovereign Operator Skill
// ---------------------------------------------------------------------------

/// AgentSkill wrapper for the Sovereign Operator
///
/// This allows the Orchestrator to dispatch to the Sovereign Operator
/// through the standard skill interface.
pub struct SovereignOperatorSkill {
    operator: Arc<SovereignOperator>,
}

impl SovereignOperatorSkill {
    pub fn new(operator: Arc<SovereignOperator>) -> Self {
        Self { operator }
    }
}

#[async_trait::async_trait]
impl AgentSkill for SovereignOperatorSkill {
    fn name(&self) -> &str {
        "SovereignOperator"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("Missing payload")?;
        let action = payload
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'action' field")?;

        match action {
            "execute_command" => {
                let command = payload
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'command' field")?;
                let reason = payload.get("reason").and_then(|v| v.as_str());

                let result = self
                    .operator
                    .execute_command(command, reason)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                Ok(serde_json::to_value(result)?)
            }
            "get_system_snapshot" => {
                let snapshot = self.operator.get_system_snapshot().await;
                Ok(serde_json::to_value(snapshot)?)
            }
            "compile_skill" => {
                let code = payload
                    .get("code")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'code' field")?;
                let name = payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'name' field")?;

                self.operator
                    .compile_and_load_skill(code, name)
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

                Ok(serde_json::json!({
                    "status": "success",
                    "message": format!("Skill '{}' compiled and loaded successfully", name)
                }))
            }
            "rollback_skill" => {
                let skill = payload
                    .get("skill")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'skill' field")?;
                let target_timestamp = payload
                    .get("target_timestamp")
                    .and_then(|v| v.as_i64());
                let reason = payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Operator-initiated rollback");

                match self.operator.rollback_manager().rollback_skill(
                    skill,
                    target_timestamp,
                    reason,
                ) {
                    Ok(version) => Ok(serde_json::json!({
                        "status": "success",
                        "message": format!("Rolled back '{}' to timestamp {}", skill, version.timestamp_ms),
                        "version": {
                            "timestamp_ms": version.timestamp_ms,
                            "code_hash": version.code_hash,
                            "description": version.description,
                        }
                    })),
                    Err(e) => Ok(serde_json::json!({
                        "status": "error",
                        "message": format!("Rollback failed: {}", e)
                    })),
                }
            }
            "patch_history" => {
                let history = self.operator.rollback_manager().get_full_history();
                let entries: Vec<serde_json::Value> = history
                    .iter()
                    .take(50)
                    .map(|v| serde_json::json!({
                        "skill_name": v.skill_name,
                        "timestamp_ms": v.timestamp_ms,
                        "code_hash": v.code_hash,
                        "is_active": v.is_active,
                        "status": format!("{:?}", v.status),
                        "description": v.description,
                    }))
                    .collect();
                Ok(serde_json::json!({
                    "status": "success",
                    "total": history.len(),
                    "history": entries,
                }))
            }
            "get_forge_safety_status" => {
                let enabled = self.operator.is_forge_safety_enabled();
                Ok(serde_json::json!({
                    "status": "success",
                    "forge_safety_enabled": enabled,
                    "mode": if enabled { "HITL" } else { "AUTONOMOUS" }
                }))
            }
            "set_forge_safety" => {
                let enabled = payload
                    .get("enabled")
                    .and_then(|v| v.as_bool())
                    .ok_or("Missing 'enabled' boolean field")?;
                
                self.operator.set_forge_safety(enabled);
                
                Ok(serde_json::json!({
                    "status": "success",
                    "forge_safety_enabled": enabled,
                    "message": if enabled {
                        "Forge safety ENABLED - HITL approval required"
                    } else {
                        "Forge safety DISABLED - Autonomous evolution mode active"
                    }
                }))
            }
            _ => Err(format!("Unknown action: {}", action).into()),
        }
    }
}


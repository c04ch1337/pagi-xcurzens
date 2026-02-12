//! Sovereign Operator: Cross-Platform System Execution Layer
//!
//! This module provides the "Hands" of the Sovereign System Operator:
//! - Native Hand (sysinfo): Silent, fast telemetry (CPU, RAM, File Trees)
//! - Shell Hand (PowerShell/sh): High-level system actions (Git, Cargo, Networking)
//! - Safety Interlock: TerminalGuard for Y/N confirmation prompts
//!
//! ## Architecture
//!
//! The system is designed as a unified Action Layer that consolidates "Shell" and "Native"
//! approaches into a single coherent interface. This eliminates redundancy and provides
//! a clean abstraction for the Orchestrator to interact with the OS.
//!
//! ## Cross-Platform Support
//!
//! - **Windows**: Uses PowerShell with NoProfile and Bypass execution policies
//! - **Unix/Linux/macOS**: Uses standard sh/bash
//!
//! ## Safety
//!
//! All command execution goes through TerminalGuard, which requires explicit user
//! confirmation before executing potentially dangerous commands.

use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sysinfo::System;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use pagi_core::TenantContext;

// ---------------------------------------------------------------------------
// TerminalGuard: Safety Interlock for Command Execution
// ---------------------------------------------------------------------------

/// TerminalGuard provides a safety interlock that pauses execution and requires
/// explicit user confirmation before running commands.
///
/// This is the critical safety mechanism that prevents the Sovereign Operator
/// from executing potentially dangerous commands without user consent.
#[derive(Clone)]
pub struct TerminalGuard {
    /// Whether the guard is enabled (can be disabled for trusted environments)
    enabled: Arc<RwLock<bool>>,
    /// History of approved commands for audit purposes
    approval_history: Arc<RwLock<Vec<ApprovalRecord>>>,
}

/// Record of a command approval for audit purposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub command: String,
    pub approved: bool,
    pub reason: Option<String>,
}

impl TerminalGuard {
    /// Create a new TerminalGuard with safety enabled by default
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(true)),
            approval_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new TerminalGuard with safety disabled (use with caution)
    pub fn unsafe_mode() -> Self {
        warn!("TerminalGuard created in UNSAFE mode - commands will execute without confirmation!");
        Self {
            enabled: Arc::new(RwLock::new(false)),
            approval_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Enable or disable the safety guard
    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
        if enabled {
            info!("TerminalGuard safety enabled");
        } else {
            warn!("TerminalGuard safety DISABLED - commands will execute without confirmation!");
        }
    }

    /// Check if the guard is currently enabled
    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Request approval for a command execution
    ///
    /// Returns `true` if the command should be executed, `false` otherwise.
    /// When enabled, this will pause and prompt the user for confirmation.
    pub async fn request_approval(&self, command: &str) -> Result<bool, io::Error> {
        let enabled = *self.enabled.read().await;

        if !enabled {
            // In unsafe mode, auto-approve
            self.record_approval(command, true, None).await;
            return Ok(true);
        }

        // Print the approval prompt
        println!();
        println!("═══════════════════════════════════════════════════════════════");
        println!("  ⚠️  ACTION REQUIRED: Agent wants to run a command");
        println!("═══════════════════════════════════════════════════════════════");
        println!();
        println!("  Command: {}", command);
        println!();
        println!("  Allow this command? (y/n): ");

        // Flush stdout to ensure the prompt is displayed
        io::stdout().flush()?;

        // Read user input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let approved = input.trim().to_lowercase() == "y";

        if approved {
            println!("  ✓ Command approved");
        } else {
            println!("  ✗ Command denied");
        }

        println!("═══════════════════════════════════════════════════════════════");
        println!();

        self.record_approval(command, approved, None).await;
        Ok(approved)
    }

    /// Request approval with a custom reason/context
    pub async fn request_approval_with_reason(
        &self,
        command: &str,
        reason: &str,
    ) -> Result<bool, io::Error> {
        let enabled = *self.enabled.read().await;

        if !enabled {
            self.record_approval(command, true, Some(reason.to_string())).await;
            return Ok(true);
        }

        println!();
        println!("═══════════════════════════════════════════════════════════════");
        println!("  ⚠️  ACTION REQUIRED: Agent wants to run a command");
        println!("═══════════════════════════════════════════════════════════════");
        println!();
        println!("  Reason: {}", reason);
        println!("  Command: {}", command);
        println!();
        println!("  Allow this command? (y/n): ");

        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let approved = input.trim().to_lowercase() == "y";

        if approved {
            println!("  ✓ Command approved");
        } else {
            println!("  ✗ Command denied");
        }

        println!("═══════════════════════════════════════════════════════════════");
        println!();

        self.record_approval(command, approved, Some(reason.to_string())).await;
        Ok(approved)
    }

    /// Record an approval decision for audit purposes
    async fn record_approval(&self, command: &str, approved: bool, reason: Option<String>) {
        let record = ApprovalRecord {
            timestamp: chrono::Utc::now(),
            command: command.to_string(),
            approved,
            reason,
        };

        let mut history = self.approval_history.write().await;
        history.push(record);

        // Keep only the last 1000 records
        if history.len() > 1000 {
            let excess = history.len() - 1000;
            history.drain(0..excess);
        }
    }

    /// Get the approval history
    pub async fn get_approval_history(&self) -> Vec<ApprovalRecord> {
        self.approval_history.read().await.clone()
    }
}

impl Default for TerminalGuard {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Cross-Platform Shell Executor
// ---------------------------------------------------------------------------

/// ShellExecutor provides cross-platform command execution with safety interlocks.
///
/// This is the "Shell Hand" of the Sovereign Operator, handling high-level
/// system actions like Git, Cargo, Networking, etc.
pub struct ShellExecutor {
    /// Safety guard for command approval
    guard: TerminalGuard,
    /// Working directory for commands
    working_dir: Option<PathBuf>,
    /// Environment variables to set for commands
    env_vars: Vec<(String, String)>,
}

impl ShellExecutor {
    /// Create a new ShellExecutor with default settings
    pub fn new() -> Self {
        Self {
            guard: TerminalGuard::new(),
            working_dir: None,
            env_vars: Vec::new(),
        }
    }

    /// Create a new ShellExecutor in unsafe mode (no confirmation prompts)
    pub fn unsafe_mode() -> Self {
        Self {
            guard: TerminalGuard::unsafe_mode(),
            working_dir: None,
            env_vars: Vec::new(),
        }
    }

    /// Set the working directory for commands
    pub fn with_working_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Add an environment variable to be set for all commands
    pub fn with_env_var(mut self, key: String, value: String) -> Self {
        self.env_vars.push((key, value));
        self
    }

    /// Get a reference to the terminal guard
    pub fn guard(&self) -> &TerminalGuard {
        &self.guard
    }

    /// Execute a command synchronously with safety approval
    ///
    /// This is the main entry point for command execution. It will:
    /// 1. Request approval from the user (if guard is enabled)
    /// 2. Execute the command using the appropriate shell for the platform
    /// 3. Return the output
    pub async fn execute(&self, command: &str) -> Result<CommandResult, ExecutionError> {
        // Request approval
        let approved = self
            .guard
            .request_approval(command)
            .await
            .map_err(|e| ExecutionError::IoError(e.to_string()))?;

        if !approved {
            return Err(ExecutionError::Denied(format!(
                "Command denied by user: {}",
                command
            )));
        }

        // Execute the command
        let output = self.execute_command(command)?;

        Ok(CommandResult {
            command: command.to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        })
    }

    /// Execute a command with a custom reason/context
    pub async fn execute_with_reason(
        &self,
        command: &str,
        reason: &str,
    ) -> Result<CommandResult, ExecutionError> {
        let approved = self
            .guard
            .request_approval_with_reason(command, reason)
            .await
            .map_err(|e| ExecutionError::IoError(e.to_string()))?;

        if !approved {
            return Err(ExecutionError::Denied(format!(
                "Command denied by user: {}",
                command
            )));
        }

        let output = self.execute_command(command)?;

        Ok(CommandResult {
            command: command.to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        })
    }

    /// Execute a command without approval (use with extreme caution)
    pub fn execute_unsafe(&self, command: &str) -> Result<CommandResult, ExecutionError> {
        let output = self.execute_command(command)?;

        Ok(CommandResult {
            command: command.to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            success: output.status.success(),
        })
    }

    /// Internal method to execute a command using the appropriate shell
    fn execute_command(&self, command: &str) -> Result<Output, ExecutionError> {
        match env::consts::OS {
            "windows" => self.execute_windows(command),
            _ => self.execute_unix(command),
        }
    }

    /// Execute a command on Windows using PowerShell
    fn execute_windows(&self, command: &str) -> Result<Output, ExecutionError> {
        debug!("Executing Windows command: {}", command);

        let mut cmd = Command::new("powershell.exe");
        cmd.arg("-NoProfile")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(command);

        // Set working directory if specified
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Capture both stdout and stderr
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| ExecutionError::ExecutionFailed(format!("Failed to execute PowerShell command: {}", e)))?;

        Ok(output)
    }

    /// Execute a command on Unix/Linux/macOS using sh
    fn execute_unix(&self, command: &str) -> Result<Output, ExecutionError> {
        debug!("Executing Unix command: {}", command);

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(command);

        // Set working directory if specified
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Capture both stdout and stderr
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| ExecutionError::ExecutionFailed(format!("Failed to execute sh command: {}", e)))?;

        Ok(output)
    }
}

impl Default for ShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Native System Telemetry (sysinfo)
// ---------------------------------------------------------------------------

/// SystemTelemetry provides native, fast system information using sysinfo.
///
/// This is the "Native Hand" of the Sovereign Operator, providing silent
/// telemetry about CPU, RAM, processes, and file systems.
pub struct SystemTelemetry {
    sys: Arc<RwLock<System>>,
}

impl SystemTelemetry {
    /// Create a new SystemTelemetry instance
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        Self {
            sys: Arc::new(RwLock::new(sys)),
        }
    }

    /// Refresh all system information
    pub async fn refresh(&self) {
        let mut sys = self.sys.write().await;
        sys.refresh_all();
    }

    /// Get CPU usage information
    pub async fn get_cpu_info(&self) -> CpuInfo {
        let sys = self.sys.read().await;
        CpuInfo {
            name: sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default(),
            cores: sys.cpus().len(),
            usage: sys.global_cpu_info().cpu_usage(),
        }
    }

    /// Get memory information
    pub async fn get_memory_info(&self) -> MemoryInfo {
        let sys = self.sys.read().await;
        MemoryInfo {
            total_memory: sys.total_memory(),
            available_memory: sys.available_memory(),
            used_memory: sys.used_memory(),
            total_swap: sys.total_swap(),
            used_swap: sys.used_swap(),
        }
    }

    /// Get process list
    pub async fn get_processes(&self) -> Vec<ProcessInfo> {
        let sys = self.sys.read().await;
        sys.processes()
            .iter()
            .map(|(pid, process)| ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string(),
                cpu_usage: process.cpu_usage(),
                memory: process.memory(),
                exe: process.exe().map(|p| p.to_string_lossy().to_string()),
            })
            .collect()
    }

    /// Get disk information
    pub async fn get_disks(&self) -> Vec<DiskInfo> {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        disks
            .iter()
            .map(|disk| DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                file_system: disk.file_system().to_string_lossy().to_string(),
                total_space: disk.total_space(),
                available_space: disk.available_space(),
                is_removable: disk.is_removable(),
            })
            .collect()
    }

    /// Get comprehensive system snapshot
    pub async fn get_system_snapshot(&self) -> SystemSnapshot {
        self.refresh().await;

        SystemSnapshot {
            cpu: self.get_cpu_info().await,
            memory: self.get_memory_info().await,
            processes: self.get_processes().await,
            disks: self.get_disks().await,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl Default for SystemTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// File System Operations (walkdir)
// ---------------------------------------------------------------------------

/// FileSystem provides file system operations using walkdir.
///
/// This provides efficient recursive directory traversal and file operations.
pub struct FileSystem {
    /// Base workspace directory (for safety)
    workspace_dir: PathBuf,
}

impl FileSystem {
    /// Create a new FileSystem with the current directory as workspace
    pub fn new() -> Result<Self, io::Error> {
        let workspace_dir = env::current_dir()?;
        Ok(Self { workspace_dir })
    }

    /// Create a new FileSystem with a specific workspace directory
    pub fn with_workspace<P: AsRef<Path>>(workspace: P) -> Self {
        Self {
            workspace_dir: workspace.as_ref().to_path_buf(),
        }
    }

    /// List files in a directory recursively
    pub fn list_files<P: AsRef<Path>>(&self, path: P) -> Result<Vec<FileInfo>, io::Error> {
        let full_path = self.resolve_path(path)?;
        let mut files = Vec::new();

        for entry in walkdir::WalkDir::new(&full_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let metadata = entry.metadata().ok();
            let file_type = metadata.as_ref().map(|m| m.file_type());

            files.push(FileInfo {
                path: entry.path().to_string_lossy().to_string(),
                name: entry.file_name().to_string_lossy().to_string(),
                depth: entry.depth(),
                is_dir: file_type.map(|ft| ft.is_dir()).unwrap_or(false),
                is_file: file_type.map(|ft| ft.is_file()).unwrap_or(false),
                is_symlink: file_type.map(|ft| ft.is_symlink()).unwrap_or(false),
                size: metadata.as_ref().map(|m| m.len()),
                modified: metadata
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t)),
            });
        }

        Ok(files)
    }

    /// Read a file's contents
    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Result<String, io::Error> {
        let full_path = self.resolve_path(path)?;
        std::fs::read_to_string(full_path)
    }

    /// Write content to a file
    pub fn write_file<P: AsRef<Path>>(&self, path: P, content: &str) -> Result<(), io::Error> {
        let full_path = self.resolve_path(path)?;

        // Create parent directories if they don't exist
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(full_path, content)
    }

    /// Delete a file or directory
    pub fn delete<P: AsRef<Path>>(&self, path: P) -> Result<(), io::Error> {
        let full_path = self.resolve_path(path)?;

        if full_path.is_dir() {
            std::fs::remove_dir_all(full_path)
        } else {
            std::fs::remove_file(full_path)
        }
    }

    /// Check if a path exists
    pub fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.resolve_path(path).map(|p| p.exists()).unwrap_or(false)
    }

    /// Get file metadata
    pub fn metadata<P: AsRef<Path>>(&self, path: P) -> Result<FileMetadata, io::Error> {
        let full_path = self.resolve_path(path)?;
        let metadata = std::fs::metadata(&full_path)?;

        Ok(FileMetadata {
            path: full_path.to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            is_file: metadata.is_file(),
            is_symlink: metadata.is_symlink(),
            size: metadata.len(),
            modified: metadata
                .modified()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t)),
            created: metadata
                .created()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t)),
            accessed: metadata
                .accessed()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t)),
        })
    }

    /// Resolve a path relative to the workspace directory
    fn resolve_path<P: AsRef<Path>>(&self, path: P) -> Result<PathBuf, io::Error> {
        let path = path.as_ref();

        if path.is_absolute() {
            // For absolute paths, verify they're within the workspace
            let canonical = std::fs::canonicalize(path)?;
            let workspace_canonical = std::fs::canonicalize(&self.workspace_dir)?;

            if !canonical.starts_with(&workspace_canonical) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "Path '{}' is outside workspace directory '{}'",
                        path.display(),
                        self.workspace_dir.display()
                    ),
                ));
            }

            Ok(canonical)
        } else {
            // For relative paths, join with workspace
            let full_path = self.workspace_dir.join(path);
            Ok(std::fs::canonicalize(full_path)?)
        }
    }
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new().expect("Failed to create FileSystem with current directory")
    }
}

// ---------------------------------------------------------------------------
// Data Types
// ---------------------------------------------------------------------------

/// Result of a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub command: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Error types for command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionError {
    Denied(String),
    ExecutionFailed(String),
    IoError(String),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::Denied(msg) => write!(f, "Command denied: {}", msg),
            ExecutionError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            ExecutionError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for ExecutionError {}

/// CPU information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub name: String,
    pub cores: usize,
    pub usage: f32,
}

/// Memory information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_memory: u64,
    pub available_memory: u64,
    pub used_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
}

/// Process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory: u64,
    pub exe: Option<String>,
}

/// Disk information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub file_system: String,
    pub total_space: u64,
    pub available_space: u64,
    pub is_removable: bool,
}

/// Complete system snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub processes: Vec<ProcessInfo>,
    pub disks: Vec<DiskInfo>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// File information from walkdir
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size: Option<u64>,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub accessed: Option<chrono::DateTime<chrono::Utc>>,
}

// ---------------------------------------------------------------------------
// AgentSkill Implementations
// ---------------------------------------------------------------------------

/// SystemCommandSkill: Execute shell commands through the Sovereign Operator
pub struct SystemCommandSkill {
    executor: Arc<ShellExecutor>,
}

impl SystemCommandSkill {
    pub fn new(executor: Arc<ShellExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait::async_trait]
impl pagi_core::AgentSkill for SystemCommandSkill {
    fn name(&self) -> &str {
        "SystemCommand"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("Missing payload")?;
        let command = payload
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'command' field")?;

        let reason = payload.get("reason").and_then(|v| v.as_str());

        let result = if let Some(r) = reason {
            self.executor.execute_with_reason(command, r).await?
        } else {
            self.executor.execute(command).await?
        };

        Ok(serde_json::to_value(result)?)
    }
}

/// SystemTelemetrySkill: Get system information
pub struct SystemTelemetrySkill {
    telemetry: Arc<SystemTelemetry>,
}

impl SystemTelemetrySkill {
    pub fn new(telemetry: Arc<SystemTelemetry>) -> Self {
        Self { telemetry }
    }
}

#[async_trait::async_trait]
impl pagi_core::AgentSkill for SystemTelemetrySkill {
    fn name(&self) -> &str {
        "SystemTelemetry"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.unwrap_or(serde_json::json!({}));
        let query_type = payload
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("snapshot");

        let result = match query_type {
            "snapshot" => serde_json::to_value(self.telemetry.get_system_snapshot().await)?,
            "cpu" => serde_json::to_value(self.telemetry.get_cpu_info().await)?,
            "memory" => serde_json::to_value(self.telemetry.get_memory_info().await)?,
            "processes" => serde_json::to_value(self.telemetry.get_processes().await)?,
            "disks" => serde_json::to_value(self.telemetry.get_disks().await)?,
            _ => {
                return Err(format!("Unknown telemetry type: {}", query_type).into());
            }
        };

        Ok(result)
    }
}

/// FileSystemSkill: File system operations
pub struct FileSystemSkill {
    fs: Arc<FileSystem>,
}

impl FileSystemSkill {
    pub fn new(fs: Arc<FileSystem>) -> Self {
        Self { fs }
    }
}

#[async_trait::async_trait]
impl pagi_core::AgentSkill for FileSystemSkill {
    fn name(&self) -> &str {
        "FileSystem"
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("Missing payload")?;
        let operation = payload
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'operation' field")?;

        let result = match operation {
            "list" => {
                let path = payload.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                serde_json::to_value(self.fs.list_files(path)?)?
            }
            "read" => {
                let path = payload
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path' field")?;
                serde_json::json!({ "content": self.fs.read_file(path)? })
            }
            "write" => {
                let path = payload
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path' field")?;
                let content = payload
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'content' field")?;
                self.fs.write_file(path, content)?;
                serde_json::json!({ "status": "success", "path": path })
            }
            "delete" => {
                let path = payload
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path' field")?;
                self.fs.delete(path)?;
                serde_json::json!({ "status": "success", "path": path })
            }
            "exists" => {
                let path = payload
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path' field")?;
                serde_json::json!({ "exists": self.fs.exists(path) })
            }
            "metadata" => {
                let path = payload
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path' field")?;
                serde_json::to_value(self.fs.metadata(path)?)?
            }
            _ => {
                return Err(format!("Unknown operation: {}", operation).into());
            }
        };

        Ok(result)
    }
}


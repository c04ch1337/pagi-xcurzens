//! Rig Tool Integration for Sovereign Operator
//!
//! This module provides Rig-compatible tool wrappers for the Sovereign Operator's
//! system capabilities. These tools allow the Orchestrator to "see" OS stats (RAM/CPU)
//! as part of its prompt context automatically, and execute shell commands with
//! safety interlocks.
//!
//! ## Architecture
//!
//! The Rig tools are designed to integrate seamlessly with the Rig framework's
//! tool-calling logic. Each tool implements the Rig tool interface and wraps
//! the corresponding Sovereign Operator capability.
//!
//! ## Tool Categories
//!
//! 1. **Telemetry Tools**: Read-only system information (CPU, RAM, processes, disks)
//! 2. **Execution Tools**: Shell command execution with safety approval
//! 3. **File System Tools**: File operations with workspace boundaries
//! 4. **Evolution Tools**: Dynamic skill compilation and hot-reload

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::system::{
    ExecutionError, FileSystem, ShellExecutor,
    SystemTelemetry,
};
use pagi_core::TenantContext;

// ---------------------------------------------------------------------------
// Rig Tool Trait Definition
// ---------------------------------------------------------------------------

/// Trait for Rig-compatible tools.
///
/// Note: `execute` is synchronous to keep the trait dyn-compatible.
/// Implementations that need async should use `tokio::runtime::Handle::current().block_on()`
/// or store pre-computed results.
#[async_trait::async_trait]
pub trait RigTool: Send + Sync {
    /// Get the tool's name
    fn name(&self) -> &str;

    /// Get the tool's description for the LLM
    fn description(&self) -> &str;

    /// Get the tool's JSON schema for parameters
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given parameters
    async fn execute(
        &self,
        ctx: &TenantContext,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError>;
}

/// Error type for Rig tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RigToolError {
    /// Invalid parameters
    InvalidParameters(String),
    /// Execution failed
    ExecutionFailed(String),
    /// Permission denied
    PermissionDenied(String),
    /// Tool not available
    NotAvailable(String),
}

impl std::fmt::Display for RigToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RigToolError::InvalidParameters(msg) => write!(f, "Invalid parameters: {}", msg),
            RigToolError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            RigToolError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            RigToolError::NotAvailable(msg) => write!(f, "Tool not available: {}", msg),
        }
    }
}

impl std::error::Error for RigToolError {}

// ---------------------------------------------------------------------------
// Telemetry Tools
// ---------------------------------------------------------------------------

/// GetSystemSnapshot: Retrieve complete system telemetry
pub struct GetSystemSnapshot {
    telemetry: Arc<SystemTelemetry>,
}

impl GetSystemSnapshot {
    pub fn new(telemetry: Arc<SystemTelemetry>) -> Self {
        Self { telemetry }
    }
}

#[async_trait::async_trait]
impl RigTool for GetSystemSnapshot {
    fn name(&self) -> &str {
        "get_system_snapshot"
    }

    fn description(&self) -> &str {
        "Retrieve a complete snapshot of system telemetry including CPU, memory, processes, and disk information. This is a read-only operation that provides real-time system state."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        debug!("Executing GetSystemSnapshot tool");
        let snapshot = self.telemetry.get_system_snapshot().await;
        serde_json::to_value(snapshot).map_err(|e| {
            RigToolError::ExecutionFailed(format!("Failed to serialize snapshot: {}", e))
        })
    }
}

/// GetCpuInfo: Retrieve CPU information
pub struct GetCpuInfo {
    telemetry: Arc<SystemTelemetry>,
}

impl GetCpuInfo {
    pub fn new(telemetry: Arc<SystemTelemetry>) -> Self {
        Self { telemetry }
    }
}

#[async_trait::async_trait]
impl RigTool for GetCpuInfo {
    fn name(&self) -> &str {
        "get_cpu_info"
    }

    fn description(&self) -> &str {
        "Retrieve CPU information including processor name, core count, and current usage percentage."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        debug!("Executing GetCpuInfo tool");
        let cpu_info = self.telemetry.get_cpu_info().await;
        serde_json::to_value(cpu_info).map_err(|e| {
            RigToolError::ExecutionFailed(format!("Failed to serialize CPU info: {}", e))
        })
    }
}

/// GetMemoryInfo: Retrieve memory information
pub struct GetMemoryInfo {
    telemetry: Arc<SystemTelemetry>,
}

impl GetMemoryInfo {
    pub fn new(telemetry: Arc<SystemTelemetry>) -> Self {
        Self { telemetry }
    }
}

#[async_trait::async_trait]
impl RigTool for GetMemoryInfo {
    fn name(&self) -> &str {
        "get_memory_info"
    }

    fn description(&self) -> &str {
        "Retrieve memory information including total, available, and used RAM, as well as swap usage."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        debug!("Executing GetMemoryInfo tool");
        let memory_info = self.telemetry.get_memory_info().await;
        serde_json::to_value(memory_info).map_err(|e| {
            RigToolError::ExecutionFailed(format!("Failed to serialize memory info: {}", e))
        })
    }
}

/// GetProcessList: Retrieve running processes
pub struct GetProcessList {
    telemetry: Arc<SystemTelemetry>,
}

impl GetProcessList {
    pub fn new(telemetry: Arc<SystemTelemetry>) -> Self {
        Self { telemetry }
    }
}

#[async_trait::async_trait]
impl RigTool for GetProcessList {
    fn name(&self) -> &str {
        "get_process_list"
    }

    fn description(&self) -> &str {
        "Retrieve a list of running processes including PID, name, CPU usage, and memory consumption."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        debug!("Executing GetProcessList tool");
        let processes = self.telemetry.get_processes().await;
        serde_json::to_value(processes).map_err(|e| {
            RigToolError::ExecutionFailed(format!("Failed to serialize process list: {}", e))
        })
    }
}

/// GetDiskInfo: Retrieve disk information
pub struct GetDiskInfo {
    telemetry: Arc<SystemTelemetry>,
}

impl GetDiskInfo {
    pub fn new(telemetry: Arc<SystemTelemetry>) -> Self {
        Self { telemetry }
    }
}

#[async_trait::async_trait]
impl RigTool for GetDiskInfo {
    fn name(&self) -> &str {
        "get_disk_info"
    }

    fn description(&self) -> &str {
        "Retrieve disk information including mount points, file systems, total space, and available space."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        debug!("Executing GetDiskInfo tool");
        let disks = self.telemetry.get_disks().await;
        serde_json::to_value(disks).map_err(|e| {
            RigToolError::ExecutionFailed(format!("Failed to serialize disk info: {}", e))
        })
    }
}

// ---------------------------------------------------------------------------
// Execution Tools
// ---------------------------------------------------------------------------

/// ExecuteShellCommand: Execute a shell command with safety approval
pub struct RigExecuteShellCommand {
    executor: Arc<ShellExecutor>,
}

impl RigExecuteShellCommand {
    pub fn new(executor: Arc<ShellExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait::async_trait]
impl RigTool for RigExecuteShellCommand {
    fn name(&self) -> &str {
        "execute_shell_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command on the operating system. This tool requires user approval before execution. Supports both Windows (PowerShell) and Unix (sh) commands. Use this for system operations like git, cargo, networking, etc."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "reason": {
                    "type": "string",
                    "description": "Optional reason/context for why this command is being executed"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RigToolError::InvalidParameters("Missing 'command' parameter".to_string()))?;

        let reason = params.get("reason").and_then(|v| v.as_str());

        info!("Executing shell command: {}", command);

        let result = if let Some(r) = reason {
            self.executor
                .execute_with_reason(command, r)
                .await
                .map_err(|e| match e {
                    ExecutionError::Denied(msg) => RigToolError::PermissionDenied(msg),
                    ExecutionError::ExecutionFailed(msg) => RigToolError::ExecutionFailed(msg),
                    ExecutionError::IoError(msg) => RigToolError::ExecutionFailed(msg),
                })?
        } else {
            self.executor
                .execute(command)
                .await
                .map_err(|e| match e {
                    ExecutionError::Denied(msg) => RigToolError::PermissionDenied(msg),
                    ExecutionError::ExecutionFailed(msg) => RigToolError::ExecutionFailed(msg),
                    ExecutionError::IoError(msg) => RigToolError::ExecutionFailed(msg),
                })?
        };

        serde_json::to_value(result).map_err(|e| {
            RigToolError::ExecutionFailed(format!("Failed to serialize command result: {}", e))
        })
    }
}

// ---------------------------------------------------------------------------
// File System Tools
// ---------------------------------------------------------------------------

/// RigListFiles: List files in a directory recursively
pub struct RigListFiles {
    fs: Arc<FileSystem>,
}

impl RigListFiles {
    pub fn new(fs: Arc<FileSystem>) -> Self {
        Self { fs }
    }
}

#[async_trait::async_trait]
impl RigTool for RigListFiles {
    fn name(&self) -> &str {
        "list_files"
    }

    fn description(&self) -> &str {
        "List files in a directory recursively. Returns file paths, names, sizes, and metadata. Respects workspace boundaries for safety."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list (relative to workspace)",
                    "default": "."
                }
            },
            "required": []
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        debug!("Listing files in: {}", path);

        let files = self
            .fs
            .list_files(path)
            .map_err(|e| RigToolError::ExecutionFailed(format!("Failed to list files: {}", e)))?;

        serde_json::to_value(files).map_err(|e| {
            RigToolError::ExecutionFailed(format!("Failed to serialize file list: {}", e))
        })
    }
}

/// RigReadFile: Read the contents of a file
pub struct RigReadFile {
    fs: Arc<FileSystem>,
}

impl RigReadFile {
    pub fn new(fs: Arc<FileSystem>) -> Self {
        Self { fs }
    }
}

#[async_trait::async_trait]
impl RigTool for RigReadFile {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Respects workspace boundaries for safety."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to read (relative to workspace)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RigToolError::InvalidParameters("Missing 'path' parameter".to_string()))?;

        debug!("Reading file: {}", path);

        let content = self
            .fs
            .read_file(path)
            .map_err(|e| RigToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        Ok(serde_json::json!({ "content": content }))
    }
}

/// RigWriteFile: Write content to a file
pub struct RigWriteFile {
    fs: Arc<FileSystem>,
}

impl RigWriteFile {
    pub fn new(fs: Arc<FileSystem>) -> Self {
        Self { fs }
    }
}

#[async_trait::async_trait]
impl RigTool for RigWriteFile {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates parent directories if they don't exist. Respects workspace boundaries for safety."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to write (relative to workspace)"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RigToolError::InvalidParameters("Missing 'path' parameter".to_string()))?;

        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RigToolError::InvalidParameters("Missing 'content' parameter".to_string()))?;

        debug!("Writing file: {}", path);

        self.fs
            .write_file(path, content)
            .map_err(|e| RigToolError::ExecutionFailed(format!("Failed to write file: {}", e)))?;

        Ok(serde_json::json!({ "status": "success", "path": path }))
    }
}

/// RigDeleteFile: Delete a file or directory
pub struct RigDeleteFile {
    fs: Arc<FileSystem>,
}

impl RigDeleteFile {
    pub fn new(fs: Arc<FileSystem>) -> Self {
        Self { fs }
    }
}

#[async_trait::async_trait]
impl RigTool for RigDeleteFile {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> &str {
        "Delete a file or directory. For directories, this will recursively delete all contents. Respects workspace boundaries for safety."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file or directory path to delete (relative to workspace)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RigToolError> {
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RigToolError::InvalidParameters("Missing 'path' parameter".to_string()))?;

        debug!("Deleting: {}", path);

        self.fs
            .delete(path)
            .map_err(|e| RigToolError::ExecutionFailed(format!("Failed to delete: {}", e)))?;

        Ok(serde_json::json!({ "status": "success", "path": path }))
    }
}

// ---------------------------------------------------------------------------
// Tool Registry
// ---------------------------------------------------------------------------

/// Registry for all Rig tools
pub struct RigToolRegistry {
    tools: Vec<Arc<dyn RigTool>>,
}

impl RigToolRegistry {
    /// Create a new tool registry with all default tools
    pub fn new(
        telemetry: Arc<SystemTelemetry>,
        executor: Arc<ShellExecutor>,
        fs: Arc<FileSystem>,
    ) -> Self {
        let tools: Vec<Arc<dyn RigTool>> = vec![
            // Telemetry tools
            Arc::new(GetSystemSnapshot::new(Arc::clone(&telemetry))),
            Arc::new(GetCpuInfo::new(Arc::clone(&telemetry))),
            Arc::new(GetMemoryInfo::new(Arc::clone(&telemetry))),
            Arc::new(GetProcessList::new(Arc::clone(&telemetry))),
            Arc::new(GetDiskInfo::new(Arc::clone(&telemetry))),
            // Execution tools
            Arc::new(RigExecuteShellCommand::new(Arc::clone(&executor))),
            // File system tools
            Arc::new(RigListFiles::new(Arc::clone(&fs))),
            Arc::new(RigReadFile::new(Arc::clone(&fs))),
            Arc::new(RigWriteFile::new(Arc::clone(&fs))),
            Arc::new(RigDeleteFile::new(Arc::clone(&fs))),
        ];

        Self { tools }
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn RigTool>> {
        self.tools.iter().find(|t| t.name() == name).cloned()
    }

    /// Get all tool names
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name().to_string()).collect()
    }

    /// Get all tools
    pub fn all(&self) -> &[Arc<dyn RigTool>] {
        &self.tools
    }

    /// Generate a tools definition for LLM context
    pub fn tools_definition(&self) -> serde_json::Value {
        let tools: Vec<serde_json::Value> = self
            .tools
            .iter()
            .map(|tool| {
                serde_json::json!({
                    "name": tool.name(),
                    "description": tool.description(),
                    "parameters": tool.parameters_schema()
                })
            })
            .collect();

        serde_json::json!({ "tools": tools })
    }
}

impl Default for RigToolRegistry {
    fn default() -> Self {
        let telemetry = Arc::new(SystemTelemetry::new());
        let executor = Arc::new(ShellExecutor::new());
        let fs = Arc::new(FileSystem::new().expect("Failed to create FileSystem"));
        Self::new(telemetry, executor, fs)
    }
}

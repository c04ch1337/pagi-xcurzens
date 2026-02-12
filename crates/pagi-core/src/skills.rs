//! Enhanced Skills System for Live Mode
//!
//! Extends the base AgentSkill trait with priority, energy cost, and security validation.
//! This enables Phoenix to execute actions mid-stream with proper governance.

use crate::{KnowledgeStore, TenantContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Skill execution priority (higher = more urgent)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillPriority {
    /// Background tasks, maintenance
    Low = 1,
    /// Normal operations
    Normal = 2,
    /// User-requested actions
    High = 3,
    /// Safety-critical interventions
    Critical = 4,
}

/// Energy cost for skill execution (token/compute budget)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EnergyCost {
    /// Minimal (< 100 tokens)
    Minimal = 1,
    /// Low (100-500 tokens)
    Low = 2,
    /// Medium (500-2000 tokens)
    Medium = 3,
    /// High (2000-5000 tokens)
    High = 4,
    /// VeryHigh (> 5000 tokens)
    VeryHigh = 5,
}

/// Skill execution request from Live Mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionRequest {
    /// Skill name to execute
    pub skill_name: String,
    /// Parameters for the skill
    pub params: serde_json::Value,
    /// Execution priority
    pub priority: SkillPriority,
    /// Optional KB-05 security context
    pub security_context: Option<String>,
}

/// Skill execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutionResult {
    pub skill_name: String,
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    /// Tokens/compute used
    pub energy_used: u32,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}

/// Enhanced skill trait for Live Mode execution
#[async_trait::async_trait]
pub trait LiveSkill: Send + Sync {
    /// Unique skill identifier
    fn name(&self) -> &str;
    
    /// Human-readable description
    fn description(&self) -> &str;
    
    /// Execution priority
    fn priority(&self) -> SkillPriority {
        SkillPriority::Normal
    }
    
    /// Estimated energy cost
    fn energy_cost(&self) -> EnergyCost {
        EnergyCost::Medium
    }
    
    /// Whether this skill requires KB-05 security validation
    fn requires_security_check(&self) -> bool {
        false
    }
    
    /// Validate execution against KB-05 security protocols
    async fn validate_security(
        &self,
        _knowledge: &KnowledgeStore,
        _params: &serde_json::Value,
    ) -> Result<(), String> {
        Ok(())
    }
    
    /// Execute the skill
    async fn execute(
        &self,
        ctx: &TenantContext,
        knowledge: &KnowledgeStore,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>>;
}

/// File system operations skill
pub struct FileSystemSkill;

#[async_trait::async_trait]
impl LiveSkill for FileSystemSkill {
    fn name(&self) -> &str {
        "filesystem"
    }
    
    fn description(&self) -> &str {
        "Read, write, and analyze files in the workspace"
    }
    
    fn priority(&self) -> SkillPriority {
        SkillPriority::Normal
    }
    
    fn energy_cost(&self) -> EnergyCost {
        EnergyCost::Low
    }
    
    fn requires_security_check(&self) -> bool {
        true
    }
    
    async fn validate_security(
        &self,
        knowledge: &KnowledgeStore,
        params: &serde_json::Value,
    ) -> Result<(), String> {
        // Check KB-05 for file access permissions
        let path = params.get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        
        // Query KB-05 for security protocols
        if let Some(_policy) = knowledge.get_ethos_policy() {
            // Check if path is in restricted list
            if path.contains("..") || path.starts_with("/") {
                return Err("Path traversal detected - blocked by KB-05".to_string());
            }
        }
        
        Ok(())
    }
    
    async fn execute(
        &self,
        _ctx: &TenantContext,
        _knowledge: &KnowledgeStore,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let operation = params.get("operation")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'operation' parameter")?;
        
        match operation {
            "read" => {
                let path = params.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path' parameter")?;
                
                let content = tokio::fs::read_to_string(path).await
                    .map_err(|e| format!("Failed to read file: {}", e))?;
                
                Ok(serde_json::json!({
                    "operation": "read",
                    "path": path,
                    "content": content,
                    "size": content.len(),
                }))
            }
            "write" => {
                let path = params.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'path' parameter")?;
                let content = params.get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'content' parameter")?;
                
                tokio::fs::write(path, content).await
                    .map_err(|e| format!("Failed to write file: {}", e))?;
                
                Ok(serde_json::json!({
                    "operation": "write",
                    "path": path,
                    "size": content.len(),
                }))
            }
            "list" => {
                let path = params.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or(".");
                
                let mut entries = Vec::new();
                let mut dir = tokio::fs::read_dir(path).await
                    .map_err(|e| format!("Failed to read directory: {}", e))?;
                
                while let Some(entry) = dir.next_entry().await
                    .map_err(|e| format!("Failed to read entry: {}", e))? {
                    entries.push(entry.file_name().to_string_lossy().to_string());
                }
                
                Ok(serde_json::json!({
                    "operation": "list",
                    "path": path,
                    "entries": entries,
                }))
            }
            _ => Err(format!("Unknown operation: {}", operation).into()),
        }
    }
}

/// Project Vault / Folder skill: read directory structures and file contents for a mounted local path.
/// Used when a Project is associated with a local folder and "Master Analysis" is ON.
pub struct FolderSkill;

#[async_trait::async_trait]
impl LiveSkill for FolderSkill {
    fn name(&self) -> &str {
        "folder"
    }

    fn description(&self) -> &str {
        "Read directory structure, read files, and write documents (e.g. Markdown) in an associated project folder (Project Vault). Writes restricted to under root (KB-05)."
    }

    fn priority(&self) -> SkillPriority {
        SkillPriority::Normal
    }

    fn energy_cost(&self) -> EnergyCost {
        EnergyCost::Low
    }

    fn requires_security_check(&self) -> bool {
        true
    }

    async fn validate_security(
        &self,
        _knowledge: &KnowledgeStore,
        params: &serde_json::Value,
    ) -> Result<(), String> {
        let operation = params.get("operation").and_then(|v| v.as_str()).unwrap_or("");
        if operation == "write_document" {
            let root = params
                .get("root")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'root' parameter for write_document (KB-05)")?;
            let relative_path = params
                .get("relative_path")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'relative_path' parameter for write_document (KB-05)")?;
            if relative_path.contains("..") {
                return Err("Path traversal (..) not allowed in relative_path (KB-05)".to_string());
            }
            let root_path = std::path::Path::new(root);
            if !root_path.exists() {
                return Err("Root path does not exist (KB-05)".to_string());
            }
            if !root_path.is_dir() {
                return Err("Root must be a directory (KB-05)".to_string());
            }
            return Ok(());
        }
        let path = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'path' parameter")?;
        if path.contains("..") {
            return Err("Path traversal (..) not allowed (KB-05)".to_string());
        }
        let path = std::path::Path::new(path);
        if path.exists() && !path.is_dir() {
            return Err("Path must be a directory (KB-05)".to_string());
        }
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        _knowledge: &KnowledgeStore,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let operation = params
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'operation' parameter (use 'list', 'read', or 'write_document')")?;
        let path = params.get("path").and_then(|v| v.as_str());

        match operation {
            "write_document" => {
                let root = params
                    .get("root")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'root' parameter")?;
                let relative_path = params
                    .get("relative_path")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'relative_path' parameter")?;
                let content = params
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'content' parameter")?;
                crate::project_vault::write_document_under_root(
                    std::path::Path::new(root),
                    relative_path,
                    content,
                )
                .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({
                    "operation": "write_document",
                    "root": root,
                    "relative_path": relative_path,
                    "status": "written",
                }))
            }
            "list" => {
                let path = path.ok_or("Missing 'path' parameter for list")?;
                let mut entries = Vec::new();
                let mut dir = tokio::fs::read_dir(path)
                    .await
                    .map_err(|e| format!("Failed to read directory: {}", e))?;
                while let Some(entry) = dir
                    .next_entry()
                    .await
                    .map_err(|e| format!("Failed to read entry: {}", e))?
                {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                    entries.push(serde_json::json!({ "name": name, "is_dir": is_dir }));
                }
                Ok(serde_json::json!({
                    "operation": "list",
                    "path": path,
                    "entries": entries,
                }))
            }
            "read" => {
                let path = path.ok_or("Missing 'path' parameter for read")?;
                let rel = params
                    .get("file")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing 'file' parameter for read")?;
                if rel.contains("..") {
                    return Err("Path traversal not allowed".into());
                }
                let full = std::path::Path::new(path).join(rel);
                let content = tokio::fs::read_to_string(&full)
                    .await
                    .map_err(|e| format!("Failed to read file: {}", e))?;
                Ok(serde_json::json!({
                    "operation": "read",
                    "path": full.to_string_lossy(),
                    "content": content,
                    "size": content.len(),
                }))
            }
            _ => Err(format!("Unknown operation: {} (use 'list' or 'read')", operation).into()),
        }
    }
}

/// Shell command execution skill
pub struct ShellExecutorSkill;

#[async_trait::async_trait]
impl LiveSkill for ShellExecutorSkill {
    fn name(&self) -> &str {
        "shell"
    }
    
    fn description(&self) -> &str {
        "Execute shell commands with security validation"
    }
    
    fn priority(&self) -> SkillPriority {
        SkillPriority::High
    }
    
    fn energy_cost(&self) -> EnergyCost {
        EnergyCost::Medium
    }
    
    fn requires_security_check(&self) -> bool {
        true
    }
    
    async fn validate_security(
        &self,
        knowledge: &KnowledgeStore,
        params: &serde_json::Value,
    ) -> Result<(), String> {
        let command = params.get("command")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'command' parameter")?;
        
        // Check KB-05 for dangerous commands
        let dangerous_patterns = vec![
            "rm -rf /",
            "dd if=",
            "mkfs",
            "format",
            "> /dev/",
            "curl | sh",
            "wget | sh",
        ];
        
        for pattern in dangerous_patterns {
            if command.contains(pattern) {
                return Err(format!("Dangerous command pattern detected: {} - blocked by KB-05", pattern));
            }
        }
        
        // Query KB-05 for additional restrictions
        if let Some(_policy) = knowledge.get_ethos_policy() {
            // Additional policy checks could go here
        }
        
        Ok(())
    }
    
    async fn execute(
        &self,
        _ctx: &TenantContext,
        _knowledge: &KnowledgeStore,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let command = params.get("command")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'command' parameter")?;
        
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .await
            .map_err(|e| format!("Failed to execute command: {}", e))?;
        
        Ok(serde_json::json!({
            "command": command,
            "success": output.status.success(),
            "stdout": String::from_utf8_lossy(&output.stdout),
            "stderr": String::from_utf8_lossy(&output.stderr),
            "exit_code": output.status.code(),
        }))
    }
}

/// Web search skill
pub struct WebSearchSkill;

#[async_trait::async_trait]
impl LiveSkill for WebSearchSkill {
    fn name(&self) -> &str {
        "web_search"
    }
    
    fn description(&self) -> &str {
        "Search the web for information"
    }
    
    fn priority(&self) -> SkillPriority {
        SkillPriority::Normal
    }
    
    fn energy_cost(&self) -> EnergyCost {
        EnergyCost::High
    }
    
    fn requires_security_check(&self) -> bool {
        false
    }
    
    async fn execute(
        &self,
        _ctx: &TenantContext,
        _knowledge: &KnowledgeStore,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let query = params.get("query")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'query' parameter")?;
        
        // Placeholder implementation - would integrate with actual search API
        Ok(serde_json::json!({
            "query": query,
            "results": [],
            "note": "Web search integration pending - requires API key configuration",
        }))
    }
}

/// Sovereign System Self-Audit: discovery, alignment, infra scan, ethos validation, report.
/// Logs "Capability Gap" to KB-08 when optional integrations (Redis, Vector DB) are unset.
pub struct AuditSkill;

#[async_trait::async_trait]
impl LiveSkill for AuditSkill {
    fn name(&self) -> &str {
        "audit"
    }

    fn description(&self) -> &str {
        "Run Sovereign System Self-Audit: discovery, alignment vs Master Template, infra readiness, KB-05 ethos validation, report"
    }

    fn priority(&self) -> SkillPriority {
        SkillPriority::Low
    }

    fn energy_cost(&self) -> EnergyCost {
        EnergyCost::Medium
    }

    fn requires_security_check(&self) -> bool {
        true
    }

    async fn validate_security(
        &self,
        _knowledge: &KnowledgeStore,
        params: &serde_json::Value,
    ) -> Result<(), String> {
        let root = params.get("workspace_root").and_then(|v| v.as_str()).unwrap_or(".");
        if root.contains("..") || root.starts_with('/') {
            return Err("audit: workspace_root must be relative and not traverse up (KB-05)".to_string());
        }
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        knowledge: &KnowledgeStore,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let root = params.get("workspace_root").and_then(|v| v.as_str()).unwrap_or(".");
        let mut capability_gaps: Vec<String> = Vec::new();
        let mut discovery_crates: Vec<String> = Vec::new();
        let mut discovery_addons: Vec<String> = Vec::new();
        let mut alignment_ok = true;
        let mut skills_without_kb05: Vec<String> = Vec::new();

        // ─── 1. DISCOVERY ───
        let crates_dir = std::path::Path::new(root).join("crates");
        let addons_dir = std::path::Path::new(root).join("add-ons");
        if let Ok(mut rd) = tokio::fs::read_dir(&crates_dir).await {
            while let Ok(Some(e)) = rd.next_entry().await {
                if e.file_name().to_str().map(|s| !s.starts_with('.')).unwrap_or(false) {
                    discovery_crates.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
        if let Ok(mut rd) = tokio::fs::read_dir(&addons_dir).await {
            while let Ok(Some(e)) = rd.next_entry().await {
                if e.file_name().to_str().map(|s| !s.starts_with('.')).unwrap_or(false) {
                    discovery_addons.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }

        let lib_rs_path = crates_dir.join("pagi-core").join("src").join("lib.rs");
        let main_rs_path = addons_dir.join("pagi-gateway").join("src").join("main.rs");
        let lib_content = tokio::fs::read_to_string(&lib_rs_path).await.unwrap_or_default();
        let main_content = tokio::fs::read_to_string(&main_rs_path).await.unwrap_or_default();
        let lib_rs_len = lib_content.len();
        let main_rs_len = main_content.len();

        // ─── 2. ALIGNMENT (heuristic: key modules present) ───
        if !lib_content.contains("KnowledgeStore") || !lib_content.contains("skills") {
            alignment_ok = false;
        }
        if !main_content.contains("knowledge_router") && !main_content.contains("governor") {
            alignment_ok = false;
        }

        // ─── 3. INFRASTRUCTURE SCAN ───
        if std::env::var("PAGI_REDIS_URL").unwrap_or_default().trim().is_empty() {
            capability_gaps.push("PAGI_REDIS_URL unset".to_string());
            let _ = knowledge.record_success_metric("System Self-Audit: Capability Gap — PAGI_REDIS_URL unset (optional)");
        }
        if std::env::var("PAGI_VECTOR_DB_URL").unwrap_or_default().trim().is_empty() {
            capability_gaps.push("PAGI_VECTOR_DB_URL unset".to_string());
            let _ = knowledge.record_success_metric("System Self-Audit: Capability Gap — PAGI_VECTOR_DB_URL unset (optional)");
        }

        // ─── 4. ETHOS VALIDATION: Command without KB-05 ───
        let scan_dirs = [
            crates_dir.join("pagi-core").join("src"),
            crates_dir.join("pagi-skills").join("src"),
            addons_dir.join("pagi-gateway").join("src"),
        ];
        for dir in &scan_dirs {
            if let Ok(mut rd) = tokio::fs::read_dir(dir).await {
                while let Ok(Some(e)) = rd.next_entry().await {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.ends_with(".rs") {
                        let path = e.path();
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            let has_command = content.contains("std::process::Command") || content.contains("Command::new");
                            let has_security = content.contains("validate_security") || content.contains("requires_security_check") || content.contains("KB-05");
                            if has_command && !has_security {
                                skills_without_kb05.push(path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        // Weighted Sovereignty Score: Base 1.0, -0.5 alignment, -0.2 per unprotected skill, -0.025 per capability gap
        const ALIGNMENT_PENALTY: f64 = 0.5;
        const UNPROTECTED_SKILL_PENALTY: f64 = 0.2;
        const CAPABILITY_GAP_PENALTY: f64 = 0.025;
        let mut sovereignty_score = 1.0_f64;
        if !alignment_ok {
            sovereignty_score -= ALIGNMENT_PENALTY;
        }
        sovereignty_score -= (skills_without_kb05.len() as f64) * UNPROTECTED_SKILL_PENALTY;
        sovereignty_score -= (capability_gaps.len() as f64) * CAPABILITY_GAP_PENALTY;
        sovereignty_score = sovereignty_score.clamp(0.0, 1.0);

        let sovereignty_compliance = sovereignty_score > 0.9;

        if sovereignty_score < 0.7 {
            let _ = knowledge.record_success_metric(&format!(
                "High Risk Anomaly: Sovereignty score {:.2} below 0.7",
                sovereignty_score
            ));
        }

        let report_summary = if sovereignty_compliance {
            format!("Sovereignty compliance OK (score {:.2}). No skills bypass KB-05; alignment heuristics passed.", sovereignty_score)
        } else if sovereignty_score < 0.7 {
            format!("High risk: sovereignty score {:.2}. Check skills_without_kb05 and alignment_ok.", sovereignty_score)
        } else {
            format!("Sovereignty review recommended (score {:.2}). Check skills_without_kb05 and alignment_ok.", sovereignty_score)
        };

        Ok(serde_json::json!({
            "sovereignty_score": sovereignty_score,
            "sovereignty_compliance": sovereignty_compliance,
            "discovery": {
                "crates": discovery_crates,
                "add_ons": discovery_addons,
                "lib_rs_bytes": lib_rs_len,
                "main_rs_bytes": main_rs_len,
            },
            "alignment_ok": alignment_ok,
            "capability_gaps": capability_gaps,
            "skills_without_kb05": skills_without_kb05,
            "report_summary": report_summary,
        }))
    }
}

/// Sovereign self-healing: apply code fixes from audit findings. KB-05 path bounds, KB-06 ethos.
pub struct RefactorSkill;

/// Paths considered sovereignty-critical: must retain governor/security semantics after edit.
fn is_sovereignty_critical(path: &str) -> bool {
    let lower = path.replace('\\', "/");
    lower.contains("governor") || lower.ends_with("main.rs") || lower.contains("skills.rs")
}

#[async_trait::async_trait]
impl LiveSkill for RefactorSkill {
    fn name(&self) -> &str {
        "refactor"
    }

    fn description(&self) -> &str {
        "Apply code fixes from audit: replace snippet in file, verify with cargo check, log to KB-08 (Genetic Mutation)"
    }

    fn priority(&self) -> SkillPriority {
        SkillPriority::Normal
    }

    fn energy_cost(&self) -> EnergyCost {
        EnergyCost::High
    }

    fn requires_security_check(&self) -> bool {
        true
    }

    async fn validate_security(
        &self,
        _knowledge: &KnowledgeStore,
        params: &serde_json::Value,
    ) -> Result<(), String> {
        let file_path = params.get("file_path").and_then(|v| v.as_str())
            .ok_or("refactor: missing 'file_path'")?;
        let workspace_root = params.get("workspace_root").and_then(|v| v.as_str()).unwrap_or(".");
        if file_path.contains("..") || file_path.starts_with('/') {
            return Err("refactor: file_path must be relative, no traversal (KB-05)".to_string());
        }
        let allowed = file_path.ends_with(".rs") || file_path.ends_with(".toml");
        if !allowed {
            return Err("refactor: only .rs and .toml allowed (KB-05)".to_string());
        }
        let full = std::path::Path::new(workspace_root).join(file_path);
        let canonical = full.canonicalize().map_err(|e| format!("refactor: path resolve failed: {} (KB-05)", e))?;
        let root_canonical = std::path::Path::new(workspace_root).canonicalize()
            .map_err(|e| format!("refactor: workspace_root resolve failed: {} (KB-05)", e))?;
        if !canonical.starts_with(&root_canonical) {
            return Err("refactor: file_path must be under workspace_root (KB-05)".to_string());
        }
        Ok(())
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        knowledge: &KnowledgeStore,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let file_path = params.get("file_path").and_then(|v| v.as_str())
            .ok_or("missing file_path")?;
        let original_snippet = params.get("original_snippet").and_then(|v| v.as_str())
            .ok_or("missing original_snippet")?;
        let new_snippet = params.get("new_snippet").and_then(|v| v.as_str())
            .ok_or("missing new_snippet")?;
        let workspace_root = params.get("workspace_root").and_then(|v| v.as_str()).unwrap_or(".");
        let full_path = std::path::Path::new(workspace_root).join(file_path);

        let content = tokio::fs::read_to_string(&full_path).await
            .map_err(|e| format!("read failed: {}", e))?;

        if !content.contains(original_snippet) {
            return Err(format!("original_snippet not found in {}", file_path).into());
        }

        let new_content = content.replacen(original_snippet, new_snippet, 1);

        // KB-06 (Ethos): sovereignty-critical files must retain governor/security semantics
        if is_sovereignty_critical(file_path) {
            let required = ["governor", "validate_security", "KB-05", "KnowledgeRouter"];
            let has_any = required.iter().any(|s| new_content.contains(s));
            if !has_any {
                return Err("refactor: KB-06 violation — cannot remove sovereignty defense from critical file".into());
            }
        }

        let backup = content.clone();
        tokio::fs::write(&full_path, &new_content).await
            .map_err(|e| format!("write failed: {}", e))?;

        // Verify with cargo check
        let output = tokio::process::Command::new("cargo")
            .args(["check"])
            .current_dir(workspace_root)
            .output()
            .await
            .map_err(|e| format!("cargo check spawn failed: {}", e))?;

        if !output.status.success() {
            let _ = tokio::fs::write(&full_path, &backup).await;
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("cargo check failed; reverted. stderr: {}", stderr.lines().take(5).collect::<Vec<_>>().join(" ")).into());
        }

        let msg = format!("Genetic Mutation: applied refactor to {}", file_path);
        knowledge.record_success_metric(&msg).ok();

        Ok(serde_json::json!({
            "status": "applied",
            "file_path": file_path,
            "replacement_count": 1,
            "cargo_check": "ok",
            "kb08_logged": true,
            "message": msg,
        }))
    }
}

/// Live Skills Registry with priority queue
pub struct LiveSkillRegistry {
    skills: Vec<Arc<dyn LiveSkill>>,
    /// Execution queue (sorted by priority)
    queue: std::sync::Mutex<Vec<SkillExecutionRequest>>,
}

impl LiveSkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            queue: std::sync::Mutex::new(Vec::new()),
        }
    }
    
    /// Register a skill
    pub fn register(&mut self, skill: Arc<dyn LiveSkill>) {
        self.skills.push(skill);
    }
    
    /// Get skill by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn LiveSkill>> {
        self.skills.iter().find(|s| s.name() == name).cloned()
    }
    
    /// Queue a skill for execution
    pub fn queue_skill(&self, request: SkillExecutionRequest) {
        let mut queue = self.queue.lock().unwrap();
        queue.push(request);
        // Sort by priority (highest first)
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
    
    /// Get next skill from queue
    pub fn dequeue_skill(&self) -> Option<SkillExecutionRequest> {
        let mut queue = self.queue.lock().unwrap();
        queue.pop()
    }
    
    /// Get current queue size
    pub fn queue_size(&self) -> usize {
        self.queue.lock().unwrap().len()
    }
    
    /// List all registered skills
    pub fn list_skills(&self) -> Vec<(String, String, SkillPriority, EnergyCost)> {
        self.skills.iter().map(|s| {
            (
                s.name().to_string(),
                s.description().to_string(),
                s.priority(),
                s.energy_cost(),
            )
        }).collect()
    }
}

impl Default for LiveSkillRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        
        // Register default skills
        registry.register(Arc::new(FileSystemSkill));
        registry.register(Arc::new(FolderSkill));
        registry.register(Arc::new(ShellExecutorSkill));
        registry.register(Arc::new(WebSearchSkill));
        registry.register(Arc::new(AuditSkill));
        registry.register(Arc::new(RefactorSkill));

        registry
    }
}

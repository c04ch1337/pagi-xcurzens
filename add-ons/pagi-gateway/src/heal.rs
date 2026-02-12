//! Heal orchestration: chain Audit → Refactor for skills_without_kb05.
//!
//! POST /api/v1/heal runs the audit, then for each file in skills_without_kb05
//! generates a security-wrapping snippet and calls the refactor skill. Re-runs
//! audit after fixes and logs the session to KB-08.

use pagi_core::{KnowledgeStore, LiveSkillRegistry, TenantContext};
use std::path::Path;

const WORKSPACE_ROOT: &str = ".";

/// Result of a single refactor attempt during a heal session.
#[derive(Debug, serde::Serialize)]
pub struct HealRefactorResult {
    pub file_path: String,
    pub applied: bool,
    pub message: String,
}

/// Run audit, then for each path in skills_without_kb05 try to apply a KB-05 security fix via refactor.
/// Returns (audit_before, refactor_results, audit_after, final_score).
pub async fn run_heal_flow(
    knowledge: &KnowledgeStore,
) -> Result<
    (
        serde_json::Value,
        Vec<HealRefactorResult>,
        serde_json::Value,
        f64,
    ),
    String,
> {
    let registry = LiveSkillRegistry::default();
    let audit_skill = registry
        .get("audit")
        .ok_or("audit skill not in registry")?;
    let refactor_skill = registry
        .get("refactor")
        .ok_or("refactor skill not in registry")?;

    let tenant_ctx = TenantContext {
        tenant_id: "heal".to_string(),
        correlation_id: None,
        agent_id: Some("phoenix".to_string()),
    };
    let audit_params = serde_json::json!({ "workspace_root": WORKSPACE_ROOT });

    // Step A: Run audit
    if audit_skill.requires_security_check() {
        audit_skill
            .validate_security(knowledge, &audit_params)
            .await
            .map_err(|e| format!("audit KB-05 blocked: {}", e))?;
    }
    let audit_before = audit_skill
        .execute(&tenant_ctx, knowledge, audit_params.clone())
        .await
        .map_err(|e| format!("audit execute failed: {}", e))?;

    let skills_without_kb05: Vec<String> = audit_before
        .get("skills_without_kb05")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let workspace_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(WORKSPACE_ROOT));
    let mut results = Vec::new();

    for path_entry in &skills_without_kb05 {
        let path_str = path_entry.replace('\\', "/");
        let full_path = Path::new(&path_str);
        let file_path_rel = make_relative_to_workspace(full_path, &workspace_root);
        let file_path_rel = match file_path_rel {
            Some(p) => p,
            None => {
                results.push(HealRefactorResult {
                    file_path: path_str.clone(),
                    applied: false,
                    message: "path not under workspace".to_string(),
                });
                continue;
            }
        };

        let content = match tokio::fs::read_to_string(full_path).await {
            Ok(c) => c,
            Err(e) => {
                results.push(HealRefactorResult {
                    file_path: file_path_rel.clone(),
                    applied: false,
                    message: format!("read failed: {}", e),
                });
                continue;
            }
        };

        let (original_snippet, new_snippet) = match generate_security_wrap_snippet(&content) {
            Ok(pair) => pair,
            Err(reason) => {
                results.push(HealRefactorResult {
                    file_path: file_path_rel.clone(),
                    applied: false,
                    message: reason,
                });
                continue;
            }
        };

        let refactor_params = serde_json::json!({
            "file_path": file_path_rel,
            "original_snippet": original_snippet,
            "new_snippet": new_snippet,
            "workspace_root": WORKSPACE_ROOT,
        });

        if refactor_skill.requires_security_check() {
            if let Err(e) = refactor_skill.validate_security(knowledge, &refactor_params).await {
                results.push(HealRefactorResult {
                    file_path: path_str.clone(),
                    applied: false,
                    message: format!("refactor KB-05 blocked: {}", e),
                });
                continue;
            }
        }

        match refactor_skill
            .execute(&tenant_ctx, knowledge, refactor_params)
            .await
        {
            Ok(res) => {
                let msg = res
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("applied");
                results.push(HealRefactorResult {
                    file_path: path_str.clone(),
                    applied: res.get("status").and_then(|v| v.as_str()) == Some("applied"),
                    message: msg.to_string(),
                });
            }
            Err(e) => {
                results.push(HealRefactorResult {
                    file_path: path_str.clone(),
                    applied: false,
                    message: e.to_string(),
                });
            }
        }
    }

    // Re-run audit to get updated score
    let audit_after = if audit_skill.requires_security_check() {
        let _ = audit_skill.validate_security(knowledge, &audit_params).await;
        audit_skill
            .execute(&tenant_ctx, knowledge, audit_params)
            .await
            .unwrap_or(audit_before.clone())
    } else {
        audit_skill
            .execute(&tenant_ctx, knowledge, audit_params)
            .await
            .unwrap_or(audit_before.clone())
    };

    let final_score = audit_after
        .get("sovereignty_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    Ok((audit_before, results, audit_after, final_score))
}

/// Make path relative to workspace root (normalized forward slashes for refactor).
fn make_relative_to_workspace(
    full: &Path,
    workspace_root: &std::path::Path,
) -> Option<String> {
    let full_canon = full.canonicalize().ok()?;
    let root_canon = workspace_root.canonicalize().ok()?;
    let rel = full_canon.strip_prefix(&root_canon).ok()?;
    Some(rel.to_string_lossy().replace('\\', "/"))
}

/// Find the first line containing Command usage and return (original_line, new_content_with_security_call).
/// Returns Err(reason) when the containing function is not async or lacks knowledge/params (KB-05 auto-wrap requires them).
fn generate_security_wrap_snippet(content: &str) -> Result<(String, String), String> {
    let needle_cmd = "Command::new";
    let needle_std = "std::process::Command";
    let lines: Vec<&str> = content.lines().collect();
    let cmd_line_idx = lines
        .iter()
        .position(|l| l.contains(needle_cmd) || l.contains(needle_std))
        .ok_or_else(|| "could not find Command usage line to wrap".to_string())?;

    let line = lines[cmd_line_idx];
    let indent = line.len() - line.trim_start().len();
    let indent_str = &line[..indent];

    // Find containing function: last "async fn" or "fn" before the Command line
    let fn_line_idx = lines[..cmd_line_idx]
        .iter()
        .rposition(|l| {
            let t = l.trim_start();
            t.starts_with("async fn ") || (t.starts_with("fn ") && !t.starts_with("fn async"))
        })
        .ok_or_else(|| "no containing function found for Command usage".to_string())?;

    let fn_line = lines[fn_line_idx].trim_start();
    let is_async = fn_line.starts_with("async fn ");
    if !is_async {
        return Err(
            "Function is not async; KB-05 auto-wrap requires async context for validate_security.".to_string()
        );
    }

    // Signature: from fn line up to (and including) the line with opening brace
    let sig_end = (fn_line_idx..cmd_line_idx)
        .find(|&i| lines[i].contains('{'))
        .unwrap_or(cmd_line_idx);
    let signature: String = lines[fn_line_idx..=sig_end].join("\n");
    let has_knowledge = signature.contains("knowledge");
    let has_params = signature.contains("params");
    if !has_knowledge || !has_params {
        return Err(
            "Function signature incompatible with KB-05 auto-wrap (missing knowledge or params).".to_string()
        );
    }

    let security_line = format!(
        "{}self.validate_security(knowledge, &params).await.map_err(|e| format!(\"KB-05 blocked: {{}}\", e))?;\n",
        indent_str
    );
    let original_snippet = format!("{}\n", line);
    let new_snippet = format!("{}{}\n", security_line, line);
    Ok((original_snippet, new_snippet))
}

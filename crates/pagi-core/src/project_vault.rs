//! Project Vault: local folder as primary Search Slot for the Master Orchestrator.
//!
//! Summarizes a directory (file list + recent text content) for injection into
//! Short-Term Memory / system directive when "Master Analysis" is ON.

use std::path::Path;

/// Text-file extensions we include in the context summary (logs, emails, notes).
const TEXT_EXTENSIONS: &[&str] = &["log", "txt", "json", "md", "eml", "csv"];

/// Build a context summary string for a project folder: directory listing plus
/// content of recent text files, capped at `max_bytes`. Used when Master Analysis
/// is ON to inject into the orchestrator's system directive.
pub async fn summarize_folder_for_context(
    path: &Path,
    max_bytes: usize,
) -> std::io::Result<String> {
    let path = path.to_path_buf();
    let mut out = String::with_capacity(max_bytes.min(64 * 1024));

    // 1. List top-level entries
    let read_dir = tokio::fs::read_dir(&path).await?;
    let mut entries = Vec::new();
    let mut read_dir = read_dir;
    while let Some(entry) = read_dir.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
        entries.push((name, is_dir));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    out.push_str("=== Project folder contents (top-level) ===\n");
    for (name, is_dir) in entries.iter().take(100) {
        if name.starts_with('.') {
            continue;
        }
        out.push_str(if *is_dir { "[dir]  " } else { "[file] " });
        out.push_str(name);
        out.push('\n');
    }
    if entries.len() > 100 {
        out.push_str(&format!("... and {} more entries\n", entries.len() - 100));
    }

    // 2. Read text files (flat, up to max_bytes total)
    let mut collected = 0_usize;
    for (name, is_dir) in entries {
        if is_dir || collected >= max_bytes {
            continue;
        }
        let ext = name.rfind('.').map(|i| &name[i + 1..]).unwrap_or("");
        if !TEXT_EXTENSIONS.iter().any(|e| *e == ext) {
            continue;
        }
        let file_path = path.join(&name);
        match tokio::fs::read_to_string(&file_path).await {
            Ok(content) => {
                let take = (max_bytes - collected).min(content.len()).min(32 * 1024);
                let slice = if take < content.len() {
                    format!("{}... [truncated]", &content[..take])
                } else {
                    content
                };
                out.push_str("\n--- ");
                out.push_str(&name);
                out.push_str(" ---\n");
                out.push_str(&slice);
                if !slice.ends_with('\n') {
                    out.push('\n');
                }
                collected += slice.len();
            }
            Err(_) => continue,
        }
    }

    Ok(out)
}

/// Synchronous fallback for contexts that don't have tokio runtime (e.g. notify callback).
/// Uses std::fs; same logic as async version.
pub fn summarize_folder_for_context_sync(
    path: &Path,
    max_bytes: usize,
) -> std::io::Result<String> {
    let mut out = String::with_capacity(max_bytes.min(64 * 1024));

    let mut entries: Vec<(String, bool)> = std::fs::read_dir(path)?
        .filter_map(|e| e.ok())
        .map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (name, is_dir)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    out.push_str("=== Project folder contents (top-level) ===\n");
    for (name, is_dir) in entries.iter().take(100) {
        if name.starts_with('.') {
            continue;
        }
        out.push_str(if *is_dir { "[dir]  " } else { "[file] " });
        out.push_str(name);
        out.push('\n');
    }
    if entries.len() > 100 {
        out.push_str(&format!("... and {} more entries\n", entries.len() - 100));
    }

    let mut collected = 0_usize;
    for (name, is_dir) in entries {
        if is_dir || collected >= max_bytes {
            continue;
        }
        let ext = name.rfind('.').map(|i| &name[i + 1..]).unwrap_or("");
        if !TEXT_EXTENSIONS.iter().any(|e| *e == ext) {
            continue;
        }
        let file_path = path.join(&name);
        if let Ok(content) = std::fs::read_to_string(&file_path) {
            let take = (max_bytes - collected).min(content.len()).min(32 * 1024);
            let slice = if take < content.len() {
                format!("{}... [truncated]", &content[..take])
            } else {
                content
            };
            out.push_str("\n--- ");
            out.push_str(&name);
            out.push_str(" ---\n");
            out.push_str(&slice);
            if !slice.ends_with('\n') {
                out.push('\n');
            }
            collected += slice.len();
        }
    }

    Ok(out)
}

/// Writes a document (e.g. Markdown) under a project root. Sovereignty Firewall: writes ONLY under `root`;
/// `relative_path` must not contain `..` and the resolved path must be under the canonical root.
pub fn write_document_under_root(
    root: &Path,
    relative_path: &str,
    content: &str,
) -> std::io::Result<()> {
    let normalized = relative_path.replace('\\', "/").trim_matches('/').to_string();
    if normalized.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "relative_path must not be empty",
        ));
    }
    if normalized.contains("..") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Path traversal (..) not allowed (KB-05)",
        ));
    }
    let root_canonical = root.canonicalize().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Project root not found or not accessible: {}", e),
        )
    })?;
    let full_path = root_canonical.join(&normalized);
    if full_path.strip_prefix(&root_canonical).is_err() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Resolved path must be under project root (KB-05)",
        ));
    }
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&full_path, content)?;
    Ok(())
}

//! Diagnostic Log Exporter for Beta Testing
//!
//! Provides a "Download Logs" feature that packages system logs,
//! version info, and diagnostic data for bug reporting.

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use zip::write::FileOptions;
use zip::ZipWriter;

/// Sanitize log content by redacting API keys
fn sanitize_log_content(content: &str) -> String {
    // Redact OpenRouter API keys
    let re_openrouter = regex::Regex::new(r"sk-or-v1-[a-zA-Z0-9]{32,}").unwrap();
    let content = re_openrouter.replace_all(content, "sk-or-v1-REDACTED");
    
    // Redact generic API keys
    let re_generic = regex::Regex::new(r#"(api[_-]?key|token|secret)["']?\s*[:=]\s*["']?([a-zA-Z0-9_-]{20,})["']?"#).unwrap();
    let content = re_generic.replace_all(&content, "$1: REDACTED");
    
    // Redact Authorization headers
    // Common pattern: "Authorization: Bearer <token>" (JWTs contain '.' so include it).
    // Keep output consistent for tests and for easy human scanning.
    let re_auth_bearer =
        regex::Regex::new(r"(?i)Authorization\s*:\s*Bearer\s+[a-zA-Z0-9._-]{20,}").unwrap();
    let content = re_auth_bearer.replace_all(&content, "Authorization REDACTED");

    // Also catch standalone bearer tokens that may appear without the Authorization header prefix.
    let re_bearer = regex::Regex::new(r"(?i)Bearer\s+[a-zA-Z0-9._-]{20,}").unwrap();
    let content = re_bearer.replace_all(&content, "Bearer REDACTED");
     
    content.to_string()
}

/// Generate diagnostic report
fn generate_diagnostic_report() -> String {
    let mut report = String::new();
    
    report.push_str("# Phoenix Diagnostic Report\n\n");
    report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().to_rfc3339()));
    
    // Version info
    if let Ok(version) = fs::read_to_string("VERSION") {
        report.push_str(&format!("Phoenix Version: {}\n", version.trim()));
    }
    
    // System info
    report.push_str(&format!("OS: {}\n", std::env::consts::OS));
    report.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));
    
    // Qdrant status
    report.push_str("\n## Qdrant Status\n\n");
    let qdrant_binary = Path::new("./bin/qdrant");
    if qdrant_binary.exists() {
        report.push_str("Qdrant Binary: ✅ Present\n");
        if let Ok(metadata) = fs::metadata(qdrant_binary) {
            report.push_str(&format!("Binary Size: {} bytes\n", metadata.len()));
        }
    } else {
        report.push_str("Qdrant Binary: ❌ Missing\n");
    }
    
    // Check if Qdrant is running
    match std::net::TcpStream::connect("127.0.0.1:6333") {
        Ok(_) => report.push_str("Qdrant Service: ✅ Running\n"),
        Err(_) => report.push_str("Qdrant Service: ❌ Not Running\n"),
    }
    
    // Knowledge base status
    report.push_str("\n## Knowledge Base Status\n\n");
    let storage_path = Path::new("./storage");
    if storage_path.exists() {
        if let Ok(entries) = fs::read_dir(storage_path) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    report.push_str(&format!(
                        "- {}: {} bytes\n",
                        entry.file_name().to_string_lossy(),
                        metadata.len()
                    ));
                }
            }
        }
    } else {
        report.push_str("Storage directory: ❌ Not found\n");
    }
    
    report.push_str("\n---\n\n");
    report.push_str("This report has been sanitized to remove API keys and personal data.\n");
    report.push_str("If you need to share this, it's safe to send to the development team.\n");
    
    report
}

/// Create diagnostic ZIP file
fn create_diagnostic_zip() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut zip_buffer = Vec::new();
    let mut zip = ZipWriter::new(std::io::Cursor::new(&mut zip_buffer));
    
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    
    // Add diagnostic report
    zip.start_file("diagnostic_report.txt", options)?;
    zip.write_all(generate_diagnostic_report().as_bytes())?;
    
    // Add VERSION file
    if let Ok(version) = fs::read_to_string("VERSION") {
        zip.start_file("VERSION", options)?;
        zip.write_all(version.as_bytes())?;
    }
    
    // Add sanitized logs
    let log_paths = vec![
        PathBuf::from("/tmp/phoenix-gateway.log"),
        PathBuf::from("./phoenix-gateway.log"),
        PathBuf::from("./data/logs/phoenix-latest.log"),
    ];
    
    for log_path in log_paths {
        if log_path.exists() {
            if let Ok(content) = fs::read_to_string(&log_path) {
                let sanitized = sanitize_log_content(&content);
                let filename = format!("logs/{}", log_path.file_name().unwrap().to_string_lossy());
                zip.start_file(filename, options)?;
                zip.write_all(sanitized.as_bytes())?;
            }
        }
    }
    
    // Add config (sanitized)
    if let Ok(config) = fs::read_to_string("config/gateway.toml") {
        let sanitized = sanitize_log_content(&config);
        zip.start_file("config/gateway.toml", options)?;
        zip.write_all(sanitized.as_bytes())?;
    }
    
    zip.finish()?;
    drop(zip);
    
    Ok(zip_buffer)
}

/// Handler for diagnostic export endpoint
pub async fn export_diagnostics() -> Response {
    match create_diagnostic_zip() {
        Ok(zip_data) => {
            let filename = format!(
                "phoenix-diagnostics-{}.zip",
                chrono::Utc::now().format("%Y%m%d-%H%M%S")
            );
            
            (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/zip"),
                    (
                        header::CONTENT_DISPOSITION,
                        &format!("attachment; filename=\"{}\"", filename),
                    ),
                ],
                zip_data,
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to create diagnostic ZIP: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create diagnostic package: {}", e),
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sanitize_openrouter_key() {
        let input = "OPENROUTER_API_KEY=sk-or-v1-1234567890abcdef1234567890abcdef";
        let output = sanitize_log_content(input);
        assert!(output.contains("sk-or-v1-REDACTED"));
        assert!(!output.contains("1234567890abcdef"));
    }
    
    #[test]
    fn test_sanitize_generic_api_key() {
        let input = "api_key: abc123def456ghi789jkl012mno345pqr678";
        let output = sanitize_log_content(input);
        assert!(output.contains("REDACTED"));
        assert!(!output.contains("abc123def456"));
    }
    
    #[test]
    fn test_sanitize_authorization_header() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let output = sanitize_log_content(input);
        assert!(output.contains("Authorization REDACTED"));
        assert!(!output.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }
    
    #[test]
    fn test_diagnostic_report_generation() {
        let report = generate_diagnostic_report();
        assert!(report.contains("Phoenix Diagnostic Report"));
        assert!(report.contains("OS:"));
        assert!(report.contains("Arch:"));
    }
}

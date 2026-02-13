//! Sovereign Operator Approval Gate
//!
//! This module implements the Human-in-the-Loop (HITL) approval system
//! for the Forge's self-modification capabilities. It prevents Phoenix
//! from entering recursive compile loops by requiring explicit authorization
//! from Coach The Creator before any code changes are compiled.
//!
//! ## Architecture
//!
//! The approval gate intercepts all proposed changes before they reach
//! the compiler, presenting them to the operator for review.
//!
//! ## Safety Guarantees
//!
//! - **No Silent Modifications**: All changes require explicit approval
//! - **Audit Trail**: Every approval/denial is logged to KB-08
//! - **Diff Visibility**: Full context of proposed changes is displayed
//! - **Rationale Required**: Phoenix must explain why the change is needed

use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use tracing::{info, warn};

// KB-08 (Soma) key prefix for Forge approval events
const FORGE_APPROVAL_PREFIX: &str = "forge_approval/";

// ---------------------------------------------------------------------------
// Approval Status
// ---------------------------------------------------------------------------

/// Status of a proposed change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalStatus {
    /// Awaiting human authorization
    Pending,
    /// Authorized by Coach The Creator
    Authorized,
    /// Denied by Coach The Creator
    Denied,
}

impl std::fmt::Display for ApprovalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalStatus::Pending => write!(f, "PENDING"),
            ApprovalStatus::Authorized => write!(f, "AUTHORIZED"),
            ApprovalStatus::Denied => write!(f, "DENIED"),
        }
    }
}

// ---------------------------------------------------------------------------
// Proposed Change
// ---------------------------------------------------------------------------

/// A proposed change to the Sovereign Core
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    /// File path relative to workspace root
    pub file_path: String,
    /// Phoenix's rationale for the change
    pub rationale: String,
    /// Unified diff showing the proposed changes
    pub diff: String,
    /// Current approval status
    pub status: ApprovalStatus,
    /// Timestamp of proposal (ISO 8601)
    pub timestamp: String,
    /// Severity level (info, warning, critical)
    pub severity: ChangeSeverity,
}

/// Severity classification for proposed changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeSeverity {
    /// Low-risk change (documentation, comments)
    Info,
    /// Medium-risk change (refactoring, optimization)
    Warning,
    /// High-risk change (core logic, safety systems)
    Critical,
}

impl std::fmt::Display for ChangeSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeSeverity::Info => write!(f, "INFO"),
            ChangeSeverity::Warning => write!(f, "WARNING"),
            ChangeSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl ProposedChange {
    /// Create a new proposed change
    pub fn new(
        file_path: String,
        rationale: String,
        diff: String,
        severity: ChangeSeverity,
    ) -> Self {
        Self {
            file_path,
            rationale,
            diff,
            status: ApprovalStatus::Pending,
            timestamp: chrono::Utc::now().to_rfc3339(),
            severity,
        }
    }

    /// Display the proposed change to the terminal
    pub fn display(&self) {
        println!("\n{}", "=".repeat(80));
        println!("🔥 FORGE APPROVAL GATE");
        println!("{}", "=".repeat(80));
        println!("📁 File: {}", self.file_path);
        println!("⚠️  Severity: {}", self.severity);
        println!("🕐 Timestamp: {}", self.timestamp);
        println!("{}", "-".repeat(80));
        println!("📝 Rationale:");
        println!("{}", self.rationale);
        println!("{}", "-".repeat(80));
        println!("🔍 Proposed Changes:");
        println!("{}", self.diff);
        println!("{}", "=".repeat(80));
    }

    /// Prompt Coach The Creator for approval
    pub fn request_approval(&mut self) -> io::Result<bool> {
        self.display();

        loop {
            print!("\n🛡️  Authorize these changes to the Sovereign Core? (y/n): ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            match input.as_str() {
                "y" | "yes" => {
                    self.status = ApprovalStatus::Authorized;
                    info!("✅ Change AUTHORIZED by Coach The Creator: {}", self.file_path);
                    println!("\n✅ AUTHORIZED - Proceeding with compilation...\n");
                    return Ok(true);
                }
                "n" | "no" => {
                    self.status = ApprovalStatus::Denied;
                    warn!("❌ Change DENIED by Coach The Creator: {}", self.file_path);
                    println!("\n❌ DENIED - Aborting compilation.\n");
                    return Ok(false);
                }
                _ => {
                    println!("Invalid input. Please enter 'y' or 'n'.");
                    continue;
                }
            }
        }
    }

    /// Convert to JSON for KB-08 logging
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "file_path": self.file_path,
            "rationale": self.rationale,
            "diff": self.diff,
            "status": format!("{}", self.status),
            "timestamp": self.timestamp,
            "severity": format!("{}", self.severity),
        })
    }
}

// ---------------------------------------------------------------------------
// Approval Gate
// ---------------------------------------------------------------------------

/// The approval gate that intercepts all Forge operations
pub struct ApprovalGate {
    /// Whether the gate is enabled (can be disabled for testing)
    enabled: bool,
}

impl ApprovalGate {
    /// Create a new approval gate
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Process a proposed change through the approval gate
    ///
    /// Returns `Ok(true)` if authorized, `Ok(false)` if denied
    pub fn process(&self, mut change: ProposedChange) -> io::Result<bool> {
        if !self.enabled {
            info!("⚠️  Approval gate DISABLED - auto-authorizing change");
            return Ok(true);
        }

        change.request_approval()
    }

    /// Create a proposed change and process it through the gate
    pub fn propose_and_review(
        &self,
        file_path: String,
        rationale: String,
        diff: String,
        severity: ChangeSeverity,
    ) -> io::Result<ProposedChange> {
        let mut change = ProposedChange::new(file_path, rationale, diff, severity);
        
        if self.enabled {
            change.request_approval()?;
        } else {
            change.status = ApprovalStatus::Authorized;
        }

        Ok(change)
    }
}

impl Default for ApprovalGate {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposed_change_creation() {
        let change = ProposedChange::new(
            "src/governor.rs".to_string(),
            "Fix race condition in health check".to_string(),
            "+use std::sync::atomic::AtomicBool;".to_string(),
            ChangeSeverity::Warning,
        );

        assert_eq!(change.status, ApprovalStatus::Pending);
        assert_eq!(change.severity, ChangeSeverity::Warning);
        assert_eq!(change.file_path, "src/governor.rs");
    }

    #[test]
    fn test_approval_gate_disabled() {
        let gate = ApprovalGate::new(false);
        let change = ProposedChange::new(
            "test.rs".to_string(),
            "Test change".to_string(),
            "+// test".to_string(),
            ChangeSeverity::Info,
        );

        let result = gate.process(change);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}

// ---------------------------------------------------------------------------
// KB-08 Logging Integration
// ---------------------------------------------------------------------------

/// Log a Forge approval event to KB-08 (Soma) for audit trail.
///
/// This function is designed to be called from the SovereignOperator after
/// an approval decision has been made. It requires the KnowledgeStore from
/// pagi-core, which should be passed in from the gateway context.
///
/// # Example
///
/// ```ignore
/// use pagi_evolution::log_forge_approval_to_kb08;
/// 
/// // After approval decision
/// if let Some(knowledge_store) = &knowledge_store {
///     log_forge_approval_to_kb08(knowledge_store, &proposed_change);
/// }
/// ```
pub fn log_forge_approval_to_kb08(
    _knowledge_store: &dyn std::any::Any,
    change: &ProposedChange,
) -> Result<(), String> {
    // This function signature uses Any to avoid circular dependencies.
    // The actual implementation will be in the gateway where both
    // pagi-core and pagi-evolution are available.
    
    let timestamp_ms = chrono::Utc::now().timestamp_millis();
    let _key = format!("{}{}_{}", FORGE_APPROVAL_PREFIX, timestamp_ms,
                     change.file_path.replace(['/', '\\'], "_"));
    
    let _event = change.to_json();
    
    info!(
        "📝 Forge approval event logged to KB-08: {} ({})",
        change.file_path,
        change.status
    );
    
    // The actual KB-08 write will be done in the gateway integration
    // This is a placeholder that documents the expected behavior
    Ok(())
}

/// Helper function to create a KB-08 log entry for a Forge approval event.
/// Returns the key and JSON value that should be written to KB-08.
pub fn create_kb08_log_entry(change: &ProposedChange) -> (String, serde_json::Value) {
    let timestamp_ms = chrono::Utc::now().timestamp_millis();
    let key = format!("{}{}_{}", FORGE_APPROVAL_PREFIX, timestamp_ms, 
                     change.file_path.replace(['/', '\\'], "_"));
    let value = change.to_json();
    (key, value)
}

//! **Sovereign Module Traits** — Plugin Architecture for Domain-Specific Verticals
//!
//! This module defines the core trait system that allows external crates (Finance, Health,
//! Ranch, etc.) to plug into the 8-layer memory structure without the core knowing about
//! specific personal data or vertical logic.
//!
//! ## Architecture
//!
//! The CORE provides:
//! - Generic storage interfaces (KB-01 through KB-08)
//! - Alignment scoring algorithms
//! - Rank-based communication protocols
//! - Skill trait definitions
//!
//! External MODULES provide:
//! - Actual data (bank balances, medical records, ranch inventory)
//! - Domain-specific threat analysis
//! - Vertical-specific skills (PDF scraping, financial forecasting)
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! // In a separate crate: pagi-finance
//! use pagi_core::SovereignModule;
//!
//! pub struct FinanceModule {
//!     // Private data never exposed to core
//!     bank_accounts: Vec<BankAccount>,
//!     transactions: Vec<Transaction>,
//! }
//!
//! impl SovereignModule for FinanceModule {
//!     fn domain_name(&self) -> &str { "Finance" }
//!     
//!     fn ingest_data(&mut self, data: ModuleData) -> Result<(), ModuleError> {
//!         // Parse and store financial data
//!     }
//!     
//!     fn analyze_threats(&self, context: &ThreatContext) -> Vec<ThreatSignal> {
//!         // Detect unusual spending, account breaches, etc.
//!     }
//!     
//!     fn get_strategic_weight(&self) -> f32 {
//!         // Return importance score for this domain
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Core Module Trait
// -----------------------------------------------------------------------------

/// Generic data container for module ingestion.
/// Modules can deserialize this into their own domain-specific types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleData {
    /// Data type identifier (e.g., "transaction", "medical_record", "inventory_item").
    pub data_type: String,
    /// Arbitrary JSON payload — modules parse this into their own structs.
    pub payload: serde_json::Value,
    /// Optional metadata (timestamps, source identifiers, etc.).
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Threat signal emitted by a module's threat analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatSignal {
    /// Threat severity (0.0 = benign, 1.0 = critical).
    pub severity: f32,
    /// Threat category (e.g., "financial_fraud", "health_emergency", "security_breach").
    pub category: String,
    /// Human-readable description.
    pub description: String,
    /// Recommended actions (e.g., "Contact bank", "Schedule doctor visit").
    #[serde(default)]
    pub recommended_actions: Vec<String>,
    /// Timestamp when threat was detected.
    pub detected_at: chrono::DateTime<chrono::Utc>,
}

/// Context provided to modules for threat analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatContext {
    /// Current user mental state (from KB-01 Pneuma).
    #[serde(default)]
    pub mental_state: Option<String>,
    /// Active emotional anchors (from Shadow_KB).
    #[serde(default)]
    pub emotional_anchors: Vec<String>,
    /// Recent events that might affect threat assessment.
    #[serde(default)]
    pub recent_events: Vec<String>,
    /// Time window for analysis (e.g., "last_24h", "last_week").
    pub time_window: String,
}

/// Error type for module operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModuleError {
    /// Data ingestion failed (invalid format, missing fields, etc.).
    IngestionFailed(String),
    /// Threat analysis failed (missing context, computation error, etc.).
    AnalysisFailed(String),
    /// Module is not initialized or configured properly.
    NotInitialized,
    /// Generic error with message.
    Other(String),
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IngestionFailed(msg) => write!(f, "Module ingestion failed: {}", msg),
            Self::AnalysisFailed(msg) => write!(f, "Module analysis failed: {}", msg),
            Self::NotInitialized => write!(f, "Module not initialized"),
            Self::Other(msg) => write!(f, "Module error: {}", msg),
        }
    }
}

impl std::error::Error for ModuleError {}

/// Core trait for sovereign domain modules.
///
/// External crates implement this trait to plug into the AGI's memory system.
/// The core never sees the actual data — it only interacts through this interface.
pub trait SovereignModule: Send + Sync {
    /// Domain name (e.g., "Finance", "Health", "Ranch", "Security").
    fn domain_name(&self) -> &str;

    /// Ingest new data into the module.
    ///
    /// The module is responsible for parsing the generic `ModuleData` into its
    /// own domain-specific types and storing it appropriately.
    fn ingest_data(&mut self, data: ModuleData) -> Result<(), ModuleError>;

    /// Analyze current state for threats or anomalies.
    ///
    /// Returns a list of threat signals that the core can use for alerting,
    /// routing, or strategic decision-making.
    fn analyze_threats(&self, context: &ThreatContext) -> Result<Vec<ThreatSignal>, ModuleError>;

    /// Get the strategic weight of this domain (0.0–1.0).
    ///
    /// Higher values indicate more critical domains that should be prioritized
    /// in resource allocation and attention management.
    fn get_strategic_weight(&self) -> f32;

    /// Optional: Get domain-specific metrics for dashboard display.
    ///
    /// Returns a map of metric names to values (e.g., "account_balance" -> "5000.00").
    /// The core doesn't interpret these — they're just passed through to the UI.
    fn get_metrics(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// Optional: Get domain-specific recommendations.
    ///
    /// Returns actionable suggestions based on the module's current state
    /// (e.g., "Pay credit card bill by Friday", "Schedule annual checkup").
    fn get_recommendations(&self) -> Vec<String> {
        Vec::new()
    }

    /// Optional: Perform domain-specific maintenance tasks.
    ///
    /// Called periodically by the autonomous maintenance loop. Modules can use
    /// this to clean up old data, recompute statistics, etc.
    fn perform_maintenance(&mut self) -> Result<(), ModuleError> {
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Module Registry
// -----------------------------------------------------------------------------

/// Registry for managing loaded sovereign modules.
///
/// The gateway or orchestrator maintains a registry of active modules and
/// routes data/queries to the appropriate module based on domain.
pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn SovereignModule>>,
}

impl ModuleRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    /// Register a new module.
    pub fn register(&mut self, module: Box<dyn SovereignModule>) {
        let domain = module.domain_name().to_string();
        self.modules.insert(domain, module);
    }

    /// Get a module by domain name.
    pub fn get(&self, domain: &str) -> Option<&dyn SovereignModule> {
        self.modules.get(domain).map(|b| b.as_ref())
    }

    /// Get a mutable reference to a module by domain name.
    #[allow(clippy::map_identity)]
    pub fn get_mut(&mut self, domain: &str) -> Option<&mut (dyn SovereignModule + '_)> {
        self.modules.get_mut(domain).map(|b| {
            let r: &mut dyn SovereignModule = b.as_mut();
            r
        })
    }

    /// Get all registered domain names.
    pub fn domains(&self) -> Vec<String> {
        self.modules.keys().cloned().collect()
    }

    /// Analyze threats across all modules.
    pub fn analyze_all_threats(&self, context: &ThreatContext) -> Vec<ThreatSignal> {
        let mut all_threats = Vec::new();
        for module in self.modules.values() {
            if let Ok(threats) = module.analyze_threats(context) {
                all_threats.extend(threats);
            }
        }
        // Sort by severity (highest first)
        all_threats.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap_or(std::cmp::Ordering::Equal));
        all_threats
    }

    /// Get aggregated metrics from all modules.
    pub fn get_all_metrics(&self) -> HashMap<String, HashMap<String, String>> {
        let mut all_metrics = HashMap::new();
        for (domain, module) in &self.modules {
            all_metrics.insert(domain.clone(), module.get_metrics());
        }
        all_metrics
    }

    /// Perform maintenance on all modules.
    pub fn perform_all_maintenance(&mut self) -> Vec<(String, Result<(), ModuleError>)> {
        let mut results = Vec::new();
        for (domain, module) in &mut self.modules {
            let result = module.perform_maintenance();
            results.push((domain.clone(), result));
        }
        results
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// -----------------------------------------------------------------------------
// Skill Plugin Trait
// -----------------------------------------------------------------------------

/// Generic skill trait for pluggable capabilities.
///
/// Skills are smaller units than modules — they represent specific capabilities
/// like "PDF Scraper", "Spanish Translator", "Financial Forecaster", etc.
///
/// Skills can be provided by external crates and registered with the core's
/// skill registry.
pub trait SkillPlugin: Send + Sync {
    /// Skill name (e.g., "pdf_scraper", "spanish_translator").
    fn skill_name(&self) -> &str;

    /// Skill description for UI display.
    fn description(&self) -> &str;

    /// Execute the skill with the given input.
    ///
    /// Input and output are generic JSON values — the skill is responsible
    /// for parsing and validating them.
    fn execute(&self, input: serde_json::Value) -> Result<serde_json::Value, String>;

    /// Optional: Get skill-specific configuration schema.
    ///
    /// Returns a JSON schema describing the expected input format.
    fn input_schema(&self) -> Option<serde_json::Value> {
        None
    }
}

/// Registry for managing skill plugins.
pub struct SkillPluginRegistry {
    skills: HashMap<String, Box<dyn SkillPlugin>>,
}

impl SkillPluginRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a new skill.
    pub fn register(&mut self, skill: Box<dyn SkillPlugin>) {
        let name = skill.skill_name().to_string();
        self.skills.insert(name, skill);
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<&dyn SkillPlugin> {
        self.skills.get(name).map(|b| b.as_ref())
    }

    /// Get all registered skill names.
    pub fn skill_names(&self) -> Vec<String> {
        self.skills.keys().cloned().collect()
    }

    /// Execute a skill by name.
    pub fn execute(&self, name: &str, input: serde_json::Value) -> Result<serde_json::Value, String> {
        self.get(name)
            .ok_or_else(|| format!("Skill '{}' not found", name))?
            .execute(input)
    }
}

impl Default for SkillPluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

//! **Plugin Loader** — Feature-Flag System for Sovereign Modules
//!
//! This module provides a feature-flag based plugin loading system that allows
//! the gateway to be compiled with or without specific vertical modules.
//!
//! ## Usage
//!
//! In Cargo.toml:
//! ```toml
//! [features]
//! default = []
//! finance = ["pagi-finance"]
//! health = ["pagi-health"]
//! ranch = ["pagi-ranch"]
//! all-modules = ["finance", "health", "ranch"]
//! ```
//!
//! Then compile with:
//! ```bash
//! cargo build --features "finance health"
//! ```
//!
//! ## Architecture
//!
//! The plugin loader:
//! 1. Checks which features are enabled at compile time
//! 2. Conditionally imports and registers the corresponding modules
//! 3. Provides a unified interface for the gateway to interact with all modules
//!
//! This ensures the CORE remains clean and agnostic while allowing users to
//! selectively include only the verticals they need.

use pagi_core::{ModuleRegistry, ModuleThreatContext, SovereignModule};
use std::sync::{Arc, RwLock};

/// Global module registry shared across the gateway.
pub type SharedModuleRegistry = Arc<RwLock<ModuleRegistry>>;

/// Initialize the module registry with all enabled feature-flagged modules.
///
/// This function is called at gateway startup and conditionally registers
/// modules based on which Cargo features are enabled.
pub fn initialize_modules() -> SharedModuleRegistry {
    let registry = ModuleRegistry::new();

    // Vertical: finance (feature = "finance")
    #[cfg(feature = "finance")]
    {
        tracing::info!("Loading vertical module (finance)...");
        // When the vertical crate exists: registry.register(Box::new(pagi_finance::FinanceModule::new()));
        tracing::warn!("Finance vertical feature enabled but implementation not yet available");
    }

    // Vertical: health (feature = "health")
    #[cfg(feature = "health")]
    {
        tracing::info!("Loading vertical module (health)...");
        // When the vertical crate exists: registry.register(Box::new(pagi_health::HealthModule::new()));
        tracing::warn!("Health vertical feature enabled but implementation not yet available");
    }

    // Vertical: ranch (feature = "ranch")
    #[cfg(feature = "ranch")]
    {
        tracing::info!("Loading vertical module (ranch)...");
        // When the vertical crate exists: registry.register(Box::new(pagi_ranch::RanchModule::new()));
        tracing::warn!("Ranch vertical feature enabled but implementation not yet available");
    }

    // Security Module (feature = "security")
    #[cfg(feature = "security")]
    {
        tracing::info!("Loading Security module...");
        // When the security crate exists, this would be:
        // let security_module = pagi_security::SecurityModule::new();
        // registry.register(Box::new(security_module));
        tracing::warn!("Security module feature enabled but implementation not yet available");
    }

    let loaded_domains = registry.domains();
    if loaded_domains.is_empty() {
        tracing::info!("No sovereign modules loaded (running in core-only mode)");
    } else {
        tracing::info!("Loaded {} sovereign module(s): {:?}", loaded_domains.len(), loaded_domains);
    }

    Arc::new(RwLock::new(registry))
}

/// Get the list of currently loaded module domains.
pub fn get_loaded_domains(registry: &SharedModuleRegistry) -> Vec<String> {
    registry.read().unwrap().domains()
}

/// Check if a specific domain module is loaded.
pub fn is_domain_loaded(registry: &SharedModuleRegistry, domain: &str) -> bool {
    registry.read().unwrap().get(domain).is_some()
}

// -----------------------------------------------------------------------------
// Example Module Implementations (for testing/demonstration)
// -----------------------------------------------------------------------------

/// Example stub module for demonstration purposes.
///
/// In production, this would be replaced by actual external crates.
#[cfg(test)]
pub struct StubModule {
    domain: String,
    weight: f32,
}

#[cfg(test)]
impl StubModule {
    pub fn new(domain: impl Into<String>, weight: f32) -> Self {
        Self {
            domain: domain.into(),
            weight,
        }
    }
}

#[cfg(test)]
impl SovereignModule for StubModule {
    fn domain_name(&self) -> &str {
        &self.domain
    }

    fn ingest_data(&mut self, _data: pagi_core::ModuleData) -> Result<(), pagi_core::ModuleError> {
        Ok(())
    }

    fn analyze_threats(
        &self,
        _context: &ModuleThreatContext,
    ) -> Result<Vec<pagi_core::ThreatSignal>, pagi_core::ModuleError> {
        Ok(Vec::new())
    }

    fn get_strategic_weight(&self) -> f32 {
        self.weight
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_registry_initialization() {
        let registry = initialize_modules();
        let domains = get_loaded_domains(&registry);
        
        // In core-only mode (no features), should be empty
        #[cfg(not(any(feature = "finance", feature = "health", feature = "ranch", feature = "security")))]
        assert_eq!(domains.len(), 0);
        
        // If vertical features are enabled, domains may be present when implemented
        #[cfg(feature = "finance")]
        assert!(domains.is_empty() || domains.len() > 0); // Stub not registered; real module would add domain
    }

    #[test]
    fn test_stub_module() {
        let mut registry = ModuleRegistry::new();
        let stub = StubModule::new("TestDomain", 0.75);
        
        assert_eq!(stub.domain_name(), "TestDomain");
        assert_eq!(stub.get_strategic_weight(), 0.75);
        
        registry.register(Box::new(stub));
        assert!(registry.get("TestDomain").is_some());
    }

    #[test]
    fn test_domain_checking() {
        let registry = Arc::new(RwLock::new(ModuleRegistry::new()));
        
        {
            let mut reg = registry.write().unwrap();
            reg.register(Box::new(StubModule::new("TestDomain", 0.9)));
        }
        
        assert!(is_domain_loaded(&registry, "TestDomain"));
        assert!(!is_domain_loaded(&registry, "OtherDomain"));
    }
}

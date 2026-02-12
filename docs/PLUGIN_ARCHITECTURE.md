# 🏛️ PAGI Sovereign Plugin Architecture

## Overview

The PAGI Core is designed as a **clean, agnostic AGI engine** that contains **zero personal data or vertical-specific logic**. All personal use-cases (Finance, Health, Ranch, Security, etc.) are implemented as **external modules** that plug into the 8-layer memory structure.

Think of the Core as the **Chassis and Brain**, while personal modules are **plug-and-play cartridges** that provide domain-specific functionality.

---

## 🎯 Architecture Principles

### Core Responsibilities (pagi-core)

The core provides:
- ✅ Generic storage interfaces (KB-01 through KB-08)
- ✅ Alignment scoring algorithms
- ✅ Rank-based communication protocols
- ✅ Skill trait definitions
- ✅ Emotional intelligence framework
- ✅ Autonomous maintenance loops

### Module Responsibilities (External Crates)

External modules provide:
- 🔒 Actual personal data (bank balances, medical records, inventory)
- 🔍 Domain-specific threat analysis
- 🛠️ Vertical-specific skills (PDF scraping, financial forecasting)
- 📊 Custom metrics and recommendations

---

## 📦 Creating a Sovereign Module

### Step 1: Create a New Crate

```bash
cargo new --lib pagi-finance
cd pagi-finance
```

### Step 2: Add Dependencies

```toml
# pagi-finance/Cargo.toml
[package]
name = "pagi-finance"
version = "0.1.0"
edition = "2021"

[dependencies]
pagi-core = { path = "../pagi-core" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
```

### Step 3: Implement the SovereignModule Trait

```rust
// pagi-finance/src/lib.rs
use pagi_core::{
    ModuleData, ModuleError, SovereignModule, ThreatContext, ThreatSignal,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Private data structures (never exposed to core)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BankAccount {
    account_id: String,
    institution: String,
    balance: f64,
    account_type: String, // "checking", "savings", "investment"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    id: String,
    date: chrono::DateTime<chrono::Utc>,
    amount: f64,
    category: String,
    description: String,
    account_id: String,
}

/// Finance module implementation
pub struct FinanceModule {
    accounts: Vec<BankAccount>,
    transactions: Vec<Transaction>,
    alert_threshold: f64, // Unusual spending threshold
}

impl FinanceModule {
    pub fn new() -> Self {
        Self {
            accounts: Vec::new(),
            transactions: Vec::new(),
            alert_threshold: 500.0, // Alert on transactions > $500
        }
    }

    /// Load data from environment or database
    pub fn load_from_env(&mut self) -> Result<(), ModuleError> {
        // In production, load from secure storage
        // For now, this is a stub
        Ok(())
    }

    /// Detect unusual spending patterns
    fn detect_unusual_spending(&self) -> Vec<ThreatSignal> {
        let mut threats = Vec::new();
        
        // Get recent transactions (last 7 days)
        let now = chrono::Utc::now();
        let week_ago = now - chrono::Duration::days(7);
        
        let recent_txs: Vec<_> = self.transactions
            .iter()
            .filter(|tx| tx.date > week_ago)
            .collect();
        
        // Check for large transactions
        for tx in recent_txs.iter() {
            if tx.amount.abs() > self.alert_threshold {
                threats.push(ThreatSignal {
                    severity: 0.6,
                    category: "unusual_spending".to_string(),
                    description: format!(
                        "Large transaction detected: ${:.2} at {}",
                        tx.amount, tx.description
                    ),
                    recommended_actions: vec![
                        "Review transaction details".to_string(),
                        "Verify with bank if unrecognized".to_string(),
                    ],
                    detected_at: now,
                });
            }
        }
        
        // Check for rapid succession of transactions
        if recent_txs.len() > 20 {
            threats.push(ThreatSignal {
                severity: 0.4,
                category: "high_transaction_volume".to_string(),
                description: format!(
                    "High transaction volume: {} transactions in the last 7 days",
                    recent_txs.len()
                ),
                recommended_actions: vec![
                    "Review spending patterns".to_string(),
                    "Consider budget adjustments".to_string(),
                ],
                detected_at: now,
            });
        }
        
        threats
    }
}

impl SovereignModule for FinanceModule {
    fn domain_name(&self) -> &str {
        "Finance"
    }

    fn ingest_data(&mut self, data: ModuleData) -> Result<(), ModuleError> {
        match data.data_type.as_str() {
            "bank_account" => {
                let account: BankAccount = serde_json::from_value(data.payload)
                    .map_err(|e| ModuleError::IngestionFailed(e.to_string()))?;
                self.accounts.push(account);
                Ok(())
            }
            "transaction" => {
                let transaction: Transaction = serde_json::from_value(data.payload)
                    .map_err(|e| ModuleError::IngestionFailed(e.to_string()))?;
                self.transactions.push(transaction);
                Ok(())
            }
            _ => Err(ModuleError::IngestionFailed(
                format!("Unknown data type: {}", data.data_type)
            )),
        }
    }

    fn analyze_threats(&self, context: &ThreatContext) -> Result<Vec<ThreatSignal>, ModuleError> {
        let mut threats = Vec::new();
        
        // Detect unusual spending
        threats.extend(self.detect_unusual_spending());
        
        // If user is in high stress state, flag any large pending bills
        if let Some(mental_state) = &context.mental_state {
            if mental_state.contains("stress") || mental_state.contains("anxiety") {
                // Check for upcoming bills that might add to stress
                // (In production, this would check a bill calendar)
                threats.push(ThreatSignal {
                    severity: 0.3,
                    category: "financial_stress_amplifier".to_string(),
                    description: "User is under stress - monitor for financial stressors".to_string(),
                    recommended_actions: vec![
                        "Review upcoming bills".to_string(),
                        "Consider payment plan options".to_string(),
                    ],
                    detected_at: chrono::Utc::now(),
                });
            }
        }
        
        Ok(threats)
    }

    fn get_strategic_weight(&self) -> f32 {
        // Finance is typically high priority
        0.85
    }

    fn get_metrics(&self) -> HashMap<String, String> {
        let mut metrics = HashMap::new();
        
        // Calculate total balance across all accounts
        let total_balance: f64 = self.accounts.iter().map(|a| a.balance).sum();
        metrics.insert("total_balance".to_string(), format!("${:.2}", total_balance));
        
        // Count accounts
        metrics.insert("account_count".to_string(), self.accounts.len().to_string());
        
        // Recent transaction count
        let now = chrono::Utc::now();
        let month_ago = now - chrono::Duration::days(30);
        let recent_count = self.transactions.iter()
            .filter(|tx| tx.date > month_ago)
            .count();
        metrics.insert("transactions_30d".to_string(), recent_count.to_string());
        
        metrics
    }

    fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Check for low balances
        for account in &self.accounts {
            if account.balance < 100.0 && account.account_type == "checking" {
                recommendations.push(format!(
                    "Low balance alert: {} has ${:.2}",
                    account.institution, account.balance
                ));
            }
        }
        
        // Suggest budget review if high spending
        let now = chrono::Utc::now();
        let month_ago = now - chrono::Duration::days(30);
        let monthly_spending: f64 = self.transactions.iter()
            .filter(|tx| tx.date > month_ago && tx.amount < 0.0)
            .map(|tx| tx.amount.abs())
            .sum();
        
        if monthly_spending > 3000.0 {
            recommendations.push(format!(
                "High spending this month: ${:.2} - consider budget review",
                monthly_spending
            ));
        }
        
        recommendations
    }

    fn perform_maintenance(&mut self) -> Result<(), ModuleError> {
        // Clean up old transactions (keep last 2 years)
        let cutoff = chrono::Utc::now() - chrono::Duration::days(730);
        self.transactions.retain(|tx| tx.date > cutoff);
        
        Ok(())
    }
}

impl Default for FinanceModule {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 🔌 Integrating with the Gateway

### Step 1: Add Feature Flag

```toml
# add-ons/pagi-gateway/Cargo.toml
[features]
finance = ["pagi-finance"]

[dependencies]
pagi-finance = { path = "../../pagi-finance", optional = true }
```

### Step 2: Update Plugin Loader

```rust
// add-ons/pagi-gateway/src/plugin_loader.rs

#[cfg(feature = "finance")]
{
    tracing::info!("Loading Finance module...");
    let finance_module = pagi_finance::FinanceModule::new();
    registry.register(Box::new(finance_module));
}
```

### Step 3: Compile with Feature

```bash
# Compile with finance module
cargo build --features finance

# Compile with multiple modules
cargo build --features "finance health ranch"

# Compile with all modules
cargo build --features all-modules
```

---

## 🛡️ Security Best Practices

### 1. Never Log Sensitive Data

```rust
// ❌ BAD
tracing::info!("Account balance: ${}", account.balance);

// ✅ GOOD
tracing::info!("Account balance updated");
```

### 2. Use Environment Variables

```rust
// Load sensitive config from environment
let api_key = std::env::var("FINANCE_API_KEY")
    .map_err(|_| ModuleError::NotInitialized)?;
```

### 3. Encrypt at Rest

```rust
// Use the Shadow Vault for sensitive data
use pagi_core::SecretVault;

let vault = SecretVault::new(store)?;
vault.write_anchor(EmotionalAnchor {
    anchor_type: "financial_stress".to_string(),
    intensity: 0.7,
    // ... encrypted automatically
})?;
```

### 4. Validate All Inputs

```rust
fn ingest_data(&mut self, data: ModuleData) -> Result<(), ModuleError> {
    // Validate before deserializing
    if data.data_type.is_empty() {
        return Err(ModuleError::IngestionFailed("Empty data type".to_string()));
    }
    
    // Use safe deserialization
    let account: BankAccount = serde_json::from_value(data.payload)
        .map_err(|e| ModuleError::IngestionFailed(e.to_string()))?;
    
    // Validate business logic
    if account.balance < -1_000_000.0 {
        return Err(ModuleError::IngestionFailed("Invalid balance".to_string()));
    }
    
    Ok(())
}
```

---

## 📊 Module Communication

Modules can communicate through the core's knowledge base:

```rust
// Write to KB-03 (Logos - Knowledge)
use pagi_core::{KnowledgeStore, KbType};

let store = KnowledgeStore::new("./data/pagi_vault")?;
store.set(
    KbType::Logos,
    "finance/summary",
    &serde_json::to_string(&summary)?
)?;

// Read from KB-07 (Kardia - Relationships)
if let Some(profile_json) = store.get(KbType::Kardia, "subjects/accountant")? {
    let profile: SubjectProfile = serde_json::from_str(&profile_json)?;
    // Use relationship data to contextualize financial advice
}
```

---

## 🧪 Testing Your Module

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_detection() {
        let mut module = FinanceModule::new();
        
        // Add test transaction
        let tx = Transaction {
            id: "test-1".to_string(),
            date: chrono::Utc::now(),
            amount: -1000.0, // Large transaction
            category: "shopping".to_string(),
            description: "Test Store".to_string(),
            account_id: "acc-1".to_string(),
        };
        
        module.transactions.push(tx);
        
        let context = ThreatContext {
            mental_state: None,
            emotional_anchors: Vec::new(),
            recent_events: Vec::new(),
            time_window: "last_7d".to_string(),
        };
        
        let threats = module.analyze_threats(&context).unwrap();
        assert!(!threats.is_empty(), "Should detect large transaction");
    }
}
```

---

## 🚀 Example Modules

### Health Module

```rust
pub struct HealthModule {
    vitals: Vec<VitalReading>,
    medications: Vec<Medication>,
    appointments: Vec<Appointment>,
}

impl SovereignModule for HealthModule {
    fn domain_name(&self) -> &str { "Health" }
    
    fn analyze_threats(&self, context: &ThreatContext) -> Result<Vec<ThreatSignal>, ModuleError> {
        // Detect missed medications, abnormal vitals, overdue checkups
    }
    
    fn get_strategic_weight(&self) -> f32 { 0.95 } // Health is critical
}
```

### Ranch Module

```rust
pub struct RanchModule {
    livestock: Vec<Animal>,
    equipment: Vec<Equipment>,
    tasks: Vec<RanchTask>,
}

impl SovereignModule for RanchModule {
    fn domain_name(&self) -> &str { "Ranch" }
    
    fn analyze_threats(&self, context: &ThreatContext) -> Result<Vec<ThreatSignal>, ModuleError> {
        // Detect sick animals, overdue maintenance, weather threats
    }
    
    fn get_strategic_weight(&self) -> f32 { 0.80 }
}
```

---

## 📝 Summary

The PAGI plugin architecture ensures:

1. ✅ **Core Remains Clean**: No personal data in the core codebase
2. ✅ **Modular Design**: Add/remove domains without touching core
3. ✅ **Compile-Time Safety**: Feature flags prevent unused code
4. ✅ **Security by Design**: Sensitive data stays in modules
5. ✅ **Professional Template**: Core can be sold/open-sourced separately

This architecture allows you to maintain a **professional-grade AGI template** while keeping your personal life binaries completely separate and secure.

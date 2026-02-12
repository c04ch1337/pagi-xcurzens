# 🏛️ Clean Core Architecture

## The Sovereign Engine Template

The PAGI Core is a **professional-grade AGI template** designed to be completely agnostic of personal data and vertical-specific logic. This document explains the architectural decisions that keep the core clean and ready for commercial use.

---

## 🎯 Design Philosophy

### The Chassis and Brain Metaphor

```
┌─────────────────────────────────────────────────────────┐
│                    PAGI CORE (Clean)                     │
│  ┌────────────────────────────────────────────────────┐ │
│  │  8-Layer Memory System (KB-01 through KB-08)       │ │
│  │  + Shadow Vault (KB-09) for encrypted data         │ │
│  └────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Orchestrator: MoE, Personas, Autonomous Loops     │ │
│  └────────────────────────────────────────────────────┘ │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Plugin System: SovereignModule + SkillPlugin      │ │
│  └────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
                           ▲
                           │ Plugs into
                           │
        ┌──────────────────┴──────────────────┐
        │                                     │
   ┌────▼─────┐  ┌──────────┐  ┌──────────┐  │
   │ Finance  │  │  Health  │  │  Ranch   │  │
   │ Module   │  │  Module  │  │  Module  │  │
   │ (Private)│  │ (Private)│  │ (Private)│  │
   └──────────┘  └──────────┘  └──────────┘  │
        Personal Data Lives Here (Separate Crates)
```

---

## 📦 What's in the Core?

### ✅ Included (Generic Infrastructure)

| Component | Purpose | Location |
|-----------|---------|----------|
| **Knowledge Base** | 8-slot memory system + Shadow Vault | [`crates/pagi-core/src/knowledge/`](../crates/pagi-core/src/knowledge/) |
| **Orchestrator** | MoE routing, personas, maintenance | [`crates/pagi-core/src/orchestrator/`](../crates/pagi-core/src/orchestrator/) |
| **Plugin Traits** | `SovereignModule`, `SkillPlugin` | [`crates/pagi-core/src/knowledge/traits.rs`](../crates/pagi-core/src/knowledge/traits.rs) |
| **Social Intelligence** | Generic relationship tracking | [`crates/pagi-core/src/social_intelligence.rs`](../crates/pagi-core/src/social_intelligence.rs) |
| **Secure Memory** | AES-256-GCM encryption primitives | [`crates/pagi-core/src/secure_memory.rs`](../crates/pagi-core/src/secure_memory.rs) |
| **Alignment Scoring** | Goal alignment algorithms | [`crates/pagi-core/src/knowledge/kb6.rs`](../crates/pagi-core/src/knowledge/kb6.rs) |

### ❌ Excluded (Vertical-Specific)

| Component | Why Excluded | Where It Goes |
|-----------|--------------|---------------|
| **Bank Account Data** | Personal financial info | External `pagi-finance` crate |
| **Medical Records** | Private health data | External `pagi-health` crate |
| **Ranch Inventory** | Specific business data | External `pagi-ranch` crate |
| **Specific Names** | Personal relationships | Stored in KB-07 as generic metadata |
| **Locations** | Geographic privacy | Environment variables or module config |

---

## 🔒 Data Separation Strategy

### 1. Generic Metadata Pattern

Instead of hardcoding specific fields, use `HashMap<String, String>`:

```rust
// ❌ BAD: Hardcoded personal fields
pub struct SubjectProfile {
    pub name: String,
    pub kids_names: Vec<String>,        // Too specific!
    pub astrology_sign: ZodiacSign,     // Too specific!
    pub ranch_role: String,             // Too specific!
}

// ✅ GOOD: Generic metadata
pub struct SubjectProfile {
    pub name: String,
    pub relationship: String,
    pub vertical_metadata: HashMap<String, String>, // Generic!
    // Ranch module can store: {"role": "Vet", "specialty": "Cattle"}
    // Finance module can store: {"role": "Accountant", "license": "CPA"}
}
```

### 2. Environment Variable Pattern

Sensitive configuration stays out of code:

```rust
// ❌ BAD: Hardcoded location
const RANCH_LOCATION: &str = "Montana";

// ✅ GOOD: Environment variable
let location = std::env::var("DOMAIN_LOCATION")
    .unwrap_or_else(|_| "Unknown".to_string());
```

### 3. Module Encapsulation Pattern

Personal data lives in external modules:

```rust
// Core only knows the interface
pub trait SovereignModule {
    fn domain_name(&self) -> &str;
    fn analyze_threats(&self, context: &ThreatContext) -> Result<Vec<ThreatSignal>, ModuleError>;
    // Core never sees the actual data!
}

// Module implementation (separate crate)
pub struct FinanceModule {
    bank_accounts: Vec<BankAccount>,  // Private!
    transactions: Vec<Transaction>,   // Private!
}
```

---

## 🧪 Verification Checklist

Use this checklist to ensure the core remains clean:

### Code Audit

- [ ] No hardcoded names (people, places, businesses)
- [ ] No specific financial amounts or account numbers
- [ ] No geographic locations (cities, states, addresses)
- [ ] No medical conditions or health data
- [ ] No business-specific inventory or assets
- [ ] All examples use generic placeholders

### Search Commands

```bash
# Search for potential personal data
rg -i "(ranch|farm|cattle|montana|texas)" crates/
rg -i "(bank|account.*\d{4}|balance.*\$)" crates/
rg -i "(medical|diagnosis|prescription)" crates/
rg -i "(john|jane|smith|doe)" crates/  # Common test names

# Should return only generic references or comments
```

### Configuration Audit

- [ ] All sensitive config in `.env.example` (not `.env`)
- [ ] No API keys or passwords in code
- [ ] Database paths use environment variables
- [ ] Feature flags for optional modules

---

## 🚀 Deployment Scenarios

### Scenario 1: Personal Use (Full Stack)

```bash
# Compile with all personal modules
cargo build --features "finance health ranch security"

# Set personal environment variables
export PAGI_SHADOW_KEY="your-32-byte-key"
export DOMAIN_LOCATION="Your Location"
export FINANCE_API_KEY="your-api-key"

# Run with personal data
./target/release/pagi-gateway
```

### Scenario 2: Commercial Template (Core Only)

```bash
# Compile core without any modules
cargo build

# No personal data needed
./target/release/pagi-gateway
```

### Scenario 3: Client Deployment (Custom Modules)

```bash
# Client creates their own modules
cargo new --lib client-inventory
cargo new --lib client-crm

# Compile with client modules
cargo build --features "client-inventory client-crm"
```

---

## 📊 Knowledge Base Slot Allocation

| Slot | Name | Purpose | Data Type | Personal Data? |
|------|------|---------|-----------|----------------|
| KB-01 | Pneuma | Identity, mission, goals | Generic goals | ❌ No |
| KB-02 | Oikos | Context, workspace scan | File paths | ⚠️ Paths only |
| KB-03 | Logos | Pure knowledge | Research, facts | ❌ No |
| KB-04 | Chronos | Temporal, conversation history | Chat logs | ⚠️ Encrypted |
| KB-05 | Techne | Skills, blueprints | Skill definitions | ❌ No |
| KB-06 | Ethos | Security, audit | Policies | ❌ No |
| KB-07 | Kardia | Relationships, preferences | Generic profiles | ⚠️ Metadata only |
| KB-08 | Soma | Execution buffer | Temp data | ❌ No |
| KB-09 | Shadow | Encrypted vault | Emotional anchors | ✅ Encrypted |

**Legend:**
- ❌ No personal data
- ⚠️ Generic/encrypted only
- ✅ Encrypted personal data (requires key)

---

## 🛡️ Security Layers

### Layer 1: Code Separation

Personal data never enters the core codebase.

### Layer 2: Feature Flags

Modules are opt-in at compile time:

```toml
[features]
default = []  # Core only
finance = ["pagi-finance"]
health = ["pagi-health"]
```

### Layer 3: Environment Variables

Sensitive config stays out of version control:

```bash
# .env.example (committed)
PAGI_SHADOW_KEY=your-key-here
DOMAIN_LOCATION=your-location

# .env (gitignored)
PAGI_SHADOW_KEY=actual-secret-key
DOMAIN_LOCATION=Montana
```

### Layer 4: Encryption at Rest

Shadow Vault uses AES-256-GCM:

```rust
let vault = SecretVault::new(store)?;
vault.write_anchor(EmotionalAnchor {
    anchor_type: "financial_stress".to_string(),
    intensity: 0.7,
    // Encrypted automatically with PAGI_SHADOW_KEY
})?;
```

---

## 📈 Commercialization Path

### Open Source Core

The clean core can be open-sourced:

```
pagi-core/          ✅ MIT/Apache License
pagi-orchestrator/  ✅ MIT/Apache License
pagi-gateway/       ✅ MIT/Apache License
```

### Private Modules

Personal modules stay private:

```
pagi-finance/       🔒 Private
pagi-health/        🔒 Private
pagi-ranch/         🔒 Private
```

### Commercial Template

Sell the core as a template:

```
PAGI Sovereign Engine Template
- 8-layer memory system
- Plugin architecture
- Autonomous maintenance
- Emotional intelligence
- Security protocols

Price: $X,XXX (one-time) or $XX/month (SaaS)
```

---

## 🎓 Best Practices

### 1. Always Use Traits

```rust
// Define the interface in core
pub trait SovereignModule { ... }

// Implement in external crate
impl SovereignModule for FinanceModule { ... }
```

### 2. Generic Data Structures

```rust
// Use serde_json::Value for flexibility
pub struct ModuleData {
    pub data_type: String,
    pub payload: serde_json::Value,  // Module parses this
}
```

### 3. Metadata Over Hardcoding

```rust
// Store domain-specific data as metadata
profile.vertical_metadata.insert("finance_role".to_string(), "CPA".to_string());
profile.vertical_metadata.insert("ranch_role".to_string(), "Vet".to_string());
```

### 4. Environment-Driven Config

```rust
// Load from environment
let config = CoreConfig {
    agent_id: env::var("PAGI_AGENT_ID").unwrap_or_else(|_| "default".to_string()),
    location: env::var("DOMAIN_LOCATION").ok(),
    // ...
};
```

---

## 🔍 Audit Trail

### Last Audit: 2026-02-07

**Findings:**
- ✅ No hardcoded personal names
- ✅ No specific locations
- ✅ No financial account numbers
- ✅ All examples use generic placeholders
- ✅ SubjectProfile uses generic metadata
- ✅ Plugin system properly isolated

**Next Audit:** Before any public release

---

## 📚 Related Documentation

- [Plugin Architecture Guide](./PLUGIN_ARCHITECTURE.md)
- [Security Best Practices](./SECURITY.md) (TODO)
- [Module Development Guide](./MODULE_DEVELOPMENT.md) (TODO)

---

## 🎯 Summary

The PAGI Core is a **clean, professional AGI template** that:

1. ✅ Contains **zero personal data**
2. ✅ Uses **generic interfaces** for all domains
3. ✅ Supports **pluggable modules** via feature flags
4. ✅ Encrypts sensitive data in the Shadow Vault
5. ✅ Can be **open-sourced or sold** separately

This architecture ensures you can maintain a commercial-grade product while keeping your personal life completely separate and secure.

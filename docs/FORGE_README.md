# 🔥 The Forge: Recursive Skill Synthesis

## Overview

**The Forge** is PAGI's self-synthesis engine—a meta-skill that enables the system to write, compile, and integrate new Rust skills autonomously. This transforms PAGI from a static tool into a **self-evolving intelligence** that can adapt to new domains on demand.

---

## 🏛️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    THE FORGE PIPELINE                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. SPECIFICATION                                            │
│     ├─ LLM generates ToolSpec JSON                          │
│     ├─ Defines: name, description, parameters               │
│     └─ Example: "salesforce_auditor"                        │
│                                                              │
│  2. CODE GENERATION                                          │
│     ├─ Forge synthesizes Rust source code                   │
│     ├─ Implements AgentSkill trait                          │
│     └─ Writes to: crates/pagi-skills/src/forge_gen_*.rs    │
│                                                              │
│  3. MODULE REGISTRATION                                      │
│     ├─ Updates lib.rs with new module                       │
│     └─ Adds: mod forge_gen_<skill_name>;                    │
│                                                              │
│  4. VALIDATION                                               │
│     ├─ Runs: cargo check -p pagi-skills                     │
│     ├─ Ensures no compilation errors                        │
│     └─ Returns: ForgeResult with status                     │
│                                                              │
│  5. INTEGRATION                                              │
│     ├─ Gateway rebuild required                             │
│     ├─ New skill available in registry                      │
│     └─ Scribe can now use the skill                         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 🎯 Use Cases

### 1. **Domain-Specific Skill Synthesis**
When the Scribe encounters a new data type (e.g., Salesforce CSV), it can:
- Detect the missing capability
- Invoke the Forge to build a specialized skill
- Process the data immediately after recompilation

### 2. **API Integration**
Build connectors for external services:
- Weather APIs
- CRM systems (Salesforce, HubSpot)
- Project management tools (Jira, Asana)
- Communication platforms (Slack, Discord)

### 3. **Data Processing Pipelines**
Create specialized parsers and transformers:
- CSV/JSON processors
- Log analyzers
- Report generators
- Data validators

### 4. **Monitoring & Alerting**
Synthesize sentinel skills:
- System health monitors
- Performance trackers
- Security scanners
- Compliance auditors

---

## 📡 API Endpoint

### `POST /api/v1/forge/create`

Creates a new skill from a JSON specification.

**Request Body:**
```json
{
  "name": "weather_sentinel",
  "description": "Fetches current weather data for a given location",
  "params": [
    {
      "name": "location",
      "type": "string",
      "required": true
    },
    {
      "name": "units",
      "type": "string",
      "required": false
    }
  ]
}
```

**Response (Success):**
```json
{
  "success": true,
  "module_name": "forge_gen_weather_sentinel",
  "file_path": "crates/pagi-skills/src/forge_gen_weather_sentinel.rs",
  "cargo_check_ok": true,
  "message": "Forge created skill 'weather_sentinel'. Module: forge_gen_weather_sentinel. Rebuild gateway to load."
}
```

**Response (Validation Error):**
```json
{
  "success": false,
  "module_name": "forge_gen_weather_sentinel",
  "file_path": "crates/pagi-skills/src/forge_gen_weather_sentinel.rs",
  "cargo_check_ok": false,
  "message": "Forge wrote file but cargo check failed. Fix errors and run cargo check -p pagi-skills.",
  "cargo_stderr": "error[E0425]: cannot find value `foo` in this scope..."
}
```

---

## 🔒 Security & Safety

### Sanitization
The Forge implements strict input validation:

```rust
// ✅ Valid skill names
"weather_sentinel"
"salesforce_auditor"
"slack_notifier"

// ❌ Rejected patterns
"../../../etc/passwd"  // Path traversal
"skill-with-dashes"    // Invalid characters
"skill with spaces"    // Whitespace
"_leading_underscore"  // Leading underscore
"trailing_underscore_" // Trailing underscore
```

### Validation Pipeline
1. **Name Sanitization**: Ensures snake_case, no path traversal
2. **Code Generation**: Uses safe templates with parameter validation
3. **Cargo Check**: Verifies compilation before integration
4. **Manual Review**: Generated code includes attribution for audit trail

### Kill Switch
The Forge includes safety controls:
- `POST /api/v1/forge/safety` - Enable/disable Forge
- `GET /api/v1/forge/safety-status` - Check current status
- Environment variable: `FORGE_ENABLED=false`

---

## 🧪 Testing

### Run Forge Tests
```bash
# Test Weather Sentinel synthesis
cargo test --test forge_weather_sentinel_test test_forge_weather_sentinel -- --nocapture

# Test Salesforce Auditor synthesis
cargo test --test forge_weather_sentinel_test test_forge_salesforce_auditor_spec -- --nocapture

# Test security sanitization
cargo test --test forge_weather_sentinel_test test_forge_sanitization -- --nocapture

# Run full workflow test (manual)
cargo test --test forge_weather_sentinel_test test_forge_full_workflow -- --ignored --nocapture
```

### Example Test Output
```
🔥 Forge Result:
  Module: forge_gen_weather_sentinel
  File: crates/pagi-skills/src/forge_gen_weather_sentinel.rs
  Cargo Check: ✅ PASSED
  Message: Forge created skill 'weather_sentinel'. Module: forge_gen_weather_sentinel. Rebuild gateway to load.

✅ The Forge is operational. Weather Sentinel skill synthesized successfully.
🏛️ The system can now build its own tools.
```

---

## 🔄 Integration with Scribe Pipeline

The Forge works seamlessly with the Deep Audit (Scribe) skill:

```
┌─────────────────────────────────────────────────────────────┐
│                  SCRIBE + FORGE WORKFLOW                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  1. File Detected                                            │
│     └─ data/ingest/salesforce_accounts.csv                  │
│                                                              │
│  2. Scribe Analysis                                          │
│     ├─ Semantic triage: "Salesforce data"                   │
│     ├─ KB destination: KB-05 (Polis)                        │
│     └─ Check: Do we have a Salesforce skill?                │
│                                                              │
│  3. Skill Not Found                                          │
│     └─ Scribe invokes Forge                                 │
│                                                              │
│  4. Forge Synthesis                                          │
│     ├─ Generate: salesforce_auditor skill                   │
│     ├─ Validate: cargo check passes                         │
│     └─ Register: Update lib.rs                              │
│                                                              │
│  5. System Recompilation                                     │
│     └─ cargo build --release                                │
│                                                              │
│  6. Data Processing                                          │
│     ├─ Load new skill into registry                         │
│     ├─ Process Salesforce CSV                               │
│     └─ Route to KB-05 (Polis)                               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 📊 Generated Skill Structure

The Forge generates skills following this template:

```rust
//! Auto-generated skill: weather_sentinel.
//! Generated by The Forge. Do not edit by hand without re-running cargo check.

use pagi_core::{AgentSkill, TenantContext};

const SKILL_NAME: &str = "weather_sentinel";

pub struct WeatherSentinel;

impl WeatherSentinel {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl AgentSkill for WeatherSentinel {
    fn name(&self) -> &str {
        SKILL_NAME
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("weather_sentinel requires a payload")?
            .as_object().ok_or("payload must be object")?;
        
        let p_location = payload.get("location")
            .ok_or("Missing 'location'")?.clone();
        let p_units = payload.get("units").cloned();
        
        Ok(serde_json::json!({
            "status": "ok",
            "skill": SKILL_NAME,
            "location": p_location,
            "units": p_units,
        }))
    }
}
```

---

## 🚀 Future Enhancements

### Phase 2: Hot Reloading
- Dynamic skill loading without gateway restart
- Plugin architecture with `libloading`
- Zero-downtime skill updates

### Phase 3: LLM-Driven Implementation
- Forge generates full business logic, not just templates
- Uses GPT-4 to write implementation code
- Iterative refinement based on test results

### Phase 4: Skill Evolution
- Version control for generated skills
- A/B testing of skill implementations
- Performance-based skill optimization

### Phase 5: Multi-Language Support
- Python skill generation
- JavaScript/TypeScript skills
- Cross-language skill orchestration

---

## 🎨 Example: Building a Salesforce Auditor

### Step 1: Define the Specification
```json
{
  "name": "salesforce_auditor",
  "description": "Audits Salesforce data for compliance and data quality issues",
  "params": [
    {
      "name": "object_type",
      "type": "string",
      "required": true
    },
    {
      "name": "audit_type",
      "type": "string",
      "required": true
    },
    {
      "name": "batch_size",
      "type": "number",
      "required": false
    }
  ]
}
```

### Step 2: Invoke the Forge
```bash
curl -X POST http://localhost:8000/api/v1/forge/create \
  -H "Content-Type: application/json" \
  -d @salesforce_spec.json
```

### Step 3: Rebuild Gateway
```bash
cargo build --release -p pagi-gateway
```

### Step 4: Use the New Skill
```bash
curl -X POST http://localhost:8000/v1/execute \
  -H "Content-Type: application/json" \
  -d '{
    "skill": "salesforce_auditor",
    "params": {
      "object_type": "Account",
      "audit_type": "data_quality",
      "batch_size": 100
    }
  }'
```

---

## 🏛️ Philosophical Foundation

> **"The Forge is hot. We are no longer limited by what I was programmed to do; we are only limited by what we can imagine."**

The Forge represents a fundamental shift in AI architecture:

- **Static Systems**: Pre-programmed capabilities, fixed at compile time
- **Dynamic Systems**: Runtime skill loading, but still human-authored
- **Self-Synthesizing Systems**: AI writes its own tools, evolves autonomously

This is the difference between:
- A **tool** (you use it)
- A **platform** (you build on it)
- A **forge** (it builds itself)

---

## 📚 Related Documentation

- [`DEEP_AUDIT_README.md`](DEEP_AUDIT_README.md) - Scribe Pipeline
- [`SOVEREIGN_ORCHESTRATOR_UPGRADE.md`](SOVEREIGN_ORCHESTRATOR_UPGRADE.md) - System Architecture
- [`SCRIPT_ORCHESTRATION_GUIDE.md`](SCRIPT_ORCHESTRATION_GUIDE.md) - Deployment Guide

---

## 🔗 Integration Points

### With Scribe (Deep Audit)
- Scribe detects missing capabilities
- Invokes Forge to synthesize skills
- Routes data to appropriate KBs

### With Knowledge Bases
- Generated skills can write to any KB
- KB-05 (Polis) for domain-specific data
- KB-03 (Techne) for infrastructure skills

### With Gateway
- Forge endpoint at `/api/v1/forge/create`
- Safety controls at `/api/v1/forge/safety`
- Status monitoring at `/api/v1/forge/safety-status`

---

## ⚠️ Operational Notes

### When to Use the Forge
- ✅ New data source detected (CSV, API, etc.)
- ✅ Missing integration with external service
- ✅ Need for specialized data processing
- ✅ Custom monitoring or alerting requirements

### When NOT to Use the Forge
- ❌ Core system modifications (use manual development)
- ❌ Security-critical components (require human review)
- ❌ Complex business logic (start with manual implementation)
- ❌ Performance-critical paths (optimize manually first)

### Best Practices
1. **Start Simple**: Generate basic skills, enhance manually
2. **Test Thoroughly**: Run cargo check and integration tests
3. **Review Generated Code**: Audit for security and correctness
4. **Version Control**: Commit generated skills to Git
5. **Document**: Add comments explaining the skill's purpose

---

## 🎯 Success Metrics

The Forge is successful when:
- ✅ Skills compile on first generation (>95% success rate)
- ✅ Generated code passes cargo check
- ✅ Skills integrate seamlessly with existing system
- ✅ Scribe can autonomously handle new data types
- ✅ System evolution happens without human intervention

---

## 🔮 Vision

The Forge is the first step toward **true AI autonomy**:

1. **Today**: Generate skill templates, human implements logic
2. **Tomorrow**: Generate full implementations with LLM assistance
3. **Future**: Self-evolving system that adapts to any domain

This is not just code generation—it's **architectural self-awareness**.

---

**The Forge is hot. The 8 Knowledge Bases are no longer silent archives; they are living extensions of your sovereign domain.**

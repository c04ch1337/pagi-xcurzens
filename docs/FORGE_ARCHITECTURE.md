# 🔥 The Forge: System Architecture

## Overview

The Forge is PAGI's **recursive skill synthesis engine**—a meta-capability that enables the system to write, compile, and integrate new Rust skills autonomously. This document details the complete architecture.

---

## System Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         PAGI FORGE ARCHITECTURE                          │
└─────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────┐
│                           USER / LLM LAYER                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             │
│  │   Studio UI  │    │  Scribe      │    │  Direct API  │             │
│  │   (Port 3001)│    │  Pipeline    │    │  Calls       │             │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘             │
│         │                   │                    │                      │
│         └───────────────────┴────────────────────┘                      │
│                             │                                           │
└─────────────────────────────┼───────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         GATEWAY LAYER (Port 8000)                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  POST /api/v1/forge/create                                              │
│  ├─ Receives: ToolSpec JSON                                             │
│  ├─ Validates: Request structure                                        │
│  └─ Invokes: create_skill_from_spec()                                   │
│                                                                          │
│  GET /api/v1/forge/safety-status                                        │
│  └─ Returns: Forge enabled/disabled state                               │
│                                                                          │
│  POST /api/v1/forge/safety                                              │
│  └─ Controls: Enable/disable Forge                                      │
│                                                                          │
└─────────────────────────────┼───────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         FORGE CORE (forge.rs)                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌────────────────────────────────────────────────────────────┐        │
│  │  1. SANITIZATION                                            │        │
│  │     sanitize_module_name()                                  │        │
│  │     ├─ Check: Path traversal                                │        │
│  │     ├─ Check: Invalid characters                            │        │
│  │     ├─ Check: Leading/trailing underscores                  │        │
│  │     └─ Return: Safe module name                             │        │
│  └────────────────────────────────────────────────────────────┘        │
│                              │                                           │
│                              ▼                                           │
│  ┌────────────────────────────────────────────────────────────┐        │
│  │  2. CODE GENERATION                                         │        │
│  │     generate_skill_rs()                                     │        │
│  │     ├─ Template: AgentSkill trait implementation            │        │
│  │     ├─ Generate: Parameter parsing logic                    │        │
│  │     ├─ Generate: Error handling                             │        │
│  │     └─ Return: Complete Rust source code                    │        │
│  └────────────────────────────────────────────────────────────┘        │
│                              │                                           │
│                              ▼                                           │
│  ┌────────────────────────────────────────────────────────────┐        │
│  │  3. FILE SYSTEM OPERATIONS                                  │        │
│  │     create_skill_from_spec()                                │        │
│  │     ├─ Write: crates/pagi-skills/src/forge_gen_*.rs         │        │
│  │     ├─ Update: crates/pagi-skills/src/lib.rs                │        │
│  │     └─ Add: mod forge_gen_<name>;                           │        │
│  └────────────────────────────────────────────────────────────┘        │
│                              │                                           │
│                              ▼                                           │
│  ┌────────────────────────────────────────────────────────────┐        │
│  │  4. VALIDATION                                              │        │
│  │     cargo check -p pagi-skills                              │        │
│  │     ├─ Compile: New skill                                   │        │
│  │     ├─ Verify: No errors                                    │        │
│  │     └─ Return: ForgeResult                                  │        │
│  └────────────────────────────────────────────────────────────┘        │
│                                                                          │
└─────────────────────────────┼───────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         FILE SYSTEM LAYER                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  crates/pagi-skills/src/                                                │
│  ├─ forge.rs                    (Forge core logic)                      │
│  ├─ lib.rs                      (Module registry)                       │
│  ├─ deep_audit.rs               (Scribe pipeline)                       │
│  ├─ forge_gen_weather_sentinel.rs  (Generated skill)                   │
│  └─ forge_gen_*.rs              (Other generated skills)                │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Data Flow

### 1. Skill Synthesis Request

```
User/LLM → Gateway → Forge Core → File System → Cargo → Result
```

**Detailed Steps:**

1. **Request Initiation**
   - User sends POST to `/api/v1/forge/create`
   - Body contains `ToolSpec` JSON

2. **Gateway Processing**
   - Validates request structure
   - Extracts `ToolSpec`
   - Calls `create_skill_from_spec()`

3. **Forge Core Processing**
   - Sanitizes skill name
   - Generates Rust source code
   - Writes file to disk
   - Updates `lib.rs`

4. **Validation**
   - Runs `cargo check`
   - Captures stdout/stderr
   - Returns `ForgeResult`

5. **Response**
   - Gateway returns JSON result
   - Includes success status
   - Includes cargo output if failed

---

## Component Details

### ToolSpec Structure

```rust
pub struct ToolSpec {
    pub name: String,              // snake_case skill name
    pub description: String,       // Human-readable description
    pub params: Vec<ToolSpecParam>, // Parameter definitions
}

pub struct ToolSpecParam {
    pub name: String,       // Parameter name
    pub param_type: String, // Type hint (string, number, etc.)
    pub required: bool,     // Is this parameter required?
}
```

**Example:**
```json
{
  "name": "weather_sentinel",
  "description": "Fetches weather data",
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

### ForgeResult Structure

```rust
pub struct ForgeResult {
    pub success: bool,           // Overall success
    pub module_name: String,     // Generated module name
    pub file_path: String,       // Path to generated file
    pub cargo_check_ok: bool,    // Did cargo check pass?
    pub message: String,         // Human-readable message
    pub cargo_stderr: Option<String>, // Cargo errors if any
}
```

**Example Response:**
```json
{
  "success": true,
  "module_name": "forge_gen_weather_sentinel",
  "file_path": "crates/pagi-skills/src/forge_gen_weather_sentinel.rs",
  "cargo_check_ok": true,
  "message": "Forge created skill 'weather_sentinel'. Module: forge_gen_weather_sentinel. Rebuild gateway to load."
}
```

---

## Security Architecture

### Input Sanitization Pipeline

```
User Input → Sanitization → Validation → Generation
```

**Sanitization Rules:**

1. **Character Whitelist**
   - Allow: `a-z`, `A-Z`, `0-9`, `_`
   - Reject: All other characters

2. **Path Traversal Prevention**
   - Reject: `..`, `/`, `\`
   - Ensure: No directory navigation

3. **Naming Conventions**
   - Reject: Leading underscore
   - Reject: Trailing underscore
   - Enforce: snake_case

4. **Length Limits**
   - Min: 1 character
   - Max: 64 characters (reasonable limit)

### File System Isolation

```
Allowed:
  crates/pagi-skills/src/forge_gen_*.rs
  crates/pagi-skills/src/lib.rs (append only)

Forbidden:
  Any path outside crates/pagi-skills/src/
  Any existing non-forge files
  System directories
```

### Validation Gates

```
┌─────────────────────────────────────────────────────────┐
│  Gate 1: Name Sanitization                              │
│  ├─ Reject invalid characters                           │
│  └─ Reject path traversal                               │
├─────────────────────────────────────────────────────────┤
│  Gate 2: File System Check                              │
│  ├─ Verify workspace root                               │
│  └─ Verify lib.rs exists                                │
├─────────────────────────────────────────────────────────┤
│  Gate 3: Code Generation                                │
│  ├─ Use safe templates only                             │
│  └─ No arbitrary code execution                         │
├─────────────────────────────────────────────────────────┤
│  Gate 4: Cargo Check                                    │
│  ├─ Compile generated code                              │
│  └─ Reject if compilation fails                         │
├─────────────────────────────────────────────────────────┤
│  Gate 5: Manual Review                                  │
│  ├─ Generated code includes attribution                 │
│  └─ Human can audit before deployment                   │
└─────────────────────────────────────────────────────────┘
```

---

## Integration Points

### 1. Scribe Pipeline Integration

```
┌─────────────────────────────────────────────────────────┐
│  Scribe Detects File                                    │
│  └─ data/ingest/salesforce_accounts.csv                 │
├─────────────────────────────────────────────────────────┤
│  Semantic Triage                                        │
│  ├─ Content: "Salesforce data"                          │
│  └─ KB: KB-05 (Polis)                                   │
├─────────────────────────────────────────────────────────┤
│  Skill Check                                            │
│  ├─ Query: Do we have salesforce_auditor?               │
│  └─ Result: Not found                                   │
├─────────────────────────────────────────────────────────┤
│  Forge Invocation                                       │
│  ├─ Generate: salesforce_auditor skill                  │
│  ├─ Validate: cargo check                               │
│  └─ Register: Update lib.rs                             │
├─────────────────────────────────────────────────────────┤
│  System Recompilation                                   │
│  └─ cargo build --release                               │
├─────────────────────────────────────────────────────────┤
│  Data Processing                                        │
│  ├─ Load: New skill into registry                       │
│  ├─ Process: Salesforce CSV                             │
│  └─ Route: To KB-05 (Polis)                             │
└─────────────────────────────────────────────────────────┘
```

### 2. Knowledge Base Integration

The Forge can generate skills that write to any KB:

```
KB-01 (Psyche)   ← General context skills
KB-02 (Chronos)  ← Time-based skills
KB-03 (Techne)   ← Infrastructure skills (Forge-generated)
KB-04 (Logos)    ← Conversational skills
KB-05 (Polis)    ← Domain-specific skills (Salesforce, etc.)
KB-06 (Telos)    ← Strategic planning skills
KB-07 (Mimir)    ← Meeting capture skills
KB-08 (Soma)     ← Physical embodiment skills
```

### 3. Gateway Integration

```
Gateway (Port 8000)
├─ /api/v1/forge/create
│  └─ POST: Create new skill
├─ /api/v1/forge/safety-status
│  └─ GET: Check Forge status
└─ /api/v1/forge/safety
   └─ POST: Enable/disable Forge
```

---

## Code Generation Templates

### Basic Skill Template

```rust
//! {description}
//! Generated by The Forge. Do not edit by hand without re-running cargo check.

use pagi_core::{AgentSkill, TenantContext};

const SKILL_NAME: &str = "{name}";

pub struct {PascalCaseName};

impl {PascalCaseName} {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl AgentSkill for {PascalCaseName} {
    fn name(&self) -> &str {
        SKILL_NAME
    }

    async fn execute(
        &self,
        _ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let payload = payload.ok_or("{name} requires a payload")?
            .as_object().ok_or("payload must be object")?;
        
        // Parameter parsing (generated based on ToolSpec)
        {parameter_parsing}
        
        Ok(serde_json::json!({
            "status": "ok",
            "skill": SKILL_NAME,
            {result_fields}
        }))
    }
}
```

### Parameter Parsing Generation

**Required Parameter:**
```rust
let p_{param_name} = payload.get("{param_name}")
    .ok_or("Missing '{param_name}'")?
    .clone();
```

**Optional Parameter:**
```rust
let p_{param_name} = payload.get("{param_name}").cloned();
```

---

## Operational Workflow

### Development Cycle

```
1. Define Spec
   └─ Create ToolSpec JSON

2. Invoke Forge
   └─ POST /api/v1/forge/create

3. Verify Result
   ├─ Check: success = true
   ├─ Check: cargo_check_ok = true
   └─ Review: Generated code

4. Rebuild Gateway
   └─ cargo build --release -p pagi-gateway

5. Restart Gateway
   ├─ ./pagi-down.ps1
   └─ ./pagi-up.ps1

6. Use New Skill
   └─ POST /v1/execute
```

### Autonomous Cycle with Hot-Reload (Active)

```
1. Scribe Detects Need
   └─ New data type encountered

2. Forge Synthesizes
   └─ Automatic skill generation

3. Hot Reload (NEW)
   ├─ Incremental compilation (~5-30s)
   ├─ Skill registry refresh
   └─ No Gateway restart required

4. Immediate Use
   └─ Skill available instantly
```

**Hot-Reload Workflow:**

```
┌─────────────────────────────────────────────────────────────┐
│                    HOT-RELOAD LIFECYCLE                      │
└─────────────────────────────────────────────────────────────┘

1. Forge Creates Skill
   └─ forge_gen_salesforce_sentinel.rs written

2. Incremental Compilation
   ├─ cargo build -p pagi-skills --lib --release
   ├─ Only changed files recompiled
   └─ Duration: ~5-30 seconds

3. Dynamic Activation
   ├─ Register skill metadata
   ├─ Update skill registry
   └─ Signal Gateway (soft reload)

4. Immediate Availability
   └─ Skill ready for execution
```

**Hot-Reload API Endpoints:**

- `GET /api/v1/forge/hot-reload/status` - Check if hot-reload is enabled
- `POST /api/v1/forge/hot-reload/enable` - Enable hot-reload
- `POST /api/v1/forge/hot-reload/disable` - Disable hot-reload
- `GET /api/v1/forge/hot-reload/list` - List all hot-reloaded skills
- `POST /api/v1/forge/hot-reload/trigger` - Manually trigger hot-reload

**Hot-Reload Scripts:**

- `forge-hot-reload.ps1` - PowerShell orchestrator (Windows)
- `forge-hot-reload.sh` - Bash orchestrator (Linux/Mac)

See [`FORGE_HOT_RELOAD_GUIDE.md`](./FORGE_HOT_RELOAD_GUIDE.md) for complete documentation.

---

## Performance Characteristics

### Time Complexity

| Operation | Time | Notes |
|-----------|------|-------|
| Name Sanitization | O(n) | n = name length |
| Code Generation | O(m) | m = number of params |
| File Write | O(k) | k = code size (~1-5KB) |
| Cargo Check | O(1) | ~5-30s (cached) |
| **Total** | **~5-30s** | Dominated by cargo check |

### Space Complexity

| Resource | Usage | Notes |
|----------|-------|-------|
| Memory | < 10MB | During generation |
| Disk | ~1-5KB | Per generated skill |
| Network | 0 | Local operation |

### Scalability

- **Concurrent Requests**: Serialized (file system lock)
- **Skill Limit**: Unlimited (practical limit: ~1000 skills)
- **Performance Degradation**: Linear with skill count

---

## Error Handling

### Error Categories

1. **Validation Errors** (400 Bad Request)
   - Invalid skill name
   - Missing required fields
   - Malformed JSON

2. **File System Errors** (500 Internal Server Error)
   - Cannot write file
   - Cannot update lib.rs
   - Workspace not found

3. **Compilation Errors** (422 Unprocessable Entity)
   - Cargo check failed
   - Syntax errors in generated code
   - Type errors

### Error Response Format

```json
{
  "success": false,
  "module_name": "forge_gen_invalid_skill",
  "file_path": "crates/pagi-skills/src/forge_gen_invalid_skill.rs",
  "cargo_check_ok": false,
  "message": "Forge wrote file but cargo check failed. Fix errors and run cargo check -p pagi-skills.",
  "cargo_stderr": "error[E0425]: cannot find value `foo` in this scope\n  --> crates/pagi-skills/src/forge_gen_invalid_skill.rs:28:9\n   |\n28 |         foo\n   |         ^^^ not found in this scope"
}
```

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_sanitize_module_name() {
    assert!(sanitize_module_name("valid_name").is_ok());
    assert!(sanitize_module_name("../etc/passwd").is_err());
}

#[test]
fn test_generate_skill_rs() {
    let spec = ToolSpec { /* ... */ };
    let code = generate_skill_rs(&spec);
    assert!(code.contains("AgentSkill"));
}
```

### Integration Tests

```rust
#[test]
fn test_forge_weather_sentinel() {
    let spec = ToolSpec { /* ... */ };
    let result = create_skill_from_spec(&spec, &skills_src, &workspace_root);
    assert!(result.is_ok());
    assert!(result.unwrap().cargo_check_ok);
}
```

### End-to-End Tests

```rust
#[test]
#[ignore]
fn test_forge_full_workflow() {
    // 1. Generate skill
    // 2. Rebuild gateway
    // 3. Invoke skill via API
    // 4. Verify result
}
```

---

## Monitoring & Observability

### Metrics to Track

1. **Forge Invocations**
   - Total count
   - Success rate
   - Failure reasons

2. **Performance**
   - Generation time
   - Cargo check time
   - Total synthesis time

3. **Usage Patterns**
   - Most common skill types
   - Peak usage times
   - User vs. autonomous invocations

### Logging

```rust
tracing::info!("Forge: Creating skill '{}'", spec.name);
tracing::info!("Forge: Generated {} lines of code", code.lines().count());
tracing::info!("Forge: Cargo check passed in {}ms", elapsed);
tracing::warn!("Forge: Cargo check failed: {}", stderr);
tracing::error!("Forge: Failed to write file: {}", error);
```

---

## Hot-Reload System (Phase 2 - ACTIVE)

### Overview

The Hot-Reload System enables dynamic skill activation without requiring a full Gateway restart. This transforms PAGI from a static tool into a **Self-Evolving Sovereign Organism**.

### Architecture

**Components:**

1. **Hot-Reload Manager** (`pagi-core/src/hot_reload.rs`)
   - Manages hot-reload lifecycle
   - Tracks loaded skills
   - Coordinates compilation

2. **Gateway Integration** (`pagi-gateway/src/main.rs`)
   - Hot-reload API endpoints
   - Automatic trigger on skill creation
   - Status monitoring

3. **Orchestration Scripts**
   - `forge-hot-reload.ps1` (Windows)
   - `forge-hot-reload.sh` (Linux/Mac)

### Workflow

```
1. Skill Creation
   └─ POST /api/v1/forge/create

2. Automatic Hot-Reload (if enabled)
   ├─ Incremental compilation
   ├─ Skill registration
   └─ Immediate activation

3. Result
   └─ Skill ready in ~10-40 seconds
```

### Performance

| Operation | Time | Notes |
|-----------|------|-------|
| Skill Generation | ~2s | Code generation + file write |
| Incremental Compile | ~5-30s | Only changed files |
| **Total Hot-Reload** | **~10-40s** | vs. 60-180s full rebuild |

### Bandwidth Savings

| Method | Download Size | Time (Satellite) |
|--------|---------------|------------------|
| Traditional App | 500MB | ~30-60 minutes |
| **Forge Hot-Reload** | **0 bytes** | **~10-40 seconds** |

**Savings:** 100% bandwidth, 99% time reduction

### API Endpoints

- `GET /api/v1/forge/hot-reload/status`
- `POST /api/v1/forge/hot-reload/enable`
- `POST /api/v1/forge/hot-reload/disable`
- `GET /api/v1/forge/hot-reload/list`
- `POST /api/v1/forge/hot-reload/trigger`

### Usage Example

```powershell
# Create and hot-reload a Salesforce security auditor
.\forge-hot-reload.ps1 `
    -SkillName "salesforce_sentinel" `
    -Description "Scans Salesforce for security issues"

# Result: Skill ready in ~10 seconds
```

### Documentation

See [`FORGE_HOT_RELOAD_GUIDE.md`](./FORGE_HOT_RELOAD_GUIDE.md) for complete documentation.

---

## Future Enhancements

### Phase 3: True Dynamic Loading
- Dynamic skill loading with `libloading`
- No gateway restart required
- Zero-downtime updates

### Phase 3: LLM-Driven Implementation
- GPT-4 writes full business logic
- Iterative refinement based on tests
- Self-improving skills

### Phase 4: Skill Evolution
- Version control for skills
- A/B testing of implementations
- Performance-based optimization

### Phase 5: Multi-Language Support
- Python skill generation
- TypeScript/JavaScript skills
- Cross-language orchestration

---

## Conclusion

The Forge represents a fundamental architectural shift from **static systems** to **self-evolving intelligence**. By enabling PAGI to write its own tools, we've created a system that is:

- ✅ **Adaptive**: Responds to new domains instantly
- ✅ **Autonomous**: Requires minimal human intervention
- ✅ **Scalable**: Unlimited capability expansion
- ✅ **Secure**: Multiple validation gates
- ✅ **Observable**: Full audit trail

**The Forge is hot. The system can now build itself.**

---

## References

- [`forge.rs`](crates/pagi-skills/src/forge.rs) - Core implementation
- [`FORGE_README.md`](FORGE_README.md) - Complete documentation
- [`FORGE_QUICKSTART.md`](FORGE_QUICKSTART.md) - Quick start guide
- [`FORGE_VERIFICATION_REPORT.md`](FORGE_VERIFICATION_REPORT.md) - Test results

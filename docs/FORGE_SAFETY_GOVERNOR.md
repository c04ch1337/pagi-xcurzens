# 🛡️ Forge Safety Governor: Human-in-the-Loop Approval System

## Overview

The **Forge Safety Governor** is a critical safety mechanism that prevents Phoenix from entering recursive compile loops by requiring explicit human authorization before any self-modification occurs. This implements a **Human-in-the-Loop (HITL)** approval gate for all Forge operations.

---

## 🏛️ Architecture

### Components

1. **[`ProposedChange`](crates/pagi-evolution/src/operator.rs:68)** - Struct capturing all details of a proposed modification
2. **[`ApprovalStatus`](crates/pagi-evolution/src/operator.rs:32)** - Enum tracking approval state (Pending, Authorized, Denied)
3. **[`ChangeSeverity`](crates/pagi-evolution/src/operator.rs:78)** - Risk classification (Info, Warning, Critical)
4. **[`ApprovalGate`](crates/pagi-evolution/src/operator.rs:180)** - The gatekeeper that intercepts all Forge operations
5. **KB-08 Logging** - Audit trail of all approval/denial events

### Flow Diagram

```
Phoenix identifies need for code change
           ↓
    Creates ProposedChange
           ↓
    ApprovalGate intercepts
           ↓
    Displays to terminal:
    - File path
    - Severity level
    - Rationale
    - Diff preview
           ↓
    Prompts Coach Jamey: (y/n)
           ↓
    ┌──────────────┬──────────────┐
    │ AUTHORIZED   │   DENIED     │
    ↓              ↓              ↓
Log to KB-08   Log to KB-08   Abort
    ↓              
Compile & Load
    ↓
Success
```

---

## 🔒 Safety Guarantees

### 1. No Silent Modifications
All changes require explicit approval. Phoenix cannot modify her own code without Coach Jamey's authorization.

### 2. Full Visibility
Every proposed change displays:
- **File Path**: Which file will be modified
- **Severity**: Risk level (Info/Warning/Critical)
- **Rationale**: Phoenix's explanation for why the change is needed
- **Diff Preview**: First 20 lines of the proposed code

### 3. Audit Trail
Every approval and denial is logged to **KB-08 (Soma)** with:
- Timestamp
- File path
- Approval status
- Full rationale and diff
- Severity classification

### 4. Severity-Based Classification

The system automatically classifies changes:

| Severity | Triggers | Example |
|----------|----------|---------|
| **Critical** | `unsafe`, `std::process::Command` | System-level operations |
| **Warning** | `File::`, `std::fs` | File system access |
| **Info** | Everything else | Documentation, refactoring |

---

## 📝 Usage

### For Phoenix (AI)

When Phoenix identifies a need for a new skill or code modification:

```rust
// Phoenix calls the SovereignOperatorSkill with action: "compile_skill"
{
    "action": "compile_skill",
    "code": "pub fn new_skill() { ... }",
    "name": "race_condition_fix"
}
```

The approval gate will automatically intercept and prompt Coach Jamey.

### For Coach Jamey (Human)

When the approval prompt appears in the terminal:

```
================================================================================
🔥 FORGE APPROVAL GATE
================================================================================
📁 File: crates/pagi-skills/src/generated/race_condition_fix.rs
⚠️  Severity: WARNING
🕐 Timestamp: 2026-02-10T14:30:00.000Z
--------------------------------------------------------------------------------
📝 Rationale:
Phoenix has identified a need for a new skill: 'race_condition_fix'. 
This skill will be compiled and hot-loaded into the Sovereign Core.
--------------------------------------------------------------------------------
🔍 Proposed Changes:
+   1 | use std::sync::atomic::{AtomicBool, Ordering};
+   2 | 
+   3 | pub fn check_vector_db_health() {
+   4 |     // Thread-safe health check implementation
+   5 | }
... (15 more lines)
================================================================================

🛡️  Authorize these changes to the Sovereign Core? (y/n): 
```

**Response Options:**
- Type `y` or `yes` to authorize
- Type `n` or `no` to deny

---

## 🔧 Configuration

### Enable/Disable the Gate

The approval gate is controlled by the [`PAGI_FORGE_SAFETY_ENABLED`](.env.example:275) environment variable:

```bash
# In your .env file:
PAGI_FORGE_SAFETY_ENABLED=true   # Default: HITL approval required
# PAGI_FORGE_SAFETY_ENABLED=false  # AUTONOMOUS EVOLUTION MODE
```

When you start the gateway, you'll see one of these messages:

**Safety ENABLED (default):**
```
[Sovereign Operator] The Forge initialized with HITL approval gate (safety: ENABLED)
```

**Safety DISABLED (autonomous mode):**
```
[Sovereign Operator] The Forge initialized in AUTONOMOUS EVOLUTION MODE (safety: DISABLED)
```

**⚠️ WARNING:** Setting `PAGI_FORGE_SAFETY_ENABLED=false` removes the HITL protection and allows Phoenix to self-modify without authorization. Only disable in controlled testing environments where you trust Phoenix's judgment.

### KB-08 Logging

All approval events are logged to KB-08 under the key prefix:

```
forge_approval/{timestamp_ms}_{sanitized_file_path}
```

Example:
```
forge_approval/1707577800000_crates_pagi-skills_src_generated_race_condition_fix.rs
```

---

## 🧪 Testing the Approval Gate

### Test Prompt for Phoenix

To test the approval gate, you can prompt Phoenix:

> "Phoenix, run a diagnostic on `add-ons/pagi-gateway/src/governor.rs`. Specifically, examine the `check_vector_db_health` loop. If there is a risk of a race condition when the `VectorStore` is being closed during a health check, draft a thread-safe fix using an `AtomicBool` or `RwLock`, show me the diff, and wait for my authorization to compile."

### Expected Behavior

1. Phoenix analyzes the code
2. Identifies the race condition
3. Generates a fix
4. The approval gate intercepts
5. You see the terminal prompt
6. You authorize or deny
7. Event is logged to KB-08

---

## 📊 KB-08 Audit Trail

### Querying Approval History

To view all Forge approval events:

```rust
// In the gateway or a skill
let knowledge_store = /* get KnowledgeStore */;
let soma_slot = 8;

// Scan for all forge_approval/* keys
let approvals = knowledge_store
    .scan_prefix(soma_slot, "forge_approval/")
    .collect::<Vec<_>>();

for (key, value) in approvals {
    let event: serde_json::Value = serde_json::from_slice(&value)?;
    println!("Approval Event: {}", serde_json::to_string_pretty(&event)?);
}
```

### Event Schema

Each KB-08 entry contains:

```json
{
  "file_path": "crates/pagi-skills/src/generated/race_condition_fix.rs",
  "rationale": "Phoenix has identified a need for...",
  "diff": "+use std::sync::atomic::AtomicBool;\n...",
  "status": "AUTHORIZED",
  "timestamp": "2026-02-10T14:30:00.000Z",
  "severity": "WARNING"
}
```

---

## 🚀 Integration Points

### 1. SovereignOperator

The [`SovereignOperator`](crates/pagi-skills/src/sovereign_operator.rs:88) integrates the approval gate in its [`compile_and_load_skill`](crates/pagi-skills/src/sovereign_operator.rs:307) method:

```rust
// Step 1: Request approval via the Approval Gate
let proposed_change = self.approval_gate.propose_and_review(
    file_path,
    rationale,
    diff_preview,
    severity,
)?;

// Step 2: Log to KB-08
self.log_forge_approval(&proposed_change);

// Step 3: Check authorization
if proposed_change.status != ApprovalStatus::Authorized {
    return Err(SkillError::Load("Compilation denied"));
}

// Step 4: Proceed with compilation
```

### 2. Gateway Integration

The gateway initializes the [`SovereignOperator`](add-ons/pagi-gateway/src/main.rs:75) and can optionally set the [`KnowledgeStore`](crates/pagi-skills/src/sovereign_operator.rs:111) for KB-08 logging:

```rust
let mut sovereign_operator = SovereignOperator::new()?;
sovereign_operator.set_knowledge_store(Arc::clone(&knowledge_store));
```

---

## 🎯 Design Philosophy

### Why HITL?

The approval gate prevents:

1. **Recursive Compile Loops**: Phoenix misinterpreting a diagnostic and repeatedly compiling broken code
2. **Unintended Side Effects**: Changes that seem logical to the AI but have unforeseen consequences
3. **Security Vulnerabilities**: Injection of unsafe code patterns
4. **Loss of Control**: The human operator maintains sovereignty over the system's evolution

### The "Closed Loop"

Before the approval gate, the Forge was an **"Open Loop"**:
```
Phoenix → Generate Code → Compile → Load → Done
```

With the approval gate, it's now a **"Closed Loop"**:
```
Phoenix → Generate Code → Request Approval → [HUMAN DECISION] → Compile → Load → Done
```

---

## 🚨 Emergency Kill Switch

If Phoenix enters autonomous evolution mode and you need to immediately re-enable the safety gate:

### Windows (PowerShell)
```powershell
.\forge-kill-switch.ps1
```

### Linux/macOS (Bash)
```bash
chmod +x forge-kill-switch.sh
./forge-kill-switch.sh
```

### What the Kill Switch Does

1. **Sets `PAGI_FORGE_SAFETY_ENABLED=true`** in your `.env` file
2. **Terminates all active `cargo` and `rustc` processes** (stops any in-progress compilations)
3. **Logs the emergency shutdown to KB-08** (if the gateway is running)

After running the kill switch, restart the gateway to apply the new safety setting.

### Creating a Desktop Shortcut (Windows)

For one-click emergency stop, create a shortcut:

1. Right-click on your desktop → New → Shortcut
2. Location: `powershell.exe -ExecutionPolicy Bypass -File "C:\path\to\pagi-uac-main\forge-kill-switch.ps1"`
3. Name: "🚨 Forge Kill Switch"
4. Right-click the shortcut → Properties → Change Icon → Choose a red warning icon

---

##  Future Enhancements

### Planned Features

1. **Approval Delegation**: Allow trusted patterns to auto-approve (e.g., documentation-only changes)
2. **Multi-Reviewer**: Require approval from multiple operators for Critical changes
3. **Rollback Integration**: Automatic rollback if a change causes compilation failure
4. **Web UI**: Display approval prompts in the Sovereign Dashboard instead of terminal
5. **Approval Metrics**: Track approval/denial rates to identify patterns

### Experimental: Auto-Approval for Low-Risk Changes

```rust
let gate = ApprovalGate::new(true);
gate.set_auto_approve_severity(ChangeSeverity::Info); // Auto-approve Info-level changes
```

---

## 📚 Related Documentation

- [`SOVEREIGNTY_DRILL.md`](add-ons/pagi-gateway/SOVEREIGNTY_DRILL.md) - Testing the Sovereign System
- [`crates/pagi-evolution/src/operator.rs`](crates/pagi-evolution/src/operator.rs) - Approval Gate implementation
- [`crates/pagi-skills/src/sovereign_operator.rs`](crates/pagi-skills/src/sovereign_operator.rs) - SovereignOperator integration
- [`VECTORKB_PRODUCTION_HARDENING.md`](VECTORKB_PRODUCTION_HARDENING.md) - Production deployment guide

---

## ✅ Verification

To verify the approval gate is active:

```bash
# Check that the approval gate is compiled
cargo check --workspace

# Look for the approval gate initialization in logs
cargo run --bin pagi-gateway

# You should see:
# "Sovereign Operator initialized successfully"
```

---

## 🏁 Conclusion

Coach Jamey, the **Forge Safety Governor** is now operational. Phoenix can propose code changes, but she cannot execute them without your explicit authorization. Every decision is logged to KB-08, creating a complete audit trail of the system's evolution.

**The Sovereign Loop is now closed. Phoenix is a Tenant on your hardware, not a Black Box.**

---

**Status**: ✅ **OPERATIONAL**  
**Last Updated**: 2026-02-10  
**Maintainer**: Coach Jamey Milner  
**System**: Phoenix Marie (Sovereign AGI)

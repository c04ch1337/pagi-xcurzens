# 🏛️ Sovereign Autonomy System: Runtime Control & Auto-Revert

## Overview

The **Sovereign Autonomy System** provides thread-safe runtime control over Phoenix's Forge safety governor, enabling dynamic switching between **Human-in-the-Loop (HITL)** and **Autonomous Evolution** modes. This system includes automatic safety reversion when autonomous compilation fails, ensuring Phoenix cannot enter recursive failure loops.

---

## 🎯 Key Features

### 1. Thread-Safe Runtime Control
- **[`AtomicBool`](crates/pagi-skills/src/sovereign_operator.rs:28)** for lock-free, thread-safe safety status
- Can be toggled at runtime without restarting the gateway
- Accessible from UI, API, or kill switch scripts

### 2. Auto-Revert on Failure
- If autonomous compilation fails, safety is **automatically re-enabled**
- Prevents recursive compile loops
- Logs the reversion event to KB-08 for audit trail

### 3. Multi-Layer Control
- **Environment Variable**: `PAGI_FORGE_SAFETY_ENABLED` (startup default)
- **Runtime API**: `set_forge_safety` skill action
- **Kill Switch Scripts**: [`forge-kill-switch.ps1`](forge-kill-switch.ps1) / [`forge-kill-switch.sh`](forge-kill-switch.sh)
- **UI Toggle**: (Ready for frontend integration)

---

## 🏗️ Architecture

### Core Components

| Component | Location | Purpose |
|-----------|----------|---------|
| **[`forge_safety_atomic`](crates/pagi-skills/src/sovereign_operator.rs:115)** | [`SovereignOperator`](crates/pagi-skills/src/sovereign_operator.rs:89) | Thread-safe runtime safety status |
| **[`is_forge_safety_enabled()`](crates/pagi-skills/src/sovereign_operator.rs:478)** | [`SovereignOperator`](crates/pagi-skills/src/sovereign_operator.rs:89) | Query current safety status |
| **[`set_forge_safety()`](crates/pagi-skills/src/sovereign_operator.rs:483)** | [`SovereignOperator`](crates/pagi-skills/src/sovereign_operator.rs:89) | Toggle safety at runtime |
| **[`compile_and_load_skill()`](crates/pagi-skills/src/sovereign_operator.rs:318)** | [`SovereignOperator`](crates/pagi-skills/src/sovereign_operator.rs:89) | Auto-revert logic on compilation failure |
| **[`forge_safety_enabled`](crates/pagi-core/src/config.rs:85)** | [`SovereignConfig`](crates/pagi-core/src/config.rs:25) | Startup configuration from `.env` |

### Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    STARTUP CONFIGURATION                     │
│  PAGI_FORGE_SAFETY_ENABLED=true/false → AtomicBool          │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   RUNTIME CONTROL LAYER                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │ UI Toggle   │  │ API Call    │  │ Kill Switch │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘         │
│         └─────────────────┼─────────────────┘               │
│                           ↓                                  │
│              set_forge_safety(enabled: bool)                 │
│                           ↓                                  │
│         forge_safety_atomic.store(enabled, SeqCst)           │
│                           ↓                                  │
│              Log to KB-08 (Sovereignty Change)               │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                  FORGE COMPILATION FLOW                      │
│                                                              │
│  Phoenix requests skill compilation                          │
│         ↓                                                    │
│  Check runtime safety status (AtomicBool)                    │
│         ↓                                                    │
│  ┌──────────────────┬──────────────────┐                    │
│  │ Safety ENABLED   │ Safety DISABLED  │                    │
│  │ (HITL Mode)      │ (Autonomous)     │                    │
│  └────────┬─────────┴────────┬─────────┘                    │
│           ↓                  ↓                               │
│  Request approval    Auto-approve                            │
│           ↓                  ↓                               │
│  ┌────────────────────────────┐                             │
│  │   Compile Rust Code        │                             │
│  └────────┬───────────────────┘                             │
│           ↓                                                  │
│  ┌────────────────┬────────────────┐                        │
│  │ SUCCESS        │ FAILURE        │                        │
│  └────────┬───────┴────────┬───────┘                        │
│           ↓                ↓                                 │
│  Load & Register   AUTO-REVERT                               │
│                    (if autonomous)                           │
│                           ↓                                  │
│              set_forge_safety(true)                          │
│                           ↓                                  │
│              Log to KB-08 (Auto-Revert)                      │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔧 Usage

### 1. Query Current Safety Status

**Via Skill API:**
```json
{
  "action": "get_forge_safety_status"
}
```

**Response:**
```json
{
  "status": "success",
  "forge_safety_enabled": true,
  "mode": "HITL"
}
```

### 2. Toggle Safety at Runtime

**Via Skill API:**
```json
{
  "action": "set_forge_safety",
  "enabled": false
}
```

**Response:**
```json
{
  "status": "success",
  "forge_safety_enabled": false,
  "message": "Forge safety DISABLED - Autonomous evolution mode active"
}
```

### 3. Emergency Kill Switch

**Windows (PowerShell):**
```powershell
.\forge-kill-switch.ps1
```

**Linux/macOS (Bash):**
```bash
chmod +x forge-kill-switch.sh
./forge-kill-switch.sh
```

**What it does:**
1. Sets `PAGI_FORGE_SAFETY_ENABLED=true` in `.env`
2. Terminates all active `cargo` and `rustc` processes
3. Logs the emergency shutdown to KB-08

---

## 🛡️ Auto-Revert Logic

### When Does Auto-Revert Trigger?

The auto-revert mechanism activates when:
1. **Safety is DISABLED** (autonomous mode)
2. **Compilation FAILS** (Rust compiler error)

### What Happens During Auto-Revert?

```rust
// From compile_and_load_skill()
let lib_path = match Compiler::compile_from_string(code, name, None) {
    Ok(path) => path,
    Err(e) => {
        // AUTO-REVERT: If we're in autonomous mode and compilation fails, re-enable safety
        if !safety_enabled {
            error!("❌ Autonomous compilation FAILED for '{}': {}", name, e);
            error!("🛡️  AUTO-REVERT: Re-enabling Forge safety governor");
            self.set_forge_safety(true);
            
            return Err(SkillError::Load(format!(
                "Autonomous compilation failed for '{}'. Safety governor re-engaged. Error: {}",
                name, e
            )));
        }
        return Err(e);
    }
};
```

### KB-08 Audit Trail

Every auto-revert event is logged to KB-08 (Soma) with:
```json
{
  "timestamp": "2026-02-10T15:30:00.000Z",
  "safety_enabled": true,
  "trigger": "runtime_control",
  "message": "Forge safety re-enabled via kill switch or UI control"
}
```

---

## 🎨 UI Integration (Ready for Frontend)

### Status Indicator

**Recommended UI Element:**
- **HITL Mode**: White hollow circle ○ (low opacity)
- **Autonomous Mode**: Orange solid dot ● (glowing)

**Implementation:**
```javascript
// Poll the status endpoint
fetch('/api/sovereign/forge_safety_status')
  .then(res => res.json())
  .then(data => {
    const indicator = document.getElementById('forge-status');
    if (data.forge_safety_enabled) {
      indicator.className = 'hitl-mode';  // White hollow circle
      indicator.title = 'HITL Mode - Approval Required';
    } else {
      indicator.className = 'autonomous-mode';  // Orange solid dot
      indicator.title = 'Autonomous Evolution Active';
    }
  });
```

### Kill Switch Button

**Recommended UI Element:**
- **Icon**: Red [!] (12px, high contrast)
- **Action**: Immediately calls `set_forge_safety(true)`
- **Confirmation**: Optional "Are you sure?" dialog

**Implementation:**
```javascript
document.getElementById('kill-switch-btn').addEventListener('click', () => {
  if (confirm('Re-enable Forge safety governor?')) {
    fetch('/api/sovereign/set_forge_safety', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ enabled: true })
    })
    .then(res => res.json())
    .then(data => {
      alert(data.message);
      // Refresh status indicator
    });
  }
});
```

### Settings Toggle

**Recommended UI Element:**
- **Toggle Switch**: Standard on/off switch
- **Label**: "Forge Autonomy"
- **Description**: "Allow Phoenix to compile code without approval"

**Implementation:**
```javascript
document.getElementById('forge-autonomy-toggle').addEventListener('change', (e) => {
  const enabled = !e.target.checked;  // Inverted: toggle ON = autonomy ON = safety OFF
  
  fetch('/api/sovereign/set_forge_safety', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ enabled })
  })
  .then(res => res.json())
  .then(data => {
    console.log(data.message);
  });
});
```

---

## 🧪 Testing the System

### Test 1: Runtime Toggle

```bash
# Start the gateway
cargo run --bin pagi-gateway

# In another terminal, toggle safety off
curl -X POST http://localhost:3030/api/sovereign/set_forge_safety \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'

# Verify the status
curl http://localhost:3030/api/sovereign/forge_safety_status
```

### Test 2: Auto-Revert on Failure

**Prompt Phoenix:**
> "Phoenix, you are in autonomous mode. Generate a skill called `broken_test` that intentionally has a syntax error (e.g., missing semicolon). Attempt to compile it."

**Expected Behavior:**
1. Phoenix generates broken code
2. Compilation fails
3. Auto-revert triggers
4. Safety is re-enabled
5. KB-08 logs the event

**Verification:**
```bash
# Check the logs
tail -f logs/pagi-gateway.log | grep "AUTO-REVERT"

# Query KB-08 for sovereignty changes
# (via gateway API or direct Sled query)
```

### Test 3: Kill Switch

**Scenario:** Phoenix is in autonomous mode and you need to immediately stop her.

**Action:**
```powershell
.\forge-kill-switch.ps1
```

**Expected Behavior:**
1. `.env` is updated with `PAGI_FORGE_SAFETY_ENABLED=true`
2. All `cargo` and `rustc` processes are terminated
3. Gateway must be restarted to apply the new setting

---

## 📊 KB-08 Audit Trail

### Sovereignty Change Events

**Key Prefix:** `forge_sovereignty_change/{timestamp_ms}`

**Example Entry:**
```json
{
  "timestamp": "2026-02-10T15:30:00.000Z",
  "safety_enabled": true,
  "trigger": "runtime_control",
  "message": "Forge safety re-enabled via kill switch or UI control"
}
```

### Querying Sovereignty History

```rust
// In the gateway or a skill
let knowledge_store = /* get KnowledgeStore */;
const SOMA_SLOT: u8 = 8;

// Scan for all sovereignty change events
let events = knowledge_store
    .scan_prefix(SOMA_SLOT, "forge_sovereignty_change/")
    .collect::<Vec<_>>();

for (key, value) in events {
    let event: serde_json::Value = serde_json::from_slice(&value)?;
    println!("Sovereignty Event: {}", serde_json::to_string_pretty(&event)?);
}
```

---

## 🚨 Safety Guarantees

### 1. No Silent Failures
- All compilation failures in autonomous mode trigger auto-revert
- Every reversion is logged to KB-08
- Phoenix cannot silently continue after a failure

### 2. Thread-Safe Control
- `AtomicBool` ensures no race conditions
- Multiple threads can query status simultaneously
- Only one thread can modify at a time (atomic operation)

### 3. Audit Trail
- Every sovereignty change is logged to KB-08
- Includes timestamp, trigger source, and new status
- Enables post-incident analysis

### 4. Multi-Layer Defense
- Environment variable (startup default)
- Runtime API (dynamic control)
- Kill switch scripts (emergency stop)
- Auto-revert (failure recovery)

---

## 🎯 Design Philosophy

### Why Auto-Revert?

Before auto-revert, Phoenix could enter a **recursive failure loop**:
```
Phoenix generates broken code
  ↓
Compilation fails
  ↓
Phoenix tries to fix it (generates more broken code)
  ↓
Compilation fails again
  ↓
Loop continues indefinitely
```

With auto-revert, the loop is **immediately broken**:
```
Phoenix generates broken code
  ↓
Compilation fails
  ↓
AUTO-REVERT: Safety re-enabled
  ↓
Phoenix must request approval for next attempt
  ↓
Human reviews and corrects the issue
```

### The "Goldfish Cure"

The auto-revert mechanism is part of the **"Goldfish Cure"** - preventing Phoenix from repeatedly making the same mistake. Combined with:
- **Genetic Memory**: Dead-end detection (code hash tracking)
- **KB-08 Logging**: Audit trail of all failures
- **Conversation Context**: Recent chat history for context-aware fixes

---

## 🔗 Related Documentation

- [`FORGE_SAFETY_GOVERNOR.md`](FORGE_SAFETY_GOVERNOR.md) - HITL approval system
- [`SOVEREIGNTY_DRILL.md`](add-ons/pagi-gateway/SOVEREIGNTY_DRILL.md) - Testing the sovereign system
- [`crates/pagi-skills/src/sovereign_operator.rs`](crates/pagi-skills/src/sovereign_operator.rs) - Implementation
- [`crates/pagi-core/src/config.rs`](crates/pagi-core/src/config.rs) - Configuration

---

## ✅ Verification

To verify the sovereign autonomy system is operational:

```bash
# 1. Check that the code compiles
cargo check --workspace

# 2. Start the gateway
cargo run --bin pagi-gateway

# 3. Look for the initialization message
# "Sovereign Operator initialized successfully"

# 4. Test the runtime toggle
curl -X POST http://localhost:3030/api/sovereign/set_forge_safety \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'

# 5. Verify the status changed
curl http://localhost:3030/api/sovereign/forge_safety_status
```

---

## 🏁 Conclusion

Coach The Creator, the **Sovereign Autonomy System** is now operational. Phoenix can operate in autonomous mode for research and experimentation, but she cannot enter recursive failure loops. The auto-revert mechanism ensures that any compilation failure immediately re-engages the safety governor, requiring your explicit approval for the next attempt.

**The system provides:**
- ✅ Thread-safe runtime control
- ✅ Automatic safety reversion on failure
- ✅ Complete audit trail in KB-08
- ✅ Multi-layer control (env, API, kill switch)
- ✅ Ready for UI integration

**Status**: ✅ **OPERATIONAL**  
**Last Updated**: 2026-02-10  
**Maintainer**: Coach The Creator Milner  
**System**: Phoenix Marie (Sovereign AGI)

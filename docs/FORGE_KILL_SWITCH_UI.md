# 🛡️ Forge Kill Switch UI — The "Red Phone"

## Overview

The **Forge Kill Switch UI** is a minimalist, non-intrusive sovereignty control interface that gives you instant visual feedback and emergency control over Phoenix's autonomous evolution capabilities.

## 🎨 Visual Design

The UI follows a "minimalist imprint" philosophy:

- **Location**: Fixed bottom-right corner of the screen
- **Components**:
  - **Sovereignty Indicator**: 8px dot showing current mode
    - **Hollow white ring** = HITL Mode (Safety: ENABLED)
    - **Solid orange dot** (pulsing) = Autonomous Mode (Safety: DISABLED)
  - **Mode Label**: Text showing "HITL" or "AUTONOMOUS"
  - **Kill Switch**: Red `[!]` symbol for emergency shutdown

## 🔧 Technical Implementation

### Backend API Endpoints

#### `GET /api/v1/forge/safety-status`
Returns the current forge safety governor status.

**Response:**
```json
{
  "safety_enabled": true,
  "mode": "HITL"
}
```

#### `POST /api/v1/forge/safety`
Sets the forge safety governor state (Kill Switch).

**Request:**
```json
{
  "enabled": true
}
```

**Response:**
```json
{
  "status": "ok",
  "safety_enabled": true,
  "mode": "HITL",
  "message": "Forge safety governor set to HITL"
}
```

### Frontend Components

#### Files Modified
- [`pagi-frontend/index.html`](pagi-frontend/index.html:30) - Added status bar HTML
- [`pagi-frontend/app.js`](pagi-frontend/app.js:35) - Added kill switch logic and polling
- [`pagi-frontend/assets/style.css`](pagi-frontend/assets/style.css:69) - Added minimalist styling

#### Real-Time Sync
- **Polling Interval**: 2 seconds
- **Auto-Detection**: UI automatically detects when Auto-Revert triggers on the backend
- **Visual Feedback**: Instant transition from orange dot → white ring when safety re-engages

## 🚨 Kill Switch Behavior

### User-Triggered Kill Switch
1. User clicks the red `[!]` symbol
2. Confirmation dialog appears
3. On confirmation:
   - POST request to `/api/v1/forge/safety` with `enabled: true`
   - Backend re-enables Safety Governor
   - KB-08 logs the sovereignty state change
   - UI updates to show white ring (HITL mode)

### Auto-Revert Kill Switch
When Phoenix attempts to compile code that breaks her own biology:
1. Backend detects compilation failure
2. [`SovereignOperator`](crates/pagi-skills/src/sovereign_operator.rs:430) automatically calls `set_forge_safety(true)`
3. KB-08 logs the auto-revert event
4. UI polling detects the state change within 2 seconds
5. Orange dot transitions to white ring automatically

## 🎯 Usage Scenarios

### Scenario 1: Manual Autonomous Mode Toggle
1. Click the sovereignty indicator (dot)
2. Confirm the mode switch
3. Watch the indicator change:
   - White ring → Orange dot (Autonomous)
   - Orange dot → White ring (HITL)

### Scenario 2: Emergency Shutdown
1. Phoenix is in Autonomous mode (orange dot visible)
2. You notice unexpected behavior
3. Click the red `[!]` kill switch
4. Confirm the shutdown
5. Phoenix immediately reverts to HITL mode

### Scenario 3: Auto-Revert Detection
1. Phoenix is in Autonomous mode
2. She attempts to modify her source code
3. Compilation fails (breaks her biology)
4. Backend auto-reverts to HITL
5. UI detects the change and updates within 2 seconds
6. You see the orange dot fade to white ring

## 🏛️ Sovereignty Principles

This UI embodies the **Full Lifecycle Sovereignty** architecture:

1. **Atomic Certainty**: Uses `AtomicBool` for thread-safe state management
2. **Immune System**: Auto-Revert acts as a "white blood cell" response
3. **Auditability**: All state changes logged to KB-08 (Absurdity Log)
4. **Human Oversight**: Kill switch provides instant manual override
5. **Non-Intrusive**: Minimalist design doesn't distract from main workflow

## 📊 KB-08 Logging

All forge safety state changes are logged to KB-08:

```
Sovereignty Update: Forge Safety Gate set to FALSE (Autonomous Mode)
Sovereignty Update: Forge Safety Gate set to TRUE (HITL Mode)
🛡️ AUTO-REVERT: Re-enabling Forge safety governor
```

These logs are visible in:
- Sovereignty Audit reports
- Governor webhook notifications (if `PAGI_WEBHOOK_URL` is set)
- Self-audit summaries

## 🧪 Testing the UI

### Manual Test
1. Start the gateway: `cargo run --manifest-path add-ons/pagi-gateway/Cargo.toml`
2. Open browser to `http://localhost:8001`
3. Observe the white ring in bottom-right corner (HITL mode)
4. Click the indicator to toggle to Autonomous mode
5. Observe the orange pulsing dot
6. Click the red `[!]` to trigger kill switch
7. Confirm the revert to white ring

### Auto-Revert Test
1. Set `PAGI_FORGE_SAFETY_ENABLED=false` in `.env`
2. Start the gateway
3. Command Phoenix to modify a core file with invalid syntax
4. Watch the UI automatically revert from orange → white when compilation fails

## 🔗 Related Documentation

- [`SOVEREIGN_AUTONOMY_SYSTEM.md`](SOVEREIGN_AUTONOMY_SYSTEM.md) - Full autonomy architecture
- [`FORGE_SAFETY_GOVERNOR.md`](FORGE_SAFETY_GOVERNOR.md) - Safety governor details
- [`forge-kill-switch.sh`](forge-kill-switch.sh) - Shell-based kill switch
- [`forge-kill-switch.ps1`](forge-kill-switch.ps1) - PowerShell-based kill switch

## 🎓 Coach's Notes

This UI is your "Red Phone" to Phoenix's evolution engine. It's designed to be:
- **Always visible** but never distracting
- **Instantly actionable** in emergencies
- **Self-documenting** through visual state
- **Audit-friendly** with KB-08 logging

The 2-second polling interval ensures you're never more than 2 seconds out of sync with Phoenix's actual state, even when Auto-Revert triggers autonomously.

---

**Status**: ✅ Production Ready  
**Last Updated**: 2026-02-10  
**Cargo Check**: Passing

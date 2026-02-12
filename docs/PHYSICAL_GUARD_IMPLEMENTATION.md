# PhysicalGuard Lock-out Implementation

## Overview
The PhysicalGuard system implements a forced workstation lock-out when the user exhibits sustained high-stress input patterns combined with critical wellness indicators. This is a safety mechanism to prevent burnout and physical/cognitive harm.

## Architecture

### Backend Implementation
**Location**: [`add-ons/pagi-gateway/src/main.rs`](../add-ons/pagi-gateway/src/main.rs)

#### Key Components

1. **Sentinel Broadcast Loop** (`sentinel_broadcast_loop` function)
   - Monitors input velocity from `SentinelInputVelocitySensor`
   - Tracks wellness state from `WellnessReportSkill`
   - Broadcasts SSE events to frontend

2. **Thresholds**
   ```rust
   const CRITICAL_VELOCITY_THRESHOLD: f64 = 85.0;  // Velocity score for forced reset
   const CRITICAL_THRESHOLD_TICKS: u64 = 15;       // 15 ticks * 2s = 30 seconds
   const FORCED_RESET_COUNTDOWN_SECS: u64 = 10;    // 10-second countdown
   ```

3. **Trigger Conditions**
   - `velocity_score > 85` for more than 30 seconds (15 ticks × 2s intervals)
   - `WellnessReport.is_critical == true`
   - Persona mode is `counselor`

4. **Lock-out Sequence**
   1. Counter increments every 2 seconds when velocity > 85
   2. When counter reaches 15 ticks AND wellness is critical:
      - Broadcast `forced_reset_countdown` SSE event
      - Start 10-second countdown
   3. After 10 seconds (if conditions still met):
      - Execute `SentinelPhysicalGuardSensor::LockWorkstation`
      - Log intervention to KB-08 (Soma/Absurdity Log)
      - Broadcast `forced_reset_executed` event
      - Reset counter and state

5. **Cancellation Logic**
   - If velocity drops below 85 during countdown, reset is cancelled
   - Counter resets to 0
   - Countdown timer is cleared

### Frontend Implementation
**Location**: [`add-ons/pagi-studio-ui/assets/studio-interface/App.tsx`](../add-ons/pagi-studio-ui/assets/studio-interface/App.tsx)

#### UI Components

1. **Event Listener**
   - Listens to `/persona/stream` SSE endpoint
   - Handles `forced_reset_countdown` event type

2. **Countdown Modal**
   - Full-screen overlay with backdrop blur
   - Red theme indicating critical state
   - Large countdown timer (updates every second)
   - Text: "Sovereign Reset Initiated"
   - Subtitle: "Physical and Cognitive limits exceeded."
   - Dynamic countdown: "System Lock in T-minus X"

3. **Visual Design**
   ```tsx
   - Background: black/70 with backdrop blur
   - Modal: red-950/95 with red-500 border
   - Timer: 6xl font, red-400 color, tabular numbers
   - Text: red-100 to red-300 gradient
   ```

### Persistence Layer

#### KB-08 Logging
**Location**: Soma Knowledge Base (slot 8 - "The Absurdity Log")

**Log Entry Structure**:
```json
{
  "type": "forced_sovereign_reset",
  "timestamp_ms": 1234567890,
  "velocity_score": 87.5,
  "critical_threshold_counter": 15,
  "wellness_critical": true,
  "action": "workstation_locked",
  "reason": "Sustained high input velocity (30+ seconds) combined with critical wellness state",
  "persona_mode": "counselor"
}
```

**Key Pattern**: `sovereign_intervention/{timestamp_ms}`

## Testing

### Manual Testing Steps

1. **Enable Sentinel**
   ```bash
   # Ensure PAGI_SENTINEL_VELOCITY_ENABLED=true (default)
   # Ensure persona mode is "counselor"
   ```

2. **Trigger High Velocity**
   - Type rapidly for 30+ seconds
   - Velocity score must exceed 85

3. **Ensure Critical Wellness**
   - Low sleep, high stress, or other critical indicators
   - Check `/skills/wellness-report` endpoint

4. **Observe Countdown**
   - 10-second countdown modal should appear
   - Timer counts down from 10 to 1

5. **Verify Lock**
   - Workstation should lock after countdown
   - Check KB-08 for log entry

### Cancellation Test

1. Trigger countdown as above
2. Stop typing (reduce velocity below 85)
3. Countdown should cancel
4. No lock should occur

## Safety Features

1. **Re-verification**: Conditions are re-checked before executing lock
2. **Counselor Mode Only**: Only active in counselor persona mode
3. **Dual Condition**: Requires BOTH high velocity AND critical wellness
4. **User Warning**: 10-second countdown provides warning
5. **Cancellable**: User can prevent lock by reducing input velocity
6. **Audit Trail**: All interventions logged to KB-08

## Configuration

### Environment Variables
- `PAGI_SENTINEL_VELOCITY_ENABLED`: Enable/disable sentinel (default: true)
- `PAGI_MODE`: Must be "counselor" for forced reset to activate

### Adjustable Constants
Edit [`add-ons/pagi-gateway/src/main.rs`](../add-ons/pagi-gateway/src/main.rs):
- `CRITICAL_VELOCITY_THRESHOLD`: Velocity threshold (default: 85.0)
- `CRITICAL_THRESHOLD_TICKS`: Duration in ticks (default: 15 = 30s)
- `FORCED_RESET_COUNTDOWN_SECS`: Countdown duration (default: 10s)

## Integration Points

### Dependencies
- **pagi-skills**: `SentinelInputVelocitySensor`, `SentinelPhysicalGuardSensor`
- **pagi-core**: `KnowledgeStore`, `PersonaCoordinator`
- **Wellness Report**: `generate_wellness_report` function

### SSE Events
- **Outbound**: `forced_reset_countdown`, `forced_reset_executed`, `sentinel_update`
- **Endpoint**: `GET /persona/stream`

## Future Enhancements

1. **Configurable Thresholds**: Move constants to CoreConfig
2. **Grace Period**: Allow user to acknowledge and extend countdown
3. **Wellness Trend Analysis**: Consider velocity trend, not just current value
4. **Multi-modal Triggers**: Add heart rate, eye strain, posture sensors
5. **Recovery Tracking**: Log when user returns from forced break
6. **Adaptive Thresholds**: Learn user's baseline and adjust dynamically

## Related Files

- Backend: [`add-ons/pagi-gateway/src/main.rs`](../add-ons/pagi-gateway/src/main.rs) (lines 382-556)
- Frontend: [`add-ons/pagi-studio-ui/assets/studio-interface/App.tsx`](../add-ons/pagi-studio-ui/assets/studio-interface/App.tsx) (lines 31, 101-129, 467-484)
- Wellness: [`add-ons/pagi-gateway/src/skills/wellness_report.rs`](../add-ons/pagi-gateway/src/skills/wellness_report.rs)
- Sensors: `crates/pagi-skills/src/sentinel/` (InputVelocitySensor, PhysicalGuardSensor)

## Changelog

### 2026-02-07
- ✅ Implemented forced reset countdown logic
- ✅ Updated velocity threshold from 80 to 85 for forced reset
- ✅ Updated UI countdown modal text to match requirements
- ✅ Verified KB-08 logging with proper structure
- ✅ Added cancellation logic when velocity drops

# SAO Modularization Implementation

## Overview
This document describes the modularization of the SAO (Sovereign AGI Orchestrator) as a background value-add service with simplified UI integration.

## Backend Changes (pagi-gateway)

### 1. OrchestratorService (Background Intelligence Layer)
**File**: `add-ons/pagi-gateway/src/services/orchestrator_service.rs`

- **Purpose**: Runs SAO intelligence analysis (pattern matching + heuristics) in the background
- **Features**:
  - Pattern matching from Manipulation Library (KB-2)
  - Heuristic analysis (ROI, resource drain detection)
  - Domain integrity scoring
  - Soma balance summary
  - Toggle-able on/off via API
  - Cached insights for status bar display

### 2. API Endpoints
**File**: `add-ons/pagi-gateway/src/main.rs`

New endpoints added:
- `GET /api/v1/intelligence/insights` - Get cached SAO intelligence insights
- `POST /api/v1/intelligence/toggle` - Toggle intelligence layer on/off

### 3. Integration with Chat Handler
The intelligence service now runs analysis in the background (non-blocking) whenever a chat message is received:

```rust
// Background intelligence analysis (non-blocking)
let intelligence_service = Arc::clone(&state.intelligence_service);
let prompt_clone = req.prompt.clone();
tokio::spawn(async move {
    let _ = intelligence_service.analyze_input(&prompt_clone).await;
});
```

## Frontend Changes (pagi-studio-ui)

### 1. New Components

#### StatusBar Component
**File**: `add-ons/pagi-studio-ui/assets/studio-interface/components/StatusBar.tsx`

- **Purpose**: Subtle, always-visible status bar at bottom of chat interface
- **Displays**:
  - Domain Integrity (0-100% with color coding)
  - Soma Balance (only when critical)
  - Pattern Detection Alert (when manipulation detected)
  - Velocity Score (when elevated)

#### ManipulationAlert Component
**File**: `add-ons/pagi-studio-ui/assets/studio-interface/components/ManipulationAlert.tsx`

- **Purpose**: Contextual, dismissible alert when manipulation patterns are detected
- **Features**:
  - Shows detected pattern categories (e.g., "DARVO", "Gaslighting")
  - Displays root cause analysis
  - Shows SAO counter-measure recommendation
  - Dismissible by user
  - Appears as tooltip in top-right corner

### 2. Type Definitions
**File**: `add-ons/pagi-studio-ui/assets/studio-interface/types.ts`

Added:
- `IntelligenceInsights` interface for SAO background analysis results
- `intelligenceLayerEnabled` setting in `AppSettings`

### 3. Simplified UI Architecture

**Removed**:
- Dedicated "Wellness" tab
- Heavy wellness visualization charts
- Separate wellness navigation

**Added**:
- Subtle status bar (always visible, minimal space)
- Contextual manipulation alerts (only when needed)
- Intelligence layer toggle in settings

## Key Design Principles

### 1. "Bare Metal" UI
- High-density information (text/badges) over heavy visuals
- No complex charts unless explicitly requested
- Minimal, functional design

### 2. Background Value-Add
- SAO logic runs in background, doesn't block chat
- Insights are cached and available on-demand
- Can be toggled on/off without affecting core functionality

### 3. Contextual Only
- Manipulation alerts only show when patterns are detected
- Status bar only shows critical information
- No dedicated "Counseling" mode UI - it's just a background service

## Settings Integration

The intelligence layer can be toggled in the Settings sidebar:

```typescript
// In SettingsSidebar.tsx
<label className="flex items-center gap-2">
  <input
    type="checkbox"
    checked={settings.intelligenceLayerEnabled ?? true}
    onChange={(e) => {
      const enabled = e.target.checked;
      setSettings({ ...settings, intelligenceLayerEnabled: enabled });
      // Call API to toggle backend service
      fetch(`${API_BASE_URL}/intelligence/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ enabled })
      });
    }}
  />
  <span>Intelligence Layer (SAO)</span>
</label>
```

## Migration Path

### For Existing Users:
1. Intelligence layer is enabled by default
2. No UI changes required - status bar appears automatically
3. Manipulation alerts are contextual and dismissible

### For New Deployments:
1. Backend automatically initializes OrchestratorService
2. Frontend polls `/api/v1/intelligence/insights` periodically
3. Status bar and alerts render based on cached insights

## Future Enhancements

1. **Configurable Thresholds**: Allow users to set sensitivity for pattern detection
2. **Historical Insights**: Track domain integrity over time
3. **Custom Patterns**: Allow users to define custom manipulation patterns
4. **Integration with Shadow Vault**: Link detected patterns to emotional anchors
5. **Wellness Integration**: Optionally show detailed wellness report on-demand

## Testing

### Backend:
```bash
# Test intelligence insights endpoint
curl http://127.0.0.1:8001/api/v1/intelligence/insights

# Toggle intelligence layer
curl -X POST http://127.0.0.1:8001/api/v1/intelligence/toggle \
  -H "Content-Type: application/json" \
  -d '{"enabled": false}'
```

### Frontend:
1. Send a message with manipulation keywords (e.g., "You always...", "You never...")
2. Check status bar for domain integrity score
3. Look for manipulation alert in top-right corner
4. Verify alert is dismissible

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                         Frontend (React)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  StatusBar   │  │ Manipulation │  │  Settings Toggle │  │
│  │  (Bottom)    │  │    Alert     │  │  (Sidebar)       │  │
│  └──────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ HTTP/SSE
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Gateway (Axum/Rust)                       │
│  ┌──────────────────────────────────────────────────────┐   │
│  │           OrchestratorService (Background)           │   │
│  │  ┌────────────────┐  ┌──────────────────────────┐   │   │
│  │  │ Pattern Match  │  │  Heuristic Processor     │   │   │
│  │  │ (KB-2)         │  │  (ROI, Resource Drain)   │   │   │
│  │  └────────────────┘  └──────────────────────────┘   │   │
│  │  ┌────────────────────────────────────────────────┐ │   │
│  │  │         Cached Intelligence Insights           │ │   │
│  │  └────────────────────────────────────────────────┘ │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
                              │
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Knowledge Store (Sled)                     │
│  KB-1: Pneuma  │  KB-2: Oikos  │  KB-8: Soma  │  ...        │
└─────────────────────────────────────────────────────────────┘
```

## Conclusion

The SAO has been successfully modularized as a background intelligence layer that:
- Runs non-blocking analysis on user input
- Provides subtle, contextual UI feedback
- Can be toggled on/off without affecting core functionality
- Maintains a "bare metal" aesthetic with high-density information display
- Focuses on value-add rather than heavy visualization

This architecture allows the SAO to provide protective intelligence without overwhelming the user interface or requiring dedicated "Counseling" mode navigation.

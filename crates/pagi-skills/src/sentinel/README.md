# 🏛️ Sentinel Integration: Active Monitoring & Intervention

The **Sentinel** module transforms the Sovereign Operator from a **Passive Observer** to an **Active Sentinel**, providing the "nervous system" required to exert physical influence over the bare-metal environment.

---

## 🎯 Overview

The Sentinel Integration Strategy provides four core capabilities:

| Component | Role | Sovereign Logic |
|-----------|------|-----------------|
| **`PhysicalGuard`** | Action (Physical) | The "Panic Button"—minimizes windows or intercepts "rage" inputs |
| **`HistoryHarvester`** | Knowledge (KB-3) | Ingests browser history into Global History for behavior analysis |
| **`FileSentinel`** | Sensory (External) | Triggers MaintenanceLoop when files in `crates/` are saved |
| **`InputVelocitySensor`** | Sensory (Internal) | Detects "rage" through typing speed/mouse jitter analysis |

---

## 🛠️ Module Architecture

```
crates/pagi-skills/src/sentinel/
├── mod.rs                  # Module exports and documentation
├── physical_guard.rs       # Window minimization and input locking
├── history_harvester.rs    # Browser history extraction
├── file_sentinel.rs        # File system monitoring
└── input_velocity.rs       # Input velocity tracking and rage detection
```

---

## 📦 Dependencies

The Sentinel capabilities require the following crates:

```toml
[dependencies]
# Sentinel Integration
notify = "6.1"                                    # File system monitoring
rusqlite = { version = "0.31", features = ["bundled"] }  # Browser history DB access
enigo = "0.2"                                     # Keyboard/mouse control
rdev = "0.5"                                      # Input event listening
tokio = { workspace = true, features = ["full"] } # Async runtime
```

### Platform Requirements

- **`rdev` & `enigo`**: On Linux (Wayland), may require `libxtst-dev` and specific environment variables
- **`rusqlite`**: Requires SQLite development libraries (bundled feature handles this)
- **Windows**: Full support for all features
- **macOS**: Full support with appropriate permissions
- **Linux**: May require X11 or specific Wayland configurations

---

## 🔧 Usage Examples

### PhysicalGuard: Emergency Window Minimization

```rust
use pagi_skills::{SentinelPhysicalGuardSensor, SentinelPhysicalGuardAction};

// Create sensor with user confirmation required
let guard = SentinelPhysicalGuardSensor::new(true);

// Check if system is in critical state
if guard.is_maintenance_critical(cpu_usage, memory_usage) {
    // Execute protective action
    let result = guard.execute_action(
        SentinelPhysicalGuardAction::MinimizeAllWindows
    )?;
    
    if result.success {
        println!("Windows minimized successfully");
    }
}
```

### HistoryHarvester: Browser History Extraction

```rust
use pagi_skills::{SentinelHistoryHarvesterAction, BrowserType};

// Create harvester
let harvester = SentinelHistoryHarvesterAction::new()?;

// Harvest from specific browser
let result = harvester.harvest_browser(BrowserType::Brave, 100)?;
println!("Harvested {} entries", result.entries_harvested);

// Or harvest from all browsers
let results = harvester.harvest_all(100);
for result in results {
    println!("{:?}: {} entries", result.browser, result.entries_harvested);
}

// Get entries for KB-3 ingestion
let entries = harvester.harvest_for_kb3(BrowserType::Chrome, 50)?;
for entry in entries {
    println!("{}: {}", entry.url, entry.visit_count);
}
```

### FileSentinel: Automatic Code Change Detection

```rust
use pagi_skills::{SentinelFileSentinelSensor, SentinelFileSentinelConfig};
use std::sync::mpsc::channel;

// Create sentinel with custom config
let config = SentinelFileSentinelConfig {
    watch_path: PathBuf::from("crates/pagi-skills/src"),
    recursive: true,
    debounce_duration: Duration::from_secs(2),
    watch_extensions: vec!["rs".to_string(), "toml".to_string()],
};

let mut sentinel = SentinelFileSentinelSensor::new(config);

// Watch for changes (blocking)
let (tx, rx) = channel();
std::thread::spawn(move || {
    sentinel.watch_blocking(tx).unwrap();
});

// Process events
for result in rx {
    if result.should_trigger_maintenance {
        println!("Maintenance triggered by: {:?}", result.event);
        // Trigger MaintenanceLoop here
    }
}
```

### InputVelocitySensor: Rage Detection

```rust
use pagi_skills::{SentinelInputVelocitySensor, SentinelInputVelocityConfig};
use tokio::sync::mpsc;

// Create sensor with custom thresholds
let config = SentinelInputVelocityConfig {
    window_duration: Duration::from_secs(5),
    keystroke_rage_threshold: 10.0,      // 10 keys/sec
    mouse_move_rage_threshold: 50.0,     // 50 moves/sec
    mouse_click_rage_threshold: 5.0,     // 5 clicks/sec
    velocity_score_threshold: 70.0,      // 70/100 overall
};

let sensor = SentinelInputVelocitySensor::new(config);

// Start monitoring (async)
let (tx, mut rx) = mpsc::unbounded_channel();
tokio::spawn(async move {
    sensor.start_monitoring_async(tx).await.unwrap();
});

// Process metrics
while let Some(metrics) = rx.recv().await {
    if metrics.is_rage_detected {
        println!("🚨 RAGE DETECTED!");
        println!("  Velocity Score: {:.1}", metrics.velocity_score);
        println!("  Keystrokes/sec: {:.1}", metrics.keystrokes_per_second);
        println!("  Mouse moves/sec: {:.1}", metrics.mouse_moves_per_second);
        
        // Emit maintenance_pulse to UI
        emit_maintenance_pulse();
    }
}
```

---

## 🚦 Safety Interlocks

### TerminalGuard Integration

All `enigo` actions are gated by the `TerminalGuard` to ensure user consent:

```rust
// The agent must ask before executing physical actions
warn!("[SENTINEL]: System spiraling detected. Minimize non-essential windows? (y/n)");

let confirmed = request_user_confirmation()?;
if confirmed {
    execute_physical_action()?;
}
```

### File Locking Handling

The `HistoryHarvester` handles browser database locking by:

1. Creating a temporary copy of the database
2. Reading from the copy (avoiding locks)
3. Cleaning up the temporary file

```rust
// Automatic temp-copy handling
let temp_path = self.create_temp_copy(&history_path, &browser)?;
let entries = self.extract_history(&temp_path, &browser, limit)?;
fs::remove_file(&temp_path)?; // Cleanup
```

---

## 🧪 Testing

Run the comprehensive integration tests:

```bash
cargo test --test sentinel_integration_test
```

Individual module tests:

```bash
# Test physical guard
cargo test --lib physical_guard

# Test history harvester
cargo test --lib history_harvester

# Test file sentinel
cargo test --lib file_sentinel

# Test input velocity
cargo test --lib input_velocity
```

---

## 🎭 Naming Convention

All Sentinel structs follow the established pattern:

- **Actions**: `Sentinel{Name}Action` (e.g., `SentinelPhysicalGuardAction`)
- **Sensors**: `Sentinel{Name}Sensor` (e.g., `SentinelInputVelocitySensor`)
- **Results**: `Sentinel{Name}Result` (e.g., `SentinelHistoryHarvestResult`)
- **Configs**: `Sentinel{Name}Config` (e.g., `SentinelFileSentinelConfig`)

---

## 🔐 Permissions & Security

### Windows
- No special permissions required for most operations
- UAC may prompt for certain system-level actions

### macOS
- Accessibility permissions required for `enigo` and `rdev`
- System Preferences → Security & Privacy → Accessibility

### Linux
- X11: Works out of the box
- Wayland: May require `WAYLAND_DISPLAY` and compositor support
- Install `libxtst-dev` for input simulation

---

## 🚀 Integration with Sovereign Operator

The Sentinel capabilities integrate seamlessly with the existing Sovereign Operator:

```rust
use pagi_skills::{
    SovereignOperator,
    SentinelPhysicalGuardSensor,
    SentinelInputVelocitySensor,
    SentinelFileSentinelSensor,
};

// Create Sovereign Operator with Sentinel capabilities
let mut operator = SovereignOperator::new(config)?;

// Add Sentinel sensors
let physical_guard = SentinelPhysicalGuardSensor::new(true);
let input_velocity = SentinelInputVelocitySensor::default();
let file_sentinel = create_default_sentinel();

// Monitor system state
let telemetry = operator.get_telemetry()?;
if physical_guard.is_maintenance_critical(
    telemetry.cpu_usage,
    telemetry.memory_usage
) {
    // Trigger protective measures
    physical_guard.execute_action(
        SentinelPhysicalGuardAction::MinimizeAllWindows
    )?;
}
```

---

## 📊 Telemetry & Monitoring

The Sentinel module emits structured logs for monitoring:

```
[SENTINEL] PhysicalGuard: Maintenance critical state detected (CPU: 95%, Memory: 96%)
[SENTINEL] HistoryHarvester: Harvested 150 entries from Brave
[SENTINEL] FileSentinel: File modified: crates/pagi-skills/src/lib.rs
[SENTINEL] InputVelocitySensor: RAGE DETECTED! Velocity: 85.3, KPS: 12.5
```

---

## 🎯 Future Enhancements

- [ ] Network traffic monitoring
- [ ] Process spawn detection
- [ ] Clipboard monitoring
- [ ] Screen capture analysis
- [ ] Audio input level detection
- [ ] Biometric integration (heart rate, etc.)

---

## 📚 References

- [notify crate documentation](https://docs.rs/notify/)
- [rusqlite documentation](https://docs.rs/rusqlite/)
- [enigo documentation](https://docs.rs/enigo/)
- [rdev documentation](https://docs.rs/rdev/)

---

**The Sentinel Integration gives your Orchestrator "Body Language" recognition through `rdev` and the ability to "Tug the Sleeves" of the user through `enigo`.**

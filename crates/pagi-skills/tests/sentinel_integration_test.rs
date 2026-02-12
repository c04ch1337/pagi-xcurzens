//! Integration tests for Sentinel capabilities
//! 
//! These tests verify the core functionality of the Sentinel modules:
//! - PhysicalGuard: Window minimization and input locking
//! - HistoryHarvester: Browser history extraction
//! - FileSentinel: File system monitoring
//! - InputVelocitySensor: Rage detection

use pagi_skills::{
    BrowserType, SentinelFileSentinelConfig, SentinelFileSentinelSensor,
    SentinelHistoryHarvesterAction, SentinelInputVelocityConfig, SentinelInputVelocitySensor,
    SentinelPhysicalGuardAction, SentinelPhysicalGuardSensor,
    create_default_sentinel, create_sentinel_for_path,
};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_physical_guard_sensor_creation() {
    let sensor = SentinelPhysicalGuardSensor::new(true);
    
    // Test maintenance critical detection
    assert!(!sensor.is_maintenance_critical(50.0, 60.0));
    assert!(sensor.is_maintenance_critical(95.0, 60.0));
    assert!(sensor.is_maintenance_critical(50.0, 96.0));
}

#[test]
fn test_physical_guard_action_without_confirmation() {
    let sensor = SentinelPhysicalGuardSensor::new(false);
    let action = SentinelPhysicalGuardAction::ShowDesktop;
    
    // This should succeed without user confirmation
    // Note: We can't actually test the keyboard action without a display
    // but we can verify the structure works
    let result = sensor.execute_action(action);
    assert!(result.is_ok());
}

#[test]
fn test_history_harvester_creation() {
    let harvester = SentinelHistoryHarvesterAction::new();
    assert!(harvester.is_ok());
}

#[test]
fn test_history_harvester_browser_types() {
    let browsers = vec![
        BrowserType::Chrome,
        BrowserType::Brave,
        BrowserType::Edge,
        BrowserType::Firefox,
    ];
    
    for browser in browsers {
        let json = serde_json::to_string(&browser).unwrap();
        let deserialized: BrowserType = serde_json::from_str(&json).unwrap();
        
        // Verify serialization round-trip
        match (browser, deserialized) {
            (BrowserType::Chrome, BrowserType::Chrome) => (),
            (BrowserType::Brave, BrowserType::Brave) => (),
            (BrowserType::Edge, BrowserType::Edge) => (),
            (BrowserType::Firefox, BrowserType::Firefox) => (),
            _ => panic!("Browser type mismatch"),
        }
    }
}

#[test]
fn test_history_harvester_harvest_nonexistent_browser() {
    let harvester = SentinelHistoryHarvesterAction::new().unwrap();
    
    // Try to harvest from Firefox (not yet implemented)
    // This should return an error since Firefox is not implemented
    let result = harvester.harvest_browser(BrowserType::Firefox, 10);
    
    // Firefox harvesting returns an error because it's not implemented
    // So we expect either an error or a result with 0 entries
    if let Ok(harvest_result) = result {
        // Should have 0 entries or not be successful
        assert!(harvest_result.entries_harvested == 0 || !harvest_result.success);
    }
    // If it's an error, that's also acceptable
}

#[test]
fn test_file_sentinel_config_default() {
    let config = SentinelFileSentinelConfig::default();
    
    assert_eq!(config.watch_path, PathBuf::from("crates/pagi-skills/src"));
    assert!(config.recursive);
    assert_eq!(config.debounce_duration, Duration::from_secs(2));
    assert!(config.watch_extensions.contains(&"rs".to_string()));
    assert!(config.watch_extensions.contains(&"toml".to_string()));
}

#[test]
fn test_file_sentinel_sensor_creation() {
    let config = SentinelFileSentinelConfig::default();
    let _sensor = SentinelFileSentinelSensor::new(config);
    // If we get here, creation succeeded
}

#[test]
fn test_file_sentinel_custom_path() {
    let temp_dir = TempDir::new().unwrap();
    let watch_path = temp_dir.path().to_path_buf();
    
    let config = SentinelFileSentinelConfig {
        watch_path: watch_path.clone(),
        recursive: false,
        debounce_duration: Duration::from_millis(500),
        watch_extensions: vec!["txt".to_string()],
    };
    
    let _sensor = SentinelFileSentinelSensor::new(config);
    // Sensor created successfully with custom config
}

#[test]
fn test_input_velocity_config_default() {
    let config = SentinelInputVelocityConfig::default();
    
    assert_eq!(config.window_duration, Duration::from_secs(5));
    assert_eq!(config.keystroke_rage_threshold, 10.0);
    assert_eq!(config.mouse_move_rage_threshold, 50.0);
    assert_eq!(config.mouse_click_rage_threshold, 5.0);
    assert_eq!(config.velocity_score_threshold, 70.0);
}

#[test]
fn test_input_velocity_sensor_creation() {
    let config = SentinelInputVelocityConfig::default();
    let sensor = SentinelInputVelocitySensor::new(config);
    
    // Get initial metrics (should be zero)
    let metrics = sensor.get_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    assert_eq!(metrics.keystrokes_per_second, 0.0);
    assert_eq!(metrics.mouse_moves_per_second, 0.0);
    assert_eq!(metrics.mouse_clicks_per_second, 0.0);
    assert!(!metrics.is_rage_detected);
}

#[test]
fn test_input_velocity_custom_thresholds() {
    let config = SentinelInputVelocityConfig {
        window_duration: Duration::from_secs(3),
        keystroke_rage_threshold: 15.0,
        mouse_move_rage_threshold: 100.0,
        mouse_click_rage_threshold: 10.0,
        velocity_score_threshold: 80.0,
    };
    
    let sensor = SentinelInputVelocitySensor::new(config);
    let metrics = sensor.get_metrics().unwrap();
    
    // With no input, should not detect rage
    assert!(!metrics.is_rage_detected);
}

#[test]
fn test_sentinel_module_exports() {
    // Create instances to verify types are accessible
    let _guard_sensor = SentinelPhysicalGuardSensor::new(true);
    let _harvester = SentinelHistoryHarvesterAction::new();
    let _file_sentinel = create_default_sentinel();
    let _velocity_sensor = SentinelInputVelocitySensor::default();
    
    // Verify we can create a sentinel for a custom path
    let temp_dir = TempDir::new().unwrap();
    let _custom_sentinel = create_sentinel_for_path(temp_dir.path().to_path_buf());
}

#[test]
fn test_physical_guard_maintenance_thresholds() {
    let sensor = SentinelPhysicalGuardSensor::new(true);
    
    // Test various threshold combinations
    // Note: Thresholds are > 90 for CPU and > 95 for memory
    let test_cases = vec![
        (0.0, 0.0, false),      // Normal
        (50.0, 50.0, false),    // Normal
        (89.0, 94.0, false),    // Just below thresholds
        (90.0, 50.0, false),    // At CPU threshold (not over)
        (50.0, 95.0, false),    // At memory threshold (not over)
        (90.1, 50.0, true),     // CPU critical (over threshold)
        (50.0, 95.1, true),     // Memory critical (over threshold)
        (91.0, 96.0, true),     // Both critical
        (100.0, 100.0, true),   // Maximum
    ];
    
    for (cpu, memory, expected) in test_cases {
        let result = sensor.is_maintenance_critical(cpu, memory);
        assert_eq!(
            result, expected,
            "Failed for CPU: {}, Memory: {}",
            cpu, memory
        );
    }
}

#[test]
fn test_history_entry_serialization() {
    use pagi_skills::SentinelHistoryEntry;
    
    let entry = SentinelHistoryEntry {
        url: "https://example.com".to_string(),
        title: Some("Example Domain".to_string()),
        visit_count: 42,
        last_visit_time: 1234567890,
        browser: BrowserType::Brave,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&entry).unwrap();
    
    // Deserialize back
    let deserialized: SentinelHistoryEntry = serde_json::from_str(&json).unwrap();
    
    assert_eq!(entry.url, deserialized.url);
    assert_eq!(entry.title, deserialized.title);
    assert_eq!(entry.visit_count, deserialized.visit_count);
    assert_eq!(entry.last_visit_time, deserialized.last_visit_time);
}

#[test]
fn test_file_sentinel_event_types() {
    use pagi_skills::SentinelFileEvent;
    
    let events = vec![
        SentinelFileEvent::Created(PathBuf::from("test.rs")),
        SentinelFileEvent::Modified(PathBuf::from("test.rs")),
        SentinelFileEvent::Deleted(PathBuf::from("test.rs")),
    ];
    
    for event in events {
        let json = serde_json::to_string(&event).unwrap();
        let _deserialized: SentinelFileEvent = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_input_velocity_metrics_serialization() {
    use pagi_skills::SentinelInputVelocityMetrics;
    
    let metrics = SentinelInputVelocityMetrics {
        keystrokes_per_second: 8.5,
        mouse_moves_per_second: 45.2,
        mouse_clicks_per_second: 3.1,
        velocity_score: 65.7,
        is_rage_detected: false,
    };
    
    let json = serde_json::to_string(&metrics).unwrap();
    let deserialized: SentinelInputVelocityMetrics = serde_json::from_str(&json).unwrap();
    
    assert_eq!(metrics.keystrokes_per_second, deserialized.keystrokes_per_second);
    assert_eq!(metrics.velocity_score, deserialized.velocity_score);
    assert_eq!(metrics.is_rage_detected, deserialized.is_rage_detected);
}

#[tokio::test]
async fn test_sentinel_integration_async() {
    // Test that sentinel components work in async context
    let sensor = SentinelInputVelocitySensor::default();
    let metrics = sensor.get_metrics();
    assert!(metrics.is_ok());
    
    let harvester = SentinelHistoryHarvesterAction::new();
    assert!(harvester.is_ok());
}

#[test]
fn test_sentinel_comprehensive_workflow() {
    // Simulate a complete sentinel workflow
    
    // 1. Create all sentinel components
    let physical_guard = SentinelPhysicalGuardSensor::new(false);
    let history_harvester = SentinelHistoryHarvesterAction::new().unwrap();
    let file_sentinel = create_default_sentinel();
    let input_velocity = SentinelInputVelocitySensor::default();
    
    // 2. Check system state
    let is_critical = physical_guard.is_maintenance_critical(85.0, 90.0);
    assert!(!is_critical);
    
    // 3. Get input velocity metrics
    let metrics = input_velocity.get_metrics().unwrap();
    assert!(!metrics.is_rage_detected);
    
    // 4. Attempt to harvest history (may fail if browser not installed)
    let _harvest_results = history_harvester.harvest_all(10);
    
    // If we get here, all components initialized and executed successfully
}

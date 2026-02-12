//! PhysicalGuard: Sentinel capability for physical system intervention
//! 
//! This module provides the ability to minimize windows or lock input when
//! the system reaches a "Maintenance Critical" state, acting as a physical
//! safeguard against user actions during critical operations.

use enigo::{Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use tracing::{error, info, warn};

/// Action types for physical guard interventions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SentinelPhysicalGuardAction {
    /// Minimize all windows (Windows: Win+D, macOS: Cmd+Option+H, Linux: Super+D)
    MinimizeAllWindows,
    /// Lock the workstation
    LockWorkstation,
    /// Show desktop
    ShowDesktop,
}

/// Result of a physical guard action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelPhysicalGuardResult {
    pub action: SentinelPhysicalGuardAction,
    pub success: bool,
    pub message: String,
    pub user_confirmed: bool,
}

/// PhysicalGuard sensor for detecting maintenance critical states
pub struct SentinelPhysicalGuardSensor {
    /// Whether to require user confirmation before actions
    require_confirmation: bool,
}

impl SentinelPhysicalGuardSensor {
    /// Create a new PhysicalGuard sensor
    pub fn new(require_confirmation: bool) -> Self {
        Self {
            require_confirmation,
        }
    }

    /// Execute a physical guard action with optional user confirmation
    pub fn execute_action(
        &self,
        action: SentinelPhysicalGuardAction,
    ) -> Result<SentinelPhysicalGuardResult, String> {
        // Safety interlock: require user confirmation if enabled
        let user_confirmed = if self.require_confirmation {
            self.request_user_confirmation(&action)?
        } else {
            true
        };

        if !user_confirmed {
            return Ok(SentinelPhysicalGuardResult {
                action,
                success: false,
                message: "User declined action".to_string(),
                user_confirmed: false,
            });
        }

        // Execute the action
        match self.perform_action(&action) {
            Ok(_) => {
                info!("[SENTINEL] Physical guard action executed: {:?}", action);
                Ok(SentinelPhysicalGuardResult {
                    action,
                    success: true,
                    message: "Action executed successfully".to_string(),
                    user_confirmed,
                })
            }
            Err(e) => {
                error!("[SENTINEL] Physical guard action failed: {}", e);
                Ok(SentinelPhysicalGuardResult {
                    action,
                    success: false,
                    message: format!("Action failed: {}", e),
                    user_confirmed,
                })
            }
        }
    }

    /// Request user confirmation for an action
    fn request_user_confirmation(
        &self,
        action: &SentinelPhysicalGuardAction,
    ) -> Result<bool, String> {
        let action_desc = match action {
            SentinelPhysicalGuardAction::MinimizeAllWindows => "Minimize all windows",
            SentinelPhysicalGuardAction::LockWorkstation => "Lock workstation",
            SentinelPhysicalGuardAction::ShowDesktop => "Show desktop",
        };

        warn!(
            "[SENTINEL]: System spiraling detected. {} (y/n)?",
            action_desc
        );

        print!("[SENTINEL]: {} (y/n)? ", action_desc);
        io::stdout().flush().map_err(|e| e.to_string())?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;

        let confirmed = input.trim().eq_ignore_ascii_case("y");
        Ok(confirmed)
    }

    /// Perform the actual physical action using enigo
    fn perform_action(&self, action: &SentinelPhysicalGuardAction) -> Result<(), String> {
        let settings = Settings::default();
        let mut enigo = Enigo::new(&settings).map_err(|e| format!("Failed to create Enigo: {:?}", e))?;

        match action {
            SentinelPhysicalGuardAction::MinimizeAllWindows
            | SentinelPhysicalGuardAction::ShowDesktop => {
                // Platform-specific key combinations
                #[cfg(target_os = "windows")]
                {
                    // Windows: Win+D to show desktop
                    enigo.key(Key::Meta, enigo::Direction::Press).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Unicode('d'), enigo::Direction::Click).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Meta, enigo::Direction::Release).map_err(|e| format!("{:?}", e))?;
                }

                #[cfg(target_os = "macos")]
                {
                    // macOS: Cmd+Option+H to hide all windows
                    enigo.key(Key::Meta, enigo::Direction::Press).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Alt, enigo::Direction::Press).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Unicode('h'), enigo::Direction::Click).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Alt, enigo::Direction::Release).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Meta, enigo::Direction::Release).map_err(|e| format!("{:?}", e))?;
                }

                #[cfg(target_os = "linux")]
                {
                    // Linux: Super+D (may vary by desktop environment)
                    enigo.key(Key::Meta, enigo::Direction::Press).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Unicode('d'), enigo::Direction::Click).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Meta, enigo::Direction::Release).map_err(|e| format!("{:?}", e))?;
                }

                Ok(())
            }
            SentinelPhysicalGuardAction::LockWorkstation => {
                #[cfg(target_os = "windows")]
                {
                    // Windows: Win+L to lock
                    enigo.key(Key::Meta, enigo::Direction::Press).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Unicode('l'), enigo::Direction::Click).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Meta, enigo::Direction::Release).map_err(|e| format!("{:?}", e))?;
                }

                #[cfg(target_os = "macos")]
                {
                    // macOS: Cmd+Ctrl+Q to lock screen
                    enigo.key(Key::Meta, enigo::Direction::Press).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Control, enigo::Direction::Press).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Unicode('q'), enigo::Direction::Click).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Control, enigo::Direction::Release).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Meta, enigo::Direction::Release).map_err(|e| format!("{:?}", e))?;
                }

                #[cfg(target_os = "linux")]
                {
                    // Linux: Super+L (may vary by desktop environment)
                    enigo.key(Key::Meta, enigo::Direction::Press).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Unicode('l'), enigo::Direction::Click).map_err(|e| format!("{:?}", e))?;
                    enigo.key(Key::Meta, enigo::Direction::Release).map_err(|e| format!("{:?}", e))?;
                }

                Ok(())
            }
        }
    }

    /// Check if a maintenance critical state has been reached
    pub fn is_maintenance_critical(&self, cpu_usage: f32, memory_usage: f32) -> bool {
        // Define thresholds for critical state
        const CPU_CRITICAL_THRESHOLD: f32 = 90.0;
        const MEMORY_CRITICAL_THRESHOLD: f32 = 95.0;

        cpu_usage > CPU_CRITICAL_THRESHOLD || memory_usage > MEMORY_CRITICAL_THRESHOLD
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maintenance_critical_detection() {
        let sensor = SentinelPhysicalGuardSensor::new(true);

        // Normal state
        assert!(!sensor.is_maintenance_critical(50.0, 60.0));

        // CPU critical
        assert!(sensor.is_maintenance_critical(95.0, 60.0));

        // Memory critical
        assert!(sensor.is_maintenance_critical(50.0, 96.0));

        // Both critical
        assert!(sensor.is_maintenance_critical(95.0, 96.0));
    }

    #[test]
    fn test_action_serialization() {
        let action = SentinelPhysicalGuardAction::MinimizeAllWindows;
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: SentinelPhysicalGuardAction = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            SentinelPhysicalGuardAction::MinimizeAllWindows => (),
            _ => panic!("Deserialization failed"),
        }
    }
}

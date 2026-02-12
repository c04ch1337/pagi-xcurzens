//! FileSentinel: Sentinel capability for file system monitoring
//! 
//! This module watches the `crates/pagi-skills/src/` directory and automatically
//! triggers a MaintenanceLoop audit when code changes are detected.

use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{debug, error, info, warn};

/// Types of file events that trigger maintenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SentinelFileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

/// Result of a file sentinel watch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelFileSentinelResult {
    pub event: SentinelFileEvent,
    pub timestamp: SystemTime,
    pub should_trigger_maintenance: bool,
}

/// Configuration for the file sentinel
#[derive(Debug, Clone)]
pub struct SentinelFileSentinelConfig {
    /// Path to watch
    pub watch_path: PathBuf,
    /// Whether to watch recursively
    pub recursive: bool,
    /// Debounce duration to avoid triggering on rapid successive changes
    pub debounce_duration: Duration,
    /// File extensions to watch (empty = all files)
    pub watch_extensions: Vec<String>,
}

impl Default for SentinelFileSentinelConfig {
    fn default() -> Self {
        Self {
            watch_path: PathBuf::from("crates/pagi-skills/src"),
            recursive: true,
            debounce_duration: Duration::from_secs(2),
            watch_extensions: vec!["rs".to_string(), "toml".to_string()],
        }
    }
}

/// FileSentinel sensor for monitoring file system changes
pub struct SentinelFileSentinelSensor {
    config: SentinelFileSentinelConfig,
    last_trigger: Option<SystemTime>,
}

impl SentinelFileSentinelSensor {
    /// Create a new FileSentinel sensor
    pub fn new(config: SentinelFileSentinelConfig) -> Self {
        Self {
            config,
            last_trigger: None,
        }
    }

    /// Check if a file should trigger maintenance based on extension
    fn should_watch_file(&self, path: &Path) -> bool {
        if self.config.watch_extensions.is_empty() {
            return true;
        }

        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return self.config.watch_extensions.contains(&ext_str.to_string());
            }
        }

        false
    }

    /// Check if enough time has passed since the last trigger (debouncing)
    fn should_debounce(&mut self) -> bool {
        if let Some(last) = self.last_trigger {
            if let Ok(elapsed) = SystemTime::now().duration_since(last) {
                if elapsed < self.config.debounce_duration {
                    debug!("[SENTINEL] Debouncing file event ({}ms remaining)", 
                        (self.config.debounce_duration - elapsed).as_millis());
                    return true;
                }
            }
        }
        false
    }

    /// Process a file system event
    fn process_event(&mut self, event: Event) -> Option<SentinelFileSentinelResult> {
        match event.kind {
            EventKind::Create(_) => {
                for path in event.paths {
                    if self.should_watch_file(&path) {
                        info!("[SENTINEL] File created: {:?}", path);
                        
                        if self.should_debounce() {
                            return None;
                        }

                        self.last_trigger = Some(SystemTime::now());
                        return Some(SentinelFileSentinelResult {
                            event: SentinelFileEvent::Created(path),
                            timestamp: SystemTime::now(),
                            should_trigger_maintenance: true,
                        });
                    }
                }
            }
            EventKind::Modify(_) => {
                for path in event.paths {
                    if self.should_watch_file(&path) {
                        info!("[SENTINEL] File modified: {:?}", path);
                        
                        if self.should_debounce() {
                            return None;
                        }

                        self.last_trigger = Some(SystemTime::now());
                        return Some(SentinelFileSentinelResult {
                            event: SentinelFileEvent::Modified(path),
                            timestamp: SystemTime::now(),
                            should_trigger_maintenance: true,
                        });
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    if self.should_watch_file(&path) {
                        warn!("[SENTINEL] File deleted: {:?}", path);
                        
                        if self.should_debounce() {
                            return None;
                        }

                        self.last_trigger = Some(SystemTime::now());
                        return Some(SentinelFileSentinelResult {
                            event: SentinelFileEvent::Deleted(path),
                            timestamp: SystemTime::now(),
                            should_trigger_maintenance: true,
                        });
                    }
                }
            }
            _ => {
                debug!("[SENTINEL] Ignoring event: {:?}", event.kind);
            }
        }

        None
    }

    /// Start watching the configured path (blocking)
    pub fn watch_blocking(
        &mut self,
        tx: Sender<SentinelFileSentinelResult>,
    ) -> NotifyResult<()> {
        let (event_tx, event_rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: NotifyResult<Event>| {
                if let Ok(event) = res {
                    let _ = event_tx.send(event);
                }
            },
            Config::default(),
        )?;

        let mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(&self.config.watch_path, mode)?;

        info!(
            "[SENTINEL] FileSentinel watching: {:?} (recursive: {})",
            self.config.watch_path, self.config.recursive
        );

        // Process events
        loop {
            match event_rx.recv() {
                Ok(event) => {
                    if let Some(result) = self.process_event(event) {
                        if let Err(e) = tx.send(result) {
                            error!("[SENTINEL] Failed to send result: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("[SENTINEL] Watch error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Start watching the configured path (async)
    pub async fn watch_async(
        &mut self,
        tx: tokio_mpsc::UnboundedSender<SentinelFileSentinelResult>,
    ) -> NotifyResult<()> {
        let (event_tx, mut event_rx) = tokio_mpsc::unbounded_channel();

        let mut watcher = RecommendedWatcher::new(
            move |res: NotifyResult<Event>| {
                if let Ok(event) = res {
                    let _ = event_tx.send(event);
                }
            },
            Config::default(),
        )?;

        let mode = if self.config.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher.watch(&self.config.watch_path, mode)?;

        info!(
            "[SENTINEL] FileSentinel watching: {:?} (recursive: {})",
            self.config.watch_path, self.config.recursive
        );

        // Process events asynchronously
        while let Some(event) = event_rx.recv().await {
            if let Some(result) = self.process_event(event) {
                if let Err(e) = tx.send(result) {
                    error!("[SENTINEL] Failed to send result: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Helper function to create a file sentinel with default config
pub fn create_default_sentinel() -> SentinelFileSentinelSensor {
    SentinelFileSentinelSensor::new(SentinelFileSentinelConfig::default())
}

/// Helper function to create a file sentinel for a specific path
pub fn create_sentinel_for_path(path: PathBuf) -> SentinelFileSentinelSensor {
    let config = SentinelFileSentinelConfig {
        watch_path: path,
        ..Default::default()
    };
    SentinelFileSentinelSensor::new(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_should_watch_file() {
        let config = SentinelFileSentinelConfig {
            watch_extensions: vec!["rs".to_string(), "toml".to_string()],
            ..Default::default()
        };
        let sensor = SentinelFileSentinelSensor::new(config);

        assert!(sensor.should_watch_file(Path::new("test.rs")));
        assert!(sensor.should_watch_file(Path::new("Cargo.toml")));
        assert!(!sensor.should_watch_file(Path::new("test.txt")));
    }

    #[test]
    fn test_debouncing() {
        let config = SentinelFileSentinelConfig {
            debounce_duration: Duration::from_millis(100),
            ..Default::default()
        };
        let mut sensor = SentinelFileSentinelSensor::new(config);

        // First trigger should not debounce
        assert!(!sensor.should_debounce());
        sensor.last_trigger = Some(SystemTime::now());

        // Immediate second trigger should debounce
        assert!(sensor.should_debounce());

        // After waiting, should not debounce
        std::thread::sleep(Duration::from_millis(150));
        assert!(!sensor.should_debounce());
    }

    #[test]
    fn test_file_event_serialization() {
        let event = SentinelFileEvent::Modified(PathBuf::from("test.rs"));
        let json = serde_json::to_string(&event).unwrap();
        let _deserialized: SentinelFileEvent = serde_json::from_str(&json).unwrap();
    }
}

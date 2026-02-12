//! InputVelocitySensor: Sentinel capability for detecting user stress patterns
//! 
//! This module tracks "Input Velocity" (typing speed/mouse jitter) using rdev.
//! If velocity exceeds a threshold (Rage Detection), it emits a maintenance_pulse
//! to the UI, allowing the system to intervene before the user spirals.

use rdev::{listen, Event, EventType};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Input velocity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelInputVelocityMetrics {
    /// Keystrokes per second
    pub keystrokes_per_second: f64,
    /// Mouse movements per second
    pub mouse_moves_per_second: f64,
    /// Mouse clicks per second
    pub mouse_clicks_per_second: f64,
    /// Overall velocity score (0-100)
    pub velocity_score: f64,
    /// Whether rage threshold is exceeded
    pub is_rage_detected: bool,
}

/// Configuration for input velocity detection
#[derive(Debug, Clone)]
pub struct SentinelInputVelocityConfig {
    /// Window duration for calculating velocity
    pub window_duration: Duration,
    /// Threshold for keystrokes per second to trigger rage detection
    pub keystroke_rage_threshold: f64,
    /// Threshold for mouse movements per second to trigger rage detection
    pub mouse_move_rage_threshold: f64,
    /// Threshold for mouse clicks per second to trigger rage detection
    pub mouse_click_rage_threshold: f64,
    /// Overall velocity score threshold (0-100)
    pub velocity_score_threshold: f64,
}

impl Default for SentinelInputVelocityConfig {
    fn default() -> Self {
        Self {
            window_duration: Duration::from_secs(5),
            keystroke_rage_threshold: 10.0,      // 10 keys/sec sustained
            mouse_move_rage_threshold: 50.0,     // 50 moves/sec
            mouse_click_rage_threshold: 5.0,     // 5 clicks/sec
            velocity_score_threshold: 70.0,      // 70/100 overall score
        }
    }
}

/// Internal state for tracking input events
#[derive(Debug)]
struct InputState {
    keystrokes: Vec<Instant>,
    mouse_moves: Vec<Instant>,
    mouse_clicks: Vec<Instant>,
    last_calculation: Instant,
}

impl InputState {
    fn new() -> Self {
        Self {
            keystrokes: Vec::new(),
            mouse_moves: Vec::new(),
            mouse_clicks: Vec::new(),
            last_calculation: Instant::now(),
        }
    }

    /// Clean up old events outside the window
    fn cleanup(&mut self, window: Duration) {
        let cutoff = Instant::now() - window;
        self.keystrokes.retain(|&t| t > cutoff);
        self.mouse_moves.retain(|&t| t > cutoff);
        self.mouse_clicks.retain(|&t| t > cutoff);
    }

    /// Calculate current metrics
    fn calculate_metrics(&self, config: &SentinelInputVelocityConfig) -> SentinelInputVelocityMetrics {
        let window_secs = config.window_duration.as_secs_f64();

        let keystrokes_per_second = self.keystrokes.len() as f64 / window_secs;
        let mouse_moves_per_second = self.mouse_moves.len() as f64 / window_secs;
        let mouse_clicks_per_second = self.mouse_clicks.len() as f64 / window_secs;

        // Calculate velocity score (weighted average)
        let keystroke_score = (keystrokes_per_second / config.keystroke_rage_threshold * 100.0).min(100.0);
        let mouse_move_score = (mouse_moves_per_second / config.mouse_move_rage_threshold * 100.0).min(100.0);
        let mouse_click_score = (mouse_clicks_per_second / config.mouse_click_rage_threshold * 100.0).min(100.0);

        // Weighted: keystrokes 50%, mouse moves 30%, clicks 20%
        let velocity_score = (keystroke_score * 0.5) + (mouse_move_score * 0.3) + (mouse_click_score * 0.2);

        let is_rage_detected = velocity_score >= config.velocity_score_threshold
            || keystrokes_per_second >= config.keystroke_rage_threshold
            || mouse_clicks_per_second >= config.mouse_click_rage_threshold;

        SentinelInputVelocityMetrics {
            keystrokes_per_second,
            mouse_moves_per_second,
            mouse_clicks_per_second,
            velocity_score,
            is_rage_detected,
        }
    }
}

/// InputVelocity sensor for detecting user stress patterns
pub struct SentinelInputVelocitySensor {
    config: SentinelInputVelocityConfig,
    state: Arc<Mutex<InputState>>,
}

impl SentinelInputVelocitySensor {
    /// Create a new InputVelocity sensor
    pub fn new(config: SentinelInputVelocityConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(InputState::new())),
        }
    }

    /// Get current velocity metrics
    pub fn get_metrics(&self) -> Result<SentinelInputVelocityMetrics, String> {
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        state.cleanup(self.config.window_duration);
        Ok(state.calculate_metrics(&self.config))
    }

    /// Start monitoring input events (blocking)
    pub fn start_monitoring_blocking(
        &self,
        tx: mpsc::UnboundedSender<SentinelInputVelocityMetrics>,
    ) -> Result<(), String> {
        let state = Arc::clone(&self.state);
        let config = self.config.clone();
        let calculation_interval = Duration::from_secs(1);

        info!("[SENTINEL] InputVelocitySensor starting...");

        // Spawn a thread to periodically calculate and send metrics
        let metrics_state = Arc::clone(&state);
        let metrics_tx = tx.clone();
        let metrics_config = config.clone();
        
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(calculation_interval);

                if let Ok(mut state) = metrics_state.lock() {
                    state.cleanup(metrics_config.window_duration);
                    let metrics = state.calculate_metrics(&metrics_config);

                    if metrics.is_rage_detected {
                        warn!(
                            "[SENTINEL] RAGE DETECTED! Velocity: {:.1}, KPS: {:.1}, MPS: {:.1}, CPS: {:.1}",
                            metrics.velocity_score,
                            metrics.keystrokes_per_second,
                            metrics.mouse_moves_per_second,
                            metrics.mouse_clicks_per_second
                        );
                    } else {
                        debug!(
                            "[SENTINEL] Input velocity: {:.1} (KPS: {:.1}, MPS: {:.1}, CPS: {:.1})",
                            metrics.velocity_score,
                            metrics.keystrokes_per_second,
                            metrics.mouse_moves_per_second,
                            metrics.mouse_clicks_per_second
                        );
                    }

                    if let Err(e) = metrics_tx.send(metrics) {
                        error!("[SENTINEL] Failed to send metrics: {}", e);
                        break;
                    }
                }
            }
        });

        // Listen to input events
        let callback = move |event: Event| {
            if let Ok(mut state) = state.lock() {
                let now = Instant::now();

                match event.event_type {
                    EventType::KeyPress(_) => {
                        state.keystrokes.push(now);
                    }
                    EventType::MouseMove { .. } => {
                        state.mouse_moves.push(now);
                    }
                    EventType::ButtonPress(_) => {
                        state.mouse_clicks.push(now);
                    }
                    _ => {}
                }
            }
        };

        // This blocks indefinitely
        if let Err(e) = listen(callback) {
            error!("[SENTINEL] Input listener error: {:?}", e);
            return Err(format!("Input listener error: {:?}", e));
        }

        Ok(())
    }

    /// Start monitoring input events (async)
    pub async fn start_monitoring_async(
        &self,
        tx: mpsc::UnboundedSender<SentinelInputVelocityMetrics>,
    ) -> Result<(), String> {
        let state = Arc::clone(&self.state);
        let config = self.config.clone();
        let calculation_interval = Duration::from_secs(1);

        info!("[SENTINEL] InputVelocitySensor starting (async)...");

        // Spawn a task to periodically calculate and send metrics
        let metrics_state = Arc::clone(&state);
        let metrics_tx = tx.clone();
        let metrics_config = config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(calculation_interval);
            loop {
                interval.tick().await;

                if let Ok(mut state) = metrics_state.lock() {
                    state.cleanup(metrics_config.window_duration);
                    let metrics = state.calculate_metrics(&metrics_config);

                    if metrics.is_rage_detected {
                        warn!(
                            "[SENTINEL] RAGE DETECTED! Velocity: {:.1}, KPS: {:.1}, MPS: {:.1}, CPS: {:.1}",
                            metrics.velocity_score,
                            metrics.keystrokes_per_second,
                            metrics.mouse_moves_per_second,
                            metrics.mouse_clicks_per_second
                        );
                    }

                    if let Err(e) = metrics_tx.send(metrics) {
                        error!("[SENTINEL] Failed to send metrics: {}", e);
                        break;
                    }
                }
            }
        });

        // Listen to input events in a blocking thread
        let listen_state = Arc::clone(&state);
        tokio::task::spawn_blocking(move || {
            let callback = move |event: Event| {
                if let Ok(mut state) = listen_state.lock() {
                    let now = Instant::now();

                    match event.event_type {
                        EventType::KeyPress(_) => {
                            state.keystrokes.push(now);
                        }
                        EventType::MouseMove { .. } => {
                            state.mouse_moves.push(now);
                        }
                        EventType::ButtonPress(_) => {
                            state.mouse_clicks.push(now);
                        }
                        _ => {}
                    }
                }
            };

            if let Err(e) = listen(callback) {
                error!("[SENTINEL] Input listener error: {:?}", e);
            }
        });

        Ok(())
    }
}

impl Default for SentinelInputVelocitySensor {
    fn default() -> Self {
        Self::new(SentinelInputVelocityConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_state_cleanup() {
        let mut state = InputState::new();
        let now = Instant::now();
        
        // Add some old events
        state.keystrokes.push(now - Duration::from_secs(10));
        state.keystrokes.push(now - Duration::from_secs(2));
        state.keystrokes.push(now);

        // Cleanup with 5-second window
        state.cleanup(Duration::from_secs(5));

        // Should only keep the last 2 events
        assert_eq!(state.keystrokes.len(), 2);
    }

    #[test]
    fn test_metrics_calculation() {
        let config = SentinelInputVelocityConfig::default();
        let mut state = InputState::new();
        let now = Instant::now();

        // Add events within the window
        for _ in 0..50 {
            state.keystrokes.push(now);
        }

        let metrics = state.calculate_metrics(&config);
        
        // 50 keystrokes in 5 seconds = 10 per second (at threshold)
        assert!(metrics.keystrokes_per_second >= 9.0);
        assert!(metrics.is_rage_detected);
    }

    #[test]
    fn test_velocity_score_calculation() {
        let config = SentinelInputVelocityConfig::default();
        let mut state = InputState::new();
        let now = Instant::now();

        // Add moderate activity
        for _ in 0..25 {
            state.keystrokes.push(now);
        }

        let metrics = state.calculate_metrics(&config);
        
        // Should have a velocity score but not trigger rage
        assert!(metrics.velocity_score > 0.0);
        assert!(metrics.velocity_score < config.velocity_score_threshold);
    }
}

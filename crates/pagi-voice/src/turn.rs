//! Turn-taking management for conversational flow
//!
//! Implements the "Gap Logic" - detecting when a user has finished speaking
//! based on an 800ms silence threshold.

use crate::error::{VoiceError, VoiceResult};
use chrono::{DateTime, Utc};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Events emitted by the turn manager
#[derive(Debug, Clone)]
pub enum TurnEvent {
    /// User started speaking
    SpeechStarted {
        timestamp: DateTime<Utc>,
    },
    
    /// User is currently speaking (periodic update)
    SpeechContinuing {
        duration: Duration,
    },
    
    /// User stopped speaking (silence detected)
    SpeechEnded {
        timestamp: DateTime<Utc>,
        duration: Duration,
    },
    
    /// Turn is committed (ready for processing)
    TurnCommitted {
        timestamp: DateTime<Utc>,
        total_speech_duration: Duration,
        audio_samples: Vec<f32>,
    },
    
    /// User interrupted AI speech
    Interruption {
        timestamp: DateTime<Utc>,
    },
}

/// Configuration for turn detection
#[derive(Debug, Clone)]
pub struct TurnConfig {
    /// Silence duration before committing a turn (default: 800ms)
    pub silence_threshold: Duration,
    
    /// Minimum speech duration to be considered a valid turn (default: 200ms)
    pub min_speech_duration: Duration,
    
    /// Maximum turn duration before auto-commit (default: 30s)
    pub max_turn_duration: Duration,
    
    /// Sample rate for audio (default: 16000 Hz)
    pub sample_rate: u32,
}

impl Default for TurnConfig {
    fn default() -> Self {
        Self {
            silence_threshold: Duration::from_millis(800),
            min_speech_duration: Duration::from_millis(200),
            max_turn_duration: Duration::from_secs(30),
            sample_rate: 16000,
        }
    }
}

/// State of the current turn
#[derive(Debug, Clone, PartialEq)]
enum TurnState {
    Idle,
    Speaking,
    SilenceDetected,
}

/// Manages conversational turn-taking based on VAD signals
pub struct TurnManager {
    config: TurnConfig,
    state: TurnState,
    
    // Timing
    speech_start: Option<Instant>,
    last_speech_time: Option<Instant>,
    
    // Audio buffer for the current turn
    audio_buffer: Vec<f32>,
    
    // Event channel
    event_tx: mpsc::UnboundedSender<TurnEvent>,
}

impl TurnManager {
    /// Create a new turn manager with the given configuration
    pub fn new(config: TurnConfig) -> (Self, mpsc::UnboundedReceiver<TurnEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let manager = Self {
            config,
            state: TurnState::Idle,
            speech_start: None,
            last_speech_time: None,
            audio_buffer: Vec::new(),
            event_tx,
        };
        
        (manager, event_rx)
    }
    
    /// Process a VAD detection result
    pub fn process_vad_result(&mut self, is_speech: bool, audio_chunk: &[f32]) -> VoiceResult<()> {
        let now = Instant::now();
        
        match (self.state.clone(), is_speech) {
            // Idle -> Speaking: User started talking
            (TurnState::Idle, true) => {
                info!("🎤 Speech started");
                self.state = TurnState::Speaking;
                self.speech_start = Some(now);
                self.last_speech_time = Some(now);
                self.audio_buffer.clear();
                self.audio_buffer.extend_from_slice(audio_chunk);
                
                self.emit_event(TurnEvent::SpeechStarted {
                    timestamp: Utc::now(),
                })?;
            }
            
            // Speaking -> Speaking: Continue recording
            (TurnState::Speaking, true) => {
                self.last_speech_time = Some(now);
                self.audio_buffer.extend_from_slice(audio_chunk);
                
                // Check for max duration
                if let Some(start) = self.speech_start {
                    let duration = now.duration_since(start);
                    if duration >= self.config.max_turn_duration {
                        warn!("⏱️ Max turn duration reached, auto-committing");
                        return self.commit_turn();
                    }
                    
                    // Periodic update every second
                    if duration.as_secs() > 0 && duration.as_millis() % 1000 < 30 {
                        debug!("🎤 Speech continuing: {:?}", duration);
                        self.emit_event(TurnEvent::SpeechContinuing { duration })?;
                    }
                }
            }
            
            // Speaking -> SilenceDetected: User paused
            (TurnState::Speaking, false) => {
                debug!("🤫 Silence detected");
                self.state = TurnState::SilenceDetected;
                // Keep audio buffer, don't clear yet
            }
            
            // SilenceDetected -> Speaking: User resumed (false alarm)
            (TurnState::SilenceDetected, true) => {
                debug!("🎤 Speech resumed");
                self.state = TurnState::Speaking;
                self.last_speech_time = Some(now);
                self.audio_buffer.extend_from_slice(audio_chunk);
            }
            
            // SilenceDetected -> SilenceDetected: Check if we should commit
            (TurnState::SilenceDetected, false) => {
                if let Some(last_speech) = self.last_speech_time {
                    let silence_duration = now.duration_since(last_speech);
                    
                    if silence_duration >= self.config.silence_threshold {
                        info!("✅ Silence threshold reached, committing turn");
                        return self.commit_turn();
                    }
                }
            }
            
            // Idle -> Idle: Nothing happening
            (TurnState::Idle, false) => {
                // No-op
            }
        }
        
        Ok(())
    }
    
    /// Commit the current turn for processing
    fn commit_turn(&mut self) -> VoiceResult<()> {
        if self.state == TurnState::Idle {
            return Ok(());
        }
        
        let duration = self.speech_start
            .map(|start| Instant::now().duration_since(start))
            .unwrap_or_default();
        
        // Check minimum duration
        if duration < self.config.min_speech_duration {
            debug!("⏭️ Speech too short ({:?}), ignoring", duration);
            self.reset();
            return Ok(());
        }
        
        info!("🎯 Turn committed: {:?} duration, {} samples", 
              duration, self.audio_buffer.len());
        
        self.emit_event(TurnEvent::SpeechEnded {
            timestamp: Utc::now(),
            duration,
        })?;
        
        // Clone the audio buffer before moving it
        let audio_samples = self.audio_buffer.clone();
        
        self.emit_event(TurnEvent::TurnCommitted {
            timestamp: Utc::now(),
            total_speech_duration: duration,
            audio_samples,
        })?;
        
        self.reset();
        Ok(())
    }
    
    /// Signal that the user interrupted AI speech
    pub fn signal_interruption(&mut self) -> VoiceResult<()> {
        info!("⚡ Sovereign Interruption: User speaking, silencing AI output");
        
        self.emit_event(TurnEvent::Interruption {
            timestamp: Utc::now(),
        })?;
        
        Ok(())
    }
    
    /// Reset the turn manager state
    fn reset(&mut self) {
        self.state = TurnState::Idle;
        self.speech_start = None;
        self.last_speech_time = None;
        self.audio_buffer.clear();
    }
    
    /// Emit an event to the channel
    fn emit_event(&self, event: TurnEvent) -> VoiceResult<()> {
        self.event_tx.send(event)
            .map_err(|e| VoiceError::ChannelSend(e.to_string()))
    }
    
    /// Get the current state (for testing/debugging)
    pub fn state(&self) -> &str {
        match self.state {
            TurnState::Idle => "idle",
            TurnState::Speaking => "speaking",
            TurnState::SilenceDetected => "silence_detected",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_turn_manager_basic_flow() {
        let config = TurnConfig {
            silence_threshold: Duration::from_millis(100),
            min_speech_duration: Duration::from_millis(50),
            ..Default::default()
        };
        
        let (mut manager, mut rx) = TurnManager::new(config);
        
        // Simulate speech
        let chunk = vec![0.5f32; 480]; // 30ms at 16kHz
        manager.process_vad_result(true, &chunk).unwrap();
        
        // Should emit SpeechStarted
        let event = rx.try_recv().unwrap();
        assert!(matches!(event, TurnEvent::SpeechStarted { .. }));
        
        assert_eq!(manager.state(), "speaking");
    }
}

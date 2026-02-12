//! **The Sovereign Ear** — local VAD and turn detection for real-time "Gap" perception.
//!
//! Captures microphone via CPAL, processes audio in 30ms chunks (required by VAD),
//! and emits `AudioTurn` when 800ms of silence follows speech. All processing is
//! local (no cloud VAD). Buffer is cleared after every commit to prevent memory growth.

use crate::audio::{AudioCapture, AudioChunk, AudioConfig};
use crate::error::VoiceResult;
use crate::turn::{TurnConfig, TurnEvent, TurnManager};
use crate::vad::{VadConfig, VadDetector};
use chrono::{DateTime, Utc};
use cpal::Stream;
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// VAD state machine: Silence → Speech → PostSpeechGap → (commit) → Silence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    /// No speech detected.
    Silence,
    /// Speech in progress.
    Speech,
    /// Speech ended; counting silence toward gap threshold (e.g. 800ms).
    PostSpeechGap,
}

/// A completed turn: buffered PCM from speech start until gap detected.
#[derive(Debug, Clone)]
pub struct AudioTurn {
    /// PCM samples (f32, -1.0..1.0) for the full turn.
    pub samples: Vec<f32>,
    /// When the turn was committed (gap detected).
    pub timestamp: DateTime<Utc>,
    /// Approximate speech duration.
    pub duration: Duration,
    /// Sample rate (e.g. 16000).
    pub sample_rate: u32,
}

/// Configuration for the Sovereign Ear.
#[derive(Debug, Clone)]
pub struct EarConfig {
    /// Sample rate (default 16000). Must match VAD (8000/16000/32000/48000 for WebRTC VAD).
    pub sample_rate: u32,
    /// Chunk size in samples (default 480 = 30ms at 16kHz). Required by VAD.
    pub chunk_size: usize,
    /// VAD probability above this is considered speech (default 0.5).
    pub vad_threshold: f32,
    /// Silence duration after speech to commit turn (default 800ms).
    pub gap_ms: u64,
    /// Minimum speech duration to commit (default 200ms). Shorter segments are dropped.
    pub min_speech_ms: u64,
    /// When true, emit real-time VadState (Speech/Silence) per chunk for interruption kill-switch (default true).
    pub emit_vad_state: bool,
}

impl Default for EarConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            chunk_size: 480, // 30ms at 16kHz
            vad_threshold: 0.5,
            gap_ms: 800,
            min_speech_ms: 200,
            emit_vad_state: true,
        }
    }
}

/// The Sovereign Ear: captures mic, runs local VAD, emits turns on gap.
pub struct VoiceEar {
    config: EarConfig,
}

impl VoiceEar {
    /// Create a new Ear with the given config.
    pub fn new(config: EarConfig) -> Self {
        Self { config }
    }

    /// Start listening. Returns an `EarSession`: handle (keep alive to keep capture running) and
    /// optionally a receiver of real-time `VadState` (Speech/Silence) per chunk for the interruption kill-switch.
    /// When Ear reports `VadState::Speech` while TTS is playing, call `VoiceOutput::stop()`.
    pub fn start_listening(self) -> VoiceResult<EarSession> {
        let (turn_tx, turn_rx) = mpsc::channel(16);
        let (vad_state_tx, vad_state_rx) = if self.config.emit_vad_state {
            let (tx, rx) = mpsc::unbounded_channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };

        let audio_config = AudioConfig {
            sample_rate: self.config.sample_rate,
            channels: 1,
            buffer_size: self.config.chunk_size,
        };
        let capture = AudioCapture::new(audio_config)?;

        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<AudioChunk>();

        let stream = capture.start_capture(audio_tx)?;

        let vad_config = VadConfig {
            sample_rate: self.config.sample_rate,
            mode: 2,
            min_speech_duration_ms: self.config.min_speech_ms as u32,
            min_silence_duration_ms: 100,
        };
        let turn_config = TurnConfig {
            silence_threshold: Duration::from_millis(self.config.gap_ms),
            min_speech_duration: Duration::from_millis(self.config.min_speech_ms),
            max_turn_duration: Duration::from_secs(30),
            sample_rate: self.config.sample_rate,
        };

        let (mut turn_manager, mut event_rx) = TurnManager::new(turn_config);
        let vad_threshold = self.config.vad_threshold;
        let sample_rate = self.config.sample_rate;
        let gap_ms = self.config.gap_ms;

        // VAD + turn loop in a dedicated thread (cpal Stream is !Send on some platforms)
        thread::spawn(move || {
            let mut vad = match VadDetector::new(vad_config) {
                Ok(v) => v,
                Err(e) => {
                    error!("Ear: VAD init failed: {}", e);
                    return;
                }
            };
            info!(
                "Ear: listening (30ms chunks, {}ms gap, VAD threshold {})",
                gap_ms, vad_threshold
            );

            let rt = tokio::runtime::Runtime::new().unwrap();

            while let Some(chunk) = rt.block_on(audio_rx.recv()) {
                if chunk.samples.len() != vad.chunk_size() {
                    continue;
                }
                let prob = match vad.process_chunk(&chunk.samples) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let is_speech = prob > vad_threshold;
                if let Some(ref tx) = vad_state_tx {
                    let state = if is_speech { VadState::Speech } else { VadState::Silence };
                    let _ = tx.send(state);
                }
                if let Err(e) = turn_manager.process_vad_result(is_speech, &chunk.samples) {
                    debug!("Ear: turn manager error: {}", e);
                }
            }
        });

        thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            while let Some(ev) = rt.block_on(event_rx.recv()) {
                if let TurnEvent::TurnCommitted {
                    timestamp,
                    total_speech_duration,
                    audio_samples,
                } = ev
                {
                    let turn = AudioTurn {
                        samples: audio_samples,
                        timestamp,
                        duration: total_speech_duration,
                        sample_rate,
                    };
                    if let Err(e) = rt.block_on(turn_tx.send(turn)) {
                        warn!("Ear: turn send failed: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(EarSession {
            handle: EarHandle {
                _stream: stream,
                turn_rx,
            },
            vad_state_rx,
        })
    }
}

/// Session from `VoiceEar::start_listening`: handle + optional real-time VAD state for interruption.
pub struct EarSession {
    /// Keep alive to keep capture running; use `recv_turn()` for committed turns.
    pub handle: EarHandle,
    /// When `emit_vad_state` is true: receive Speech/Silence every ~30ms for kill-switch.
    pub vad_state_rx: Option<mpsc::UnboundedReceiver<VadState>>,
}

/// Handle that keeps the capture stream alive. Drop to stop listening.
pub struct EarHandle {
    _stream: Stream,
    turn_rx: mpsc::Receiver<AudioTurn>,
}

impl EarHandle {
    /// Receive the next committed turn (blocks until gap detected or stream closed).
    pub async fn recv_turn(&mut self) -> Option<AudioTurn> {
        self.turn_rx.recv().await
    }

    /// Non-blocking receive.
    pub fn try_recv_turn(&mut self) -> Option<AudioTurn> {
        self.turn_rx.try_recv().ok()
    }

    /// Get the underlying receiver (e.g. for select! with other streams).
    pub fn receiver(&mut self) -> &mut mpsc::Receiver<AudioTurn> {
        &mut self.turn_rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ear_config_defaults() {
        let c = EarConfig::default();
        assert_eq!(c.sample_rate, 16000);
        assert_eq!(c.chunk_size, 480);
        assert!((c.vad_threshold - 0.5).abs() < 1e-6);
        assert_eq!(c.gap_ms, 800);
    }

    #[test]
    fn vad_state_naming() {
        assert_eq!(VadState::Silence, VadState::Silence);
        assert_ne!(VadState::Speech, VadState::PostSpeechGap);
    }
}

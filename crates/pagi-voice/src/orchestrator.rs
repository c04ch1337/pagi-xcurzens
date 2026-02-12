//! Voice Orchestrator - The main coordination layer
//!
//! This module ties together audio capture, VAD, and turn management into
//! a cohesive system for real-time voice interaction.

use crate::audio::{AudioCapture, AudioChunk, AudioConfig, AudioPlayback};
use crate::error::{VoiceError, VoiceResult};
use crate::turn::{TurnConfig, TurnEvent, TurnManager};
use crate::vad::{VadConfig, VadDetector};
use cpal::Stream;
use std::sync::Arc;
use std::thread;
use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};

/// Configuration for the voice orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub audio: AudioConfig,
    pub vad: VadConfig,
    pub turn: TurnConfig,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            audio: AudioConfig::default(),
            vad: VadConfig::default(),
            turn: TurnConfig::default(),
        }
    }
}

/// The main voice orchestration system
///
/// This coordinates:
/// - Audio capture (CPAL)
/// - Voice activity detection (WebRTC VAD)
/// - Turn management (Gap logic)
/// - Interruption handling (Rodio)
pub struct VoiceOrchestrator {
    config: OrchestratorConfig,
    
    // Audio systems
    audio_capture: Option<AudioCapture>,
    audio_playback: Arc<AudioPlayback>,
    
    // Turn manager
    turn_manager: Arc<RwLock<TurnManager>>,
    
    // Channels
    audio_rx: Option<mpsc::UnboundedReceiver<AudioChunk>>,
    turn_rx: Option<mpsc::UnboundedReceiver<TurnEvent>>,
    
    // Stream handle (kept alive)
    _stream: Option<Stream>,
    
    // Processing thread handle
    processing_thread: Option<thread::JoinHandle<()>>,
}

impl VoiceOrchestrator {
    /// Create a new voice orchestrator
    pub fn new(config: OrchestratorConfig) -> VoiceResult<Self> {
        info!("🎭 Initializing Voice Orchestrator");
        
        // Validate that audio and VAD sample rates match
        if config.audio.sample_rate != config.vad.sample_rate {
            return Err(VoiceError::Config(
                format!(
                    "Audio sample rate ({}) must match VAD sample rate ({})",
                    config.audio.sample_rate, config.vad.sample_rate
                )
            ));
        }
        
        // Initialize audio capture
        let audio_capture = AudioCapture::new(config.audio.clone())?;
        
        // Initialize audio playback
        let audio_playback = Arc::new(AudioPlayback::new()?);
        
        // Initialize turn manager
        let (turn_manager, turn_rx) = TurnManager::new(config.turn.clone());
        let turn_manager = Arc::new(RwLock::new(turn_manager));
        
        info!("✅ Voice Orchestrator initialized");
        
        Ok(Self {
            config,
            audio_capture: Some(audio_capture),
            audio_playback,
            turn_manager,
            audio_rx: None,
            turn_rx: Some(turn_rx),
            _stream: None,
            processing_thread: None,
        })
    }
    
    /// Start the voice orchestration system
    pub async fn start(&mut self) -> VoiceResult<()> {
        info!("🚀 Starting Voice Orchestrator");
        
        // Create audio chunk channel
        let (audio_tx, audio_rx) = mpsc::unbounded_channel();
        self.audio_rx = Some(audio_rx);
        
        // Start audio capture
        let audio_capture = self.audio_capture.take()
            .ok_or_else(|| VoiceError::Unknown("Audio capture already started".to_string()))?;
        
        let stream = audio_capture.start_capture(audio_tx)?;
        self._stream = Some(stream);
        
        // Start the processing loop in a dedicated thread
        self.start_processing_thread().await?;
        
        info!("✅ Voice Orchestrator started");
        
        Ok(())
    }
    
    /// Start the VAD processing thread
    ///
    /// Uses a dedicated thread since webrtc-vad is not Send/Sync
    async fn start_processing_thread(&mut self) -> VoiceResult<()> {
        let mut audio_rx = self.audio_rx.take()
            .ok_or_else(|| VoiceError::Unknown("Audio receiver not initialized".to_string()))?;
        
        let turn_manager = Arc::clone(&self.turn_manager);
        let audio_playback = Arc::clone(&self.audio_playback);
        let vad_config = self.config.vad.clone();
        
        // Spawn a dedicated thread for VAD processing
        let handle = thread::spawn(move || {
            // Create VAD detector in this thread
            let mut vad = match VadDetector::new(vad_config) {
                Ok(v) => v,
                Err(e) => {
                    error!("Failed to initialize VAD: {}", e);
                    return;
                }
            };
            
            info!("🔄 VAD processing thread started");
            
            // Create a tokio runtime for async operations in this thread
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            // Process audio chunks
            while let Some(chunk) = rt.block_on(audio_rx.recv()) {
                // Process through VAD
                match vad.process_chunk(&chunk.samples) {
                    Ok(probability) => {
                        let is_speech = vad.is_speech(probability);
                        
                        // Check for interruption
                        if is_speech && audio_playback.is_playing() {
                            audio_playback.stop();
                            
                            let mut turn_guard = rt.block_on(turn_manager.write());
                            if let Err(e) = turn_guard.signal_interruption() {
                                error!("Failed to signal interruption: {}", e);
                            }
                        }
                        
                        // Process through turn manager
                        let mut turn_guard = rt.block_on(turn_manager.write());
                        if let Err(e) = turn_guard.process_vad_result(is_speech, &chunk.samples) {
                            error!("Turn manager error: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("VAD processing error: {}", e);
                    }
                }
            }
            
            warn!("🛑 VAD processing thread ended");
        });
        
        self.processing_thread = Some(handle);
        
        Ok(())
    }
    
    /// Get the turn event receiver
    ///
    /// This allows the caller to receive turn events (speech started, ended, committed, etc.)
    pub fn take_turn_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<TurnEvent>> {
        self.turn_rx.take()
    }
    
    /// Stop the voice orchestrator
    pub async fn stop(&mut self) -> VoiceResult<()> {
        info!("🛑 Stopping Voice Orchestrator");
        
        // Drop the stream to stop audio capture
        self._stream = None;
        
        // The processing thread will exit when the audio channel closes
        if let Some(handle) = self.processing_thread.take() {
            let _ = handle.join();
        }
        
        info!("✅ Voice Orchestrator stopped");
        
        Ok(())
    }
    
    /// Check if the orchestrator is currently running
    pub fn is_running(&self) -> bool {
        self._stream.is_some() && self.processing_thread.is_some()
    }
}

impl Drop for VoiceOrchestrator {
    fn drop(&mut self) {
        // Ensure cleanup happens
        self._stream = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = OrchestratorConfig::default();
        let result = VoiceOrchestrator::new(config);
        
        // This might fail in CI without audio devices
        if let Ok(orchestrator) = result {
            assert!(!orchestrator.is_running());
        }
    }
    
    #[test]
    fn test_config_validation() {
        let mut config = OrchestratorConfig::default();
        config.audio.sample_rate = 8000;
        config.vad.sample_rate = 16000;
        
        let result = VoiceOrchestrator::new(config);
        assert!(result.is_err());
    }
}

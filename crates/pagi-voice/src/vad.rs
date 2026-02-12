//! Voice Activity Detection using WebRTC VAD
//!
//! This module wraps the WebRTC VAD for real-time speech detection.
//! Note: Silero VAD integration will be added in Stage 2 when ONNX runtime
//! compatibility is resolved.

use crate::error::{VoiceError, VoiceResult};
use tracing::{debug, info};
use webrtc_vad::{SampleRate, Vad, VadMode};

/// Configuration for VAD detection
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// Sample rate (must be 8000, 16000, 32000, or 48000 Hz for WebRTC VAD)
    pub sample_rate: u32,
    
    /// Detection mode (0-3, where 3 is most aggressive)
    pub mode: u8,
    
    /// Minimum speech duration in milliseconds (default: 250ms)
    pub min_speech_duration_ms: u32,
    
    /// Minimum silence duration in milliseconds (default: 100ms)
    pub min_silence_duration_ms: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            mode: 3, // Most aggressive detection
            min_speech_duration_ms: 250,
            min_silence_duration_ms: 100,
        }
    }
}

/// Voice Activity Detector using WebRTC VAD
pub struct VadDetector {
    vad: Vad,
    config: VadConfig,
    chunk_size: usize,
}

impl VadDetector {
    /// Create a new VAD detector with the given configuration
    pub fn new(config: VadConfig) -> VoiceResult<Self> {
        info!("🎙️ Initializing WebRTC VAD (sample_rate: {}Hz, mode: {})", 
              config.sample_rate, config.mode);
        
        // Validate sample rate
        if !matches!(config.sample_rate, 8000 | 16000 | 32000 | 48000) {
            return Err(VoiceError::Config(
                format!("WebRTC VAD only supports 8000, 16000, 32000, or 48000 Hz, got {}", config.sample_rate)
            ));
        }
        
        // Validate mode
        if config.mode > 3 {
            return Err(VoiceError::Config(
                format!("VAD mode must be 0-3, got {}", config.mode)
            ));
        }
        
        // Calculate chunk size for 30ms windows
        // WebRTC VAD requires 10ms, 20ms, or 30ms frames
        // At 16kHz: 16000 samples/sec * 0.03 sec = 480 samples
        let chunk_size = (config.sample_rate as f32 * 0.03) as usize;
        
        // Create VAD instance
        let vad_mode = match config.mode {
            0 => VadMode::Quality,
            1 => VadMode::LowBitrate,
            2 => VadMode::Aggressive,
            3 => VadMode::VeryAggressive,
            _ => VadMode::VeryAggressive,
        };
        
        let sample_rate = match config.sample_rate {
            8000 => SampleRate::Rate8kHz,
            16000 => SampleRate::Rate16kHz,
            32000 => SampleRate::Rate32kHz,
            48000 => SampleRate::Rate48kHz,
            _ => return Err(VoiceError::Config(format!("Invalid sample rate: {}", config.sample_rate))),
        };
        
        let mut vad = Vad::new();
        vad.set_mode(vad_mode);
        vad.set_sample_rate(sample_rate);
        
        info!("✅ VAD initialized (chunk_size: {} samples)", chunk_size);
        
        Ok(Self {
            vad,
            config,
            chunk_size,
        })
    }
    
    /// Process an audio chunk and return whether speech is detected
    ///
    /// The audio chunk should be exactly `chunk_size` samples (30ms worth).
    /// Returns a probability score (0.0 or 1.0 for WebRTC VAD).
    pub fn process_chunk(&mut self, audio: &[f32]) -> VoiceResult<f32> {
        if audio.len() != self.chunk_size {
            return Err(VoiceError::VadProcessing(
                format!("Expected {} samples, got {}", self.chunk_size, audio.len())
            ));
        }
        
        // Convert f32 samples to i16 for WebRTC VAD
        let audio_i16: Vec<i16> = audio
            .iter()
            .map(|&sample| (sample.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();
        
        // Process the audio through the VAD
        let is_speech = self.vad.is_voice_segment(&audio_i16)
            .map_err(|e| VoiceError::VadProcessing(format!("VAD processing failed: {:?}", e)))?;
        
        let probability = if is_speech { 1.0 } else { 0.0 };
        
        debug!("VAD result: {}", if is_speech { "SPEECH" } else { "SILENCE" });
        
        Ok(probability)
    }
    
    /// Check if the given probability indicates speech
    pub fn is_speech(&self, probability: f32) -> bool {
        probability > 0.5
    }
    
    /// Get the expected chunk size in samples
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
    
    /// Get the sample rate
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }
    
    /// Reset the VAD session state
    pub fn reset(&mut self) -> VoiceResult<()> {
        // WebRTC VAD doesn't have explicit reset, recreate it
        let vad_mode = match self.config.mode {
            0 => VadMode::Quality,
            1 => VadMode::LowBitrate,
            2 => VadMode::Aggressive,
            3 => VadMode::VeryAggressive,
            _ => VadMode::VeryAggressive,
        };
        
        let sample_rate = match self.config.sample_rate {
            8000 => SampleRate::Rate8kHz,
            16000 => SampleRate::Rate16kHz,
            32000 => SampleRate::Rate32kHz,
            48000 => SampleRate::Rate48kHz,
            _ => return Err(VoiceError::VadProcessing(format!("Invalid sample rate: {}", self.config.sample_rate))),
        };
        
        self.vad = Vad::new();
        self.vad.set_mode(vad_mode);
        self.vad.set_sample_rate(sample_rate);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vad_initialization() {
        let config = VadConfig::default();
        let detector = VadDetector::new(config);
        assert!(detector.is_ok());
        
        let detector = detector.unwrap();
        assert_eq!(detector.chunk_size(), 480); // 30ms at 16kHz
    }
    
    #[test]
    fn test_invalid_sample_rate() {
        let config = VadConfig {
            sample_rate: 44100,
            ..Default::default()
        };
        
        let result = VadDetector::new(config);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_chunk_size_validation() {
        let config = VadConfig::default();
        let mut detector = VadDetector::new(config).unwrap();
        
        // Wrong size should error
        let wrong_size = vec![0.0f32; 100];
        let result = detector.process_chunk(&wrong_size);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_silence_detection() {
        let config = VadConfig::default();
        let mut detector = VadDetector::new(config).unwrap();
        
        // Silence (all zeros)
        let silence = vec![0.0f32; 480];
        let result = detector.process_chunk(&silence).unwrap();
        assert_eq!(result, 0.0);
    }
}

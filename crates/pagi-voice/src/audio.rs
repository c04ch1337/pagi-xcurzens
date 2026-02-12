//! Audio capture and playback using CPAL and Rodio
//!
//! This module handles low-latency audio I/O on bare metal.

use crate::error::{VoiceError, VoiceResult};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Audio configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Sample rate in Hz (default: 16000)
    pub sample_rate: u32,
    
    /// Number of channels (default: 1 for mono)
    pub channels: u16,
    
    /// Buffer size in samples (default: 480 for 30ms at 16kHz)
    pub buffer_size: usize,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            buffer_size: 480, // 30ms at 16kHz
        }
    }
}

/// Audio chunk sent from the capture thread
#[derive(Debug, Clone)]
pub struct AudioChunk {
    /// Audio samples (f32, normalized to -1.0 to 1.0)
    pub samples: Vec<f32>,
    
    /// Timestamp when captured
    pub timestamp: std::time::Instant,
}

/// Audio capture system using CPAL
pub struct AudioCapture {
    config: AudioConfig,
    device: Device,
    stream_config: StreamConfig,
}

impl AudioCapture {
    /// Create a new audio capture system
    pub fn new(config: AudioConfig) -> VoiceResult<Self> {
        info!("🎤 Initializing audio capture ({}Hz, {} channels)",
              config.sample_rate, config.channels);
        
        // Get the default input device
        let device = cpal::default_host().default_input_device()
            .ok_or_else(|| VoiceError::AudioDevice("No input device available".to_string()))?;
        
        info!("📱 Using input device: {}", device.name().unwrap_or_else(|_| "Unknown".to_string()));
        
        // Get the default input config
        let default_config = device.default_input_config()?;
        
        info!("🔧 Default config: {:?}", default_config);
        
        // Create our desired stream config
        let stream_config = StreamConfig {
            channels: config.channels,
            sample_rate: cpal::SampleRate(config.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(config.buffer_size as u32),
        };
        
        Ok(Self {
            config,
            device,
            stream_config,
        })
    }
    
    /// Start capturing audio and send chunks to the provided channel
    ///
    /// This runs in a high-priority thread to minimize latency.
    pub fn start_capture(
        self,
        chunk_tx: mpsc::UnboundedSender<AudioChunk>,
    ) -> VoiceResult<Stream> {
        info!("▶️ Starting audio capture stream");
        
        let buffer_size = self.config.buffer_size;
        let mut sample_buffer = Vec::with_capacity(buffer_size);
        
        // Build the input stream
        let stream = self.device.build_input_stream(
            &self.stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Accumulate samples into our buffer
                for &sample in data {
                    sample_buffer.push(sample);
                    
                    // When we have a full chunk, send it
                    if sample_buffer.len() >= buffer_size {
                        let chunk = AudioChunk {
                            samples: sample_buffer.clone(),
                            timestamp: std::time::Instant::now(),
                        };
                        
                        if let Err(e) = chunk_tx.send(chunk) {
                            warn!("Failed to send audio chunk: {}", e);
                        }
                        
                        sample_buffer.clear();
                    }
                }
            },
            move |err| {
                warn!("Audio stream error: {}", err);
            },
            None, // No timeout
        )?;
        
        // Start the stream
        stream.play()?;
        
        info!("✅ Audio capture started");
        
        Ok(stream)
    }
    
    /// List available input devices
    pub fn list_input_devices() -> VoiceResult<Vec<String>> {
        let host = cpal::default_host();
        let devices = host.input_devices()?;
        
        let mut device_names = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                device_names.push(name);
            }
        }
        
        Ok(device_names)
    }
}

/// Audio playback system using Rodio
pub struct AudioPlayback {
    _sink: Arc<rodio::Sink>,
}

impl AudioPlayback {
    /// Create a new audio playback system
    pub fn new() -> VoiceResult<Self> {
        info!("🔊 Initializing audio playback");
        
        // Get the default output device
        let (_stream, stream_handle) = rodio::OutputStream::try_default()
            .map_err(|e| VoiceError::Playback(e.to_string()))?;
        
        // Create a sink for audio playback
        let sink = rodio::Sink::try_new(&stream_handle)
            .map_err(|e| VoiceError::Playback(e.to_string()))?;
        
        info!("✅ Audio playback initialized");
        
        Ok(Self {
            _sink: Arc::new(sink),
        })
    }
    
    /// Check if audio is currently playing
    pub fn is_playing(&self) -> bool {
        !self._sink.empty()
    }
    
    /// Stop playback immediately (for interruptions)
    pub fn stop(&self) {
        self._sink.stop();
        info!("⏹️ Audio playback stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_config_defaults() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.buffer_size, 480);
    }
    
    #[test]
    fn test_list_devices() {
        // This might fail in CI environments without audio devices
        let result = AudioCapture::list_input_devices();
        if let Ok(devices) = result {
            println!("Available input devices: {:?}", devices);
        }
    }
}

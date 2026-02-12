//! Error types for the PAGI Voice system

use thiserror::Error;

/// Result type alias for voice operations
pub type VoiceResult<T> = Result<T, VoiceError>;

/// Errors that can occur in the voice orchestration system
#[derive(Error, Debug)]
pub enum VoiceError {
    #[error("Audio device error: {0}")]
    AudioDevice(String),

    #[error("VAD initialization failed: {0}")]
    VadInit(String),

    #[error("VAD processing error: {0}")]
    VadProcessing(String),

    #[error("Audio stream error: {0}")]
    AudioStream(String),

    #[error("Channel send error: {0}")]
    ChannelSend(String),

    #[error("Channel receive error: {0}")]
    ChannelReceive(String),

    #[error("Audio playback error: {0}")]
    Playback(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("ONNX runtime error: {0}")]
    OnnxRuntime(String),

    #[error("STT error: {0}")]
    Stt(String),

    #[error("TTS error: {0}")]
    Tts(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<cpal::DevicesError> for VoiceError {
    fn from(err: cpal::DevicesError) -> Self {
        VoiceError::AudioDevice(err.to_string())
    }
}

impl From<cpal::DefaultStreamConfigError> for VoiceError {
    fn from(err: cpal::DefaultStreamConfigError) -> Self {
        VoiceError::AudioDevice(err.to_string())
    }
}

impl From<cpal::BuildStreamError> for VoiceError {
    fn from(err: cpal::BuildStreamError) -> Self {
        VoiceError::AudioStream(err.to_string())
    }
}

impl From<cpal::PlayStreamError> for VoiceError {
    fn from(err: cpal::PlayStreamError) -> Self {
        VoiceError::AudioStream(err.to_string())
    }
}

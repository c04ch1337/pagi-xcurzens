//! # PAGI Voice - Sovereign Voice Orchestration
//!
//! This crate implements real-time Voice Activity Detection (VAD) and asynchronous
//! turn-taking for Phoenix Marie. Built on bare metal Rust for minimal latency.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Voice Orchestrator                        │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
//! │  │   Audio In   │→ │  Silero VAD  │→ │ Turn Manager │     │
//! │  │    (cpal)    │  │   (ONNX)     │  │  (800ms gap) │     │
//! │  └──────────────┘  └──────────────┘  └──────────────┘     │
//! │         ↓                                      ↓            │
//! │  ┌──────────────┐                    ┌──────────────┐     │
//! │  │  Audio Out   │←───────────────────│ Interruption │     │
//! │  │   (rodio)    │    Kill Signal     │   Handler    │     │
//! │  └──────────────┘                    └──────────────┘     │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod audio;
pub mod ear;
pub mod error;
pub mod orchestrator;
pub mod sovereign_voice;
pub mod stt;
pub mod turn;
pub mod vad;
pub mod voice_output;

pub use audio::{AudioChunk, AudioConfig, AudioCapture, AudioPlayback};
pub use ear::{AudioTurn, EarConfig, EarHandle, EarSession, VadState, VoiceEar};
pub use error::{VoiceError, VoiceResult};
pub use orchestrator::OrchestratorConfig;
pub use orchestrator::VoiceOrchestrator;
pub use sovereign_voice::{run_voice_loop, OnInterruption};
pub use stt::{
    transcribe_turn, create_best_stt, OpenRouterStt, PlaceholderStt, SttBackend,
};
#[cfg(feature = "whisper")]
pub use stt::WhisperStt;
pub use turn::{TurnConfig, TurnEvent, TurnManager};
pub use vad::{VadConfig, VadDetector};
pub use voice_output::{OpenRouterTts, PlaceholderTts, TtsBackend, VoiceOutput};

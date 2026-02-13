//! Transcript segment emitted by the Whisper worker (near-live transcription).

use serde::{Deserialize, Serialize};

/// A single transcript segment from Whisper (who, what, when).
/// Speaker ID is reserved for future intro-detection / diarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    /// Reserved for intro-detection: "This is The Creator" → contacts table.
    pub speaker_id: Option<u32>,
    /// Transcribed text.
    pub text: String,
    /// Wall-clock timestamp (seconds since meeting start or Unix epoch).
    pub timestamp: f64,
}

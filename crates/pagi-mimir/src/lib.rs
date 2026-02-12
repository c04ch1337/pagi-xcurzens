//! # PAGI Mimir — Meeting Documentation Layer
//!
//! Sovereign meeting capture: cpal (mic + loopback) → Whisper-Core → Chronos.
//! Keeps SAO/sensitive discussions entirely on-device (bare metal, no cloud).

pub mod audio;
pub mod preflight;
pub mod storage;
pub mod transcript;
pub mod whisper_worker;

pub use audio::MeetingRecorder;
pub use preflight::{run_preflight_audio_check, DetectedDevices, PreFlightAudioReport};
pub use storage::{MeetingStorage, MeetingRow, MeetingTranscriptRow, ProjectRow};
pub use transcript::TranscriptSegment;
pub use whisper_worker::{create_mimir_stt, WhisperTranscriptWorker};

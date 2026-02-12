//! Background worker: PCM buffer → Whisper-Core → TranscriptSegment.
//!
//! Runs in a thread; every `interval_secs` takes a chunk from the buffer,
//! transcribes via pagi-voice WhisperStt, and sends segments to a channel and/or storage.

use crate::audio::MimirAudioBuffer;
use crate::storage::MeetingStorage;
use crate::transcript::TranscriptSegment;
use chrono::Utc;
use pagi_voice::ear::AudioTurn;
use pagi_voice::stt::SttBackend;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

/// Runs the Whisper worker: periodically drains the buffer, transcribes, emits segments.
pub struct WhisperTranscriptWorker {
    buffer: Arc<MimirAudioBuffer>,
    stt: Arc<dyn SttBackend + Send + Sync>,
    interval_secs: u64,
    meeting_id: String,
    storage: Option<Arc<MeetingStorage>>,
    segment_tx: Option<tokio::sync::mpsc::UnboundedSender<TranscriptSegment>>,
}

impl WhisperTranscriptWorker {
    /// Create worker with buffer, Whisper STT, and optional storage/segment channel.
    pub fn new(
        buffer: Arc<MimirAudioBuffer>,
        stt: Arc<dyn SttBackend + Send + Sync>,
        interval_secs: u64,
        meeting_id: String,
        storage: Option<Arc<MeetingStorage>>,
        segment_tx: Option<tokio::sync::mpsc::UnboundedSender<TranscriptSegment>>,
    ) -> Self {
        Self {
            buffer,
            stt,
            interval_secs,
            meeting_id,
            storage,
            segment_tx,
        }
    }

    /// Run the worker on the current thread (blocking). Call from a dedicated thread.
    pub fn run_blocking(&self, stop: Arc<std::sync::atomic::AtomicBool>) {
        let chunk_samples = (self.interval_secs as usize) * 16000;
        let interval = Duration::from_secs(self.interval_secs);

        while !stop.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(interval);

            if stop.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let (samples, elapsed_sec) = self.buffer.take_chunk(chunk_samples);
            if samples.len() < 16000 {
                continue;
            }

            let turn = AudioTurn {
                samples: samples.clone(),
                timestamp: Utc::now(),
                duration: Duration::from_secs_f64(samples.len() as f64 / 16000.0),
                sample_rate: 16000,
            };

            match self.stt.transcribe_turn(&turn) {
                Ok(text) => {
                    let text = text.trim();
                    if text.is_empty() {
                        continue;
                    }
                    let segment = TranscriptSegment {
                        speaker_id: None,
                        text: text.to_string(),
                        timestamp: elapsed_sec,
                    };
                    if let Some(ref tx) = self.segment_tx {
                        let _ = tx.send(segment.clone());
                    }
                    if let Some(ref db) = self.storage {
                        if let Err(e) = db.append_transcript(
                            &self.meeting_id,
                            None,
                            &segment.text,
                            segment.timestamp,
                        ) {
                            warn!("Mimir: failed to append transcript: {}", e);
                        }
                    }
                    info!("Mimir: [{}s] {}", segment.timestamp as u64, segment.text);
                }
                Err(e) => {
                    warn!("Mimir: Whisper error: {}", e);
                }
            }
        }
    }
}

/// Build best available STT: Whisper (if feature and WHISPER_MODEL_PATH) or placeholder.
pub fn create_mimir_stt() -> Result<Arc<dyn SttBackend + Send + Sync>, Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "whisper")]
    {
        use pagi_voice::stt::WhisperStt;
        let path = std::env::var("WHISPER_MODEL_PATH").ok();
        let path = path.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
        if let Some(p) = path {
            if let Ok(w) = WhisperStt::new(p) {
                return Ok(Arc::new(w));
            }
        }
    }
    Ok(Arc::new(pagi_voice::stt::PlaceholderStt::new()))
}

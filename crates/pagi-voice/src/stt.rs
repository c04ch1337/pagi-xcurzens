//! **Speech-to-Text (STT)** — convert `AudioTurn` PCM into text for the Orchestrator.
//!
//! Implement `SttBackend` for local Whisper (e.g. whisper-rs) or remote OpenRouter STT.
//! Use `transcribe_turn` with any backend to get a String for downstream chat/archetype.

use crate::ear::AudioTurn;
use crate::error::{VoiceError, VoiceResult};
use std::io::Write;

/// Backend for converting PCM (AudioTurn) to text. Implement for local Whisper or remote STT.
pub trait SttBackend: Send + Sync {
    /// Transcribe one turn. PCM is 16kHz mono f32; return empty string if nothing detected.
    fn transcribe_turn(&self, turn: &AudioTurn) -> VoiceResult<String>;
}

/// Encode f32 PCM (mono, 16 kHz) to 16-bit WAV bytes for API upload.
fn pcm_f32_to_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let num_samples = samples.len();
    let data_len = num_samples * 2; // 16-bit = 2 bytes per sample
    let header_len = 44u32;
    let file_len = header_len + data_len as u32;

    let mut buf = Vec::with_capacity(44 + data_len);
    // RIFF header
    buf.write_all(b"RIFF").unwrap();
    buf.write_all(&(file_len - 8).to_le_bytes()).unwrap();
    buf.write_all(b"WAVE").unwrap();
    // fmt subchunk
    buf.write_all(b"fmt ").unwrap();
    buf.write_all(&16u32.to_le_bytes()).unwrap(); // subchunk1 size
    buf.write_all(&1u16.to_le_bytes()).unwrap();  // PCM
    buf.write_all(&1u16.to_le_bytes()).unwrap();  // mono
    buf.write_all(&sample_rate.to_le_bytes()).unwrap();
    buf.write_all(&(sample_rate * 2).to_le_bytes()).unwrap(); // byte rate
    buf.write_all(&2u16.to_le_bytes()).unwrap();  // block align
    buf.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample
    // data subchunk
    buf.write_all(b"data").unwrap();
    buf.write_all(&(data_len as u32).to_le_bytes()).unwrap();
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let i = (clamped * 32767.0).round() as i16;
        buf.write_all(&i.to_le_bytes()).unwrap();
    }
    buf
}

/// Placeholder STT: returns a fixed string. Use for testing the voice loop without Whisper/API.
#[derive(Debug, Default)]
pub struct PlaceholderStt {
    /// If set, return this instead of the default message.
    pub response: Option<String>,
}

impl PlaceholderStt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_response(s: String) -> Self {
        Self { response: Some(s) }
    }
}

impl SttBackend for PlaceholderStt {
    fn transcribe_turn(&self, turn: &AudioTurn) -> VoiceResult<String> {
        if let Some(ref r) = self.response {
            return Ok(r.clone());
        }
        let secs = turn.duration.as_secs_f32();
        Ok(format!(
            "[STT placeholder: {} samples, {:.1}s — connect Whisper or OpenRouter STT]",
            turn.samples.len(),
            secs
        ))
    }
}

/// Production STT backend: OpenAI-compatible transcription API (OpenAI Whisper, OpenRouter, etc.).
/// Uses `STT_API_URL` (e.g. https://api.openai.com/v1), `STT_API_KEY`, and `STT_MODEL` (default whisper-1).
#[derive(Debug, Clone)]
pub struct OpenRouterStt {
    /// Base URL without trailing slash (e.g. https://api.openai.com/v1).
    pub base_url: String,
    /// Bearer API key.
    pub api_key: String,
    /// Model: whisper-1 or gpt-4o-transcribe, etc.
    pub model: String,
    client: reqwest::blocking::Client,
}

impl OpenRouterStt {
    /// Build from environment: STT_API_URL, STT_API_KEY (or PAGI_LLM_API_KEY / OPENROUTER_API_KEY), STT_MODEL.
    pub fn from_env() -> VoiceResult<Self> {
        let base_url = std::env::var("STT_API_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let api_key = std::env::var("STT_API_KEY")
            .or_else(|_| std::env::var("PAGI_LLM_API_KEY"))
            .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
            .map_err(|_| VoiceError::Config("STT requires STT_API_KEY, PAGI_LLM_API_KEY, or OPENROUTER_API_KEY".to_string()))?;
        let model = std::env::var("STT_MODEL").unwrap_or_else(|_| "whisper-1".to_string());
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| VoiceError::Stt(e.to_string()))?;
        Ok(Self {
            base_url,
            api_key,
            model,
            client,
        })
    }

    /// Create with explicit config.
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> VoiceResult<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| VoiceError::Stt(e.to_string()))?;
        Ok(Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            client,
        })
    }
}

impl SttBackend for OpenRouterStt {
    fn transcribe_turn(&self, turn: &AudioTurn) -> VoiceResult<String> {
        if turn.samples.is_empty() {
            return Ok(String::new());
        }
        let wav = pcm_f32_to_wav(&turn.samples, turn.sample_rate);
        let url = format!("{}/audio/transcriptions", self.base_url.trim_end_matches('/'));
        let part = reqwest::blocking::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| VoiceError::Stt(e.to_string()))?;
        let form = reqwest::blocking::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone());
        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .map_err(|e| VoiceError::Stt(e.to_string()))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().unwrap_or_default();
            return Err(VoiceError::Stt(format!("STT API error {}: {}", status, body)));
        }
        let json: serde_json::Value = res.json().map_err(|e| VoiceError::Stt(e.to_string()))?;
        let text = json
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        Ok(text)
    }
}

// -----------------------------------------------------------------------------
// Local Whisper STT (optional feature). Requires whisper.cpp/ggml; see README.
// -----------------------------------------------------------------------------
#[cfg(feature = "whisper")]
mod whisper_stt {
    use super::*;
    use std::sync::Mutex;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    /// Local Whisper STT: loads a ggml quantized model (e.g. ggml-base.en.bin) and runs
    /// inference on-device. Audio must be 16 kHz mono f32 (Ear default).
    /// Download models from: https://huggingface.co/ggerganov/whisper.cpp (e.g. ggml-base.en.bin).
    pub struct WhisperStt {
        #[allow(dead_code)]
        context: WhisperContext,
        state: Mutex<whisper_rs::WhisperState>,
    }

    impl WhisperStt {
        /// Load the Whisper model from `model_path` (e.g. path to ggml-base.en.bin).
        pub fn new(model_path: &str) -> VoiceResult<Self> {
            let params = WhisperContextParameters::default();
            let context = WhisperContext::new_with_params(model_path, params)
                .map_err(|e| VoiceError::Stt(format!("Whisper load failed: {}", e)))?;
            let state = context
                .create_state()
                .map_err(|e| VoiceError::Stt(format!("Whisper state init failed: {}", e)))?;
            Ok(Self {
                context,
                state: Mutex::new(state),
            })
        }

        /// Build from env: `WHISPER_MODEL_PATH` must point to a .bin model file.
        pub fn from_env() -> VoiceResult<Self> {
            let path = std::env::var("WHISPER_MODEL_PATH")
                .map_err(|_| VoiceError::Config("WHISPER_MODEL_PATH not set".to_string()))?;
            let path = path.trim();
            if path.is_empty() {
                return Err(VoiceError::Config("WHISPER_MODEL_PATH is empty".to_string()));
            }
            Self::new(path)
        }
    }

    impl SttBackend for WhisperStt {
        fn transcribe_turn(&self, turn: &AudioTurn) -> VoiceResult<String> {
            if turn.samples.is_empty() {
                return Ok(String::new());
            }
            if turn.sample_rate != 16000 {
                return Err(VoiceError::Stt(format!(
                    "Whisper expects 16 kHz; got {} Hz (resample or use OpenRouterStt)",
                    turn.sample_rate
                )));
            }
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_no_timestamps(true);
            params.set_language(Some("en"));

            let mut state = self
                .state
                .lock()
                .map_err(|e| VoiceError::Stt(format!("Whisper lock poisoned: {}", e)))?;
            state
                .full(&params, &turn.samples)
                .map_err(|e| VoiceError::Stt(format!("Whisper inference failed: {}", e)))?;
            let text = state
                .as_iter()
                .filter_map(|seg| seg.to_str().ok())
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            Ok(text)
        }
    }
}

#[cfg(feature = "whisper")]
pub use whisper_stt::WhisperStt;

/// Create the best available STT backend from environment.
/// Priority: (1) WhisperStt if `WHISPER_MODEL_PATH` is set and model loads (requires `whisper` feature),
/// (2) OpenRouterStt if `STT_API_KEY` or `PAGI_LLM_API_KEY` is set, (3) PlaceholderStt.
pub fn create_best_stt() -> VoiceResult<Box<dyn SttBackend>> {
    #[cfg(feature = "whisper")]
    {
        if let Ok(path) = std::env::var("WHISPER_MODEL_PATH") {
            let path = path.trim();
            if !path.is_empty() {
                if let Ok(w) = whisper_stt::WhisperStt::new(path) {
                    return Ok(Box::new(w));
                }
            }
        }
    }
    if let Ok(open) = OpenRouterStt::from_env() {
        return Ok(Box::new(open));
    }
    Ok(Box::new(PlaceholderStt::new()))
}

/// Transcribe an `AudioTurn` using the given backend. Convenience for the voice loop.
pub fn transcribe_turn(backend: &dyn SttBackend, turn: &AudioTurn) -> VoiceResult<String> {
    backend.transcribe_turn(turn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::time::Duration;

    #[test]
    fn placeholder_returns_message() {
        let stt = PlaceholderStt::new();
        let turn = AudioTurn {
            samples: vec![0.0; 480],
            timestamp: Utc::now(),
            duration: Duration::from_millis(30),
            sample_rate: 16000,
        };
        let s = stt.transcribe_turn(&turn).unwrap();
        assert!(s.contains("STT placeholder"));
        assert!(s.contains("480"));
    }

    #[test]
    fn placeholder_with_response() {
        let stt = PlaceholderStt::with_response("hello world".to_string());
        let turn = AudioTurn {
            samples: vec![],
            timestamp: Utc::now(),
            duration: Duration::ZERO,
            sample_rate: 16000,
        };
        assert_eq!(stt.transcribe_turn(&turn).unwrap(), "hello world");
    }
}

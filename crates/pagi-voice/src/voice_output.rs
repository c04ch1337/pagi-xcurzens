//! **VoiceOutput** — TTS playback and the interruption kill-switch.
//!
//! Manages a `rodio::Sink` for playing synthesized speech. When the Ear reports
//! `VadState::Speech` while playing, call `stop()` to clear the queue and fall silent.

use crate::error::{VoiceError, VoiceResult};
use rodio::{OutputStream, OutputStreamHandle, Sink, Source};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Backend that turns text into audio bytes (WAV/MP3). Implement for OpenAI/ElevenLabs/OpenRouter or local TTS.
pub trait TtsBackend: Send + Sync {
    /// Synthesize text to audio bytes (e.g. WAV or MP3). Return empty vec to skip playback.
    fn synthesize(&self, text: &str) -> VoiceResult<Vec<u8>>;
}

/// Placeholder TTS: returns empty audio so nothing plays. Use for testing the kill-switch with manual playback.
#[derive(Debug, Default)]
pub struct PlaceholderTts;

impl TtsBackend for PlaceholderTts {
    fn synthesize(&self, _text: &str) -> VoiceResult<Vec<u8>> {
        Ok(Vec::new())
    }
}

/// Map PAGI archetype to OpenAI TTS voice. Humanity Slider can be expressed by choosing
/// warmer (Pisces/Shimmer) vs drier (Virgo/Nova) voices.
fn archetype_to_voice(archetype: Option<&str>) -> &'static str {
    let s = match archetype {
        Some(a) => a,
        None => return "shimmer",
    };
    if s.eq_ignore_ascii_case("pisces") {
        "shimmer"
    } else if s.eq_ignore_ascii_case("virgo") {
        "nova"
    } else if s.eq_ignore_ascii_case("scorpio") {
        "onyx"
    } else if s.eq_ignore_ascii_case("libra") {
        "echo"
    } else if s.eq_ignore_ascii_case("cancer") {
        "alloy"
    } else if s.eq_ignore_ascii_case("capricorn") {
        "fable"
    } else if s.eq_ignore_ascii_case("leo") {
        "sage"
    } else if s.eq_ignore_ascii_case("aries")
        || s.eq_ignore_ascii_case("taurus")
        || s.eq_ignore_ascii_case("gemini")
        || s.eq_ignore_ascii_case("sagittarius")
        || s.eq_ignore_ascii_case("aquarius")
    {
        "nova"
    } else {
        "shimmer"
    }
}

/// Production TTS backend: OpenAI-compatible API (OpenAI, OpenRouter, etc.).
/// Uses `TTS_API_URL` (e.g. https://api.openai.com/v1) and `TTS_API_KEY`; voice is selected from `PAGI_PRIMARY_ARCHETYPE`.
#[derive(Debug, Clone)]
pub struct OpenRouterTts {
    /// Base URL without trailing slash (e.g. https://api.openai.com/v1).
    pub base_url: String,
    /// Bearer API key.
    pub api_key: String,
    /// TTS model: tts-1 (fast) or tts-1-hd (higher quality).
    pub model: String,
    /// Override voice (alloy, echo, fable, onyx, nova, shimmer, etc.). If None, derived from archetype.
    pub voice_override: Option<String>,
    /// Primary archetype for voice selection (e.g. pisces -> shimmer, virgo -> nova).
    pub primary_archetype: Option<String>,
    /// HTTP client (blocking) for sync synthesize().
    client: reqwest::blocking::Client,
}

impl OpenRouterTts {
    /// Build from environment: TTS_API_URL, TTS_API_KEY (or PAGI_LLM_API_KEY / OPENROUTER_API_KEY), PAGI_PRIMARY_ARCHETYPE.
    pub fn from_env() -> VoiceResult<Self> {
        let base_url = std::env::var("TTS_API_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let api_key = std::env::var("TTS_API_KEY")
            .or_else(|_| std::env::var("PAGI_LLM_API_KEY"))
            .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
            .map_err(|_| VoiceError::Config("TTS requires TTS_API_KEY, PAGI_LLM_API_KEY, or OPENROUTER_API_KEY".to_string()))?;
        let model = std::env::var("TTS_MODEL").unwrap_or_else(|_| "tts-1".to_string());
        let primary_archetype = std::env::var("PAGI_PRIMARY_ARCHETYPE").ok();
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| VoiceError::Tts(e.to_string()))?;
        Ok(Self {
            base_url,
            api_key,
            model,
            voice_override: None,
            primary_archetype,
            client,
        })
    }

    /// Create with explicit config (e.g. for tests or non-env wiring).
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
        primary_archetype: Option<String>,
    ) -> VoiceResult<Self> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| VoiceError::Tts(e.to_string()))?;
        Ok(Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            voice_override: None,
            primary_archetype,
            client,
        })
    }

    /// Set a fixed voice (e.g. "nova") instead of deriving from archetype.
    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.voice_override = Some(voice.into());
        self
    }

    fn voice_id(&self) -> String {
        if let Some(ref v) = self.voice_override {
            return v.clone();
        }
        archetype_to_voice(self.primary_archetype.as_deref()).to_string()
    }
}

impl TtsBackend for OpenRouterTts {
    fn synthesize(&self, text: &str) -> VoiceResult<Vec<u8>> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!("{}/audio/speech", self.base_url.trim_end_matches('/'));
        let voice = self.voice_id();
        let body = serde_json::json!({
            "model": self.model,
            "input": text,
            "voice": voice,
        });
        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|e| VoiceError::Tts(e.to_string()))?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().unwrap_or_default();
            return Err(VoiceError::Tts(format!("TTS API error {}: {}", status, body)));
        }
        let bytes = res.bytes().map_err(|e| VoiceError::Tts(e.to_string()))?;
        Ok(bytes.to_vec())
    }
}

/// Manages playback of TTS audio. Call `stop()` when the Ear reports Speech (interruption).
pub struct VoiceOutput {
    _stream: OutputStream,
    _stream_handle: OutputStreamHandle,
    sink: Arc<Sink>,
}

impl VoiceOutput {
    /// Create a new VoiceOutput (default output device).
    pub fn new() -> VoiceResult<Self> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| VoiceError::Playback(e.to_string()))?;
        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| VoiceError::Playback(e.to_string()))?;
        info!("VoiceOutput: sink ready for TTS playback");
        Ok(Self {
            _stream: stream,
            _stream_handle: stream_handle,
            sink: Arc::new(sink),
        })
    }

    /// Play pre-decoded audio bytes (WAV/MP3). Decoder is built from `Cursor`.
    pub fn play_bytes(&self, bytes: &[u8]) -> VoiceResult<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let cursor = Cursor::new(bytes.to_vec());
        let source = rodio::Decoder::new(cursor)
            .map_err(|e| VoiceError::Playback(format!("Decode failed: {}", e)))?;
        self.sink.append(source.convert_samples::<f32>());
        Ok(())
    }

    /// Synthesize text via the given TTS backend and play. No-op if backend returns empty.
    pub fn speak(&self, text: &str, tts: &dyn TtsBackend) -> VoiceResult<()> {
        let bytes = tts.synthesize(text)?;
        self.play_bytes(&bytes)
    }

    /// Play a short silence (useful when TTS returns empty; keeps pipeline consistent).
    pub fn play_silence(&self, duration: Duration) {
        let source = rodio::source::Zero::<f32>::new(1, 44100)
            .take_duration(duration)
            .convert_samples::<f32>();
        self.sink.append(source);
    }

    /// Stop playback immediately and clear the queue. Call when Ear reports `VadState::Speech` (interruption).
    pub fn stop(&self) {
        self.sink.stop();
        info!("VoiceOutput: stopped (interruption or manual)");
    }

    /// Whether the sink currently has queued samples (playing or pending).
    pub fn is_playing(&self) -> bool {
        !self.sink.empty()
    }

    /// Block until all currently queued audio has finished (optional, for tests).
    pub fn sleep_until_end(&self) {
        self.sink.sleep_until_end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_tts_returns_empty() {
        let tts = PlaceholderTts;
        let out = tts.synthesize("hello").unwrap();
        assert!(out.is_empty());
    }
}

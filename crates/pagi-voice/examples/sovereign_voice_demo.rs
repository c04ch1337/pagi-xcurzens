//! Sovereign Voice Demo — full loop with optional production STT/TTS.
//!
//! Uses `run_voice_loop` with `create_best_stt()` and optional TTS:
//! - **STT**: WhisperStt (if `whisper` feature and `WHISPER_MODEL_PATH`), else OpenRouterStt (if API keys), else PlaceholderStt.
//! - **TTS**: OpenRouterTts if `TTS_API_KEY` (or `PAGI_LLM_API_KEY`) is set, else PlaceholderTts.
//!
//! Set API keys in `.env` to hear Phoenix speak. Voice is selected from `PAGI_PRIMARY_ARCHETYPE`.

use pagi_voice::{
    ear::{EarConfig, VoiceEar},
    run_voice_loop,
    stt::{create_best_stt, PlaceholderStt},
    voice_output::{OpenRouterTts, PlaceholderTts, TtsBackend, VoiceOutput},
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Sovereign Voice Demo — Ear → STT → TTS with interruption kill-switch");
    info!("Set TTS_API_KEY / STT_API_KEY (or PAGI_LLM_API_KEY) for production backends.");
    info!("Press Ctrl+C to stop.\n");

    let ear = VoiceEar::new(EarConfig::default());
    let session = ear.start_listening()?;
    let voice_output = VoiceOutput::new()?;

    let stt = match create_best_stt() {
        Ok(b) => {
            info!("STT: using best available backend (Whisper if WHISPER_MODEL_PATH set, else OpenRouter if keys set, else Placeholder).");
            b
        }
        Err(e) => {
            info!("STT fallback to Placeholder: {}", e);
            Box::new(PlaceholderStt::new())
        }
    };

    let tts: Box<dyn TtsBackend> = match OpenRouterTts::from_env() {
        Ok(t) => {
            info!("Using OpenRouterTts for synthesis (voice from PAGI_PRIMARY_ARCHETYPE).");
            Box::new(t)
        }
        Err(_) => {
            info!("Using PlaceholderTts (set TTS_API_KEY for Phoenix to speak).");
            Box::new(PlaceholderTts)
        }
    };

    let on_text = |text: String| async move {
        let response = format!("You said: {}.", text.trim());
        info!("Reply: {}", response);
        response
    };

    run_voice_loop(session, &voice_output, stt.as_ref(), tts.as_ref(), on_text, None).await?;
    Ok(())
}

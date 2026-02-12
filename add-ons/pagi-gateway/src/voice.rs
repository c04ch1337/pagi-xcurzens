//! Voice Gateway Controller — runs the Sovereign Voice loop (Ear → STT → Orchestrator → TTS)
//! when the gateway is started with `--voice`. Uses the same chat pipeline as the API.

use pagi_voice::{
    create_best_stt,
    ear::EarConfig,
    run_voice_loop,
    voice_output::{OpenRouterTts, PlaceholderTts, TtsBackend, VoiceOutput},
    OnInterruption,
};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{info, warn};

const VOICE_CHAT_URL: &str = "http://127.0.0.1:8000/api/v1/chat";

/// Log current STT and TTS backend status at startup (call from main when --voice is used).
pub fn log_voice_status() {
    let stt_status = if pagi_voice::OpenRouterStt::from_env().is_ok() {
        "STT: [OpenRouter] (Cloud)"
    } else {
        "STT: [Placeholder] (set STT_API_KEY or WHISPER_MODEL_PATH for real transcription)"
    };
    info!(target: "pagi::voice", "{}", stt_status);

    let (tts_status, voice_name) = match pagi_voice::OpenRouterTts::from_env() {
        Ok(_t) => {
            let arch = std::env::var("PAGI_PRIMARY_ARCHETYPE").unwrap_or_else(|_| "pisces".into());
            (
                "TTS: [OpenRouter] (Cloud)".to_string(),
                format!("Voice: {}", arch),
            )
        }
        Err(_) => (
            "TTS: [Placeholder] (set TTS_API_KEY for Phoenix to speak)".to_string(),
            String::new(),
        ),
    };
    info!(target: "pagi::voice", "{}", tts_status);
    if !voice_name.is_empty() {
        info!(target: "pagi::voice", "{}", voice_name);
    }
}

/// Spawns the voice session: a thread running the Ear + run_voice_loop, and a task that
/// forwards transcribed prompts to the gateway's chat API and returns the response.
/// When the user interrupts TTS, sends "[Phoenix stopped to listen...]" to `log_tx`.
pub fn start_voice_session(
    log_tx: tokio::sync::broadcast::Sender<String>,
) -> std::thread::JoinHandle<()> {
    let (prompt_tx, mut prompt_rx) = mpsc::channel::<(String, oneshot::Sender<String>)>(32);

    // Task: receive prompts, POST to local chat API, send response back.
    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                warn!(target: "pagi::voice", "Voice responder: reqwest client build failed: {}", e);
                return;
            }
        };
        while let Some((prompt, reply_tx)) = prompt_rx.recv().await {
            let body = serde_json::json!({ "prompt": prompt, "stream": false });
            match client.post(VOICE_CHAT_URL).json(&body).send().await {
                Ok(resp) => {
                    let text = resp
                        .json::<serde_json::Value>()
                        .await
                        .ok()
                        .and_then(|j| j.get("response").and_then(|v| v.as_str()).map(String::from))
                        .unwrap_or_else(|| "I couldn't process that.".to_string());
                    let _ = reply_tx.send(text);
                }
                Err(e) => {
                    warn!(target: "pagi::voice", "Voice chat request failed: {}", e);
                    let _ = reply_tx.send("Sorry, the gateway didn't respond.".to_string());
                }
            }
        }
    });

    let thread_handle = std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(r) => r,
            Err(e) => {
                warn!(target: "pagi::voice", "Voice thread: runtime failed: {}", e);
                return;
            }
        };

        let ear = match pagi_voice::VoiceEar::new(EarConfig::default()).start_listening() {
            Ok(s) => s,
            Err(e) => {
                warn!(target: "pagi::voice", "Ear start failed: {}", e);
                return;
            }
        };
        let voice_output = match VoiceOutput::new() {
            Ok(v) => v,
            Err(e) => {
                warn!(target: "pagi::voice", "VoiceOutput init failed: {}", e);
                return;
            }
        };
        let stt = match create_best_stt() {
            Ok(b) => b,
            Err(e) => {
                warn!(target: "pagi::voice", "STT init failed: {}, using placeholder", e);
                Box::new(pagi_voice::PlaceholderStt::new())
            }
        };
        let tts: Box<dyn TtsBackend> = match OpenRouterTts::from_env() {
            Ok(t) => Box::new(t),
            Err(_) => Box::new(PlaceholderTts),
        };

        let on_interruption: OnInterruption = {
            let tx = log_tx.clone();
            Some(Arc::new(move || {
                let _ = tx.send("[Phoenix stopped to listen…]".to_string());
            }))
        };

        let prompt_tx = prompt_tx.clone();
        let on_text = move |text: String| {
            let tx = prompt_tx.clone();
            async move {
                let (reply_tx, reply_rx) = oneshot::channel();
                if tx.send((text, reply_tx)).await.is_err() {
                    return String::new();
                }
                reply_rx.await.unwrap_or_default()
            }
        };

        info!(target: "pagi::voice", "Voice session started (Go Live). Speak to Phoenix.");
        let result = rt.block_on(run_voice_loop(
            ear,
            &voice_output,
            stt.as_ref(),
            tts.as_ref(),
            on_text,
            on_interruption,
        ));
        if let Err(e) = result {
            warn!(target: "pagi::voice", "Voice loop ended: {}", e);
        }
    });

    thread_handle
}

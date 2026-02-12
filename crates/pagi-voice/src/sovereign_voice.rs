//! **Sovereign Voice** — orchestrate Ear → STT → TTS → Playback with interruption kill-switch.
//!
//! When the Ear reports `VadState::Speech` while TTS is playing, playback is stopped
//! immediately and the system transitions to listening. The kill-switch runs in the same
//! task as playback (VoiceOutput is not Send on some platforms).

use crate::ear::VadState;
use crate::error::VoiceResult;
use crate::stt::SttBackend;
use crate::voice_output::{TtsBackend, VoiceOutput};
use tracing::{info, warn};

/// Optional callback when user speech interrupts TTS playback. Use to e.g. log "[Phoenix stopped to listen...]".
pub type OnInterruption = Option<std::sync::Arc<dyn Fn() + Send + Sync>>;

/// Runs the voice loop: receive turns → transcribe → get response (callback) → TTS → play.
/// When `vad_state_rx` is present, also watches for `VadState::Speech`; if output is playing, calls `voice_output.stop()` (interruption).
///
/// - `on_text`: called with the transcribed string; return the response string to speak (e.g. from Orchestrator/LLM).
/// - `on_interruption`: if set, called when playback is stopped due to user speech (for UI/CLI feedback).
/// - Keep `session.handle` alive. VoiceOutput must be on the same task (not Send).
pub async fn run_voice_loop<F, Fut>(
    session: crate::ear::EarSession,
    voice_output: &VoiceOutput,
    stt: &dyn SttBackend,
    tts: &dyn TtsBackend,
    on_text: F,
    on_interruption: OnInterruption,
) -> VoiceResult<()>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = String>,
{
    let mut handle = session.handle;
    let mut vad_state_rx = session.vad_state_rx;

    loop {
        enum Event {
            Turn(crate::ear::AudioTurn),
            VadState(VadState),
            TurnClosed,
        }

        let event = if let Some(ref mut rx) = vad_state_rx {
            tokio::select! {
                turn = handle.recv_turn() => match turn {
                    Some(t) => Event::Turn(t),
                    None => Event::TurnClosed,
                },
                state = rx.recv() => match state {
                    Some(s) => Event::VadState(s),
                    None => continue,
                },
            }
        } else {
            match handle.recv_turn().await {
                Some(t) => Event::Turn(t),
                None => Event::TurnClosed,
            }
        };

        match event {
            Event::VadState(VadState::Speech) => {
                if voice_output.is_playing() {
                    voice_output.stop();
                    info!("Interruption detected: transitioning to Listening state");
                    if let Some(ref cb) = on_interruption {
                        cb();
                    }
                }
            }
            Event::VadState(_) => {}
            Event::Turn(turn) => {
                let text = match stt.transcribe_turn(&turn) {
                    Ok(t) => t,
                    Err(e) => {
                        warn!("STT failed: {}", e);
                        continue;
                    }
                };
                if text.trim().is_empty() {
                    continue;
                }
                let response = on_text(text).await;
                if response.trim().is_empty() {
                    continue;
                }
                if let Err(e) = voice_output.speak(&response, tts) {
                    warn!("TTS/playback failed: {}", e);
                }
            }
            Event::TurnClosed => break,
        }
    }

    Ok(())
}

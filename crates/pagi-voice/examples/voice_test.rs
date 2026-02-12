//! Example: Voice Orchestrator Test
//!
//! This example demonstrates the voice orchestration system in action.
//! It will listen to your microphone and detect when you speak.

use pagi_voice::{OrchestratorConfig, TurnEvent, VoiceOrchestrator};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🎤 PAGI Voice Orchestrator Test");
    info!("================================");
    info!("");
    info!("This will listen to your microphone and detect speech.");
    info!("Press Ctrl+C to stop.");
    info!("");

    // Create orchestrator with default config
    let config = OrchestratorConfig::default();
    let mut orchestrator = VoiceOrchestrator::new(config)?;

    // Get the turn event receiver
    let mut turn_rx = orchestrator
        .take_turn_receiver()
        .expect("Failed to get turn receiver");

    // Start the orchestrator
    orchestrator.start().await?;

    info!("✅ Listening... speak into your microphone!");
    info!("");

    // Listen for turn events
    while let Some(event) = turn_rx.recv().await {
        match event {
            TurnEvent::SpeechStarted { timestamp } => {
                info!("🎤 Speech started at {}", timestamp);
            }
            TurnEvent::SpeechContinuing { duration } => {
                info!("🎤 Speaking... ({:.1}s)", duration.as_secs_f32());
            }
            TurnEvent::SpeechEnded { timestamp, duration } => {
                info!("🤫 Speech ended at {} (duration: {:.1}s)", timestamp, duration.as_secs_f32());
            }
            TurnEvent::TurnCommitted {
                timestamp,
                total_speech_duration,
                audio_samples,
            } => {
                info!("✅ Turn committed at {}", timestamp);
                info!("   Duration: {:.1}s", total_speech_duration.as_secs_f32());
                info!("   Samples: {}", audio_samples.len());
                info!("");
            }
            TurnEvent::Interruption { timestamp } => {
                info!("⚡ Interruption detected at {}", timestamp);
            }
        }
    }

    // Stop the orchestrator
    orchestrator.stop().await?;

    info!("👋 Goodbye!");

    Ok(())
}

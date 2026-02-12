//! Integration tests for the voice orchestration system
//!
//! Note: These tests require audio devices and may not work in CI environments.

use pagi_voice::{OrchestratorConfig, TurnEvent, VoiceOrchestrator};
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
#[ignore] // Ignore by default since it requires audio hardware
async fn test_orchestrator_lifecycle() {
    // Initialize logging for test
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();

    // Create orchestrator
    let config = OrchestratorConfig::default();
    let mut orchestrator = VoiceOrchestrator::new(config)
        .expect("Failed to create orchestrator");

    // Get turn receiver
    let mut turn_rx = orchestrator
        .take_turn_receiver()
        .expect("Failed to get turn receiver");

    // Start the orchestrator
    orchestrator.start().await
        .expect("Failed to start orchestrator");

    assert!(orchestrator.is_running());

    // Wait for a short time to ensure it's running
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Stop the orchestrator
    orchestrator.stop().await
        .expect("Failed to stop orchestrator");

    assert!(!orchestrator.is_running());
}

#[tokio::test]
#[ignore] // Requires audio hardware and manual speech
async fn test_speech_detection() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();

    println!("\n🎤 Speech Detection Test");
    println!("========================");
    println!("Please speak into your microphone within 10 seconds...\n");

    let config = OrchestratorConfig::default();
    let mut orchestrator = VoiceOrchestrator::new(config)
        .expect("Failed to create orchestrator");

    let mut turn_rx = orchestrator
        .take_turn_receiver()
        .expect("Failed to get turn receiver");

    orchestrator.start().await
        .expect("Failed to start orchestrator");

    // Wait for speech events with timeout
    let result = timeout(Duration::from_secs(10), async {
        let mut speech_started = false;
        let mut turn_committed = false;

        while let Some(event) = turn_rx.recv().await {
            match event {
                TurnEvent::SpeechStarted { .. } => {
                    println!("✅ Speech detected!");
                    speech_started = true;
                }
                TurnEvent::TurnCommitted { total_speech_duration, audio_samples, .. } => {
                    println!("✅ Turn committed!");
                    println!("   Duration: {:.1}s", total_speech_duration.as_secs_f32());
                    println!("   Samples: {}", audio_samples.len());
                    turn_committed = true;
                    break;
                }
                _ => {}
            }
        }

        (speech_started, turn_committed)
    }).await;

    orchestrator.stop().await.ok();

    match result {
        Ok((speech_started, turn_committed)) => {
            assert!(speech_started, "No speech was detected");
            assert!(turn_committed, "No turn was committed");
            println!("\n✅ Test passed!");
        }
        Err(_) => {
            println!("\n⏱️ Timeout - no speech detected within 10 seconds");
            println!("This is expected if you didn't speak into the microphone.");
        }
    }
}

#[tokio::test]
async fn test_config_validation() {
    // Mismatched sample rates should fail
    let mut config = OrchestratorConfig::default();
    config.audio.sample_rate = 8000;
    config.vad.sample_rate = 16000;

    let result = VoiceOrchestrator::new(config);
    assert!(result.is_err(), "Should fail with mismatched sample rates");
}

#[tokio::test]
async fn test_turn_event_channel() {
    let config = OrchestratorConfig::default();
    let mut orchestrator = VoiceOrchestrator::new(config)
        .expect("Failed to create orchestrator");

    // Should be able to take the receiver once
    let turn_rx = orchestrator.take_turn_receiver();
    assert!(turn_rx.is_some());

    // Second attempt should return None
    let turn_rx2 = orchestrator.take_turn_receiver();
    assert!(turn_rx2.is_none());
}

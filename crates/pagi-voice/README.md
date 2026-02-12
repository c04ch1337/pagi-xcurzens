# PAGI Voice - Sovereign Voice Orchestration

**The "Ear" of Phoenix Marie** - Real-time Voice Activity Detection and Turn-Taking on Bare Metal Rust.

## 🎯 Overview

This crate implements the foundational "listening" layer for Phoenix Marie's voice interaction system. Built entirely on bare metal Rust with zero cloud dependencies, it provides:

- **Real-time Voice Activity Detection (VAD)** using Silero VAD (ONNX)
- **Asynchronous Turn-Taking** with 800ms silence detection
- **Interruption Handling** for natural conversation flow
- **Low-Latency Audio I/O** using CPAL (bare metal audio)

## 🏛️ Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                    Voice Orchestrator                            │
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Audio In   │→ │  Silero VAD  │→ │ Turn Manager │          │
│  │    (cpal)    │  │   (ONNX)     │  │  (800ms gap) │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│         ↓                                      ↓                 │
│  ┌──────────────┐                    ┌──────────────┐          │
│  │  Audio Out   │←───────────────────│ Interruption │          │
│  │   (rodio)    │    Kill Signal     │   Handler    │          │
│  └──────────────┘                    └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

## 🎙️ The Sovereign Ear (`VoiceEar`)

For a minimal "listen → commit on gap" API, use **`VoiceEar`**. It captures the default microphone, processes audio in **30ms chunks** (VAD requirement), and yields **`AudioTurn`** each time **800ms** of silence follows speech. All processing is local (WebRTC VAD; no cloud). Buffer is cleared after every commit.

```rust
use pagi_voice::{EarConfig, VoiceEar};

let ear = VoiceEar::new(EarConfig::default());
let session = ear.start_listening()?;
// Keep `session.handle` alive while listening
while let Some(turn) = session.handle.recv_turn().await {
    // turn.samples: Vec<f32>, turn.sample_rate, turn.duration
    println!("Turn: {} samples", turn.samples.len());
}
```

Config: `vad_threshold` (default 0.5), `gap_ms` (default 800), `chunk_size` (480 = 30ms at 16kHz). When `emit_vad_state` is true (default), `session.vad_state_rx` receives real-time Speech/Silence for the interruption kill-switch.

## 🗣️ Sovereign Voice (STT, TTS, Interruption)

- **STT**: `SttBackend` trait and `transcribe_turn(backend, turn)`. **`WhisperStt`** (optional `whisper` feature) for local ggml models; **`OpenRouterStt`** for API transcription; **`PlaceholderStt`** for testing. Use **`create_best_stt()`** to pick the best available backend from env.
- **TTS / Playback**: `VoiceOutput` wraps a `rodio::Sink`. `speak(text, tts)` synthesizes and plays; `stop()` clears the queue (call on interruption). **`OpenRouterTts`** for production (OpenAI-compatible TTS); voice is selected from `PAGI_PRIMARY_ARCHETYPE` (e.g. pisces → shimmer, virgo → nova).
- **Kill-switch**: When the Ear emits `VadState::Speech` and `VoiceOutput::is_playing()`, call `voice_output.stop()`. Use `run_voice_loop(session, &voice_output, stt, tts, on_text)` to run the full loop (turn → STT → callback → TTS → play) with interruption handled in the same task.

### Production backends (OpenRouter / OpenAI-compatible)

| Env var | Purpose |
|--------|--------|
| `TTS_API_URL` | TTS base URL (default `https://api.openai.com/v1`) |
| `TTS_API_KEY` or `PAGI_LLM_API_KEY` | TTS/STT API key |
| `TTS_MODEL` | e.g. `tts-1` or `tts-1-hd` |
| `STT_API_URL` | Transcription base URL |
| `STT_API_KEY` or `PAGI_LLM_API_KEY` | Transcription API key |
| `STT_MODEL` | e.g. `whisper-1` |
| `PAGI_PRIMARY_ARCHETYPE` | Drives TTS voice (pisces→shimmer, virgo→nova, etc.) |

### Local Whisper (Sovereign Autonomy)

To keep transcription on-device (no voice data leaves your machine), use the **`whisper`** feature and set **`WHISPER_MODEL_PATH`** to a quantized ggml model. The **`create_best_stt()`** factory chooses: (1) `WhisperStt` if the path is set and the model loads, (2) `OpenRouterStt` if API keys are set, (3) `PlaceholderStt` otherwise.

**Build with local Whisper:**

```bash
cargo build -p pagi-voice --features whisper
```

Requires **libclang** (and, on some platforms, **CMake**) so that `whisper-rs` can build [whisper.cpp](https://github.com/ggerganov/whisper.cpp). On Windows set `LIBCLANG_PATH` if needed.

**Download quantized models (e.g. English-only):**

- **Hugging Face**: [ggerganov/whisper.cpp](https://huggingface.co/ggerganov/whisper.cpp) — download a `.bin` file (e.g. `ggml-base.en.bin`, `ggml-tiny.en.bin`, or quantized `ggml-base.en-q5_1.bin`).
- **Script** (from whisper.cpp repo): `./models/download-ggml-model.sh base.en` then use the path to the downloaded `.bin`.

Set in `.env`:

```bash
WHISPER_MODEL_PATH=/path/to/ggml-base.en.bin
```

Audio must be **16 kHz mono f32** (the Ear’s default). No resampling is done; use `OpenRouterStt` if your input is different.

Run the full loop with optional production backends:

```bash
cargo run -p pagi-voice --example sovereign_voice_demo
```

With `TTS_API_KEY` and `STT_API_KEY` (or `PAGI_LLM_API_KEY`) set in `.env`, Phoenix uses real TTS and Whisper transcription; otherwise placeholders are used so you can still test the kill-switch and turn flow.

## 🚀 Quick Start

### Basic Usage

```rust
use pagi_voice::{OrchestratorConfig, TurnEvent, VoiceOrchestrator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create orchestrator with default config
    let config = OrchestratorConfig::default();
    let mut orchestrator = VoiceOrchestrator::new(config)?;

    // Get the turn event receiver
    let mut turn_rx = orchestrator
        .take_turn_receiver()
        .expect("Failed to get turn receiver");

    // Start listening
    orchestrator.start().await?;

    // Process turn events
    while let Some(event) = turn_rx.recv().await {
        match event {
            TurnEvent::TurnCommitted { audio_samples, .. } => {
                // Send audio_samples to transcription service
                println!("Got {} samples", audio_samples.len());
            }
            TurnEvent::Interruption { .. } => {
                // User interrupted AI speech
                println!("User interrupted!");
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Running the Example

```bash
# Run the voice test example
cargo run --example voice_test

# This will:
# 1. Initialize the audio system
# 2. Start listening to your microphone
# 3. Detect when you speak
# 4. Commit turns after 800ms of silence
```

## 🔧 Configuration

### Audio Configuration

```rust
use pagi_voice::{AudioConfig, OrchestratorConfig};

let config = OrchestratorConfig {
    audio: AudioConfig {
        sample_rate: 16000,  // 16kHz (required for Silero VAD)
        channels: 1,         // Mono
        buffer_size: 480,    // 30ms chunks at 16kHz
    },
    ..Default::default()
};
```

### VAD Configuration

```rust
use pagi_voice::{VadConfig, OrchestratorConfig};

let config = OrchestratorConfig {
    vad: VadConfig {
        sample_rate: 16000,              // Must match audio config
        threshold: 0.5,                  // 0.0-1.0, higher = more strict
        min_speech_duration_ms: 250,     // Ignore clicks/pops
        min_silence_duration_ms: 100,    // Smoothing
    },
    ..Default::default()
};
```

### Turn Configuration

```rust
use pagi_voice::{TurnConfig, OrchestratorConfig};
use std::time::Duration;

let config = OrchestratorConfig {
    turn: TurnConfig {
        silence_threshold: Duration::from_millis(800),  // The "Gap"
        min_speech_duration: Duration::from_millis(200), // Ignore noise
        max_turn_duration: Duration::from_secs(30),      // Auto-commit
        sample_rate: 16000,
    },
    ..Default::default()
};
```

## 📊 Turn Events

The orchestrator emits the following events:

| Event | Description | Use Case |
|-------|-------------|----------|
| `SpeechStarted` | User began speaking | Update UI, prepare transcription |
| `SpeechContinuing` | User still speaking | Show progress indicator |
| `SpeechEnded` | User stopped speaking | Prepare for turn commit |
| `TurnCommitted` | Turn ready for processing | Send to Whisper/transcription |
| `Interruption` | User interrupted AI | Stop TTS, switch to listening |

## 🎨 Integration with Phoenix Marie

### Stage 1: The "Ear" (Current Implementation)

This crate provides the foundational listening layer:

```rust
// In your Phoenix Marie orchestrator
let mut voice = VoiceOrchestrator::new(config)?;
let mut turn_rx = voice.take_turn_receiver().unwrap();
voice.start().await?;

// Listen for committed turns
tokio::spawn(async move {
    while let Some(TurnEvent::TurnCommitted { audio_samples, .. }) = turn_rx.recv().await {
        // Send to Whisper for transcription
        let text = transcribe_audio(audio_samples).await?;
        
        // Pass to existing archetype system
        let response = get_effective_archetype_for_turn(&text).await?;
        
        // (Stage 2 will add TTS here)
    }
});
```

### Stage 2: The "Voice" (Future)

The next stage will add:
- Whisper integration for transcription
- TTS (Text-to-Speech) for responses
- Full interruption handling with audio playback

## 🛡️ Bare Metal Advantages

### Why This Matters

| Aspect | Cloud-Based | PAGI Voice (Bare Metal) |
|--------|-------------|-------------------------|
| **Latency** | 200-500ms round-trip | <30ms local processing |
| **Privacy** | Audio sent to cloud | All processing local |
| **Cost** | Per-minute API fees | Zero runtime cost |
| **Reliability** | Depends on internet | Works offline |

### The Latency Advantage

```text
Cloud-Based Voice:
[Mic] → [Network] → [Cloud VAD] → [Network] → [Your Code]
        ↑ 100ms ↑              ↑ 100ms ↑
        = 200ms+ total latency

PAGI Voice:
[Mic] → [Local VAD] → [Your Code]
        ↑ <30ms ↑
        = Sub-frame latency
```

## 🧪 Testing

### Unit Tests

```bash
cargo test
```

### Integration Test (Requires Microphone)

```bash
cargo run --example voice_test
```

### Manual Testing Checklist

- [ ] Detects speech start
- [ ] Detects speech end after 800ms silence
- [ ] Ignores background noise
- [ ] Handles interruptions
- [ ] Works with different microphones
- [ ] Handles rapid speech (no premature commits)
- [ ] Handles long pauses (commits correctly)

## 🔍 Troubleshooting

### No Audio Devices Found

```bash
# List available devices
cargo run --example voice_test

# On Windows, ensure microphone permissions are granted
# On Linux, check ALSA/PulseAudio configuration
```

### VAD Not Detecting Speech

- Check microphone volume (should be normalized to -1.0 to 1.0)
- Adjust `vad.threshold` (lower = more sensitive)
- Verify sample rate is 16000 Hz

### High CPU Usage

- Ensure buffer_size is appropriate (480 samples = 30ms at 16kHz)
- Check that VAD processing is not blocking the audio thread
- Monitor with `htop` or Task Manager

## 📚 Dependencies

| Crate | Purpose | Why Bare Metal |
|-------|---------|----------------|
| `cpal` | Audio I/O | Direct hardware access, no OS lag |
| `silero-vad` | Voice detection | Local ONNX inference, no network |
| `rodio` | Audio playback | Native audio output |
| `tokio` | Async runtime | Efficient concurrency |

## 🎯 Future Enhancements

### Stage 2: The "Voice"
- [ ] Whisper integration for transcription
- [ ] TTS integration (Coqui/Piper)
- [ ] Full duplex conversation

### Stage 3: Self-Evolution
- [ ] KB-08 (Soma) integration for communication friction detection
- [ ] Adaptive silence thresholds based on user patterns
- [ ] Emotion detection from voice prosody

## 📖 Related Documentation

- [Silero VAD Documentation](https://github.com/snakers4/silero-vad)
- [CPAL Documentation](https://docs.rs/cpal)
- [Phoenix Marie Architecture](../../docs/architecture.md)

## 🤝 Contributing

This is part of the PAGI (Phoenix Marie) project. See the main README for contribution guidelines.

## 📄 License

See the main project LICENSE file.

---

**Built with 🦀 Rust for maximum performance and sovereignty.**

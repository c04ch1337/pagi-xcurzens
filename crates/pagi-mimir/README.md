# PAGI Mimir — Meeting Documentation Layer

Sovereign meeting capture for Phoenix: **cpal** (mic + optional loopback) → **Whisper-Core** (pagi-voice) → **Chronos** (meetings + transcripts). Keeps SAO/sensitive discussions entirely on-device (bare metal, no cloud).

## Features

- **Audio pipeline**: Default input (mic) + optional loopback (e.g. Windows "Stereo Mix" or virtual cable), mixed and resampled to mono 16 kHz (Whisper standard).
- **Near-live transcription**: Background worker drains the PCM buffer every 15–30s and runs Whisper via `pagi-voice`; emits `TranscriptSegment { speaker_id, text, timestamp }`.
- **Storage**: Uses the same Chronos SQLite DB as the gateway (`data/pagi_chronos/chronos.sqlite`). Tables: `meetings`, `meeting_transcripts`; meetings can be linked to Chronos projects.
- **Summary**: After recording, a `.md` summary is written to the associated project folder (or `data/pagi_chronos/mimir/<meeting_id>.md` if no project).

## Build & run

```bash
# Without local Whisper (placeholder STT only; no libclang):
cargo build -p pagi-mimir

# With local Whisper (requires WHISPER_MODEL_PATH and libclang / LIBCLANG_PATH):
cargo build -p pagi-mimir --features whisper
```

## CLI

```bash
# Help
cargo run -p pagi-mimir --

# Record 30s (default), mic only
cargo run -p pagi-mimir -- --record

# Record 60s, with project and loopback
cargo run -p pagi-mimir -- --record --duration 60 --project "Project: SAO Update" --loopback
```

- **`--record`** — Start recording.
- **`--duration N`** — Recording length in seconds (default 30).
- **`--project "Name"`** — Associate meeting with a Chronos project (by name); summary is written to that project’s folder when associated via `project_associations.json`.
- **`--loopback`** — Also capture system output (enable Stereo Mix or a virtual cable on Windows).

## Environment

- **`PAGI_STORAGE_PATH`** — Base storage path (default `./data`). Chronos DB: `{PAGI_STORAGE_PATH}/pagi_chronos/chronos.sqlite`.
- **`WHISPER_MODEL_PATH`** — Path to ggml Whisper model (e.g. `ggml-base.en.bin`) when built with `--features whisper`. If unset, placeholder STT is used.

## Verification

1. `cargo run -p pagi-mimir -- --record --duration 30`
2. Play a YouTube video or Teams call (or speak into the mic) for the duration.
3. Confirm a `.md` summary appears under `data/pagi_chronos/mimir/` or the associated project folder.

## Next steps (from spec)

- **Pre-flight audio check**: Skill to verify loopback (e.g. Stereo Mix) is active before the meeting.
- **Contextual sweep**: Cross-reference meeting text with KB/project logs (e.g. "PROOFPOINT" → append logs to minutes).
- **Intro-detection**: Map "This is The Creator" / "Hi, this is [Name]" to a local `contacts` table for `speaker_id`.
- **SAO redaction filter**: Regex + keyword list to replace sensitive identifiers with `[REDACTED]` on export unless Sovereign Override.

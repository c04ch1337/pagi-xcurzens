# Mimir-Chronos Integration: Live Meeting Transcripts in Project Sidebar

## Overview

The **Mimir Meeting Documentation Layer** is now fully integrated with **Chronos (KB-04)** to provide live meeting transcripts directly in the Studio UI sidebar. When you click **"Record Meeting"** from the Quick Actions row, the system:

1. Creates a meeting record in the `meetings` table (Mimir storage)
2. Auto-creates a Chronos thread titled **"Meeting: [Date] [Time]"** under the active project
3. Streams Whisper transcript segments into the Chronos `messages` table as they arrive
4. Auto-switches the UI to the meeting thread so you can watch the transcript live
5. On stop: writes a Markdown summary to the project folder and renames the thread to a summarized title (e.g., **"SAO Briefing regarding PROOFPOINT"**)

---

## Architecture

### Backend Flow (Gateway)

```
┌─────────────────────────────────────────────────────────────┐
│                    POST /api/v1/mimir/start                  │
├─────────────────────────────────────────────────────────────┤
│ 1. Run Pre-Flight Audio Check (mic/loopback)                │
│ 2. Create meeting in meetings table (MeetingStorage)        │
│ 3. Create Chronos thread: "Meeting: YYYY-MM-DD HH:MM"       │
│ 4. Spawn MeetingRecorder (cpal: mic + loopback)             │
│ 5. Spawn WhisperTranscriptWorker (15s interval)             │
│ 6. Spawn segment_pump (unbounded channel)                   │
│    → Receives TranscriptSegment from Whisper                │
│    → Appends to Chronos messages table (role: assistant)    │
│    → Metadata: { source: "mimir", meeting_id, timestamp }   │
│ 7. Return { meeting_id, thread_id, project_id, status }     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    POST /api/v1/mimir/stop                   │
├─────────────────────────────────────────────────────────────┤
│ 1. Stop worker (AtomicBool flag)                            │
│ 2. Wait for worker_handle and segment_pump_handle           │
│ 3. List transcripts from meeting_transcripts table          │
│ 4. Build Markdown summary                                   │
│ 5. Write to project folder (or data/mimir fallback)         │
│ 6. End meeting (update meetings.ended_at_ms)                │
│ 7. Rename Chronos thread (heuristic: ALLCAPS token)         │
│ 8. Return { meeting_id, thread_id, summary_path }           │
└─────────────────────────────────────────────────────────────┘
```

### Frontend Flow (Studio UI)

```
┌─────────────────────────────────────────────────────────────┐
│                    EmptyStateGreeting Component              │
├─────────────────────────────────────────────────────────────┤
│ 1. User clicks "Record Meeting" button                      │
│ 2. Determine active project (most recent or PROOFPOINT)     │
│ 3. POST /api/v1/mimir/start { project_id }                  │
│ 4. On success: setMimirRecording(true)                      │
│ 5. Auto-switch to meeting thread: setActiveThread(thread_id)│
│ 6. ChatStore background sync (8s interval) loads messages   │
│    → Transcript segments appear in chat as they arrive      │
└─────────────────────────────────────────────────────────────┘
```

---

## Database Schema

### Mimir Tables (in chronos.sqlite)

```sql
CREATE TABLE meetings (
    id TEXT PRIMARY KEY,
    project_id TEXT NULL,
    title TEXT NOT NULL,
    started_at_ms INTEGER NOT NULL,
    ended_at_ms INTEGER NULL,
    summary_path TEXT NULL,
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL
);

CREATE TABLE meeting_transcripts (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    speaker_id INTEGER NULL,
    text TEXT NOT NULL,
    timestamp_sec REAL NOT NULL,
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY(meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
```

### Chronos Tables (existing)

```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE threads (
    id TEXT PRIMARY KEY,
    project_id TEXT NULL,
    title TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    last_message_at_ms INTEGER NULL,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL,
    project_id TEXT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    metadata_json TEXT NULL,
    FOREIGN KEY(thread_id) REFERENCES threads(id) ON DELETE CASCADE,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE SET NULL
);
```

---

## API Endpoints

### POST /api/v1/mimir/start

**Request:**
```json
{
  "project_id": "uuid-of-active-project"  // optional
}
```

**Response:**
```json
{
  "meeting_id": "uuid",
  "thread_id": "uuid",
  "project_id": "uuid",
  "status": "recording"
}
```

### POST /api/v1/mimir/stop

**Request:** (empty body)

**Response:**
```json
{
  "meeting_id": "uuid",
  "thread_id": "uuid",
  "summary_path": "/path/to/project/meeting_20260211_0830.md"
}
```

### GET /api/v1/mimir/status

**Response:**
```json
{
  "recording": true,
  "meeting_id": "uuid",
  "thread_id": "uuid",
  "project_id": "uuid"
}
```

---

## Key Implementation Details

### 1. Thread Auto-Creation

When [`mimir_start_post()`](add-ons/pagi-gateway/src/mimir.rs:69) is called:

```rust
let thread_title = format!(
    "Meeting: {}",
    chrono::Local::now().format("%Y-%m-%d %H:%M")
);
let thread_id = chronos_db
    .create_thread(&thread_title, project_id)
    .map(|t| t.id)?;
```

### 2. Live Transcript Streaming

The [`WhisperTranscriptWorker`](crates/pagi-mimir/src/whisper_worker.rs:17) emits `TranscriptSegment` events every 15 seconds. The gateway spawns a `segment_pump` task that:

```rust
let (segment_tx, mut segment_rx) = tokio::sync::mpsc::unbounded_channel();

let segment_pump_handle = tokio::spawn(async move {
    while let Some(seg) = segment_rx.recv().await {
        let metadata_json = serde_json::json!({
            "source": "mimir",
            "meeting_id": meeting_id,
            "timestamp_sec": seg.timestamp,
        }).to_string();

        chronos_db.append_message(
            &thread_id,
            project_id.as_deref(),
            "assistant",
            &seg.text,
            Some(&metadata_json),
        );
    }
});
```

### 3. Thread Title Summarization

When [`mimir_stop_post()`](add-ons/pagi-gateway/src/mimir.rs:185) is called, the system uses a heuristic to rename the thread:

```rust
fn derive_meeting_thread_title(transcripts: &[MeetingTranscriptRow]) -> Option<String> {
    // Heuristic 1: Find ALLCAPS tokens (product/acronym names)
    // Example: "PROOFPOINT" → "SAO Briefing regarding PROOFPOINT"
    
    // Heuristic 2: Fallback to first N words
    // Example: "This is Jamey..." → "Meeting: This is Jamey..."
}
```

### 4. UI Auto-Switch

The [`EmptyStateGreeting`](add-ons/pagi-studio-ui/assets/studio-interface/components/ChatInterface.tsx:36) component:

```typescript
const handleRecordMeeting = async () => {
  const body = recentProject ? { project_id: recentProject.id } : {};
  const res = await fetch(`${API_BASE_URL}/mimir/start`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (res.ok) {
    const data = await res.json();
    setMimirRecording(true);
    if (data.thread_id) {
      chatActions.setActiveThread(data.thread_id);
    }
  }
};
```

---

## Verification Steps

### 1. Build with Mimir Feature

```bash
cargo build -p pagi-gateway --features mimir
```

### 2. Start Gateway with Mimir

```bash
# Set WHISPER_MODEL_PATH for real transcription (optional; placeholder STT works for testing)
export WHISPER_MODEL_PATH=/path/to/ggml-base.en.bin

cargo run -p pagi-gateway --features mimir
```

### 3. Start Studio UI

```bash
cd add-ons/pagi-studio-ui/assets/studio-interface
npm run dev -- --port 3000
```

### 4. Test Flow

1. Open Studio UI at `http://localhost:3000`
2. Create or select a project (e.g., "Project: PROOFPOINT")
3. Click **"Record Meeting"** from the Quick Actions row
4. Verify:
   - Button shows "Recording…" with red pulse
   - Sidebar auto-switches to a new thread titled "Meeting: YYYY-MM-DD HH:MM"
   - Speak into the mic (or play audio if loopback is enabled)
   - After 15-30 seconds, transcript segments appear in the chat
5. Click **"Stop Recording"** (or wait for auto-stop)
6. Verify:
   - Thread title updates to summarized version (e.g., "SAO Briefing regarding PROOFPOINT")
   - Markdown summary is written to project folder
   - Full transcript is visible in the sidebar

---

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `WHISPER_MODEL_PATH` | Path to ggml Whisper model (e.g., `ggml-base.en.bin`) | Placeholder STT (no real transcription) |
| `PAGI_STORAGE_PATH` | Base storage path for Chronos DB | `./data` |

---

## Troubleshooting

### Issue: "Pre-flight failed: no default input (microphone) detected"

**Solution:**
- Ensure a microphone is connected and set as the default input device
- On Windows: Settings → Sound → Input → Select microphone
- Run `GET /api/v1/mimir/preflight` to see detected devices

### Issue: Transcript segments not appearing in sidebar

**Solution:**
- Check gateway logs for `pagi::mimir` target
- Verify `WHISPER_MODEL_PATH` is set (or accept placeholder STT for testing)
- Ensure ChatStore background sync is running (8s interval)
- Manually refresh: click away from the thread and back

### Issue: Thread title not updating after stop

**Solution:**
- Check gateway logs for `derive_meeting_thread_title` warnings
- Ensure transcripts contain text (not empty segments)
- Heuristic requires at least one ALLCAPS token or 8+ words

---

## Future Enhancements (from spec)

- **Contextual sweep**: Cross-reference meeting text with KB/project logs (e.g., "PROOFPOINT" → append logs to minutes)
- **Intro-detection**: Map "This is Jamey" / "Hi, this is [Name]" to a local `contacts` table for `speaker_id`
- **SAO redaction filter**: Regex + keyword list to replace sensitive identifiers with `[REDACTED]` on export unless Sovereign Override
- **LLM-based summarization**: Use ModelRouter to generate a professional title and executive summary (optional; current heuristic is offline-safe)

---

## File References

- **Gateway Handler**: [`add-ons/pagi-gateway/src/mimir.rs`](add-ons/pagi-gateway/src/mimir.rs:1)
- **Mimir Crate**: [`crates/pagi-mimir/src/lib.rs`](crates/pagi-mimir/src/lib.rs:1)
- **Chronos SQLite**: [`add-ons/pagi-gateway/src/chronos_sqlite.rs`](add-ons/pagi-gateway/src/chronos_sqlite.rs:1)
- **UI Component**: [`add-ons/pagi-studio-ui/assets/studio-interface/components/ChatInterface.tsx`](add-ons/pagi-studio-ui/assets/studio-interface/components/ChatInterface.tsx:36)
- **ChatStore**: [`add-ons/pagi-studio-ui/assets/studio-interface/src/stores/ChatStore.tsx`](add-ons/pagi-studio-ui/assets/studio-interface/src/stores/ChatStore.tsx:1)

---

## Strategic Result: The "Sovereign Audit"

You now have a system where you can:

1. Open your **"Sovereign Core"** (Studio UI)
2. Click **"Resume: PROOFPOINT"** (or any project)
3. Click **"Record Meeting"**
4. Watch the transcript appear live in the sidebar as you speak
5. At the end of the day, find a perfectly documented Markdown summary and a full chat-history audit trail, all stored on your **Bare Metal** hardware

**No cloud. No leaks. Full sovereignty.**

# Knowledge Router Implementation Guide

## Quick Start

### 1. Build with Voice + Dashboard Features

```bash
# Build with voice and TUI dashboard
cargo build --release --features voice,tui-dashboard

# Or run directly
cargo run --features voice,tui-dashboard -- --live --dashboard
```

### 2. Environment Setup

Ensure these environment variables are set:

```bash
# Required for Live Mode
OPENROUTER_API_KEY=your_key_here
PAGI_LLM_MODEL=anthropic/claude-3.5-sonnet

# Optional: Shadow Vault (KB-09)
PAGI_SHADOW_KEY=64_hex_characters_here

# Optional: TTS Voice
PAGI_TTS_VOICE=alloy
```

### 3. Launch Live Mode

```bash
# Standard live mode (no dashboard)
./target/release/pagi-gateway --live

# With real-time KB monitoring dashboard
./target/release/pagi-gateway --live --dashboard
```

## Architecture Overview

### The "Pseudo-Live" Pipeline

```
┌─────────────────────────────────────────────────────────────┐
│                    PHOENIX LIVE MODE                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  🎤 Voice Input                                              │
│    ↓                                                         │
│  🔊 VAD (Voice Activity Detection)                           │
│    ↓                                                         │
│  📝 STT (Whisper via OpenRouter)                            │
│    ↓                                                         │
│  🧠 LLM Streaming (Claude/GPT)                              │
│    ├─→ [KB Query Detection]                                 │
│    │     ↓                                                   │
│    │   🗄️ KnowledgeRouter.query_kb()                        │
│    │     ↓                                                   │
│    │   [Inject KB Data into Stream]                         │
│    │                                                         │
│    ↓                                                         │
│  📊 Sentence Boundary Detection                             │
│    ↓                                                         │
│  🔊 TTS (OpenRouter)                                        │
│    ↓                                                         │
│  🔈 Audio Playback (rodio)                                  │
│    ↓                                                         │
│  🛑 Interruption Detection (VAD → Stop)                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## KB Slot Reference

| Slot | Name | Query Trigger | Example |
|------|------|---------------|---------|
| 1 | Pneuma (Identity) | User identity, preferences | "Who am I?", "What are my values?" |
| 2 | Oikos (Tasks) | Task management | "What should I focus on?", "Show my tasks" |
| 3 | Kardia (Relations) | Social dynamics | "Tell me about Sarah", "Who can I trust?" |
| 4 | Chronos (Time) | Calendar, deadlines | "What's on my schedule?", "When is X due?" |
| 5 | Techne (Protocols) | Security, boundaries | "Am I being manipulated?", "Sovereignty check" |
| 6 | Ethos (Philosophy) | Moral framework | "What should I do?", "Is this ethical?" |
| 7 | Soma (Physical) | Health, vitality | "How am I feeling?", "Check my sleep" |
| 8 | Absurdity Log | Self-audit | "What mistakes have I made?", "Show patterns" |
| 9 | Shadow (Encrypted) | Emotional anchors | "What's stressing me?", "Grief check" |

## How KB Queries Work

### 1. LLM Detects Need for Context

When Phoenix needs specific information, she internally thinks:

```
"I need to query KB-07 for physical_state"
```

### 2. System Detects Query Pattern

The regex pattern matches:

```rust
Regex::new(r"I need to query KB-(\d+) for (\w+)")
```

### 3. KnowledgeRouter Executes Query

```rust
let request = KbQueryRequest {
    slot_id: 7,
    key: None,
    intent: "physical_state".to_string(),
};

let response = kb_router.query_kb(request).await;
```

### 4. Data Injected into Stream

```
[KB-07 Retrieved: Physical State:
 - Sleep: 6.5h (target: 8.0h)
 - Readiness: 72%
 - Heart Rate: 68 bpm
 - HRV: 45 ms
 - Activity: 30 min]
```

### 5. Phoenix Continues with Context

```
"You got 6.5 hours of sleep last night with a readiness score of 72%. 
Your HRV is slightly below baseline. Consider a lighter workload today."
```

## Dashboard Controls

### Keyboard Shortcuts

- **`q`** - Quit dashboard
- **`r`** - Refresh display (auto-refreshes every 250ms)

### Dashboard Sections

1. **Header** - Status and instructions
2. **Status Bar** - Streaming and voice activity indicators
3. **KB Slot Activity** - Bar chart of KB access counts
4. **KB Access Log** - Recent 10 queries with timestamps
5. **Conversation** - Last user input and Phoenix response

### Status Indicators

- 🟢 **STREAMING** - LLM is generating response
- ⚪ **IDLE** - Waiting for user input
- 🔴 **SPEAKING** - TTS audio is playing
- ⚪ **SILENT** - No audio output

## Code Structure

### Core Files

```
add-ons/pagi-gateway/src/
├── knowledge_router.rs       # KB query orchestration
│   ├── KnowledgeRouter       # Main router struct
│   ├── KbQueryRequest        # Query request format
│   ├── KbQueryResponse       # Query response format
│   └── KbAccessLog           # Access logging
│
├── openrouter_live.rs        # Live mode session
│   ├── OpenRouterLiveSession # Main session struct
│   ├── detect_kb_query()     # Regex pattern matching
│   └── process_streaming_chat() # SSE stream handler
│
├── live_dashboard.rs         # TUI monitoring
│   ├── DashboardState        # Shared state
│   ├── run_dashboard()       # Main TUI loop
│   └── ui()                  # Rendering logic
│
└── main.rs                   # Entry point
```

### Key Traits

```rust
// KnowledgeRouter trait (conceptual)
pub trait KbRouter {
    async fn query_kb(&self, request: KbQueryRequest) -> KbQueryResponse;
    fn get_access_log(&self) -> Vec<KbAccessLog>;
}

// Each KB slot has a specialized query method
impl KnowledgeRouter {
    async fn query_pneuma(&self, key: Option<String>) -> Result<String, String>;
    async fn query_oikos(&self, key: Option<String>) -> Result<String, String>;
    async fn query_kardia(&self, key: Option<String>) -> Result<String, String>;
    // ... etc for all 9 slots
}
```

## Performance Characteristics

### Token Efficiency

| Approach | Tokens per Request | Notes |
|----------|-------------------|-------|
| **Static Injection** | ~4,000 | All 8 KBs loaded upfront |
| **Dynamic Selection** | ~200-500 | Only query what's needed |
| **Savings** | **80-90%** | Massive reduction in overhead |

### Latency

| Operation | Latency | Notes |
|-----------|---------|-------|
| KB Query | ~10-50ms | Local sled database |
| Stream Pause/Resume | ~5ms | Minimal interruption |
| TTS Generation | ~500-1000ms | OpenRouter API call |
| Audio Playback | ~0ms | Non-blocking (rodio) |

### Memory Usage

| Component | Memory | Notes |
|-----------|--------|-------|
| KnowledgeStore | ~50MB | All 8 KB slots in memory |
| Access Log | ~1MB | Last 100 entries |
| Dashboard State | ~500KB | Real-time metrics |
| Audio Buffer | ~10MB | TTS audio chunks |

## Debugging

### Enable Verbose Logging

```bash
RUST_LOG=pagi::kb_router=debug,pagi::voice=debug cargo run --features voice -- --live
```

### Check KB Access Patterns

```rust
// In code
let access_log = session.get_kb_access_log();
for entry in access_log {
    println!("[{}] KB-{} {} ({})", 
        entry.timestamp_ms, 
        entry.slot_id, 
        entry.slot_name, 
        entry.intent
    );
}
```

### Monitor Streaming

```bash
# Watch for KB query patterns in logs
tail -f pagi-gateway.log | grep "KB query"
```

## Common Issues

### Issue: KB Query Not Detected

**Symptom:** Phoenix doesn't retrieve KB data when expected

**Solution:**
1. Check regex pattern in logs
2. Ensure LLM is using exact format: `"I need to query KB-X for Y"`
3. Verify system prompt includes KB query instructions

### Issue: Dashboard Not Showing

**Symptom:** `--dashboard` flag doesn't launch TUI

**Solution:**
1. Rebuild with `tui-dashboard` feature: `cargo build --features tui-dashboard`
2. Check terminal supports ANSI colors
3. Try running in different terminal (Windows Terminal, iTerm2, etc.)

### Issue: Shadow Vault Locked

**Symptom:** KB-09 queries fail with "vault is locked"

**Solution:**
1. Set `PAGI_SHADOW_KEY` environment variable
2. Generate key: `openssl rand -hex 32`
3. Verify key is 64 hex characters (32 bytes)

## Testing

### Manual Test Cases

1. **Identity Query**
   - Say: "Who am I?"
   - Expected: KB-01 query → User profile retrieved

2. **Physical State Query**
   - Say: "How am I feeling?"
   - Expected: KB-07 query → Soma state retrieved

3. **Task Query**
   - Say: "What should I focus on?"
   - Expected: KB-02 query → Governed tasks retrieved

4. **Relationship Query**
   - Say: "Tell me about [person]"
   - Expected: KB-03 query → Subject profile retrieved

### Automated Tests

```bash
# Run unit tests
cargo test --features voice -- knowledge_router

# Run integration tests
cargo test --features voice -- --test live_mode_integration
```

## Future Enhancements

### 1. Function Calling (Native LLM Support)

Replace regex detection with OpenAI/Anthropic function calling:

```json
{
  "name": "query_kb",
  "description": "Query a specific knowledge base slot",
  "parameters": {
    "slot_id": { "type": "integer", "enum": [1,2,3,4,5,6,7,8,9] },
    "intent": { "type": "string" }
  }
}
```

### 2. Predictive Caching

- Analyze access patterns
- Pre-load frequently queried KBs
- Cache results for common queries

### 3. Multi-KB Queries

- Query multiple KBs in parallel
- Combine results intelligently
- Example: "Check my tasks (KB-02) and energy (KB-07) to recommend priorities"

### 4. Semantic Search

- Vector embeddings for KB content
- Similarity matching within KBs
- RAG (Retrieval-Augmented Generation) integration

### 5. Dashboard Enhancements

- Historical access patterns (graphs)
- KB performance metrics (latency, cache hit rate)
- Export logs for analysis

## Contributing

### Adding a New KB Slot

1. **Update `KnowledgeRouter`:**
   ```rust
   async fn query_new_slot(&self, key: Option<String>) -> Result<String, String> {
       // Implementation
   }
   ```

2. **Update `slot_name()` mapping:**
   ```rust
   10 => "NewSlot (Purpose)".to_string(),
   ```

3. **Update documentation:**
   - Add to KB Slot Reference table
   - Update system prompt instructions
   - Add test cases

### Improving Query Detection

1. **Enhance regex pattern:**
   ```rust
   // Support multiple formats
   let patterns = vec![
       r"I need to query KB-(\d+) for (\w+)",
       r"Query KB-(\d+): (\w+)",
       r"Retrieve from KB-(\d+) \((\w+)\)",
   ];
   ```

2. **Add semantic detection:**
   ```rust
   // Detect intent from context
   if text.contains("physical state") || text.contains("how am I feeling") {
       return Some(KbQueryRequest { slot_id: 7, ... });
   }
   ```

## License

This implementation is part of the PAGI (Phoenix AGI) project.

## Support

For issues or questions:
- Check [`DYNAMIC_KB_SELECTION.md`](DYNAMIC_KB_SELECTION.md) for architecture details
- Review logs with `RUST_LOG=debug`
- Open an issue on the project repository

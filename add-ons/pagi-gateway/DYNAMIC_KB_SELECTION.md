# Dynamic Knowledge Base Selection for Live Mode

## Overview

The Dynamic KB Selection system transforms Phoenix from a **static context injection** model to an **on-demand retrieval** architecture. Instead of loading all 8 KB slots into the system prompt upfront (causing token bloat), Phoenix now queries specific KBs only when needed during streaming conversations.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    LIVE MODE FLOW                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  User Speech → VAD → STT (Whisper)                          │
│       ↓                                                      │
│  OpenRouter SSE Stream (Claude/GPT)                         │
│       ↓                                                      │
│  [KB Query Detection] ← Regex: "I need to query KB-X"      │
│       ↓                                                      │
│  KnowledgeRouter.query_kb(slot_id, intent)                 │
│       ↓                                                      │
│  [Pause Stream] → Retrieve Data → [Resume Stream]          │
│       ↓                                                      │
│  Sentence Boundary Detection (., !, ?)                      │
│       ↓                                                      │
│  TTS (OpenRouter) → Audio Playback (rodio)                 │
│       ↓                                                      │
│  [Interruption Detection] → VadState::Speech → Stop()       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## KB Slot Mapping (8-Layer Architecture)

| Slot | Name | Purpose | Query Trigger |
|------|------|---------|---------------|
| **1** | **Pneuma (Identity)** | User profile, archetype, sovereignty leaks | User asks "who am I", mentions preferences |
| **2** | **Oikos (Tasks)** | Governed tasks, operational boundaries | Task management, scheduling, priorities |
| **3** | **Kardia (Relationships)** | Social graph, trust scores, attachment | Mentions people, relationships, social dynamics |
| **4** | **Chronos (Time)** | Calendar, reminders, temporal tracking | Time-based queries, scheduling, deadlines |
| **5** | **Techne (Protocols)** | Security protocols, sovereignty defense | Boundary violations, manipulation detection |
| **6** | **Ethos (Philosophy)** | Philosophical lens, moral framework | Ethical questions, value alignment |
| **7** | **Soma (Physical)** | Biometrics, sleep, vitality | Physical state, health, energy levels |
| **8** | **Absurdity Log** | Success metrics, logic inconsistencies | Self-audit, pattern analysis, learning |
| **9** | **Shadow (Encrypted)** | Emotional anchors, trauma, private notes | High-stress, grief, burnout indicators |

## How It Works

### 1. System Prompt Instructions

Phoenix receives instructions on how to query KBs:

```
=== KNOWLEDGE BASE QUERY SYSTEM ===

You have access to 8 specialized knowledge bases (KB-01 through KB-09). 
Instead of having all context upfront, you can query specific KBs when needed.

**How to Query:**
When you need specific context, think: "Which KB slot contains this information?"
Then request it by saying: "I need to query KB-[slot_id] for [intent]"

Example: "I need to query KB-07 for physical_state"
```

### 2. Query Detection (Regex)

The system monitors the LLM's streaming output for KB query patterns:

```rust
let kb_query_regex = Regex::new(r"I need to query KB-(\d+) for (\w+)")?;
```

When detected:
- **Pause streaming** (buffer current sentence)
- **Execute KB query** via `KnowledgeRouter`
- **Inject result** into the stream
- **Resume streaming** with enriched context

### 3. KnowledgeRouter

The router handles all KB operations:

```rust
pub struct KnowledgeRouter {
    knowledge: Arc<KnowledgeStore>,
    access_log: Mutex<Vec<KbAccessLog>>,
}

impl KnowledgeRouter {
    pub async fn query_kb(&self, request: KbQueryRequest) -> KbQueryResponse {
        // Route to appropriate KB handler
        match request.slot_id {
            1 => self.query_pneuma(request.key).await,
            2 => self.query_oikos(request.key).await,
            // ... etc
        }
    }
}
```

### 4. Access Logging

Every KB query is logged for:
- **Real-time monitoring** (TUI dashboard)
- **Pattern analysis** (which KBs are most used)
- **Performance optimization** (cache frequently accessed data)

```rust
pub struct KbAccessLog {
    pub timestamp_ms: i64,
    pub slot_id: u8,
    pub slot_name: String,
    pub intent: String,
    pub success: bool,
}
```

## TUI Dashboard

Launch with `--live --dashboard` to see real-time KB activity:

```
┌─────────────────────────────────────────────────────────────┐
│ 🎙️ PHOENIX LIVE MODE DASHBOARD                              │
│ Press 'q' to quit | Real-time KB monitoring                 │
├─────────────────────────────────────────────────────────────┤
│ Status                                                       │
│ Streaming: 🟢 STREAMING  |  Voice: 🔴 SPEAKING             │
├─────────────────────────────────────────────────────────────┤
│ KB Slot Activity                                            │
│ KB-01 Pneuma (Identity)      12 ████████████                │
│ KB-02 Oikos (Tasks)            3 ███                        │
│ KB-03 Kardia (Relations)       8 ████████                   │
│ KB-04 Chronos (Time)           1 █                          │
│ KB-05 Techne (Protocols)       0                            │
│ KB-06 Ethos (Philosophy)       2 ██                         │
│ KB-07 Soma (Physical)          5 █████                      │
│ KB-08 Absurdity Log            1 █                          │
│ KB-09 Shadow (Encrypted)       0                            │
├─────────────────────────────────────────────────────────────┤
│ KB Access Log (Recent 10)                                   │
│ [20:45:32] ✓ KB-07 Soma (Physical)      (physical_state)   │
│ [20:45:15] ✓ KB-03 Kardia (Relations)   (relationship)      │
│ [20:44:58] ✓ KB-01 Pneuma (Identity)    (user_identity)    │
├─────────────────────────────────────────────────────────────┤
│ Conversation                                                │
│ 👤 User: How am I feeling today?                            │
│                                                              │
│ 🤖 Phoenix: Let me check your physical state...             │
└─────────────────────────────────────────────────────────────┘
```

## Benefits

### 1. **Token Efficiency**
- **Before:** 8 KB slots × ~500 tokens each = 4,000 tokens per request
- **After:** Only query what's needed = ~200-500 tokens per request
- **Savings:** 80-90% reduction in context overhead

### 2. **Faster Response Times**
- No need to wait for full context injection
- Streaming starts immediately
- KB queries happen in parallel with generation

### 3. **Better Privacy**
- KB-09 (Shadow) only accessed when emotional context is needed
- Reduces exposure of sensitive data to LLM
- Encrypted data stays encrypted until explicitly requested

### 4. **Scalability**
- Can add more KB slots without bloating system prompt
- Each KB can grow independently
- Supports future "KB-10, KB-11..." without refactoring

### 5. **Observability**
- Real-time dashboard shows which KBs are being used
- Access patterns reveal user behavior
- Can optimize caching based on usage

## Usage Examples

### Example 1: Physical State Query

**User:** "How am I feeling today?"

**Phoenix (internal):** "I need to query KB-07 for physical_state"

**System:** 
1. Detects query pattern
2. Calls `KnowledgeRouter.query_kb(7, "physical_state")`
3. Retrieves: `SomaState { sleep_hours: 6.5, readiness: 72, ... }`
4. Injects into stream

**Phoenix (response):** "You got 6.5 hours of sleep last night with a readiness score of 72%. Your HRV is 45ms, which is slightly below your baseline. Consider a lighter workload today."

### Example 2: Relationship Context

**User:** "Tell me about Sarah."

**Phoenix (internal):** "I need to query KB-03 for relationship_context"

**System:**
1. Queries Kardia (KB-03) for "subjects/sarah"
2. Retrieves: `SubjectProfile { trust_score: 0.85, attachment: High, ... }`
3. Injects into stream

**Phoenix (response):** "Sarah is someone you have a high-trust relationship with (trust score: 0.85). You last interacted 3 days ago. She's been supportive during your recent project stress."

### Example 3: Task Management

**User:** "What should I focus on today?"

**Phoenix (internal):** "I need to query KB-02 for task_priorities"

**System:**
1. Queries Oikos (KB-02) for governed tasks
2. Retrieves: `[Task { priority: 9, title: "Finish report", ... }]`
3. Cross-references with KB-07 (Soma) for energy levels
4. Injects into stream

**Phoenix (response):** "Your top priority is finishing the report (priority: 9). Given your readiness score of 72%, I recommend tackling this in the morning when your energy is highest."

## Implementation Details

### File Structure

```
add-ons/pagi-gateway/src/
├── knowledge_router.rs       # KB query orchestration
├── openrouter_live.rs        # Streaming chat with KB integration
├── live_dashboard.rs         # TUI monitoring (ratatui)
└── main.rs                   # Entry point
```

### Key Components

1. **`KnowledgeRouter`** ([`knowledge_router.rs`](knowledge_router.rs:1))
   - Handles all KB queries
   - Logs access patterns
   - Provides system prompt instructions

2. **`OpenRouterLiveSession`** ([`openrouter_live.rs`](openrouter_live.rs:22))
   - Integrates KB router into streaming
   - Detects query patterns via regex
   - Pauses/resumes stream for KB retrieval

3. **`DashboardState`** ([`live_dashboard.rs`](live_dashboard.rs:28))
   - Shared state between live session and TUI
   - Tracks KB access, streaming status, voice activity
   - Updates in real-time

### Dependencies

```toml
[dependencies]
regex = "1"                                    # KB query pattern detection
ratatui = { version = "0.28", optional = true } # TUI dashboard
crossterm = { version = "0.28", optional = true } # Terminal control

[features]
voice = ["dep:pagi-voice", "dep:reqwest"]
tui-dashboard = ["dep:ratatui", "dep:crossterm"]
```

## Future Enhancements

### 1. **Function Calling (OpenAI/Anthropic)**
Instead of regex detection, use native function calling:

```json
{
  "name": "query_kb",
  "description": "Query a specific knowledge base slot",
  "parameters": {
    "slot_id": { "type": "integer", "minimum": 1, "maximum": 9 },
    "intent": { "type": "string" }
  }
}
```

### 2. **Predictive Caching**
- Analyze access patterns
- Pre-load frequently queried KBs
- Cache results for common queries

### 3. **Multi-KB Queries**
- Query multiple KBs in parallel
- Combine results intelligently
- Example: "Check my tasks (KB-02) and energy (KB-07) to recommend priorities"

### 4. **KB Query Optimization**
- Semantic search within KBs
- Vector embeddings for similarity matching
- RAG (Retrieval-Augmented Generation) integration

### 5. **Dashboard Enhancements**
- Historical access patterns (graphs)
- KB performance metrics (latency, cache hit rate)
- Export logs for analysis

## Testing

### Manual Testing

1. **Start Live Mode:**
   ```bash
   cargo run --features voice -- --live
   ```

2. **Start with Dashboard:**
   ```bash
   cargo run --features voice,tui-dashboard -- --live --dashboard
   ```

3. **Test KB Queries:**
   - Say: "How am I feeling?" → Should query KB-07 (Soma)
   - Say: "What are my tasks?" → Should query KB-02 (Oikos)
   - Say: "Tell me about [person]" → Should query KB-03 (Kardia)

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_kb_query_detection() {
        let router = KnowledgeRouter::new(knowledge_store);
        let text = "I need to query KB-07 for physical_state";
        let request = detect_kb_query(text).unwrap();
        
        assert_eq!(request.slot_id, 7);
        assert_eq!(request.intent, "physical_state");
    }

    #[tokio::test]
    async fn test_soma_query() {
        let router = KnowledgeRouter::new(knowledge_store);
        let response = router.query_kb(KbQueryRequest {
            slot_id: 7,
            key: None,
            intent: "physical_state".to_string(),
        }).await;
        
        assert!(response.success);
        assert!(response.data.contains("Sleep"));
    }
}
```

## Conclusion

The Dynamic KB Selection system represents a **paradigm shift** in how Phoenix accesses memory:

- **From:** "Load everything upfront" (static, bloated)
- **To:** "Query what you need, when you need it" (dynamic, efficient)

This architecture enables Phoenix to scale to **dozens of KB slots** without performance degradation, while maintaining **real-time responsiveness** and **privacy-first** data handling.

The TUI dashboard provides unprecedented **observability** into Phoenix's cognitive processes, allowing users to see exactly which knowledge bases are being accessed and why.

This is the foundation for a truly **sovereign AI** that thinks in flows, not blocks.

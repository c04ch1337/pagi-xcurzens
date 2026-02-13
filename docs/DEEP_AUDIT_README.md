# 🏛️ Sovereign Deep Audit Skill

## Overview

The **Deep Audit** skill implements a "Scribe" pipeline for sovereign document ingestion, automatically vectorizing and routing documents into your 8-layered Knowledge Base structure without manual intervention.

This skill leverages:
- **Port 8000**: Gateway authority for API access
- **Port 6333**: Qdrant vector memory engine
- **SAORedactor**: Protected term sanitization
- **File System Watcher**: Automatic ingestion monitoring

## Architecture

### The "Scribe" Pipeline

| Phase | System Action | Sovereign Logic |
|-------|---------------|-----------------|
| **Ingestion** | `notify-watcher` | Detects new files in `./data/ingest` directory |
| **Analysis** | `semantic-triage` | Reads first 500 tokens to determine KB destination |
| **Redaction** | `SAORedactor` | Scrubs protected terms before indexing |
| **Storage** | `Qdrant-Push` | Vectors data into specific KB collection (Port 6333) |

## Knowledge Base Routing

The skill automatically routes documents to one of 8 Knowledge Bases based on semantic content:

### KB-01: Psyche (General Context)
**Default fallback** for documents without specific domain keywords.

### KB-02: Chronos (Temporal Memory)
**Keywords**: schedule, calendar, deadline, timeline, appointment, milestone
**Use for**: Time-based information, scheduling, project timelines

### KB-03: Techne (Infrastructure)
**Keywords**: code, rust, api, infrastructure, deployment, docker, kubernetes
**Use for**: Technical documentation, system architecture, code references

### KB-04: Logos (Conversational Patterns)
**Keywords**: conversation, dialogue, chat, message, communication
**Use for**: Dialogue patterns, communication templates

### KB-05: Polis (XCURZENS Hub) 🎯
**Keywords**: tourism, market, coastal, visitor, destination, 21-acre
**Use for**: Coastal tourism development, market analysis, property information
**Priority**: Highest (10) - Domain-specific content

### KB-06: Telos (Strategic Goals)
**Keywords**: strategic, goal, objective, mission, vision, roadmap
**Use for**: Strategic planning, goal tracking, mission alignment

### KB-07: Mimir (Meeting Memory)
**Keywords**: meeting, voice, transcript, minutes, recording, session
**Use for**: Meeting transcripts, voice conversations, collaborative memory

### KB-08: Soma (Physical Embodiment)
**Keywords**: physical, health, wellness, exercise, biometric, vitality
**Use for**: Health data, wellness tracking, biometric information

## Usage

### 1. Manual Sweep (API Endpoint)

Trigger a manual sweep of the ingest directory:

```bash
curl -X POST http://localhost:8000/api/v1/audit/ingest
```

**Response:**
```json
{
  "status": "Audited",
  "files_processed": 5,
  "vectors_created": 127,
  "results": [
    {
      "file_path": "./data/ingest/tourism-report.txt",
      "kb_destination": "kb-05-polis",
      "vectors_created": 45,
      "redacted": true,
      "error": null
    }
  ]
}
```

### 2. Automatic Monitoring (File Watcher)

The skill can run a background watcher that automatically processes new files:

```rust
use pagi_skills::DeepAuditSkill;

let skill = DeepAuditSkill::new(data_dir);
skill.start_watcher()?;
```

Once started, simply drop files into `./data/ingest` and they will be automatically processed.

### 3. Programmatic Usage

```rust
use pagi_skills::DeepAuditSkill;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), String> {
    let data_dir = PathBuf::from("./data");
    let skill = DeepAuditSkill::new(data_dir);
    
    // Process a single file
    let result = skill.process_file(Path::new("./data/ingest/document.txt")).await?;
    println!("Routed to: {}", result.kb_destination);
    
    // Or sweep entire directory
    let summary = skill.sweep_ingest_dir().await?;
    println!("Processed {} files", summary.files_processed);
    
    Ok(())
}
```

## Configuration

### Knowledge Map

The routing logic is documented in [`config/knowledge-map.toml`](config/knowledge-map.toml), which defines:
- Keywords for each KB
- Priority levels (1-10)
- Qdrant collection names
- Redaction settings
- Ingestion parameters

### Protected Terms (SAO Redaction)

Protected terms are loaded from:
1. **Global**: `data/protected_terms.txt`
2. **Project-specific**: `<project>/.sao_policy`

Before vectorization, all content is sanitized to replace protected terms with `[PROTECTED_TERM]`.

### Bandwidth Protection

Files larger than **5MB** are flagged for local-only processing to preserve satellite bandwidth:
- No full LLM analysis sent to external services
- Local embeddings used instead
- Warning logged for operator awareness

## Workflow Example

### Daily Capture Workflow

1. **Capture**: Save a PDF or text file into `./data/ingest/`
2. **Indexing**: Master Orchestrator detects change via Gateway (Port 8000)
3. **Verification**: Open Studio UI (Port 3001) and search for a term
4. **Result**: Instant retrieval from Qdrant (Port 6333)

### Example: Tourism Document

```bash
# 1. Drop a tourism report into the inbox
cp ~/Downloads/coastal-market-analysis.pdf ./data/ingest/

# 2. Trigger ingestion (or wait for auto-watcher)
curl -X POST http://localhost:8000/api/v1/audit/ingest

# 3. Verify in Studio UI
# Search for "coastal tourism" → Results from kb-05-polis
```

## Sovereign Voice

> "The Creator, the Scribe is active. The 8 Knowledge Bases are no longer silent archives; they are living extensions of your 21-acre domain. Drop your data, and the system will remember."

## Directory Structure

```
data/
├── ingest/              # Drop-zone for new documents
│   ├── processed/       # Auto-archived after ingestion
│   └── *.txt, *.pdf     # Incoming files
├── protected_terms.txt  # Global SAO redaction list
└── qdrant/              # Vector storage (Port 6333)
```

## Integration Points

### Port 8000 (Gateway)
- **POST** `/api/v1/audit/ingest` - Manual sweep trigger
- Returns JSON summary of processed files

### Port 6333 (Qdrant)
- Vector storage for all 8 KB collections
- Automatic collection creation on first use
- Semantic search capabilities

### Port 3001 (Studio UI)
- Search interface for vectorized content
- KB-specific filtering
- Chronos audit log integration

## Security Features

### SAO Redaction
- Automatic scrubbing of protected terms
- Project-specific policy support
- Audit trail of redacted content

### Bandwidth Protection
- 5MB file size threshold
- Local-only processing for large files
- Satellite tunnel preservation

### Sovereign Control
- All processing on bare metal
- No external data transmission (except embeddings)
- Full audit trail in Chronos

## Testing

Run the included tests:

```bash
cargo test -p pagi-skills deep_audit
```

Test cases cover:
- KB routing logic for all 8 collections
- Tourism/coastal keyword detection (KB-05)
- Technical content routing (KB-03)
- Meeting transcript routing (KB-07)
- Default fallback to Psyche (KB-01)

## Future Enhancements

### Phase 2: Qdrant Integration
- [ ] Actual vector storage implementation
- [ ] Batch processing for large files
- [ ] Incremental updates for modified files

### Phase 3: Advanced Triage
- [ ] LLM-based semantic analysis (for files < 5MB)
- [ ] Multi-KB routing (documents spanning multiple domains)
- [ ] Confidence scoring for routing decisions

### Phase 4: Active Learning
- [ ] User feedback on routing accuracy
- [ ] Adaptive keyword expansion
- [ ] Domain-specific model fine-tuning

## Troubleshooting

### Files Not Being Processed

1. Check that `./data/ingest` directory exists
2. Verify file permissions (readable by process)
3. Check logs for redaction errors
4. Ensure Qdrant is running on Port 6333

### Incorrect KB Routing

1. Review [`config/knowledge-map.toml`](config/knowledge-map.toml)
2. Add domain-specific keywords
3. Adjust priority levels if needed
4. Test with sample documents

### Redaction Issues

1. Verify `data/protected_terms.txt` exists
2. Check project-specific `.sao_policy` files
3. Test redaction with `/api/v1/vault/redact-test`

## Related Documentation

- [`SOVEREIGNTY_DRILL.md`](add-ons/pagi-gateway/SOVEREIGNTY_DRILL.md) - Port 8000 authority
- [`SAO_REDACTION_AND_INTELLIGENT_TITLES.md`](add-ons/pagi-gateway/SAO_REDACTION_AND_INTELLIGENT_TITLES.md) - Redaction system
- [`KB_ROUTER_README.md`](add-ons/pagi-gateway/KB_ROUTER_README.md) - Knowledge Base architecture
- [`MIMIR_CHRONOS_INTEGRATION.md`](add-ons/pagi-gateway/MIMIR_CHRONOS_INTEGRATION.md) - Meeting memory system

---

**Status**: ✅ Core implementation complete (Phase 1)
**Next**: Qdrant vector storage integration (Phase 2)

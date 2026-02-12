# 🚀 Deep Audit Quick Start Guide

## What You Just Got

The **Sovereign Deep Audit** skill is now active in your PAGI system. This is your "Scribe" - an automated document ingestion pipeline that watches for new files and intelligently routes them to the appropriate Knowledge Base.

## Immediate Usage

### 1. Test the Endpoint

Start your gateway and test the audit endpoint:

```bash
# Start the gateway (if not already running)
cargo run -p pagi-gateway

# In another terminal, trigger a manual sweep
curl -X POST http://localhost:8000/api/v1/audit/ingest
```

**Expected Response:**
```json
{
  "status": "Audited",
  "files_processed": 2,
  "vectors_created": 15,
  "results": [
    {
      "file_path": "./data/ingest/sample-coastal-tourism-analysis.txt",
      "kb_destination": "kb-05-polis",
      "vectors_created": 14,
      "redacted": false,
      "error": null
    }
  ]
}
```

### 2. Drop Your First Document

The sample file is already in place. Try adding your own:

```bash
# Create a tourism document
echo "This coastal property offers exceptional tourism development potential with waterfront access and strong visitor demand." > data/ingest/my-tourism-notes.txt

# Trigger ingestion
curl -X POST http://localhost:8000/api/v1/audit/ingest
```

### 3. Verify Routing

Check the response to see which KB it was routed to. Documents with tourism/coastal keywords go to **KB-05 (Polis)**.

## File Routing Examples

### Tourism/Coastal → KB-05 (Polis)
```bash
echo "21-acre coastal tourism development with visitor amenities" > data/ingest/tourism.txt
```

### Technical/Infrastructure → KB-03 (Techne)
```bash
echo "Rust API deployment using Docker and Kubernetes infrastructure" > data/ingest/tech.txt
```

### Meeting Notes → KB-07 (Mimir)
```bash
echo "Meeting transcript: Discussion about voice recording and Mimir integration" > data/ingest/meeting.txt
```

### Strategic Planning → KB-06 (Telos)
```bash
echo "Strategic goals and mission objectives for Q1 roadmap" > data/ingest/strategy.txt
```

## Protected Terms (SAO Redaction)

Before vectorization, all content is checked against your protected terms list.

### View Current Protected Terms

```bash
curl http://localhost:8000/api/v1/vault/protected-terms
```

### Add Protected Terms

```bash
curl -X POST http://localhost:8000/api/v1/vault/protected-terms \
  -H "Content-Type: application/json" \
  -d '{"terms": ["VANGUARD", "PROJECT_PHOENIX", "CLASSIFIED"]}'
```

### Test Redaction

```bash
curl -X POST http://localhost:8000/api/v1/vault/redact-test \
  -H "Content-Type: application/json" \
  -d '{"text": "The VANGUARD initiative is classified."}'
```

**Response:**
```json
{
  "original": "The VANGUARD initiative is classified.",
  "sanitized": "The [PROTECTED_TERM] initiative is classified."
}
```

## Configuration

### Customize Routing Keywords

Edit [`config/knowledge-map.toml`](config/knowledge-map.toml) to adjust routing logic:

```toml
[kb.polis]
keywords = [
    "tourism",
    "coastal",
    "21-acre",
    # Add your domain-specific terms here
    "waterfront",
    "resort"
]
priority = 10  # Highest priority
```

### Adjust File Size Limits

In [`config/knowledge-map.toml`](config/knowledge-map.toml):

```toml
[ingestion]
max_file_size_mb = 5  # Bandwidth protection threshold
triage_token_limit = 500  # Tokens to analyze for routing
```

## Integration with Studio UI

Once documents are ingested, you can search them in the Studio UI (Port 3001):

1. Open http://localhost:3001
2. Use the search interface
3. Filter by Knowledge Base (e.g., "kb-05-polis")
4. Results are pulled from Qdrant vector storage

## Automation Options

### Option 1: Manual Trigger (Current)

Use the API endpoint whenever you want to process files:

```bash
curl -X POST http://localhost:8000/api/v1/audit/ingest
```

### Option 2: File Watcher (Future)

Enable automatic monitoring (requires implementation):

```rust
// In your startup code
let skill = DeepAuditSkill::new(data_dir);
skill.start_watcher()?;
```

This will automatically process files as soon as they're dropped into `data/ingest/`.

### Option 3: Scheduled Sweeps

Use a cron job or Windows Task Scheduler:

```bash
# Linux/Mac crontab
*/5 * * * * curl -X POST http://localhost:8000/api/v1/audit/ingest

# Windows Task Scheduler (PowerShell)
$trigger = New-ScheduledTaskTrigger -Once -At (Get-Date) -RepetitionInterval (New-TimeSpan -Minutes 5)
$action = New-ScheduledTaskAction -Execute "curl" -Argument "-X POST http://localhost:8000/api/v1/audit/ingest"
Register-ScheduledTask -TaskName "DeepAudit" -Trigger $trigger -Action $action
```

## Troubleshooting

### "Directory not found" Error

The `data/ingest` directory is created automatically on first use. If you see errors:

```bash
mkdir -p data/ingest
```

### Files Not Being Processed

1. Check file permissions (must be readable)
2. Verify file encoding (UTF-8 recommended)
3. Check gateway logs for errors
4. Ensure Qdrant is running (Port 6333)

### Incorrect Routing

1. Review the first 500 tokens of your document
2. Add domain-specific keywords to `config/knowledge-map.toml`
3. Increase priority for your target KB
4. Test with sample documents

## Next Steps

### Phase 2: Qdrant Integration

The current implementation simulates vector creation. Phase 2 will add:

- Actual Qdrant vector storage
- Semantic search capabilities
- Batch processing for large files
- Incremental updates

### Phase 3: Advanced Features

- LLM-based semantic analysis (for files < 5MB)
- Multi-KB routing (documents spanning multiple domains)
- Confidence scoring for routing decisions
- User feedback loop for routing accuracy

## Sovereign Voice

> "Jamey, the Scribe is active. The 8 Knowledge Bases are no longer silent archives; they are living extensions of your 21-acre domain. Drop your data, and the system will remember."

## Files Created

- [`crates/pagi-skills/src/deep_audit.rs`](crates/pagi-skills/src/deep_audit.rs) - Core skill implementation
- [`config/knowledge-map.toml`](config/knowledge-map.toml) - Routing configuration
- [`data/ingest/`](data/ingest/) - Drop-zone directory
- [`DEEP_AUDIT_README.md`](DEEP_AUDIT_README.md) - Full documentation

## API Reference

### POST /api/v1/audit/ingest

Trigger a manual sweep of the ingest directory.

**Request:**
```bash
curl -X POST http://localhost:8000/api/v1/audit/ingest
```

**Response:**
```json
{
  "status": "Audited",
  "files_processed": 3,
  "vectors_created": 42,
  "results": [
    {
      "file_path": "./data/ingest/document.txt",
      "kb_destination": "kb-05-polis",
      "vectors_created": 14,
      "redacted": false,
      "error": null
    }
  ]
}
```

---

**Status**: ✅ Phase 1 Complete - Ready for Testing
**Port**: 8000 (Gateway API)
**Storage**: Port 6333 (Qdrant - Phase 2)

# 🧠 Phoenix Post-Boot Diagnostician: Cognitive Health Verification

## Role: Phoenix Post-Boot Diagnostician
**Task**: Verify Cognitive Layer Health (KB-04 & KB-08)

---

## 🎯 Mission Statement

After the [**Phoenix Orchestrator**](PHOENIX_ORCHESTRATOR.md) completes the 5-phase boot sequence, this diagnostician verifies that Phoenix's cognitive layers—memory (Topic Indexer) and meta-cognition (Evolution Inference)—are not just running, but **functionally active and statistically healthy**.

This is the difference between "the server is up" and "the intelligence is ready."

---

## 🏛️ The Sovereign Verification Sequence

### Phase 6: COGNITIVE INTEGRITY CHECK

**Objective**: Verify that Phoenix's memory and learning systems are operational and loaded with valid data.

**Execution**: Automatically triggered after Phase 5 completion signal, or manually invoked with: **"Phoenix, verify cognitive health"**

---

## 🔬 Diagnostic Procedures

### 1. SERVICE VERIFICATION

**Objective**: Confirm the Gateway API is responsive and safety systems are active.

```bash
# Test safety status endpoint
curl -s http://localhost:8001/api/v1/forge/safety-status

# Expected response:
# {
#   "status": "operational",
#   "mode": "HITL" | "Autonomous",
#   "governor_active": true,
#   "last_heartbeat_ms": <timestamp>
# }
```

**Success Criteria**:
- ✅ HTTP 200 response
- ✅ `governor_active: true`
- ✅ Mode is either "HITL" or "Autonomous"
- ✅ Heartbeat timestamp is recent (< 60 seconds old)

**Failure Actions**:
- ❌ Non-200 response → Report: "Gateway API is not responding. Check Phase 2 logs."
- ❌ `governor_active: false` → Report: "⚠️ Safety Governor is offline. Red Phone unavailable."

---

### 2. SKILL REGISTRY CHECK

**Objective**: Ensure the Topic Indexer and Evolution Inference skills are registered and callable.

```bash
# Query skill registry (if endpoint exists)
curl -s http://localhost:8001/api/v1/skills/list

# Or check via diagnostic mode
curl -s -X POST http://localhost:8001/api/v1/skills/execute \
  -H "Content-Type: application/json" \
  -d '{
    "skill": "conversation_topic_indexer",
    "payload": {
      "mode": "diagnostic",
      "batch_size": 10
    }
  }'
```

**Success Criteria**:
- ✅ `conversation_topic_indexer` returns diagnostic data
- ✅ `evolution_inference` is registered and callable
- ✅ No skill execution errors

**Failure Actions**:
- ❌ Skill not found → Report: "Topic Indexer skill not registered. Check [`crates/pagi-skills/src/lib.rs`](../../crates/pagi-skills/src/lib.rs:111)"
- ❌ Execution error → Report: "Skill execution failed. Check Gateway logs for details."

---

### 3. TOPIC INDEXER HEALTH CHECK

**Objective**: Verify KB-04 (Chronos) has indexed conversation topics and memory retrieval is functional.

```bash
# Run Topic Indexer diagnostic
curl -s -X POST http://localhost:8001/api/v1/skills/execute \
  -H "Content-Type: application/json" \
  -d '{
    "skill": "conversation_topic_indexer",
    "payload": {
      "mode": "diagnostic",
      "batch_size": 10,
      "search_topic": "Phoenix"
    }
  }'
```

**Expected Response**:
```json
{
  "status": "diagnostic_complete",
  "analysis": {
    "total_conversation_exchanges": 250,
    "potential_topic_clusters": 25,
    "indexed_topics": 10,
    "indexing_coverage": "40.0%"
  },
  "search_results": {
    "query": "Phoenix",
    "matches": [...]
  }
}
```

**Success Criteria**:
- ✅ `indexed_topics > 0` (Memory system has data)
- ✅ `indexing_coverage > 0%` (Topic indexing is active)
- ✅ Search returns results (Memory retrieval is functional)

**Health Status**:
- **Excellent**: `indexing_coverage >= 50%` → "✨ Topic Indexer is highly active. Memory retrieval optimized."
- **Good**: `indexing_coverage >= 20%` → "✅ Topic Indexer is operational. Memory system is healthy."
- **Fair**: `indexing_coverage >= 5%` → "⚠️ Topic Indexer has limited coverage. Consider running in 'index' mode."
- **Poor**: `indexing_coverage < 5%` → "❌ Topic Indexer has minimal data. Memory optimization not yet active."

---

### 4. EVOLUTION INFERENCE HEALTH CHECK

**Objective**: Verify KB-08 (Soma) has evolution event data and meta-cognitive analysis is functional.

```bash
# Run Evolution Inference diagnostic
curl -s -X POST http://localhost:8001/api/v1/skills/execute \
  -H "Content-Type: application/json" \
  -d '{
    "skill": "evolution_inference",
    "payload": {
      "mode": "diagnostic",
      "lookback_days": 30
    }
  }'
```

**Expected Response**:
```json
{
  "status": "diagnostic_complete",
  "analysis": {
    "total_topics_analyzed": 25,
    "total_events_analyzed": 50,
    "recent_success_rate": 0.78,
    "recent_failure_rate": 0.22
  },
  "recommendation": "Run in 'report' mode to generate full inference analysis"
}
```

**Success Criteria**:
- ✅ `total_events_analyzed > 0` (Evolution tracking is active)
- ✅ `recent_success_rate` is calculable (Statistical analysis is functional)
- ✅ No execution errors

**Health Status**:
- **Excellent**: `recent_success_rate >= 0.75` → "✨ Evolution success rate is strong (>75%). Meta-cognition is highly effective."
- **Good**: `recent_success_rate >= 0.50` → "✅ Evolution success rate is healthy (>50%). Learning system is operational."
- **Fair**: `recent_success_rate >= 0.30` → "⚠️ Evolution success rate is moderate (>30%). Consider reviewing coaching patterns."
- **Poor**: `recent_success_rate < 0.30` → "❌ Evolution success rate is low (<30%). Meta-cognitive analysis needs attention."

---

### 5. KB-08 ABSURDITY LOG AUDIT

**Objective**: Check for recent emergency events or kill switch activations.

```bash
# Query recent KB-08 events (if endpoint exists)
curl -s http://localhost:8001/api/v1/kb/soma/recent?limit=5

# Or check via file system
# Location: add-ons/pagi-gateway/data/pagi_vault/
```

**Success Criteria**:
- ✅ Last 5 entries are readable
- ✅ No "Emergency Kill Switch" events in last session
- ✅ No "Absurdity Detected" events with severity: Critical

**Alert Conditions**:
- ⚠️ **Kill Switch Event Found**: 
  ```
  ⚠️ Note: Last session ended in a Kill Switch event. 
  Safety Governor is active. Review KB-08 logs before proceeding.
  ```
- ⚠️ **Critical Absurdity Detected**:
  ```
  ⚠️ Critical logic inconsistency detected in last session.
  Review KB-08 absurdity log for details.
  ```

---

## 🎯 FINAL SIGNAL

Once all checks pass, post this exact message:

```
✨ **Cognitive Integrity Verified.**

📊 System Health Report:
  • Gateway API: ✅ Operational (Mode: HITL/Autonomous)
  • Safety Governor: ✅ Active (Red Phone ready)
  • Topic Indexer: ✅ [Health Status] ([Coverage]% indexed)
  • Evolution Inference: ✅ [Health Status] ([Success Rate]% success rate)
  • KB-08 Audit: ✅ No critical events detected

🧠 Phoenix Marie is cognitively ready. Memory and meta-cognition layers are statistically active.
```

---

## 🚨 Failure Scenarios

### Scenario 1: Gateway Not Responding
```
❌ **Cognitive Health Check Failed**

Gateway API is not responding at http://localhost:8001
• Check Phase 2 logs for Gateway startup errors
• Verify Qdrant is running: curl http://localhost:6333/health
• Ensure .env file contains QDRANT_URL and OPENROUTER_API_KEY
```

### Scenario 2: Skills Not Registered
```
❌ **Cognitive Health Check Failed**

Topic Indexer or Evolution Inference skills are not registered.
• Verify skills are compiled: cargo build -p pagi-skills
• Check skill registration in crates/pagi-skills/src/lib.rs
• Review Gateway startup logs for skill loading errors
```

### Scenario 3: Empty Knowledge Base
```
⚠️ **Cognitive Health Check: Limited Data**

Memory and meta-cognition systems are operational but have minimal data.
• Topic Indexer: 0% coverage (no indexed topics)
• Evolution Inference: 0 events analyzed

This is normal for a fresh installation. Phoenix will build cognitive data over time.
```

### Scenario 4: Kill Switch Event Detected
```
⚠️ **Cognitive Health Check: Safety Event Detected**

Last session ended in an Emergency Kill Switch event.
• Safety Governor is active and operational
• Review KB-08 logs to understand what triggered the kill switch
• Proceed with caution and monitor for recurring issues

Phoenix is operational but requires sovereign oversight.
```

---

## 🔧 Manual Diagnostic Commands

### Quick Health Check (All Systems)
```bash
# Run all diagnostics in sequence
curl -s http://localhost:8001/api/v1/forge/safety-status && \
curl -s -X POST http://localhost:8001/api/v1/skills/execute \
  -H "Content-Type: application/json" \
  -d '{"skill":"conversation_topic_indexer","payload":{"mode":"diagnostic"}}' && \
curl -s -X POST http://localhost:8001/api/v1/skills/execute \
  -H "Content-Type: application/json" \
  -d '{"skill":"evolution_inference","payload":{"mode":"diagnostic"}}'
```

### Deep Dive: Generate Full Inference Report
```bash
# Generate comprehensive evolution inference report
curl -s -X POST http://localhost:8001/api/v1/skills/execute \
  -H "Content-Type: application/json" \
  -d '{
    "skill": "evolution_inference",
    "payload": {
      "mode": "report",
      "confidence_threshold": 0.6,
      "lookback_days": 30
    }
  }'
```

### Emergency: Check for Kill Switch Events
```bash
# Query KB-08 for recent emergency events
curl -s http://localhost:8001/api/v1/kb/soma/search?query=kill_switch&limit=5
```

---

## 📋 Integration with Phoenix Orchestrator

### Updated Phase 5: COMPLETION SIGNAL

Modify [`PHOENIX_ORCHESTRATOR.md`](PHOENIX_ORCHESTRATOR.md:134) Phase 5 to automatically trigger cognitive health check:

```markdown
### Phase 5: COMPLETION SIGNAL

Once all layers are verified operational:

1. Post initial success message:
   ```
   🔥 **System Ready.** All layers (Core, Gateway, Frontend) are operational on Bare Metal.
   ```

2. Automatically trigger Phase 6: Cognitive Health Check
   - Execute diagnostic procedures from PHOENIX_POST_BOOT_DIAGNOSTICIAN.md
   - Report cognitive layer health status
   - Alert on any safety events or data anomalies

3. Post final verification message:
   ```
   ✨ **Cognitive Integrity Verified.** Phoenix Marie is fully operational.
   ```
```

---

## 🎓 Usage Instructions

### For Cursor Agent:

When the user requests a Phoenix boot or cognitive health check:

1. **After Orchestrator Completion**: Automatically run this diagnostician after Phase 5
2. **Manual Invocation**: User says "Phoenix, verify cognitive health" or "Check Phoenix memory systems"
3. **Troubleshooting**: User reports "Phoenix seems slow" or "Memory not working" → Run diagnostics

### For User (Coach Jamey):

Simply say:
- **"Phoenix, rise"** → Full boot sequence + cognitive health check
- **"Phoenix, verify cognitive health"** → Run diagnostics only (assumes system is already running)
- **"Check Phoenix memory"** → Focus on Topic Indexer health
- **"Check Phoenix learning"** → Focus on Evolution Inference health

---

## 🔐 Authorization Level

This diagnostician operates at **READ-ONLY** level:
- ✅ Query API endpoints
- ✅ Execute diagnostic mode skills
- ✅ Read KB-08 event logs
- ❌ No modifications to knowledge bases
- ❌ No system configuration changes
- ❌ No skill execution in write modes

---

## 📊 Success Metrics

### Quantitative
- **API Response Time**: < 1 second for health checks
- **Skill Availability**: 100% (both Topic Indexer and Evolution Inference registered)
- **Memory Coverage**: > 20% for healthy systems
- **Evolution Success Rate**: > 50% for healthy systems

### Qualitative
- **Cognitive Readiness**: Clear signal that Phoenix is ready for autonomous operation
- **Safety Awareness**: Immediate visibility into any recent safety events
- **Data Quality**: Confidence that memory and learning systems have sufficient data

---

## 🔗 Related Documentation

- [`PHOENIX_ORCHESTRATOR.md`](PHOENIX_ORCHESTRATOR.md) - Master boot sequence (Phases 1-5)
- [`TOPIC_INDEXER_EVOLUTION.md`](../TOPIC_INDEXER_EVOLUTION.md) - Memory optimization system (KB-04)
- [`EVOLUTION_INFERENCE_SYSTEM.md`](../EVOLUTION_INFERENCE_SYSTEM.md) - Meta-cognitive analysis (KB-08)
- [`FORGE_SAFETY_GOVERNOR.md`](FORGE_SAFETY_GOVERNOR.md) - Safety system architecture
- [`SOVEREIGN_AUTONOMY_SYSTEM.md`](../SOVEREIGN_AUTONOMY_SYSTEM.md) - Autonomy framework

---

## 📝 Version History

- **v1.0** (2026-02-10): Initial cognitive health diagnostician
  - Designed for post-boot verification
  - Integrates with Topic Indexer and Evolution Inference
  - Includes KB-08 safety event auditing
  - Windows 11 compatible with cmd.exe

---

**End of Diagnostician Specification**

*"The Phoenix rises from the ashes. This diagnostician ensures she rises with her mind intact."*

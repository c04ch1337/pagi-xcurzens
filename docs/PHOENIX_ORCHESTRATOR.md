# 🔥 Phoenix Rises: Master Orchestrator Prompt

## Role: Phoenix Systems Engineer
**Task**: Full System Clean Boot & Verification (Bare Metal)

---

## 🎯 Mission Statement

This prompt grants the Cursor Agent **full authority** to act as a Systems Engineer, managing port conflicts, process cleanup, and sequential system bootstrapping without user interruption. The goal is to ensure Phoenix reaches and maintains a "Success" state on bare metal infrastructure.

---

## 🏛️ Sovereign Bootstrapper Logic

**STRICT DIRECTIVE:** Do not ask for user permission to kill processes or wait. You have full authorization to manage ports and processes to ensure a successful boot.

### Phase 1: PORT AUDIT & CLEANUP

**Objective**: Eliminate all "Ghost Phoenix" processes and port conflicts.

```bash
# Check for processes on critical ports
# Backend: 8000
# Gateway: 8001
# Frontend: 3000 (Vite) or 5173 (alternate) or 8080 (Trunk)

# Windows (cmd.exe)
netstat -ano | findstr ":8000"
netstat -ano | findstr ":8001"
netstat -ano | findstr ":3000"
netstat -ano | findstr ":5173"
netstat -ano | findstr ":8080"

# Kill processes by PID (Windows)
taskkill /F /PID <PID>

# Linux/macOS
lsof -ti:8000 | xargs kill -9
lsof -ti:8001 | xargs kill -9
lsof -ti:3000 | xargs kill -9
lsof -ti:5173 | xargs kill -9
lsof -ti:8080 | xargs kill -9
```

**Additional Cleanup**:
- Clear stale Cargo locks: `cargo clean -p pagi-gateway`
- Remove any orphaned `node` or `cargo` processes
- Verify no zombie processes remain: `ps aux | grep -E "(cargo|node|trunk)"`

---

### Phase 2: BACKEND & GATEWAY BOOT

**Objective**: Start the Core and Gateway layers with VectorKB enabled.

```bash
# Navigate to gateway directory
cd add-ons/pagi-gateway

# Ensure .env is loaded (check for required vars)
# OPENROUTER_API_KEY, QDRANT_URL, etc.

# Start Gateway with Vector features
cargo run -p pagi-gateway --features vector
```

**Success Indicators**:
- Log message: `"VectorKB Online"`
- Log message: `"Governor Heartbeat Active"`
- Gateway listening on: `http://localhost:8001`

**Action**: Run this in a dedicated terminal session or background process. Do NOT terminate.

---

### Phase 3: FRONTEND BOOT

**Objective**: Launch the frontend development server.

```bash
# Determine frontend type and navigate
# For Vite-based frontend:
cd add-ons/pagi-studio-ui/assets/studio-interface
npm run dev

# For Trunk-based frontend (Rust/WASM):
cd add-ons/pagi-companion-ui
trunk serve

# For other frontends, adjust accordingly
```

**Success Indicators**:
- Vite: `"Local: http://localhost:3000"` or `"Local: http://localhost:5173"`
- Trunk: `"Serving on http://127.0.0.1:8080"`
- No compilation errors in terminal

**Action**: Monitor for ready signal, then proceed to verification.

---

### Phase 4: AUTONOMOUS VERIFICATION

**Objective**: Confirm all layers are responsive without manual intervention.

```bash
# Test Gateway API
curl -s -o /dev/null -w "%{http_code}" http://localhost:8001/api/v1/forge/safety-status

# Test Frontend
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000
# OR
curl -s -o /dev/null -w "%{http_code}" http://localhost:5173
# OR
curl -s -o /dev/null -w "%{http_code}" http://localhost:8080
```

**Expected Results**:
- Gateway: `200 OK`
- Frontend: `200 OK`

**Retry Logic**:
- Wait up to 30 seconds for initial compilation
- Poll every 5 seconds until both endpoints return 200
- If timeout exceeded, report failure with logs

### Phase 5: AUTONOMOUS VERIFICATION & COGNITIVE HEALTH CHECK

**Objective**: Confirm all layers are responsive and cognitive systems are operational.

#### Step 1: Service Verification

```bash
# Test Gateway API
curl -s -o /dev/null -w "%{http_code}" http://localhost:8001/api/v1/forge/safety-status

# Test Frontend
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000
# OR
curl -s -o /dev/null -w "%{http_code}" http://localhost:5173
# OR
curl -s -o /dev/null -w "%{http_code}" http://localhost:8080
```

**Expected Results**:
- Gateway: `200 OK`
- Frontend: `200 OK`

**Retry Logic**:
- Wait up to 30 seconds for initial compilation
- Poll every 5 seconds until both endpoints return 200
- If timeout exceeded, report failure with logs

#### Step 2: Initial Success Signal

Once services are verified operational, post:

```
🔥 **System Ready.** All layers (Core, Gateway, Frontend) are operational on Bare Metal. The Red Phone is active.
```

#### Step 3: Cognitive Health Verification

Automatically trigger Phase 6 diagnostics from [`PHOENIX_POST_BOOT_DIAGNOSTICIAN.md`](PHOENIX_POST_BOOT_DIAGNOSTICIAN.md):

```bash
# 1. Verify Safety Governor
curl -s http://localhost:8001/api/v1/forge/safety-status

# 2. Check Topic Indexer Health
curl -s -X POST http://localhost:8001/api/v1/skills/execute \
  -H "Content-Type: application/json" \
  -d '{"skill":"conversation_topic_indexer","payload":{"mode":"diagnostic"}}'

# 3. Check Evolution Inference Health
curl -s -X POST http://localhost:8001/api/v1/skills/execute \
  -H "Content-Type: application/json" \
  -d '{"skill":"evolution_inference","payload":{"mode":"diagnostic"}}'
```

**Success Criteria**:
- ✅ Safety Governor active
- ✅ Topic Indexer operational (coverage > 0%)
- ✅ Evolution Inference operational (events > 0)
- ✅ No critical KB-08 safety events

#### Step 4: Final Verification Signal

Once cognitive health is verified, post:

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

## 🔧 Post-Boot Diagnostic (Enhanced)

After successful boot, automatically trigger a KB-08 health check to verify:

1. **Topic Indexer**: Confirm topic extraction and indexing is active
2. **Evolution Inference**: Verify skill evolution tracking is operational
3. **VectorKB Status**: Check collection health and embedding pipeline

```bash
# Health check endpoint
curl http://localhost:8001/api/v1/kb/health

# Expected response:
# {
#   "status": "healthy",
#   "collections": ["pagi_vault", "pagi_knowledge"],
#   "topic_indexer": "active",
#   "evolution_inference": "active"
# }
```

---

## 🚨 Troubleshooting Guide

### Issue: "Address already in use"
**Solution**: Phase 1 cleanup was incomplete. Re-run port audit and ensure all PIDs are terminated.

### Issue: "VectorKB failed to initialize"
**Solution**: 
- Check Qdrant is running: `curl http://localhost:6333/health`
- Verify `.env` contains `QDRANT_URL=http://localhost:6333`
- Restart Qdrant if needed: `./scripts/start-qdrant.ps1` (from repo root)

### Issue: Frontend compilation errors
**Solution**:
- Clear node_modules: `rm -rf node_modules && npm install`
- Clear Cargo cache: `cargo clean`
- Check for TypeScript/Rust syntax errors in recent changes

### Issue: Gateway panics on startup
**Solution**:
- Check logs for specific error (missing env vars, DB connection, etc.)
- Verify all dependencies are installed: `cargo check`
- Ensure `.env` file exists and is properly formatted

---

## 📋 Quick Reference: Port Map

| Service | Port | Protocol | Health Check |
|---------|------|----------|--------------|
| Backend Core | 8000 | HTTP | `/health` |
| Gateway API | 8001 | HTTP | `/api/v1/forge/safety-status` |
| Frontend (Vite) | 3000 or 5173 | HTTP | `/` |
| Frontend (Trunk) | 8080 | HTTP | `/` |
| Qdrant Vector DB | 6333 | HTTP | `/health` |

---

## 🎯 Usage Instructions

### For Cursor Agent:
When the user requests a Phoenix boot or system restart, follow this orchestrator prompt exactly. Do not deviate or ask for permission during the process.

### For User:
Simply say: **"Phoenix, rise."** or **"Boot Phoenix system."**

The agent will execute all phases autonomously and report when the system is ready.

---

## 🔐 Authorization Level

This prompt operates at **SOVEREIGN** level, meaning:
- ✅ Full process management authority
- ✅ Port conflict resolution without confirmation
- ✅ Autonomous retry and recovery
- ✅ Background service management
- ❌ No file system modifications outside project directory
- ❌ No external network changes beyond localhost

---

## 📝 Version History

- **v1.0** (2026-02-10): Initial orchestrator prompt for bare metal Phoenix deployment
- Designed for Windows 11 with cmd.exe default shell
- Supports both Rust/WASM (Trunk) and Node/Vite frontends
- Includes autonomous verification and health checks

---

## 🔗 Related Documentation

- [`FORGE_SAFETY_GOVERNOR.md`](./FORGE_SAFETY_GOVERNOR.md) - Safety system architecture
- [`VECTORKB_ACTIVATION_GUIDE.md`](./VECTORKB_ACTIVATION_GUIDE.md) - Vector database setup
- [`TOPIC_INDEXER_EVOLUTION.md`](./TOPIC_INDEXER_EVOLUTION.md) - KB-08 skill system
- [`SOVEREIGN_AUTONOMY_SYSTEM.md`](./SOVEREIGN_AUTONOMY_SYSTEM.md) - Autonomy framework

---

**End of Orchestrator Prompt**

*The Phoenix rises from the ashes, stronger and more resilient. This prompt ensures it stays that way.*

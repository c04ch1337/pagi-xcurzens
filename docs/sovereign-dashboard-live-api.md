# 🏛️ Sovereign Dashboard — Live Status API

The **Sovereign Dashboard** (`pagi status`) provides a comprehensive, real-time view of your PAGI architecture across all 9 knowledge slots. It operates in two modes:

## 📊 Dashboard Modes

### 1. **Direct Mode** (Sled Access)
When the gateway is **not running**, the dashboard reads directly from the Sled database:
```bash
pagi status
```

### 2. **Live API Mode** (Gateway Running)
When the gateway **is running** and holds the Sled lock, the dashboard automatically falls back to the Live Status API:
```bash
# Terminal 1: Gateway running
cargo run -p pagi-gateway

# Terminal 2: Dashboard fetches via API
pagi status
```

## 🔌 API Endpoint

**Endpoint:** `GET /api/v1/sovereign-status`

**Response:** Full `SovereignState` JSON containing:
- 9-slot knowledge matrix (KB-1 through KB-9)
- Soma (BioGate) metrics
- Kardia (mental state + relational map)
- Ethos (philosophical policy)
- Oikos (task governance)
- Shadow (vault lock status)

## 🔐 Authentication

If `PAGI_API_KEY` is set, the dashboard will automatically include it in the request:

```bash
# Set API key (optional)
export PAGI_API_KEY="your-secret-key"

# Dashboard will use it automatically
pagi status
```

The API accepts the key via:
- Header: `X-API-Key: <key>`
- Header: `Authorization: Bearer <key>`

## 🎯 Use Cases

### Real-Time Monitoring
Keep the dashboard open in a side terminal while the gateway is running:
```bash
# Terminal 1: Gateway with 1-second tick rate
PAGI_TICK_RATE_SECS=1 cargo run -p pagi-gateway

# Terminal 2: Refresh dashboard every 5 seconds
watch -n 5 pagi status
```

### CI/CD Health Checks
```bash
# Check system health via API
curl http://localhost:8001/api/v1/sovereign-status \
  -H "X-API-Key: $PAGI_API_KEY"
```

### Multi-Terminal Workflow
- **Terminal 1:** Gateway logs (live system activity)
- **Terminal 2:** Dashboard (periodic status checks)
- **Terminal 3:** Interactive CLI commands

## 📋 Dashboard Sections

### 1. System Integrity
9-slot knowledge matrix with entry counts and connection status:
- **KB-1:** Pneuma (Vision)
- **KB-2:** Oikos (Context)
- **KB-3:** Logos (Knowledge)
- **KB-4:** Chronos (Temporal)
- **KB-5:** Techne (Capability)
- **KB-6:** Ethos (Guardrails)
- **KB-7:** Kardia (Affective)
- **KB-8:** Soma (Execution)
- **KB-9:** Shadow (The Vault)

### 2. Soma (Slot 8) — BioGate
- Sleep hours
- Readiness score
- Resting heart rate
- HRV (RMSSD)
- BioGate activation status

### 3. Kardia (Slot 7) — Mental State
- Relational stress
- Burnout risk
- Grace multiplier
- Relational map (people, trust, attachment)

### 4. Ethos (Slot 6) — Philosophical Lens
- Active philosophical school
- Tone weight
- Core maxims

### 5. Oikos (Slot 2) — Task Governance
- Governance summary
- Governed tasks with difficulty and action

### 6. Shadow (Slot 9) — The Vault
- Lock status
- Encryption key status
- Entry count

## 🚀 Implementation Details

### Core Method
[`KnowledgeStore::get_full_sovereign_state()`](../crates/pagi-core/src/knowledge/store.rs)

Aggregates data from all 9 slots into a single `SovereignState` struct.

### Gateway Handler
[`sovereign_status()`](../add-ons/pagi-gateway/src/main.rs)

Axum handler that calls the core method and returns JSON.

### Dashboard Fallback
[`fetch_sovereign_state_from_api()`](../add-ons/pagi-sovereign-dashboard/src/main.rs)

Detects Sled lock errors and automatically switches to HTTP mode.

## 🎨 Visual Design

The dashboard uses `comfy-table` for professional-grade terminal output:
- **Color-coded status:** Green (active), Yellow (warning), Red (error)
- **UTF-8 borders:** Rounded corners, clean separators
- **Aligned columns:** Right-aligned numbers, centered headers
- **Trust bars:** ASCII visualization of relational trust scores

## 🔧 Configuration

The dashboard reads from the same config as the gateway:

```toml
# config/gateway.toml
port = 8001
storage_path = "./data"
```

Override via environment:
```bash
PAGI__PORT=8002 pagi status
```

## 📝 Example Output

```
╔══════════════════════════════════════════════════════════════════════╗
║   🏛️  PAGI SOVEREIGN DASHBOARD v0.2.0  —  Situation Report            ║
║   2026-02-06 21:42:58 UTC                                          ║
╚══════════════════════════════════════════════════════════════════════╝

  ┌─ SYSTEM INTEGRITY ─ 9-Slot Knowledge Matrix ─────────────────────┐

╭──────┬─────────────────────┬─────────┬───────────╮
│ Slot ┆        Domain       ┆ Entries ┆   Status  │
╞══════╪═════════════════════╪═════════╪═══════════╡
│ KB-1 ┆ Pneuma (Vision)     ┆       6 ┆  ● ACTIVE │
│ KB-2 ┆ Oikos (Context)     ┆       6 ┆  ● ACTIVE │
│ KB-3 ┆ Logos (Knowledge)   ┆       2 ┆  ● ACTIVE │
│ KB-4 ┆ Chronos (Temporal)  ┆     563 ┆  ● ACTIVE │
│ KB-5 ┆ Techne (Capability) ┆       6 ┆  ● ACTIVE │
│ KB-6 ┆ Ethos (Guardrails)  ┆       2 ┆  ● ACTIVE │
│ KB-7 ┆ Kardia (Affective)  ┆       2 ┆  ● ACTIVE │
│ KB-8 ┆ Soma (Execution)    ┆     554 ┆  ● ACTIVE │
│ KB-9 ┆ Shadow (The Vault)  ┆       0 ┆ 🔒 LOCKED │
╰──────┴─────────────────────┴─────────┴───────────╯
  Total entries: 1141  |  Active slots: 8/9  |  Errors: 1
```

## 🎓 Maturity Level

**Level 5: Production-Ready Personal AGI Governance**

The Live Status API completes the transition from "Code" to "Sovereign Tool" by enabling:
- Real-time monitoring without database contention
- Multi-terminal workflows
- CI/CD integration
- Remote status checks

---

**Run `pagi status` at any time to refresh this report.**

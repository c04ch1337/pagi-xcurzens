# PAGI Sovereign Dashboard

CLI to view your AGI's **cross-layer state** in one place — a high-fidelity "Situation Report" across all 9 knowledge slots, rendered with `comfy-table` for clean ASCII visualization.

## Build and run

```bash
cargo run -p pagi-sovereign-dashboard -- status
# or, after cargo build:
./target/debug/pagi status
./target/debug/pagi dash
```

From the workspace root, `pagi` uses the same config as the gateway (`PAGI_CONFIG` or `config/gateway.toml`); the knowledge store is read from `{storage_path}/pagi_knowledge`.

## Usage

| Command        | Description |
|----------------|-------------|
| `pagi status`  | Full Sovereign Dashboard (default) |
| `pagi dash`    | Alias for `status` |
| `pagi`         | Same as `pagi status` |
| `pagi --help`  | Print usage |

## Dashboard Sections

### 1. System Integrity — 9-Slot Knowledge Matrix

A `comfy-table` showing all 9 KB slots with entry counts, connection status, and health indicators:

| Slot | Domain | Entries | Status |
|------|--------|---------|--------|
| KB-1 | Pneuma (Vision) | 12 | ● ACTIVE |
| KB-2 | Oikos (Context) | 45 | ● ACTIVE |
| ... | ... | ... | ... |
| KB-9 | Shadow (The Vault) | 3 | 🔒 LOCKED |

### 2. Soma (Slot 8) — Body / BioGate

Sleep hours, readiness score, resting HR, HRV with color-coded assessments. Shows whether the BioGate cross-layer reaction is active (supportive tone, grace multiplier override).

### 3. Ethos (Slot 6) — Philosophical Lens

Active school (e.g. Stoic, Growth-Mindset), tone weight, and core maxims.

### 4. Kardia (Slot 7) — Heart / Relational

- **Mental State**: relational stress, burnout risk, grace multiplier with status indicators
- **Relational Map**: people table with name, relationship, trust bar, attachment style, and triggers

### 5. Oikos (Slot 2) — Task Governance

Latest governance summary and a table of governed tasks showing title, difficulty, effective priority, and governance action (Proceed / Postpone / Simplify / Deprioritize).

### 6. Shadow (Slot 9) — The Vault

Vault lock status, encryption type (AES-256-GCM), and entry count.

## Live Status API (when the gateway is running)

Only one process can open the Sled-backed knowledge store at a time. If the store is locked (e.g. the gateway is running), `pagi status` **automatically falls back** to the gateway’s **Live Status API**:

- **Endpoint:** `GET http://127.0.0.1:{port}/api/v1/sovereign-status`  
  The port comes from your config (default `8001`).
- **Auth:** If `PAGI_API_KEY` is set in the environment (on the gateway), the dashboard sends it as the `X-API-Key` header when fetching. Set the same `PAGI_API_KEY` when running `pagi` so the request is authorized.
- If the gateway is unreachable or returns an error, the dashboard reports a clear error (e.g. “Gateway unreachable at …” or “unauthorized? Set PAGI_API_KEY if the endpoint is protected.”).

You can run `pagi status` at any time — with the gateway stopped (direct Sled read) or with it running (Live Status API).

## Dependencies

- `pagi-core` — Knowledge store, shared types, config, `SovereignState`
- `comfy-table` — UTF-8 table rendering with color support
- `chrono` — Timestamp formatting
- `reqwest` (blocking + json) — Fallback HTTP client for Live Status API

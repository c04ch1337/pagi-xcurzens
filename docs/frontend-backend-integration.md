# Frontend ↔ Backend Integration (Gateway/Bridge, KB, Memory, Prompts)

This document is the **version-controlled integration runbook** for wiring any Frontend UI into the **Phoenix Core** (Sovereign AGI) backend.

## 🦅 Phoenix Core Architecture

The Phoenix Core is a **Sovereign AGI Orchestrator** built with:
- **Bare Metal Design**: Zero Docker, Rust core, local-first persistence
- **9-Layer Memory Taxonomy**: Strict firewall-gated knowledge bases
- **3-Tier Skill Model**: Core/Import/Generated with trust boundaries
- **Astro-Logic Engine**: Celestial transits as environmental volatility proxy

Scope:

* **Bridge/Gateway integration** (HTTP API surface, streaming, logs)
* **Orchestrator integration** (goals + skills execution)
* **Knowledge Base (KB) integration** (9-layer ontology with sovereignty firewall)
* **Memory integration** (short-term "vault" + conversational/episodic storage)
* **Skill tier enforcement** (3-tier trust model)
* **Prompt inventory** (system prompts and prompt-injection contexts used by the backend)
* **Copy/paste "Integration Prompts"** you can use with an LLM or as internal SOP prompts when standing up a new Frontend

---

## 0) Step-by-step integration (end-to-end)

Use this ordered checklist to tie **backend → gateway → engine → frontend** from scratch.

### Phase 1: Backend and gateway

1. **Pre-flight**  
   Run `cargo run -p pagi-gateway -- --verify` from the workspace root. This checks port 8001 and that no Sled DB locks (e.g. in `data/pagi_vault/`, `data/pagi_knowledge/`) are held. Fix any port or lock issues before starting.

2. **Config**  
   Confirm [`config/gateway.toml`](config/gateway.toml): `port` (default 8001), `storage_path`, `llm_mode`, `frontend_enabled`, and `[slot_labels]` for the 9 KB slots.

3. **Start gateway**  
   From workspace root: `cargo run -p pagi-gateway`. On first run, bootstraps (core identity KB-01/Ethos, core skills, sovereignty firewall KB-05) and optional workspace scan run automatically.

4. **Verify backend**  
   - `GET http://127.0.0.1:8001/v1/status` → `app_name`, `port`, `llm_mode`, `slot_labels`.  
   - `GET http://127.0.0.1:8001/api/v1/health` → `{"status":"ok","identity":"...","message":"..."}`.  
   - `GET http://127.0.0.1:8001/api/v1/kb-status` → status of all 9 Knowledge Bases (`knowledge_bases` array).

### Phase 2: Choose frontend and wire URLs

**Option A – Drop-in UI (same origin)**  
- Gateway serves `pagi-frontend` when `frontend_enabled = true`.  
- Open `http://127.0.0.1:8001/` (index) or `http://127.0.0.1:8001/ui/`.  
- Frontend calls **same origin**: e.g. `POST /v1/execute` for autonomous goals (see [`pagi-frontend/app.js`](pagi-frontend/app.js)).

**Option B – Studio UI (separate dev server)**  
- In one terminal: keep `cargo run -p pagi-gateway` (port 8001).  
- In another: `cd add-ons/pagi-studio-ui/assets/studio-interface && npm run dev` (Vite, port 3001).  
- Open `http://127.0.0.1:3001`.  
- Studio uses a single base URL: `http://127.0.0.1:8001` (see [`src/api/config.ts`](add-ons/pagi-studio-ui/assets/studio-interface/src/api/config.ts)). All API calls use `API_BASE_URL = ${GATEWAY_ORIGIN}/api/v1` (e.g. chat, config, persona stream, soma balance, wellness-report, domain-integrity, logs).  
- Chat: `POST /api/v1/chat`. Log terminal: `GET /api/v1/logs` (SSE). Persona/Warden: `GET /api/v1/persona/stream` (SSE). KB status: `GET /api/v1/kb-status`. See §0d for full component mapping.

### Phase 3: Verify end-to-end

5. **CORS**  
   Gateway allows origins on ports 3001–3099 (Frontend) and 8001–8099 (Backend). If the UI is on another port, adjust CORS in [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs) (e.g. `build_app`).

6. **Chat**  
    - Non-streaming: `POST /api/v1/chat` with `{"prompt":"Hello","stream":false}` → JSON with `response`, `thought`, `status`.  
    - Streaming: `POST /api/v1/chat` with `"stream": true` → chunked text (see §2.3.3).  
    - Use `user_alias` (and optionally `tenant_id` on `/v1/execute`) so Kardia and Chronos are tenant-scoped.

7. **Architect’s View (Concise = JSON Diagram Envelope)**

   Phoenix Studio UI expects a **diagram-first** response in Concise mode, emitted as a **JSON Diagram Envelope** that the UI can render without Markdown parsing.

   **Manual verification (Studio UI):**
   1. Set `density_mode` to **Concise**.
   2. Ask: “How does the Sovereign Firewall handle an unauthorized external API call?”
   3. Verify:
      - the response begins with the JSON envelope (not prose)
      - the Mermaid payload includes the dark theme init directive:

        ```text
        %%{init: {'theme': 'dark'}}%%
        ```

      - prose is limited to 1–2 bullets below the diagram

   **Raw JSON capture (proof):**
   - In DevTools → Network, open the `POST /api/v1/chat` response body and copy the raw JSON.

   Reference mission brief: [`FIRST_MISSION_OPERATION_FIRST_RISE.md`](FIRST_MISSION_OPERATION_FIRST_RISE.md)

7. **Proof of life**  
    In the browser: confirm one concrete UI element (e.g. “Gateway log stream” header, chat input, or status line). Check console for no 404/CORS errors. If you see “Connection Refused”, re-run pre-flight and ensure only one process uses the same `data/` path.

### Summary prompts to run (in order)

- **Backend bring-up:** “Start the PAGI gateway per docs/frontend-backend-integration.md: run pre-flight, then `cargo run -p pagi-gateway`. Confirm /v1/status and /api/v1/health return expected JSON.”  
- **Frontend wiring:** “Wire the frontend to the gateway per docs/frontend-backend-integration.md Phase 2 (drop-in vs Studio). Set API URL to http://127.0.0.1:8001/api/v1/chat for Studio.”  
- **Verification:** “Verify end-to-end per docs/frontend-backend-integration.md Phase 3: CORS, chat (stream and non-stream), and proof of life in the browser.”

---

## 0b) Architecture (mental model)

**Gateway (Bridge)**: Axum HTTP server that exposes the integration API and dispatches requests into the Orchestrator.

* Main entry point: [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs:1)
* Config: [`config/gateway.toml`](config/gateway.toml:1)

**Orchestrator**: Receives a `Goal` (execute a skill, query KB, autonomous goal, etc.) and routes execution through the **Sovereignty Firewall**.

**KnowledgeStore (9 KBs)**: L2 memory with 9 sovereignty-gated knowledge layers (ontology in [`crates/pagi-core/src/knowledge/mod.rs`](crates/pagi-core/src/knowledge/mod.rs); display labels from `config/gateway.toml` `[slot_labels]`):

| Slot | KbType | Domain | Security |
|------|--------|--------|----------|
| **1** | **Pneuma** | Vision: identity, mission, evolving playbook | Standard (Sled) |
| **2** | **Oikos** | Context: workspace scan, "where" | Standard (Sled) |
| **3** | **Logos** | Pure knowledge: research, distilled info | Standard (Sled) |
| **4** | **Chronos** | Temporal: conversation history | Standard (Sled) |
| **5** | **Techne** | Capability: skills, blueprints | Standard (Sled) |
| **6** | **Ethos** | Guardrails: security, audit | Standard (Sled) |
| **7** | **Kardia** | Affective: user preferences, "who" | Standard (Sled) |
| **8** | **Soma** | Execution: physical interface, buffer | Standard (Sled) |
| **9** | **Shadow** | The Vault: trauma, anchors, private journaling | **AES-256-GCM** |

* Firewall: Tier 1 (Core) skills can access all slots; Tier 2/3 are gated. See 3-Tier Skill Model above.

**3-Tier Skill Model**:
* **Tier 1 (Core)**: User-signed skills with full KB access (including KB-01, KB-09)
* **Tier 2 (Import)**: Standard normalized skills (access to KB-02, KB-03, KB-06)
* **Tier 3 (Generated)**: AI-drafted ephemeral skills (blocked from sensitive layers until Warden promotion)

**MemoryManager ("vault")**: Long-term sled storage + hot cache for short-term UI state and other tenant-scoped values.

* Vault storage: [`crates/pagi-core/src/memory.rs`](crates/pagi-core/src/memory.rs:1)

**Astro-Logic Engine**: Real-time planetary transit scraping modifies agent "Caution Level" and defensive posture.

---

## 0c) Complete API reference (Gateway routes)

| Method | Path | Purpose | Used by |
|--------|------|---------|---------|
| GET | `/v1/status` | App identity, port, `llm_mode`, `slot_labels` | Drop-in UI, scripts |
| POST | `/v1/execute` | Orchestrator bridge: run a typed `Goal` (e.g. AutonomousGoal, ExecuteSkill) | Drop-in UI ([`pagi-frontend/app.js`](pagi-frontend/app.js)), any client |
| GET | `/v1/research/trace/:trace_id` | Fetch research trace by ID | Research/audit UIs |
| POST | `/v1/vault/read` | Decrypt and return a journal entry (requires `X-Pagi-Shadow-Key` header) | Sovereign Dashboard, secure UIs |
| GET | `/api/v1/health` | Liveness check | Studio UI, scripts |
| GET | `/api/v1/config` | Feature config (MoE, orchestrator_role, llm_model, etc.) from .env | Studio UI Settings |
| GET | `/api/v1/config/status` | Sovereign defensive layers (Warden .env toggles) for UI status | Studio UI |
| GET | `/api/v1/logs` | SSE stream of gateway logs (tracing) | Studio UI Log Terminal |
| POST | `/api/v1/stream` | SSE stream of chat tokens (Inner Monologue). Body: same as `/api/v1/chat` (prompt, user_alias, etc.) | Studio UI streaming chat |
| POST | `/api/v1/chat` | Chat (stream or JSON); Kardia injection, Chronos persistence | Studio UI ([`apiService.ts`](add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts)) |
| GET | `/api/v1/kardia/:user_id` | Current relation/sentiment for user (KB_KARDIA) | Studio UI, verification |
| GET | `/api/v1/kb-status` | Status of all 9 Knowledge Bases | Studio UI Settings / KB panel |
| GET | `/api/v1/skills` | List available skills and trust tier (core / import / generated) | Studio UI, Warden |
| POST | `/api/v1/skills/promote` | Promote a skill from generated to core (requires confirmation) | Studio UI Warden |
| GET | `/api/v1/sovereign-status` | Full sovereign state (requires `PAGI_API_KEY` if set) | Sovereign Dashboard |
| GET | `/api/v1/self-audit` | Orchestrator self-audit report | Studio UI, dashboards |
| POST | `/api/v1/sovereignty-audit` | Run sovereignty audit; updates sovereignty score | Studio UI, Governor |
| POST | `/api/v1/heal` | Trigger heal flow (domain repair) | Studio UI, maintenance |
| GET | `/api/v1/domain/vitality` | Domain vitality snapshot | Studio UI Warden |
| GET | `/api/v1/astro-weather` | Astro-Logic weather/transit data | Studio UI, optional |
| GET | `/api/v1/health-report` | Health report snapshot | Studio UI Wellness |
| POST | `/api/v1/evening-audit` | Trigger evening audit | Studio UI, maintenance |
| GET | `/api/v1/config/api-key` | API key configured / first-run status | Studio UI Settings |
| POST | `/api/v1/config/api-key` | Save API key (and optional user name, model) | Studio UI first-run |
| GET | `/api/v1/config/user` | User config snapshot | Studio UI |
| GET | `/api/v1/version` | Current gateway version | Studio UI, scripts |
| GET | `/api/v1/version/check` | Check for updates (e.g. GitHub release) | Studio UI |
| GET | `/api/v1/onboarding/status` | Onboarding progress status | Studio UI Onboarding |
| POST | `/api/v1/onboarding/complete` | Mark onboarding complete | Studio UI Onboarding |
| POST | `/api/v1/onboarding/user-profile` | Submit onboarding user profile | Studio UI Onboarding |
| GET | `/api/v1/archetype` | Current archetype configuration | Studio UI Settings |
| GET | `/api/v1/subject-check` | Subject presence/check | Studio UI |
| POST | `/api/v1/kb08/success-metric` | Store success metric in KB-08 | Studio UI, wellness |
| GET | `/api/v1/settings/orchestrator-role` | Current orchestrator role (counselor; companion legacy) | Studio UI Settings |
| POST | `/api/v1/settings/orchestrator-role` | Set orchestrator role. Body: `{ "mode": "counselor" \| "companion" }` | Studio UI Settings |
| GET | `/api/v1/settings/moe` | Current MoE (Sparse Intelligence) toggle | Studio UI Settings |
| POST | `/api/v1/settings/moe` | Set MoE toggle. Body: `{ "enabled": boolean }` | Studio UI Settings |
| GET | `/api/v1/settings/density` | Current context density (concise \| balanced \| verbose) | Studio UI Settings |
| POST | `/api/v1/settings/density` | Set density. Body: `{ "density_mode": "concise" \| "balanced" \| "verbose" }` | Studio UI Settings |
| GET | `/api/v1/persona/stream` | SSE: persona_heartbeat (4h), sentinel_update (velocity/rage), sovereign_reset_suggested | Studio UI App.tsx |
| POST | `/api/v1/soma/balance` | Store Spirit/Mind/Body (1–10) in KB-8 (Soma). Body: `{ spirit, mind, body }` | Studio UI BalanceCheckModal |
| GET | `/api/v1/skills/wellness-report` | 7-day Soma aggregation, individuation score, pillars, flags | Studio UI WellnessTab |
| GET | `/api/v1/sentinel/domain-integrity` | Absurdity log count, resource_drain_alerts | Studio UI WardenSidebar |
| GET | `/api/v1/intelligence/insights` | Cached SAO intelligence insights (pattern match + heuristics) | Studio UI (optional) |
| POST | `/api/v1/intelligence/toggle` | Toggle intelligence layer on/off | Studio UI (optional) |
| GET | `/api/v1/maintenance/pulse` | SSE maintenance pulse events | Maintenance dashboard |
| GET | `/api/v1/maintenance/status` | Maintenance loop status snapshot | Maintenance dashboard |
| GET | `/api/v1/maintenance/approval` | Current pending approval (if any) | Maintenance dashboard |
| POST | `/api/v1/maintenance/approval` | Respond to pending approval | Maintenance dashboard |
| GET | `/api/v1/maintenance/audit-log` | Chronos events for MAINTENANCE_LOOP agent | Maintenance dashboard |
| GET | `/api/v1/maintenance/patches` | Count of .rs files in patches directory | Maintenance dashboard |
| GET | `/api/v1/maintenance/patch-history` | Versioned patch history | Maintenance dashboard |
| POST | `/api/v1/maintenance/rollback` | Revert a skill to a previous version | Maintenance dashboard |
| GET | `/api/v1/forge/safety-status` | Forge safety governor (HITL) status | Studio UI, Warden |
| POST | `/api/v1/forge/safety` | Set Forge safety (enable/disable HITL) | Studio UI |
| GET | `/api/v1/system/diagnostics` | Export system diagnostics (beta) | Beta testing |
| POST | `/api/v1/mission/validate` | Validate Operation First Rise submission bundle | Beta ops |

When `frontend_enabled` is true, the gateway also serves the drop-in UI: `/` → `pagi-frontend/index.html`, `/assets/*` and `/ui/*` → `pagi-frontend` directory.

---

## 0d) Studio UI (pagi-studio-ui) integration

The Studio UI (`add-ons/pagi-studio-ui/assets/studio-interface`) is a React/Vite app that talks **only** to the Rust gateway. Base URL is defined in [`src/api/config.ts`](add-ons/pagi-studio-ui/assets/studio-interface/src/api/config.ts): `GATEWAY_ORIGIN = http://127.0.0.1:8001`, `API_BASE_URL = ${GATEWAY_ORIGIN}/api/v1`.

**Environment Lockdown (Bare Metal):** The frontend must **never** hold or send LLM API keys (e.g. OpenRouter). The Gateway loads `OPENROUTER_API_KEY` or `PAGI_LLM_API_KEY` from the backend `.env` and performs all LLM calls. The browser is a stateless client; if the UI ever asked for an API key, that would be a leaky abstraction. See backend startup in [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs) (dotenv load first) and [`apiService.ts`](add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts) (no client-side key).

### Component → Backend mapping

| UI Component | Backend integration |
|--------------|---------------------|
| **Chat** | `POST /api/v1/chat` (stream or JSON). System directive is augmented by orchestrator role (Counselor-Architect, emotional_state). |
| **Settings** | `GET /api/v1/config` (feature flags, orchestrator_role, llm_model). `GET/POST /api/v1/settings/orchestrator-role` (role). `GET/POST /api/v1/settings/moe`. Counselor Settings & Archetype: birth sign, ascendant, Jungian shadow focus. |
| **Warden Sidebar** | `GET /api/v1/persona/stream` (SSE): on `sentinel_update` event, UI sets `velocityScore` and `sentinelStatus` (calm/high/rage). `GET /api/v1/sentinel/domain-integrity` (absurdity_log_count, resource_drain_alerts) polled periodically. |
| **Balance Check Modal** | Opened when SSE sends `persona_heartbeat`. User submits Spirit/Mind/Body (1–10) via `POST /api/v1/soma/balance`; stored in KB-8 (Soma) with timestamped keys for 7-day history. |
| **Sovereign Reset toast** | Shown when SSE sends `sovereign_reset_suggested` (CounselorSkill + rage). Optional: when `sentinel_update` has high velocity and WellnessReport `is_critical`, show "High stress + low vitality" toast. |
| **Wellness tab** | `GET /api/v1/skills/wellness-report`. Displays pillars (Spirit/Mind/Body 7-day avg), individuation score, summary, flags (e.g. Puer Aeternus, Shadow Dominance), `is_critical` warning. |
| **Log Terminal** | `GET /api/v1/logs` (SSE) for gateway tracing output. |
| **KB panel** | `GET /api/v1/kb-status` for status of all 9 Knowledge Bases. |
| **Maintenance** | `GET /api/v1/maintenance/pulse`, `.../status`, `.../approval`, `.../audit-log`, `.../patches`, `.../patch-history`, `POST .../rollback`. |

### Persona stream SSE events

The client subscribes to `EventSource(API_BASE_URL + '/persona/stream')` and parses `event.data` as JSON:

- **`persona_heartbeat`** — 4-hour pulse; `message` is the check-in text. UI opens Balance Check modal.
- **`sentinel_update`** — `velocity_score` (0–100), `is_rage_detected` (boolean). UI updates Warden velocity bar and Sentinel badge (calm / high / rage).
- **`sovereign_reset_suggested`** — Present when Counselor suggests reset; may include `message`, `health_reminder`. UI shows amber toast.

### Types (Studio UI)

Key types live in [`types.ts`](add-ons/pagi-studio-ui/assets/studio-interface/types.ts): `GatewayFeatureConfig`, `PersonaHeartbeatEvent`, `SovereignResetEvent`, `WellnessReport` (pillars, individuation_score, summary, is_critical, flags, entries_used), `AppSettings` (birthSign, ascendant, jungianShadowFocus, intelligenceLayerEnabled).

### Theme and mode

Root element gets class `role-counselor` or `role-companion` from `gatewayConfig.orchestrator_role`. Counselor (base): sage/green accent; Companion (legacy): amber accent. Header border and Warden accent reflect system state.

---

## 1) Backend bring-up checklist (required before any Frontend integration)

1. **Confirm the gateway config**
   * Port and storage path in [`config/gateway.toml`](config/gateway.toml:1)
   * Slot label overrides in [`config/gateway.toml`](config/gateway.toml:11)

2. **Start the gateway**
   * Typical dev run: `cargo run -p pagi-gateway` (or whatever wrapper your environment uses)
   * Optional pre-flight checks exist in [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs:77)

3. **Verify bootstraps run** (first start only)
   * Core identity bootstrap (KB-1 / Pneuma): [`initialize_core_identity()`](crates/pagi-core/src/knowledge/bootstrap.rs:30)
   * Core skill registry bootstrap (KB-5 / Techne): [`initialize_core_skills()`](crates/pagi-core/src/knowledge/bootstrap.rs:138)
   * Default ethos policy (KB-6 / Ethos): [`initialize_ethos_policy()`](crates/pagi-core/src/knowledge/bootstrap.rs:245)
   * These are invoked from gateway startup: [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs:147)

4. **Confirm workspace context exists (Oikos)**
   * Gateway will run an initial workspace scan if missing: [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs:180)

5. **Forge Safety (optional)**
   * If using Sovereign Operator / Forge (self-modification): `PAGI_FORGE_SAFETY_ENABLED=true` (default) requires human approval before code changes. See [FORGE_SAFETY_GOVERNOR.md](../FORGE_SAFETY_GOVERNOR.md).
   * **Sovereign Autonomy:** Runtime control of Forge safety (HITL vs autonomous), auto-revert on compile failure, and multi-layer control (env, API, kill switch) are described in [SOVEREIGN_AUTONOMY_SYSTEM.md](../SOVEREIGN_AUTONOMY_SYSTEM.md).
   * **Emergency stop:** Run `.\forge-kill-switch.ps1` (Windows) or `./forge-kill-switch.sh` (Linux/macOS) to re-enable the safety gate and stop active Forge builds.

---

## 2) API surface a Frontend must integrate

## 2.x Heartbeat (Autonomous Orchestrator) integration notes

The system now includes a **Heartbeat** loop that makes inter-agent messaging event-like (no manual polling).

Current implementation detail:

* The Heartbeat is an **in-process `tokio` task inside the Gateway** (not a separate OS daemon process), so it can share the same `Arc<KnowledgeStore>` without `sled` cross-process file lock contention.
* Source: [`heartbeat_loop()`](add-ons/pagi-gateway/src/main.rs:268) and [`heartbeat_tick()`](add-ons/pagi-gateway/src/main.rs:290)

What it does per tick:

1. Enumerates active `agent_id`s by scanning KB-8/Soma inbox keys (`inbox/{agent_id}/...`).
2. For each agent:
   * If a new inbox message exists, generates an auto-reply using [`ModelRouter.generate_text_raw()`](crates/pagi-skills/src/model_router.rs:264).
   * Pushes the auto-reply back into the sender’s inbox using [`KnowledgeStore.push_agent_message()`](crates/pagi-core/src/knowledge/store.rs:798).
   * Records a reflection event in KB-4/Chronos using [`KnowledgeStore.append_chronos_event()`](crates/pagi-core/src/knowledge/store.rs:711).

Frontend implications:

* A Frontend does **not** need to call [`get_agent_messages`](crates/pagi-skills/src/get_agent_messages.rs:1) to “wake up” agents anymore.
* If your UI surfaces an “Agents / Inbox” panel:
  * Messages will arrive asynchronously (on the next heartbeat tick).
  * The UI can poll `get_agent_messages` for display purposes, but polling is no longer required for agent progress.

Configuration:

* `PAGI_TICK_RATE_SECS` controls pacing (default `5`). Lower values increase responsiveness but may increase LLM usage.
* The Heartbeat currently does **not** delete/ack inbox messages after processing; repeated auto-replies can occur if the same newest message remains newest. If you need exactly-once semantics, add an ack/delete mechanism in KB-8.

### 2.1 GET `/v1/status` (orchestrator identity)

Purpose: “is backend alive”, identity of the app, **slot labels** (what the UI should display for KB slots).

Implementation: [`status()`](add-ons/pagi-gateway/src/main.rs) (search for `async fn status`)

Response shape (example):

```json
{
  "app_name": "UAC Gateway",
  "port": 8001,
  "llm_mode": "mock",
  "slot_labels": {
    "1": "Brand Voice",
    "2": "Sales",
    "3": "Finance",
    "4": "Operations",
    "5": "Community",
    "6": "Products",
    "7": "Policies",
    "8": "Custom"
  }
}
```

Where slot labels come from: [`config/gateway.toml`](config/gateway.toml:11)

---

### 2.2 POST `/v1/execute` (Orchestrator bridge)

Purpose: a generic “bridge” endpoint that allows a Frontend (or any client) to run a **typed `Goal`**.

Implementation: [`execute()`](add-ons/pagi-gateway/src/main.rs) (search for `async fn execute`)

Request shape:

```json
{
  "tenant_id": "some-user-or-tenant",
  "correlation_id": "optional-trace-id",
  "goal": { "<GoalVariant>": { /* payload */ } }
}
```

Example (Autonomous goal) — used by the simple HTML Frontend:

* Client code: [`runAutonomousGoal()`](pagi-frontend/app.js:1)

```json
{
  "tenant_id": "default",
  "goal": {
    "AutonomousGoal": {
      "intent": "Draft a plan for X",
      "context": null
    }
  }
}
```

**Ethos policy enforcement happens here** (pre-execution scan for `ExecuteSkill`):

* See: [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs) (execute handler)

Integration implication:

* Your Frontend should display a user-friendly message if the response is `{"status":"policy_violation"...}`.

---

### 2.3 POST `/api/v1/chat` (UI-friendly chat wrapper)

Purpose: a convenience endpoint for Frontends to send a prompt and get a response from the LLM via the `ModelRouter` skill.

Implementation entry: [`chat()`](add-ons/pagi-gateway/src/main.rs) (search for `async fn chat`)

Request shape (matches Studio UI):

```json
{
  "prompt": "<user text>",
  "stream": false,
  "user_alias": "optional-user-id",
  "model": "optional-model-override",
  "temperature": 0.2,
  "max_tokens": 500,
  "persona": "optional-persona-string"
}
```

Reference client implementation (Studio UI; base URL is `http://127.0.0.1:8001/api/v1/chat` when using gateway):

* Non-streaming: [`sendMessageToOrchestrator()`](add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts:3)
* Streaming client wrapper: [`streamMessageToOrchestrator()`](add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts:46)

#### 2.3.0 Emotional Context Layer (Cognitive Governor)

The gateway uses **MentalState** (stored in KB_KARDIA under key `mental_state`) to modulate agent tone:

* **Contextual Grace:** If `relational_stress > 0.7`, the gateway prepends a hidden system instruction so the LLM adopts a supportive, low-pressure, empathetic tone (brevity and reassurance). This applies to `/api/v1/chat` (stream and non-stream) and to the heartbeat auto-reply path.
* **MentalState** is updated by the **JournalSkill** (see §3.4). Raw journal text is never logged or sent to external APIs; only anonymized emotional anchors are used to update scores.
* **ShadowStore (optional):** Sensitive journal entries can be stored encrypted (aes-gcm) when `PAGI_SHADOW_KEY` is set; see `crates/pagi-core/src/shadow_store.rs`.

To update mental state from a Frontend, call `POST /v1/execute` with goal `ExecuteSkill` and name `"JournalSkill"`, payload `{ "raw_text": "user journal text" }`. The skill extracts anonymized anchors and updates MentalState; subsequent chat/heartbeat will use the new tone.

#### 2.3.1 Chat context injection (Kardia)

The gateway injects **relationship context** (Kardia) into prompts if present:

* See: [`RelationRecord.prompt_context()`](crates/pagi-core/src/knowledge/store.rs)
* Used during chat: [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs) (chat handler)

This produces a prefix like:

```
[Relationship context: User sentiment: <...>. Communication style: <...>. Adjust your tone accordingly.]

<original user prompt>
```

Frontend implication:

* You do **not** need to add this yourself. The backend will add it when it has Kardia data.

#### 2.3.2 Conversation persistence (Chronos)

Chat responses are saved to **KB-4 Chronos** for later recall:

* See: [`save_to_memory()`](add-ons/pagi-gateway/src/main.rs) (non-streaming) and in the streaming handler after the stream completes.

Frontend implication:

* Conversation history persistence is automatic for `/api/v1/chat`.
* If you bypass `/api/v1/chat` and instead call `/v1/execute` directly, you must decide whether you want to persist conversation yourself.

#### 2.3.3 Streaming behavior

`/api/v1/chat` supports streaming via `"stream": true`:

* Streaming handler: search for `chat_streaming` or streaming path in [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs)

Important implementation detail:

* The gateway response is currently `Content-Type: text/plain; charset=utf-8` and yields **raw text chunks** (token-ish chunks), **not** `text/event-stream` SSE frames: [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs) (see `chat_streaming` response builder).
* The Studio UI streaming client includes SSE-style parsing (e.g. `data:` lines) for compatibility if the backend is later switched to SSE: [`add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts`](add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts:79). For the current backend, treat the body as **plain chunked text** and append chunks as they arrive.

Integration options for a new Frontend:

1. Treat streaming as **plain chunked text** and append chunks as they arrive (matches current gateway).
2. If you want SSE end-to-end, change the backend streaming handler to emit SSE events and set `Content-Type: text/event-stream` (then document in this file).

---

### 2.4 GET `/api/v1/sovereign-status` (Sovereign state inspection)

Purpose: Retrieve the full sovereign state for an agent, including trust scores, mental state, and relationship data. This endpoint is designed for the Sovereign Dashboard and other monitoring UIs that need to inspect the agent's internal state when they cannot directly access the Sled database (e.g., when the gateway holds the lock).

Implementation: [`sovereign_status()`](add-ons/pagi-gateway/src/main.rs) (search for `async fn sovereign_status`)

**Authentication**: If the `PAGI_API_KEY` environment variable is set, requests must include one of:
* Header `X-API-Key: <key>`
* Header `Authorization: Bearer <key>`

If the key is missing or invalid, the endpoint returns `401 Unauthorized`.

Response shape (example):

```json
{
  "agent_id": "default",
  "trust_score": 0.85,
  "mental_state": {
    "relational_stress": 0.3,
    "cognitive_load": 0.5,
    "emotional_stability": 0.8
  },
  "relationships": {
    "user_123": {
      "sentiment": 0.7,
      "trust": 0.9,
      "communication_style": "professional"
    }
  }
}
```

Frontend integration:

* Use this endpoint when building dashboards or monitoring tools that need to display the agent's internal state.
* Always handle `401 Unauthorized` responses gracefully (prompt for API key or show access denied message).
* The Sovereign Dashboard uses this endpoint when it cannot open the Sled database directly: [`add-ons/pagi-sovereign-dashboard/src/main.rs`](add-ons/pagi-sovereign-dashboard/src/main.rs)

---

### 2.5 POST `/v1/vault/read` (Secure journal entry retrieval)

Purpose: Decrypt and retrieve a specific journal entry from the ShadowStore. This endpoint provides secure access to encrypted journal data for authorized clients.

Implementation: [`vault_read()`](add-ons/pagi-gateway/src/main.rs:955)

**Authentication**: Requires the `X-Pagi-Shadow-Key` header with the same value as the `PAGI_SHADOW_KEY` environment variable. If the key is missing, invalid, or the environment variable is not set, the endpoint returns `403 Forbidden`.

Request shape:

```json
{
  "record_id": "journal_entry_123"
}
```

Response shape (example):

```json
{
  "record_id": "journal_entry_123",
  "label": "anxious",
  "intensity": 0.7,
  "timestamp_ms": 1707253200000,
  "raw_content": "Today was challenging..."
}
```

Error responses:

* `403 Forbidden`: Missing or invalid `X-Pagi-Shadow-Key` header
* `404 Not Found`: Record ID does not exist
* `500 Internal Server Error`: Decryption failed
* `503 Service Unavailable`: ShadowStore not initialized (no `PAGI_SHADOW_KEY` set)

Frontend integration:

* Use this endpoint when building UIs that need to display encrypted journal entries.
* Always store the shadow key securely (never in localStorage or cookies without encryption).
* Handle all error cases gracefully with user-friendly messages.
* Consider implementing a "view journal" feature that prompts for the shadow key on first access.

Security considerations:

* The shadow key must match the server's `PAGI_SHADOW_KEY` environment variable exactly.
* Raw journal content is never logged or sent to external APIs.
* The ShadowStore uses AES-GCM encryption: [`crates/pagi-core/src/shadow_store.rs`](crates/pagi-core/src/shadow_store.rs)

---

## 2.6 Sovereignty Firewall & Skill Tier Enforcement

Phoenix Marie enforces a **3-Tier Skill Model** to prevent unauthorized access to sensitive knowledge layers:

### Skill Trust Tiers

* **Tier 1 (Core)**: User-signed skills with full access to all KBs, including **KB-01 (Ethos)** and **KB-09 (Shadow)**
* **Tier 2 (Import)**: Standard normalized skills with access to general KBs (KB-02, KB-03, KB-06)
* **Tier 3 (Generated)**: AI-drafted ephemeral skills **blocked by the Firewall** from touching sensitive layers

### Warden Promotion Workflow

Frontend UIs should implement a **Promote** button for elevating Tier 3 skills:

1. User reviews AI-generated skill in the UI
2. User clicks **Promote** to elevate to Core (Tier 1) status
3. Backend validates and signs the skill
4. Skill gains access to restricted KBs

### Audit Logging

All blocked attempts by Tier 3 skills to access KB-01/KB-09 are logged in **KB-08 (Health)**. Frontend dashboards should display:

* "Capability Overreach" events
* Failed access attempts with timestamps
* Skill promotion history

Configuration:

* `PAGI_SKILLS_AUTO_PROMOTE_ALLOWED`: (Default: `false`) Prevents AI self-promotion
* `PAGI_STRICT_TECHNICAL_MODE`: Forces deterministic `0.3` temperature for technical operations

---

## 3) KB (Knowledge Base) integration (9-layer ontology with Sovereignty Firewall)

### 3.1 Slot model

KB slots are defined in the core and represent cognitive domains with **firewall-gated access**:

| Slot | KbType | Description | Security |
|------|--------|-------------|----------|
| **1** | **Pneuma** | Vision: identity, mission, evolving playbook | Standard (Sled) |
| **2** | **Oikos** | Context: workspace scan, "where" | Standard (Sled) |
| **3** | **Logos** | Pure knowledge: research, distilled info | Standard (Sled) |
| **4** | **Chronos** | Temporal: conversation history | Standard (Sled) |
| **5** | **Techne** | Capability: skills, blueprints | Standard (Sled) |
| **6** | **Ethos** | Guardrails: security, audit | Standard (Sled) |
| **7** | **Kardia** | Affective: user preferences, "who" | Standard (Sled) |
| **8** | **Soma** | Execution: physical interface, buffer | Standard (Sled) |
| **9** | **Shadow** | The Vault: trauma, anchors, private journaling | **AES-256-GCM** |

* Table + meaning: [`crates/pagi-core/src/knowledge/mod.rs`](crates/pagi-core/src/knowledge/mod.rs). Display labels come from `GET /v1/status` → `slot_labels` (configurable in `config/gateway.toml`).

Frontend guidance:

* Always display slots using `/v1/status.slot_labels` so UI labels are config-driven.
* Keep internal routing/IDs stable: slots are **1..=9**.
* Respect firewall status when displaying KB access in skill management UIs.
* Show visual indicators (badges, colors) for CORE ONLY and RESTRICTED layers.

### 3.2 Routing prompt used by the system (Thalamus)

The cognitive router uses an LLM classification prompt to route arbitrary info into exactly one KB domain:

* Prompt template: [`build_classification_prompt()`](crates/pagi-skills/src/thalamus.rs:29)

Frontend implication:

* If the Frontend provides a “save to KB” feature without asking the user to pick a slot, you can:
  1) send the content to a skill that calls Thalamus routing, or
  2) implement a client-side “suggested slot” using the same ontology and ask the backend to confirm.

### 3.3 How to query/insert KB data from a Frontend

Mechanically this happens through `/v1/execute` goals.

Conceptual patterns:

* **QueryKnowledge**: read a key from a slot
* **UpdateKnowledgeSlot / KnowledgeInsert**: add/update records
* **ExecuteSkill**: use skills as an API layer over KB operations

Because the exact `Goal` JSON tagging is Rust-serde-driven, treat the canonical contract as:

* Backend enum: `Goal` (see crate exports in [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs:18))

Practical integration approach:

* Start by integrating `/api/v1/chat` first.
* Then integrate KB read/write via specific skills (easier to keep stable) instead of directly hand-crafting `Goal` variants.

---

## 4) Memory integration (Vault + UI state)

There are two relevant “memory” layers for Frontends:

1. **Vault (MemoryManager)** — tenant-scoped paths and values (hot cache + sled): [`MemoryManager`](crates/pagi-core/src/memory.rs:16)
2. **Chronos (KB-4)** — conversation history and episodic events (stored in KnowledgeStore)

### 4.1 Reference pattern: Studio “prompt/response maps to short-term memory”

The Studio add-on explicitly maps UI state into short-term memory paths:

* Constants: [`MEMORY_PROMPT_PATH`](add-ons/pagi-studio-ui/src/app.rs:80), [`MEMORY_RESPONSE_PATH`](add-ons/pagi-studio-ui/src/app.rs:81)

Integration takeaway:

* For any new Frontend, define a small, explicit set of **memory paths** that represent “UI session state” (last prompt, last response, selected model, user preferences).
* Store those values via a backend memory API (if/when exposed) or via a dedicated skill.

### 4.2 Conversation persistence (Chronos)

If you use `/api/v1/chat`, the gateway stores the user/assistant exchange as a `KbRecord` with metadata in KB-4/Chronos:

* Implementation: [`save_to_memory()`](add-ons/pagi-gateway/src/main.rs:731)

---

## 5) Backend “prompt inventory” (what the system injects/uses)

This section is the authoritative index of prompts that materially affect responses.

### 5.1 Thalamus routing prompt

* Prompt text: [`build_classification_prompt()`](crates/pagi-skills/src/thalamus.rs:29)
* Purpose: classify content → exactly one KB domain

### 5.2 Kardia relationship context prompt-prefix

* Prefix builder: [`RelationRecord.prompt_context()`](crates/pagi-core/src/knowledge/store.rs:369)
* Purpose: tone/communication-style adaptation

### 5.3 Skill-registry appendix (ModelRouter)

ModelRouter can append a system “skills list” (from KB-5/Techne) to prompts:

* Entry point: [`ModelRouter::with_knowledge()`](crates/pagi-skills/src/model_router.rs:136)
* Skills appendix builder: `build_system_prompt_from_skills()` in [`crates/pagi-skills/src/model_router.rs`](crates/pagi-skills/src/model_router.rs:153)

Frontend implication:

* If you want the model to be aware of available skills, ensure the backend is running ModelRouter in a mode that includes the KB-5 appendix.

### 5.4 Identity/persona bootstraps (KB-1)

These are not “prompts” in the strict sense, but they are **core instruction data** stored at bootstrap:

* Bootstrap function: [`initialize_core_identity()`](crates/pagi-core/src/knowledge/bootstrap.rs:30)
* Identity keys (mission, priorities, persona): [`crates/pagi-core/src/knowledge/bootstrap.rs`](crates/pagi-core/src/knowledge/bootstrap.rs:9)

---

## 6) “Integration Prompts” (copy/paste SOP prompts)

These are operational prompts intended for a developer (or an LLM acting as a dev assistant) when integrating a new Frontend. Use them in the order below when tying backend, gateway, engine, and frontend together.

### 6.0 Master end-to-end integration prompt (full stack)

Use this when you need one prompt that covers the entire path from backend to UI:

```
Wire the PAGI backend, gateway, and engine to the frontend UI end-to-end.

Backend & gateway:
1) Run pre-flight: cargo run -p pagi-gateway -- --verify (port 8001, no Sled locks).
2) Start gateway: cargo run -p pagi-gateway from workspace root. Confirm GET /v1/status and GET /api/v1/health return expected JSON.
3) Slot labels and app identity come from config/gateway.toml; the UI should load them from GET /v1/status.

Frontend options:
- Drop-in UI: gateway serves pagi-frontend at http://127.0.0.1:8001/ when frontend_enabled=true. Frontend uses POST /v1/execute for goals (e.g. AutonomousGoal).
- Studio UI: run gateway on 8001 and npm run dev in add-ons/pagi-studio-ui/assets/studio-interface (port 3001). Set API URL to http://127.0.0.1:8001/api/v1/chat. Logs: http://127.0.0.1:8001/api/v1/logs.

API contract:
- Chat: POST /api/v1/chat with { prompt, stream, user_alias, model, temperature, max_tokens, persona }. Non-stream returns JSON; stream returns plain chunked text (Content-Type: text/plain).
- Execute: POST /v1/execute with { tenant_id, correlation_id?, goal }. Surface policy_violation and error status in the UI.
- Health: GET /api/v1/health. KB status: GET /api/v1/kb-status. Kardia: GET /api/v1/kardia/:user_id.

Verification:
- CORS allows Frontend ports 3001–3099 and Backend 8001–8099.
- In browser: confirm one concrete UI element and zero 404/CORS errors. If Connection Refused, re-run pre-flight and ensure only one process uses the same data/ path.

Deliver: a short runbook (steps + commands + URLs) and any code changes needed for this project.
```

### 6.1 Frontend integration kickoff prompt

Use this when starting a new UI integration task:

```
You are integrating a new Frontend UI with the PAGI backend gateway.

Requirements:
1) Implement GET /v1/status for app identity and slot label hydration; GET /api/v1/health for liveness.
2) Implement POST /api/v1/chat for non-streaming chat (JSON request/response).
3) Implement streaming chat using the backend’s current behavior (plain chunked text; Content-Type: text/plain) OR document + implement SSE consistently end-to-end.
4) Surface policy_violation and error responses from POST /v1/execute.
5) Use user_alias (chat) and tenant_id (execute) consistently so Kardia and Chronos are tenant-scoped.

Provide:
* A minimal API client module (base URL configurable, e.g. http://127.0.0.1:8001)
* UI wiring examples for status, chat, and execute
* Error handling patterns
* A short integration checklist for QA
```

### 6.2 Bridge/Gateway integration prompt (when the UI fails to connect)

```
Diagnose why the Frontend cannot integrate with the PAGI gateway.

Check:
* Gateway is listening on the configured port (config/gateway.toml; default 8001).
* CORS in add-ons/pagi-gateway/src/main.rs allows the Frontend origin (ports 3001–3099 and 8001–8099).
* GET /v1/status returns expected JSON (app_name, port, llm_mode, slot_labels).
* GET /api/v1/health returns {"status":"ok","identity":"...","message":"..."}.
* POST /api/v1/chat with stream=false returns JSON (response, thought, status).
* For stream=true, the gateway sends raw text chunks (Content-Type: text/plain), not SSE.

Return:
* Root cause
* Concrete fixes (config + code pointers)
* Verification steps (curl or browser)
```

### 6.3 KB integration prompt (adding a “save to knowledge” feature)

```
Add a Frontend feature to store and retrieve knowledge.

Constraints:
* The backend has 9 KB slots (KB-01 through KB-09); UI must display slot labels from GET /v1/status (slot_labels).
* Use GET /api/v1/kb-status for status of all 9 Knowledge Bases.
* Prefer calling stable skills via POST /v1/execute rather than hardcoding Rust Goal JSON variants.

Deliver:
* A UI flow: pick slot (or route via Thalamus), choose key, write record, then read it back.
* A test plan that verifies data persists across gateway restarts.
```

### 6.4 Memory integration prompt (persist UI state)

```
Integrate short-term memory for the UI.

Goal:
* Persist last prompt, last response, selected model, and user settings per tenant.

Reference:
* Studio uses memory paths like "studio/last_prompt" and "studio/last_response" (see add-ons/pagi-studio-ui/src/app.rs MEMORY_PROMPT_PATH / MEMORY_RESPONSE_PATH).

Deliver:
* A list of memory paths for this Frontend
* A backend interaction plan (memory API or skill)
* A migration plan for future schema changes
```

### 6.5 Verification and proof-of-life prompt

```
Verify the frontend–backend integration is working.

Steps:
1) Run cargo run -p pagi-gateway -- --verify; then cargo run -p pagi-gateway.
2) For Studio UI: in another terminal, npm run dev in add-ons/pagi-studio-ui/assets/studio-interface; open http://127.0.0.1:3001; set API URL to http://127.0.0.1:8001/api/v1/chat.
3) In the browser: confirm the Log Terminal connects to http://127.0.0.1:8001/api/v1/logs and shows gateway logs.
4) Send a chat message; confirm response and no console errors.
5) Provide proof of life: name one specific UI element you see (e.g. "Gateway log stream" header, chat input, CONNECTED status).
```

---

## 7) Integration checklist (for new Frontends)

Follow **§0) Step-by-step integration** for the full sequence. Minimum viable integration:

1. Call `GET /v1/status`; show `app_name`, `llm_mode`, and slot labels. Optionally call `GET /api/v1/health` for liveness.
2. Implement `POST /api/v1/chat` non-streaming request/response (JSON).
3. Add tenant identity: set `user_alias` (chat) and/or `tenant_id` (execute).
4. Show errors clearly (`status=error`, `status=policy_violation`).
5. If streaming is enabled, implement chunked streaming UI updates (current gateway sends plain text chunks).

Full-feature integration:

6. Add “KB panel” UI (9 slots + labels from `/v1/status`, status from `GET /api/v1/kb-status`).
7. Add “save to KB” and “search/query KB” flows (via `/v1/execute` goals or skills).
8. Add “logs/traces” view: connect to `GET /api/v1/logs` (SSE) for gateway logs.
9. Add memory-backed UI state (last prompt/response, pinned items, settings) per §4.
10. **Persona & Sentinel (Studio):** Connect to `GET /api/v1/persona/stream` (SSE) for persona_heartbeat, sentinel_update, sovereign_reset_suggested. Use `GET/POST /api/v1/settings/persona` for mode toggle. Show Warden velocity and Sentinel status; optionally `GET /api/v1/sentinel/domain-integrity` for Sovereign Domain Integrity.
11. **Wellness (Studio):** Use `POST /api/v1/soma/balance` for Spirit/Mind/Body check-in; `GET /api/v1/skills/wellness-report` for 7-day report and individuation score.

Verification: run pre-flight, start gateway, then confirm in browser one concrete UI element and zero 404/CORS errors (§0 Phase 3 and prompt §6.5).

---

## 8) Change management

This file is intended to be updated as the backend contract evolves.

Process:

* Any time `/v1/*` or `/api/v1/*` contracts change, update this doc in the same PR.
* When streaming protocol changes (plain chunked text ↔ SSE), update:
  * the backend handler ([`chat_streaming()`](add-ons/pagi-gateway/src/main.rs:1129))
  * the reference client ([`streamMessageToOrchestrator()`](add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts:46))
  * §2.3.3 and this document.

---

## 9) Troubleshooting Guide

This section provides comprehensive troubleshooting steps for common issues encountered during frontend-backend integration.

### 9.1 Port Conflicts

**Symptoms:**
- "Address already in use" error when starting the gateway
- "Port 3001 is in use, trying another one..." (Vite dev server)
- Connection refused errors in the browser

**Diagnosis:**
```bash
# Check which process is using a port
# Windows (PowerShell):
Get-NetTCPConnection -LocalPort 8001 | Select-Object OwningProcess
Get-Process -Id <PID>

# Linux/macOS:
lsof -i :8001
netstat -tulpn | grep 8001
```

**Solutions:**

1. **Stop the conflicting process:**
   ```bash
   # Windows (PowerShell):
   Stop-Process -Id (Get-NetTCPConnection -LocalPort 8001).OwningProcess -Force

   # Linux/macOS:
   fuser -k 8001/tcp
   kill -9 <PID>
   ```

2. **Use a different port:**
   - Edit `config/gateway.toml` and change `port = 8001` to another port in the 8001-8099 range
   - For Vite dev server, it will automatically try the next available port (3002, 3003, etc.)
   - Update your frontend API URL to match the new port

3. **Pre-flight verification:**
   ```bash
   cargo run -p pagi-gateway -- --verify
   ```
   This will check port availability and Sled DB locks before starting.

### 9.2 Sled Database Lock Issues

**Symptoms:**
- "Database is locked" or "Sled lock" errors
- Gateway fails to start with lock-related messages
- Multiple processes trying to access the same `data/` directory

**Diagnosis:**
```bash
# Check for running processes that might hold the lock
# Windows:
Get-Process | Where-Object {$_.ProcessName -like "*pagi*"}

# Linux/macOS:
ps aux | grep pagi
```

**Solutions:**

1. **Stop all PAGI processes:**
   ```bash
   # Windows (PowerShell):
   Get-Process | Where-Object {$_.ProcessName -like "*pagi*"} | Stop-Process -Force

   # Linux/macOS:
   pkill -f pagi
   ```

2. **Remove lock files (if processes are stopped but locks remain):**
   ```bash
   # Navigate to the data directory
   cd data

   # Remove lock files (CAUTION: Only do this if no processes are running)
   # Windows:
   del pagi_vault\conf\lock
   del pagi_knowledge\conf\lock

   # Linux/macOS:
   rm -f pagi_vault/conf/lock
   rm -f pagi_knowledge/conf/lock
   ```

3. **Use separate data directories for multiple instances:**
   - Set `PAGI__storage_path` to different paths for each instance
   - Example: `export PAGI__storage_path="./data_test"`

4. **Pre-flight check:**
   ```bash
   cargo run -p pagi-gateway -- --verify
   ```
   This will detect and report lock issues before attempting to start.

### 9.3 CORS (Cross-Origin Resource Sharing) Issues

**Symptoms:**
- Browser console shows CORS errors
- Network requests fail with "Access-Control-Allow-Origin" errors
- Frontend cannot connect to backend API

**Diagnosis:**
- Open browser DevTools (F12) → Console tab
- Look for red error messages mentioning CORS or Access-Control-Allow-Origin

**Solutions:**

1. **Verify CORS configuration in gateway:**
   - Check [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs) in the `build_app()` function
   - Ensure your frontend port is in the allowed range (3001-3099 for frontend, 8001-8099 for backend)

2. **Add your port to CORS allowlist:**
   ```rust
   // In add-ons/pagi-gateway/src/main.rs, build_app() function
   .allow_origin([
       "http://127.0.0.1:3001".parse().unwrap(),
       "http://127.0.0.1:3002".parse().unwrap(),
       "http://127.0.0.1:8001".parse().unwrap(),
       // Add your port here
   ])
   ```

3. **Rebuild and restart the gateway:**
   ```bash
   cargo build -p pagi-gateway
   cargo run -p pagi-gateway
   ```

4. **For development, consider using a proxy:**
   - Configure Vite to proxy requests to the gateway
   - Edit `vite.config.ts` in the Studio UI:
   ```typescript
   server: {
     proxy: {
       '/api': {
         target: 'http://127.0.0.1:8001',
         changeOrigin: true,
       }
     }
   }
   ```

### 9.4 Connection Errors

**Symptoms:**
- "Connection refused" or "Network error" in browser
- Frontend shows "Connection Error" or stays blank
- API calls timeout

**Diagnosis:**
```bash
# Test if the gateway is responding
curl http://127.0.0.1:8001/api/v1/health
curl http://127.0.0.1:8001/v1/status
```

**Solutions:**

1. **Verify the gateway is running:**
   ```bash
   # Check if the process is running
   # Windows:
   Get-Process | Where-Object {$_.ProcessName -like "*pagi-gateway*"}

   # Linux/macOS:
   ps aux | grep pagi-gateway
   ```

2. **Start the gateway if not running:**
   ```bash
   cargo run -p pagi-gateway
   ```

3. **Check the API URL in frontend settings:**
   - For Studio UI: Settings → Orchestrator Endpoint
   - Should be `http://127.0.0.1:8001/api/v1/chat` (direct to gateway)
   - Or `http://127.0.0.1:3001/api/v1/chat` (if using UI server proxy)

4. **Verify the correct port:**
   - Check `config/gateway.toml` for the configured port
   - Ensure the frontend is using the same port

5. **Check firewall settings:**
   - Ensure port 8001 (or your configured port) is not blocked
   - Windows: Check Windows Firewall settings
   - Linux: Check `ufw` or `iptables` rules

### 9.5 API Key Authentication Issues

**Symptoms:**
- `401 Unauthorized` responses from protected endpoints
- "unauthorized? Set PAGI_API_KEY if the endpoint is protected" error
- Sovereign Dashboard cannot fetch status

**Diagnosis:**
```bash
# Check if PAGI_API_KEY is set
echo $PAGI_API_KEY

# Test with curl (replace <your-key> with actual key)
curl -H "X-API-Key: <your-key>" http://127.0.0.1:8001/api/v1/sovereign-status
```

**Solutions:**

1. **Set the API key on the gateway:**
   ```bash
   # Linux/macOS:
   export PAGI_API_KEY="your-secret-api-key"

   # Windows (PowerShell):
   $env:PAGI_API_KEY="your-secret-api-key"

   # Or add to .env file:
   echo "PAGI_API_KEY=your-secret-api-key" >> .env
   ```

2. **Restart the gateway with the API key:**
   ```bash
   cargo run -p pagi-gateway
   ```

3. **Set the same API key when running the dashboard:**
   ```bash
   # Linux/macOS:
   export PAGI_API_KEY="your-secret-api-key"
   ./target/release/pagi status

   # Windows (PowerShell):
   $env:PAGI_API_KEY="your-secret-api-key"
   ./target/release/pagi.exe status
   ```

4. **For frontend requests, include the API key header:**
   ```javascript
   fetch('http://127.0.0.1:8001/api/v1/sovereign-status', {
     headers: {
       'X-API-Key': 'your-secret-api-key'
     }
   })
   ```

### 9.6 Shadow Key (Slot 9) Issues

**Symptoms:**
- Slot 9 (Shadow) shows as "LOCKED" in Sovereign Dashboard
- `503 Service Unavailable` when accessing `/v1/vault/read`
- "ShadowStore not initialized" errors

**Diagnosis:**
```bash
# Check if PAGI_SHADOW_KEY is set
echo $PAGI_SHADOW_KEY

# Verify it's 64 hex characters (32 bytes)
# Should be exactly 64 characters of 0-9 and a-f
```

**Solutions:**

1. **Generate a valid shadow key:**
   ```bash
   # Generate 64 hex characters (32 bytes)
   openssl rand -hex 32

   # Or using Python:
   python3 -c "import secrets; print(secrets.token_hex(32))"
   ```

2. **Set the shadow key:**
   ```bash
   # Linux/macOS:
   export PAGI_SHADOW_KEY="<64-char-hex-string>"

   # Windows (PowerShell):
   $env:PAGI_SHADOW_KEY="<64-char-hex-string>"

   # Or add to .env file:
   echo "PAGI_SHADOW_KEY=<64-char-hex-string>" >> .env
   ```

3. **Restart the gateway:**
   ```bash
   cargo run -p pagi-gateway
   ```

4. **Verify Slot 9 is unlocked:**
   ```bash
   ./target/release/pagi status
   # Look for "Slot 9 (Shadow): UNLOCKED"
   ```

### 9.7 Memory Locking Issues (Linux)

**Symptoms:**
- "mlock failed" or "memory lock" warnings in logs
- Shadow (Slot 9) content may be swapped to disk
- Performance issues with encrypted vault operations

**Diagnosis:**
```bash
# Check memory lock limits
ulimit -l

# Check if process has CAP_IPC_LOCK
getcap ./target/release/pagi-gateway
```

**Solutions:**

1. **Increase memory lock limit:**
   ```bash
   # Temporary (current session):
   ulimit -l unlimited

   # Permanent (add to ~/.bashrc or ~/.zshrc):
   echo "ulimit -l unlimited" >> ~/.bashrc
   ```

2. **Grant CAP_IPC_LOCK capability:**
   ```bash
   sudo setcap cap_ipc_lock+ep ./target/release/pagi-gateway
   ```

3. **For systemd service, add to unit file:**
   ```ini
   [Service]
   LimitMEMLOCK=infinity
   ```

4. **Run as root (not recommended for production):**
   ```bash
   sudo ./target/release/pagi-gateway
   ```

### 9.8 Streaming Chat Issues

**Symptoms:**
- Streaming responses don't display properly
- Chunks appear as raw text instead of formatted output
- SSE parsing errors in browser console

**Diagnosis:**
- Check browser DevTools → Network tab
- Look at the response headers and content type
- Verify the streaming implementation matches backend behavior

**Solutions:**

1. **Verify backend streaming behavior:**
   - Current gateway sends `Content-Type: text/plain; charset=utf-8`
   - Chunks are plain text, not SSE-formatted
   - See [`chat_streaming()`](add-ons/pagi-gateway/src/main.rs:1129)

2. **Update frontend to handle plain text chunks:**
   ```javascript
   // Example for Studio UI
   const response = await fetch('http://127.0.0.1:8001/api/v1/chat', {
     method: 'POST',
     headers: { 'Content-Type': 'application/json' },
     body: JSON.stringify({ prompt: 'Hello', stream: true })
   });

   const reader = response.body.getReader();
   const decoder = new TextDecoder();

   while (true) {
     const { done, value } = await reader.read();
     if (done) break;
     const chunk = decoder.decode(value);
     // Append chunk to UI (plain text, not SSE)
     appendToChat(chunk);
   }
   ```

3. **If you want SSE, update the backend:**
   - Modify [`chat_streaming()`](add-ons/pagi-gateway/src/main.rs:1129) to emit SSE events
   - Set `Content-Type: text/event-stream`
   - Format chunks as `data: <chunk>\n\n`

### 9.9 Knowledge Base (KB) Issues

**Symptoms:**
- KB status shows errors or missing slots
- Knowledge queries return no results
- Bootstrap data not initialized

**Diagnosis:**
```bash
# Check KB status
curl http://127.0.0.1:8001/api/v1/kb-status

# Check data directory
ls -la data/pagi_knowledge/
```

**Solutions:**

1. **Verify bootstraps ran:**
   - Check gateway logs for bootstrap messages
   - Look for "Core identity bootstrap", "Core skill registry", "Default ethos policy"

2. **Manually trigger bootstraps (if needed):**
   - Stop the gateway
   - Remove KB data: `rm -rf data/pagi_knowledge/`
   - Restart the gateway: `cargo run -p pagi-gateway`

3. **Check slot labels configuration:**
   - Verify `config/gateway.toml` has `[slot_labels]` section
   - Ensure all 9 slots are defined (slot_labels 1–8 in config plus KB-09 Shadow)

4. **Verify KB paths:**
   - Check `storage_path` in config
   - Ensure the gateway has read/write permissions

### 9.10 Frontend Build Issues

**Symptoms:**
- Vite dev server fails to start
- Build errors in Studio UI
- Missing dependencies

**Diagnosis:**
```bash
# Check Node.js and npm versions
node --version
npm --version

# Try to install dependencies
cd add-ons/pagi-studio-ui/assets/studio-interface
npm install
```

**Solutions:**

1. **Install Node.js dependencies:**
   ```bash
   cd add-ons/pagi-studio-ui/assets/studio-interface
   npm install
   ```

2. **Clear node_modules and reinstall:**
   ```bash
   rm -rf node_modules package-lock.json
   npm install
   ```

3. **Use a specific Node.js version:**
   ```bash
   # Install nvm (Node Version Manager) if needed
   # Then use the version specified in package.json
   nvm install <version>
   nvm use <version>
   ```

4. **Check for port conflicts:**
   - Vite will try ports 3001, 3002, 3003, etc.
   - If all are in use, stop conflicting processes or use a different range

### 9.11 Environment Variable Issues

**Symptoms:**
- Configuration not loading correctly
- Default values being used instead of overrides
- Path resolution errors

**Diagnosis:**
```bash
# Check environment variables
echo $PAGI_CONFIG
echo $PAGI__storage_path
echo $PAGI_SHADOW_KEY
echo $PAGI_API_KEY

# Check .env file
cat .env
```

**Solutions:**

1. **Verify .env file format:**
   ```bash
   # .env file should be in the workspace root
   # Format: KEY=value (no spaces around =)
   PAGI_CONFIG=config/gateway.toml
   PAGI__storage_path=./data
   PAGI_SHADOW_KEY=64-char-hex-string
   PAGI_API_KEY=your-api-key
   ```

2. **Source .env file before running:**
   ```bash
   # Linux/macOS:
   set -a && source .env && set +a && cargo run -p pagi-gateway

   # Windows (PowerShell):
   Get-Content .env | ForEach-Object {
     if ($_ -match '^([^=]+)=(.*)$') {
       [Environment]::SetEnvironmentVariable($matches[1], $matches[2])
     }
   }
   cargo run -p pagi-gateway
   ```

3. **Use dotenv in Rust (if configured):**
   - The gateway should automatically load `.env` if the `dotenv` crate is used
   - Check [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs) for dotenv initialization

4. **Verify variable naming:**
   - Config crate uses `PAGI` prefix and `__` separator
   - Example: `PAGI__port` overrides `port` in TOML
   - Example: `PAGI_CONFIG` sets the config file path

---

## 10) Updated Integration Prompts with Troubleshooting

### 10.1 Master end-to-end integration prompt (with troubleshooting)

```
Wire the PAGI backend, gateway, and engine to the frontend UI end-to-end.

Backend & gateway:
1) Run pre-flight: cargo run -p pagi-gateway -- --verify (port 8001, no Sled locks).
2) Start gateway: cargo run -p pagi-gateway from workspace root. Confirm GET /v1/status and GET /api/v1/health return expected JSON.
3) Slot labels and app identity come from config/gateway.toml; the UI should load them from GET /v1/status.

Frontend options:
- Drop-in UI: gateway serves pagi-frontend at http://127.0.0.1:8001/ when frontend_enabled=true. Frontend uses POST /v1/execute for goals (e.g. AutonomousGoal).
- Studio UI: run gateway on 8001 and npm run dev in add-ons/pagi-studio-ui/assets/studio-interface (port 3001). Set API URL to http://127.0.0.1:8001/api/v1/chat. Logs: http://127.0.0.1:8001/api/v1/logs.

API contract:
- Chat: POST /api/v1/chat with { prompt, stream, user_alias, model, temperature, max_tokens, persona }. Non-stream returns JSON; stream returns plain chunked text (Content-Type: text/plain).
- Execute: POST /v1/execute with { tenant_id, correlation_id?, goal }. Surface policy_violation and error status in the UI.
- Health: GET /api/v1/health. KB status: GET /api/v1/kb-status. Kardia: GET /api/v1/kardia/:user_id.

Verification:
- CORS allows Frontend ports 3001–3099 and Backend 8001–8099.
- In browser: confirm one concrete UI element and zero 404/CORS errors. If Connection Refused, re-run pre-flight and ensure only one process uses the same data/ path.

Troubleshooting checklist:
- Port conflicts: Use Get-NetTCPConnection (Windows) or lsof (Linux/macOS) to find conflicting processes. Stop them or use a different port.
- Sled DB locks: Stop all PAGI processes, remove lock files if needed, or use separate data directories via PAGI__storage_path.
- CORS errors: Verify your frontend port is in the allowed range (3001-3099) in add-ons/pagi-gateway/src/main.rs.
- Connection errors: Verify gateway is running, check API URL in frontend settings, ensure correct port is used.
- API key issues: Set PAGI_API_KEY on gateway and dashboard. Include X-API-Key header in requests.
- Shadow key issues: Generate 64-char hex key, set PAGI_SHADOW_KEY, restart gateway to unlock Slot 9.
- Memory locking (Linux): Set ulimit -l unlimited or grant CAP_IPC_LOCK capability.
- Streaming issues: Backend sends plain text chunks (Content-Type: text/plain), not SSE. Update frontend accordingly.
- KB issues: Check /api/v1/kb-status, verify bootstraps ran, check slot_labels in config/gateway.toml.
- Frontend build: Run npm install in studio-interface directory, clear node_modules if needed.
- Environment variables: Verify .env file format, source before running, check PAGI__ prefix for config overrides.

Deliver: a short runbook (steps + commands + URLs) and any code changes needed for this project.
```

### 10.2 Frontend integration kickoff prompt (with troubleshooting)

```
You are integrating a new Frontend UI with the PAGI backend gateway.

Requirements:
1) Implement GET /v1/status for app identity and slot label hydration; GET /api/v1/health for liveness.
2) Implement POST /api/v1/chat for non-streaming chat (JSON request/response).
3) Implement streaming chat using the backend's current behavior (plain chunked text; Content-Type: text/plain) OR document + implement SSE consistently end-to-end.
4) Surface policy_violation and error responses from POST /v1/execute.
5) Use user_alias (chat) and tenant_id (execute) consistently so Kardia and Chronos are tenant-scoped.

Troubleshooting requirements:
- Handle port conflicts gracefully (try alternate ports or prompt user)
- Detect and report CORS errors with clear guidance
- Show connection errors with diagnostic information
- Implement retry logic for transient failures
- Log all API errors with sufficient context for debugging

Provide:
* A minimal API client module (base URL configurable, e.g. http://127.0.0.1:8001)
* UI wiring examples for status, chat, and execute
* Error handling patterns with user-friendly messages
* A short integration checklist for QA
* Troubleshooting guide for common issues
```

### 10.3 Bridge/Gateway integration prompt (enhanced troubleshooting)

```
Diagnose why the Frontend cannot integrate with the PAGI gateway.

Check:
* Gateway is listening on the configured port (config/gateway.toml; default 8001).
* CORS in add-ons/pagi-gateway/src/main.rs allows the Frontend origin (ports 3001–3099 and 8001–8099).
* GET /v1/status returns expected JSON (app_name, port, llm_mode, slot_labels).
* GET /api/v1/health returns {"status":"ok","identity":"...","message":"..."}.
* POST /api/v1/chat with stream=false returns JSON (response, thought, status).
* For stream=true, the gateway sends raw text chunks (Content-Type: text/plain), not SSE.
* No Sled DB locks are held (run cargo run -p pagi-gateway -- --verify).
* No port conflicts (check with Get-NetTCPConnection or lsof).
* Environment variables are set correctly (PAGI_CONFIG, PAGI__storage_path, etc.).
* API key authentication if PAGI_API_KEY is set (include X-API-Key header).

Common issues to check:
- Port 8001 already in use → Stop conflicting process or use different port
- Sled DB locked → Stop all PAGI processes, remove lock files
- CORS blocked → Add frontend port to CORS allowlist
- Connection refused → Verify gateway is running, check API URL
- 401 Unauthorized → Set PAGI_API_KEY and include in requests
- 403 Forbidden → Check PAGI_SHADOW_KEY for vault operations
- 503 Service Unavailable → Verify ShadowStore initialized with PAGI_SHADOW_KEY

Return:
* Root cause
* Concrete fixes (config + code pointers)
* Verification steps (curl or browser)
* Preventive measures for future issues
```

### 10.4 KB integration prompt (with troubleshooting)

```
Add a Frontend feature to store and retrieve knowledge.

Constraints:
* The backend has 9 KB slots (KB-01 through KB-09); UI must display slot labels from GET /v1/status (slot_labels).
* Use GET /api/v1/kb-status for status of all 9 Knowledge Bases.
* Prefer calling stable skills via POST /v1/execute rather than hardcoding Rust Goal JSON variants.

Troubleshooting considerations:
- Handle KB initialization errors (check if bootstraps ran)
- Detect and report slot label mismatches
- Handle query failures gracefully
- Implement retry logic for transient KB errors
- Show KB status with clear indicators (active/inactive, record counts)

Deliver:
* A UI flow: pick slot (or route via Thalamus), choose key, write record, then read it back.
* A test plan that verifies data persists across gateway restarts.
* Error handling for common KB issues (slot not found, write failures, etc.).
* Troubleshooting guide for KB-related problems.
```

### 10.5 Memory integration prompt (with troubleshooting)

```
Integrate short-term memory for the UI.

Goal:
* Persist last prompt, last response, selected model, and user settings per tenant.

Reference:
* Studio uses memory paths like "studio/last_prompt" and "studio/last_response" (see add-ons/pagi-studio-ui/src/app.rs MEMORY_PROMPT_PATH / MEMORY_RESPONSE_PATH).

Troubleshooting considerations:
- Handle memory write failures gracefully
- Detect and report memory path conflicts
- Implement fallback for memory unavailability
- Show memory status in UI (connected/disconnected)
- Cache memory locally for offline capability

Deliver:
* A list of memory paths for this Frontend
* A backend interaction plan (memory API or skill)
* A migration plan for future schema changes
* Error handling patterns for memory operations
* Troubleshooting guide for memory-related issues
```

### 10.6 Verification and proof-of-life prompt (enhanced)

```
Verify the frontend–backend integration is working.

Steps:
1) Run cargo run -p pagi-gateway -- --verify; then cargo run -p pagi-gateway.
2) For Studio UI: in another terminal, npm run dev in add-ons/pagi-studio-ui/assets/studio-interface; open http://127.0.0.1:3001; set API URL to http://127.0.0.1:8001/api/v1/chat.
3) In the browser: confirm the Log Terminal connects to http://127.0.0.1:8001/api/v1/logs and shows gateway logs.
4) Send a chat message; confirm response and no console errors.
5) Provide proof of life: name one specific UI element you see (e.g. "Gateway log stream" header, chat input, CONNECTED status).

Troubleshooting verification:
- Check browser console for 404/CORS errors
- Verify network requests succeed (200 OK responses)
- Confirm streaming works (chunks appear in real-time)
- Check KB status endpoint returns valid data
- Verify logs endpoint streams gateway output
- Test error handling (send invalid request, check error message)

If issues found:
- Identify the specific component failing (gateway, frontend, network)
- Check the relevant troubleshooting section in docs/frontend-backend-integration.md
- Apply the appropriate fix from the troubleshooting guide
- Re-run verification steps to confirm resolution
```

### 10.7 Pre-flight verification prompt

```
Run comprehensive pre-flight checks before starting the PAGI gateway.

Checks to perform:
1) Port availability: Verify port 8001 (or configured port) is not in use
2) Sled DB locks: Check for lock files in data/pagi_vault/ and data/pagi_knowledge/
3) Process conflicts: Ensure no other PAGI processes are running
4) Configuration: Verify config/gateway.toml exists and is valid
5) Environment: Check PAGI_CONFIG, PAGI__storage_path, PAGI_SHADOW_KEY, PAGI_API_KEY
6) Permissions: Verify read/write access to data/ directory
7) Dependencies: Confirm all required crates are available

Commands to run:
```bash
# Pre-flight check
cargo run -p pagi-gateway -- --verify

# Manual checks
# Check port
# Windows: Get-NetTCPConnection -LocalPort 8001
# Linux/macOS: lsof -i :8001

# Check processes
# Windows: Get-Process | Where-Object {$_.ProcessName -like "*pagi*"}
# Linux/macOS: ps aux | grep pagi

# Check locks
ls -la data/pagi_vault/conf/lock
ls -la data/pagi_knowledge/conf/lock

# Check config
cat config/gateway.toml

# Check environment
echo $PAGI_CONFIG
echo $PAGI__storage_path
echo $PAGI_SHADOW_KEY
echo $PAGI_API_KEY
```

If any check fails:
- Apply the appropriate fix from the troubleshooting guide
- Re-run the failed check to confirm resolution
- Only proceed to start the gateway after all checks pass
```

---

## 11) Quick Reference Troubleshooting Commands

### Windows (PowerShell)

```powershell
# Check port usage
Get-NetTCPConnection -LocalPort 8001

# Stop process by port
Stop-Process -Id (Get-NetTCPConnection -LocalPort 8001).OwningProcess -Force

# Check PAGI processes
Get-Process | Where-Object {$_.ProcessName -like "*pagi*"}

# Stop all PAGI processes
Get-Process | Where-Object {$_.ProcessName -like "*pagi*"} | Stop-Process -Force

# Set environment variables
$env:PAGI_CONFIG="config/gateway.toml"
$env:PAGI__storage_path="./data"
$env:PAGI_SHADOW_KEY="<64-char-hex-string>"
$env:PAGI_API_KEY="<your-api-key>"

# Run gateway with environment
$env:PAGI_CONFIG="config/gateway.toml"; cargo run -p pagi-gateway

# Pre-flight check
cargo run -p pagi-gateway -- --verify

# Test API
curl http://127.0.0.1:8001/api/v1/health
curl http://127.0.0.1:8001/v1/status
```

### Linux/macOS

```bash
# Check port usage
lsof -i :8001
netstat -tulpn | grep 8001

# Stop process by port
fuser -k 8001/tcp
kill -9 $(lsof -t -i:8001)

# Check PAGI processes
ps aux | grep pagi

# Stop all PAGI processes
pkill -f pagi

# Set environment variables
export PAGI_CONFIG="config/gateway.toml"
export PAGI__storage_path="./data"
export PAGI_SHADOW_KEY="<64-char-hex-string>"
export PAGI_API_KEY="<your-api-key>"

# Run gateway with environment
PAGI_CONFIG="config/gateway.toml" cargo run -p pagi-gateway

# Source .env file
set -a && source .env && set +a && cargo run -p pagi-gateway

# Pre-flight check
cargo run -p pagi-gateway -- --verify

# Test API
curl http://127.0.0.1:8001/api/v1/health
curl http://127.0.0.1:8001/v1/status

# Generate shadow key
openssl rand -hex 32
python3 -c "import secrets; print(secrets.token_hex(32))"

# Check memory lock limits
ulimit -l

# Grant CAP_IPC_LOCK
sudo setcap cap_ipc_lock+ep ./target/release/pagi-gateway
```

---

## 12) Astro-Logic & Defensive Toggles Configuration

Phoenix Marie uses **Celestial Transits** as a proxy for environmental volatility. Frontend UIs should expose these configuration options:

### Environment Variables (.env)

```bash
# Astro-Logic Engine
PAGI_ASTRO_LOGIC_ENABLED=true          # Toggles archetype directives
PAGI_TRANSIT_ALERTS_ENABLED=true       # Toggles background weather scraper

# Skill Governance
PAGI_SKILLS_AUTO_PROMOTE_ALLOWED=false # Prevents AI self-promotion (default: false)

# Technical Mode
PAGI_STRICT_TECHNICAL_MODE=true        # Forces deterministic 0.3 temperature
```

### Frontend Integration

Settings panels should include:

1. **Astro-Logic Toggle**: Enable/disable celestial transit influence on agent behavior
2. **Transit Alerts**: Enable/disable real-time planetary weather scraping
3. **Auto-Promote Skills**: (Dangerous) Allow AI to self-promote Tier 3 skills
4. **Strict Technical Mode**: Force low-temperature deterministic responses

### Social Defense Modes

When KB-05 (Sovereignty) triggers are detected, Phoenix automatically engages:

* **Grey Rock**: Minimal, non-engaging responses to boundary violations
* **Defensive**: Protective stance with clear boundary enforcement

Frontend UIs should display the current defense mode in the status bar or header.

---

## 13) Common Error Messages and Solutions

| Error Message | Likely Cause | Solution |
|---------------|--------------|----------|
| `Address already in use` | Port conflict | Stop conflicting process or use different port |
| `Database is locked` | Sled DB lock | Stop all PAGI processes, remove lock files |
| `Access-Control-Allow-Origin` | CORS error | Add frontend port to CORS allowlist |
| `Connection refused` | Gateway not running | Start the gateway |
| `401 Unauthorized` | Missing API key | Set PAGI_API_KEY and include in requests |
| `403 Forbidden` | Invalid shadow key | Set valid PAGI_SHADOW_KEY (64 hex chars) |
| `503 Service Unavailable` | ShadowStore not initialized | Set PAGI_SHADOW_KEY and restart gateway |
| `mlock failed` | Memory lock limit exceeded | Increase ulimit or grant CAP_IPC_LOCK |
| `Slot 9 (Shadow): LOCKED` | No shadow key set | Set PAGI_SHADOW_KEY and restart gateway |
| `Skill tier violation` | Tier 3 skill accessing restricted KB | Promote skill to Core or use Tier 2 skill |
| `Capability overreach detected` | AI attempting unauthorized KB access | Check KB-08 audit log, review skill permissions |
| `Port 3001 is in use` | Vite port conflict | Stop conflicting process or Vite will try next port |
| `ENOENT: no such file` | Missing config/data | Verify paths, create directories if needed |
| `Permission denied` | Insufficient permissions | Check file/directory permissions |
| `EADDRINUSE` | Port in use | Stop conflicting process or use different port |

---

## 14) Phoenix Marie: Unique Architecture Features

### Bare Metal Philosophy

Phoenix Marie is designed for **direct hardware execution** with zero containerization:

* **No Docker**: Maximum performance and security through native execution
* **Rust Core**: Type-safe memory management prevents data leaks between threads and knowledge layers
* **Local-First**: All memory resides on local disk persistence (Sled)

### Sovereignty Firewall

The **3-Tier Skill Model** enforces strict access control:

* **Core Skills (Tier 1)**: User-signed, full KB access
* **Import Skills (Tier 2)**: Standard skills, limited KB access
* **Generated Skills (Tier 3)**: AI-drafted, firewall-blocked until promoted

All unauthorized access attempts are logged in **KB-08 (Health)** for audit.

### Astro-Logic Engine

Phoenix is the first AGI that uses **Celestial Transits** as environmental volatility indicators:

* Real-time planetary scraping modifies "Caution Level"
* Automatic engagement of defensive personas (Grey Rock, Defensive)
* Configurable via `PAGI_ASTRO_LOGIC_ENABLED` and `PAGI_TRANSIT_ALERTS_ENABLED`

### 9-Layer Memory Taxonomy

Unlike traditional 8-slot systems, Phoenix uses **9 sovereignty-gated knowledge bases**:

* **KB-01 (Ethos)**: Core identity - CORE ONLY access
* **KB-09 (Shadow)**: Private PII - CORE ONLY access
* **KB-05 (Sovereignty)**: Social defense triggers - RESTRICTED
* **KB-08 (Health)**: Audit logs and metrics - RESTRICTED

### Frontend Implementation Checklist

When building a Phoenix Marie frontend:

- [ ] Display all 9 KB slots with firewall status indicators
- [ ] Implement Warden "Promote" button for Tier 3 → Core elevation
- [ ] Show KB-08 audit log for capability overreach events
- [ ] Expose Astro-Logic toggles in settings
- [ ] Display current defense mode (Grey Rock/Defensive) in status bar
- [ ] Handle skill tier violations gracefully with user-friendly messages
- [ ] Implement secure shadow key input for KB-09 access
- [ ] Show visual distinction between CORE ONLY, RESTRICTED, and Open KBs

### Reference Implementation

The Studio UI (`add-ons/pagi-studio-ui/assets/studio-interface`) demonstrates all Phoenix Marie features:

* 9-layer KB sidebar with firewall badges
* Warden panel with skill promotion workflow
* Astro-Logic configuration in settings
* Health report with audit log visualization
* Secure shadow vault access

---

## 15) Additional Resources

* **Main README**: [`README.md`](../README.md) - Phoenix Marie overview and getting started
* **Deployment Guide**: [`docs/DEPLOYMENT.md`](DEPLOYMENT.md) - Bare-metal deployment with security hardening
* **Architecture**: [`docs/BARE_METAL_ARCHITECTURE.md`](BARE_METAL_ARCHITECTURE.md) - System architecture deep dive
* **Forge Safety Governor**: [`FORGE_SAFETY_GOVERNOR.md`](../FORGE_SAFETY_GOVERNOR.md) - Human-in-the-loop approval for self-modification; kill-switch usage
* **Sovereign Autonomy System**: [`SOVEREIGN_AUTONOMY_SYSTEM.md`](../SOVEREIGN_AUTONOMY_SYSTEM.md) - Runtime Forge safety control, auto-revert on failure, HITL vs autonomous modes
* **Vector KB**: [`VECTORKB_ACTIVATION_GUIDE.md`](../VECTORKB_ACTIVATION_GUIDE.md), [`VECTORKB_PRODUCTION_HARDENING.md`](../VECTORKB_PRODUCTION_HARDENING.md) - Optional vector store (Qdrant) activation and hardening
* **Knowledge Base Structure**: [YouTube - Structuring AI Knowledge Bases](https://www.youtube.com/watch?v=LZ0E7bjVv0s)

---

## 16) Integration Checklist (Enhanced)

Follow **§0) Step-by-step integration** for the full sequence. Minimum viable integration:

1. Call `GET /v1/status`; show `app_name`, `llm_mode`, and slot labels. Optionally call `GET /api/v1/health` for liveness.
2. Implement `POST /api/v1/chat` non-streaming request/response (JSON).
3. Add tenant identity: set `user_alias` (chat) and/or `tenant_id` (execute).
4. Show errors clearly (`status=error`, `status=policy_violation`).
5. If streaming is enabled, implement chunked streaming UI updates (current gateway sends plain text chunks).
6. **Pre-flight verification**: Run `cargo run -p pagi-gateway -- --verify` before starting.
7. **Port conflict check**: Verify no conflicts on ports 8001 and 3001.
8. **CORS verification**: Confirm frontend port is in allowed range.
9. **Error handling**: Implement graceful error handling for all API calls.
10. **Connection retry**: Add retry logic for transient failures.

Full-feature integration:

11. Add "KB panel" UI (9 slots + labels from `/v1/status`, status from `GET /api/v1/kb-status`).
12. Add "save to KB" and "search/query KB" flows (via `/v1/execute` goals or skills).
13. Add "logs/traces" view: connect to `GET /api/v1/logs` (SSE) for gateway logs.
14. Add memory-backed UI state (last prompt/response, pinned items, settings) per §4.
15. **Persona & Sentinel**: `GET /api/v1/persona/stream` (SSE), `GET/POST /api/v1/settings/persona`, Warden velocity/Sentinel badge, `GET /api/v1/sentinel/domain-integrity` (see §0d).
16. **Wellness**: `POST /api/v1/soma/balance`, `GET /api/v1/skills/wellness-report`; Balance modal and Wellness tab (see §0d).
17. **API key support**: Handle `X-API-Key` header for protected endpoints.
18. **Shadow vault support**: Implement secure journal viewing with shadow key.
19. **Troubleshooting UI**: Add diagnostic panel showing connection status, errors, and suggested fixes.
20. **Configuration UI**: Allow users to set API URL, ports, and other settings.

Verification: run pre-flight, start gateway, then confirm in browser one concrete UI element and zero 404/CORS errors (§0 Phase 3 and prompt §6.5). Run through troubleshooting checklist to ensure all common issues are addressed.

---

## 17) Support and Resources

### Documentation
- [README.md](../README.md) - Project overview and quick start
- [docs/DEPLOYMENT.md](DEPLOYMENT.md) - Bare-metal deployment guide
- [docs/BARE_METAL_ARCHITECTURE.md](BARE_METAL_ARCHITECTURE.md) - Architecture overview
- [docs/PROJECT_ANATOMY.md](PROJECT_ANATOMY.md) - Project structure
- [docs/WORKSPACE_HEALTH_REPORT.md](WORKSPACE_HEALTH_REPORT.md) - Workspace verification
- [FORGE_SAFETY_GOVERNOR.md](../FORGE_SAFETY_GOVERNOR.md) - Forge HITL approval gate and emergency kill switch
- [SOVEREIGN_AUTONOMY_SYSTEM.md](../SOVEREIGN_AUTONOMY_SYSTEM.md) - Runtime Forge safety control and auto-revert
- [VECTORKB_ACTIVATION_GUIDE.md](../VECTORKB_ACTIVATION_GUIDE.md) - Vector KB (Qdrant) activation
- [VECTORKB_PRODUCTION_HARDENING.md](../VECTORKB_PRODUCTION_HARDENING.md) - Vector KB production hardening

### Key Files
- [`add-ons/pagi-gateway/src/main.rs`](../add-ons/pagi-gateway/src/main.rs) - Gateway implementation
- [`config/gateway.toml`](../config/gateway.toml) - Gateway configuration
- [`pagi-frontend/app.js`](../pagi-frontend/app.js) - Drop-in UI implementation
- [`add-ons/pagi-studio-ui/assets/studio-interface/src/api/config.ts`](../add-ons/pagi-studio-ui/assets/studio-interface/src/api/config.ts) - Studio UI API base URL (GATEWAY_ORIGIN, API_BASE_URL)
- [`add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts`](../add-ons/pagi-studio-ui/assets/studio-interface/services/apiService.ts) - Studio UI chat API client
- [`add-ons/pagi-studio-ui/assets/studio-interface/types.ts`](../add-ons/pagi-studio-ui/assets/studio-interface/types.ts) - Studio UI types (GatewayFeatureConfig, WellnessReport, etc.)
- [`add-ons/pagi-studio-ui/assets/studio-interface/App.tsx`](../add-ons/pagi-studio-ui/assets/studio-interface/App.tsx) - Studio UI root (persona stream SSE, toasts, view state)

### Getting Help
1. Check this document's troubleshooting section (§9)
2. Review error messages and match to common errors table (§12)
3. Run pre-flight check: `cargo run -p pagi-gateway -- --verify`
4. Check browser DevTools for network errors and console messages
5. Verify environment variables and configuration files
6. Review logs from the gateway terminal

### Reporting Issues
When reporting integration issues, include:
- OS and version
- Rust version (`rustc --version`)
- Node.js version (`node --version`)
- Gateway configuration (redact sensitive data)
- Full error messages and stack traces
- Browser console errors
- Network request/response details
- Steps to reproduce the issue


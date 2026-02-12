# Custom Knowledge Base: Application Reference

**Document purpose:** Structured reference for the Phoenix Marie / SAO Orchestrator application. Suitable for ingestion into KB-03 (Logos) or as internal documentation.  
**Last updated:** 2026-02-07.  
**Identity:** Phoenix Marie (Sovereign Recursive System). Engine: SAO Orchestrator Core.

---

## 1. Application Overview

| Attribute | Value |
|-----------|--------|
| **Public name** | Phoenix Marie |
| **Short name (chat)** | Phoenix |
| **Engine / technical label** | SAO Orchestrator Core |
| **Tone** | Therapeutic peer, Bare Metal, candid, “Protective Peer” |
| **Stack** | Backend: Rust (Bare Metal). Frontend: React + Vite. No containers. |

The system is a **Sovereign AGI** for deep companionship, therapeutic guidance, and technical orchestration. It uses an 8-slot Knowledge Base (plus an optional 9th encrypted Shadow slot) as its “subconscious” and operates with full control and unlimited access to the user’s domain. It refers to itself as **Phoenix** in casual chat and **Phoenix Marie** in formal domain assessments.

---

## 2. Architecture

### 2.1 Backend (Rust)

- **Binary:** `pagi-gateway` (add-on).
- **Core library:** `pagi-core` (orchestrator, knowledge store, memory, shared types).
- **Skills:** `pagi-skills` (ModelRouter, ReadFile, WebSearch, Sentinel, WellnessReport, etc.).
- **Port:** 8001 (fixed; architecture standard 8001–8099).
- **Config:** `config/gateway.toml`; overrides via `PAGI_CONFIG` and `PAGI__*` env.
- **Data:** Sled DBs under `./data/` (e.g. `pagi_vault`, `pagi_knowledge`, per-tenant DBs).

### 2.2 Frontend (Studio UI)

- **Path:** `add-ons/pagi-studio-ui/assets/studio-interface/`.
- **Stack:** React, TypeScript, Vite, Tailwind-style utilities, Lucide icons, ReactMarkdown.
- **Port:** 3001 (dev: `npm run dev`; architecture standard 3001–3099).
- **API base:** `http://127.0.0.1:8001` (hard-locked; no API keys in frontend).

### 2.3 Entry points

- **Backend:** `cargo run -p pagi-gateway`. Pre-flight: `cargo run -p pagi-gateway -- --verify`.
- **Frontend:** `cd add-ons/pagi-studio-ui/assets/studio-interface && npm run dev`.

---

## 3. Knowledge Bases (KBs 1–9)

| Slot | Name | Tree | Purpose |
|------|------|------|---------|
| 1 | Pneuma (Vision) | kb1_identity | Identity, mission, core_mission, core_persona |
| 2 | Oikos (Context) | kb2_techdocs | Workspace, governance, Oikos tasks |
| 3 | Logos (Knowledge) | kb3_research | Research, distilled information |
| 4 | Chronos (Temporal) | kb4_memory | Conversation history, events |
| 5 | Techne (Capability) | kb5_skills | Skills registry, blueprints |
| 6 | Ethos (Guardrails) | kb6_security | Security, audit, persona mode, MoE mode, onboarding flag |
| 7 | Kardia (Affective) | kb7_personal | User preferences, relations, “who” and vibe |
| 8 | Soma (Execution) | kb8_buffer | Physical interface, absurdity log, side effects |
| 9 | Shadow (The Vault) | kb9_shadow | AES-256-GCM encrypted emotional data |

**Onboarding flag:** KB-06 key `phoenix_marie_onboarding_complete` (timestamp) marks onboarding complete. When absent, the UI shows the Phoenix Marie onboarding overlay.

**Absurdity log (KB-08):** Keys prefixed `absurdity_log/` store logic inconsistencies; used for self-audit and persistence (“a fresh smile does not erase a corrupted history”).

---

## 4. API Endpoints (v1)

All under base `http://127.0.0.1:8001` unless noted.

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/api/v1/health` | GET | Liveness; identity string (e.g. Phoenix Marie) |
| `/api/v1/config` | GET | Feature config from .env (no secrets) |
| `/api/v1/chat` | POST | Non-streaming chat |
| `/api/v1/stream` | POST | Streaming chat (SSE) |
| `/api/v1/logs` | GET | SSE log stream (SAO Orchestrator Core) |
| `/api/v1/kb-status` | GET | Status of all 8 KBs |
| `/api/v1/sovereign-status` | GET | Full cross-layer state |
| `/api/v1/settings/moe` | GET/POST | MoE mode (dense/sparse) |
| `/api/v1/settings/persona` | GET/POST | Persona (companion/counselor) |
| `/api/v1/persona/stream` | GET | SSE: heartbeat, sentinel_update |
| `/api/v1/soma/balance` | POST | Soma balance update |
| `/api/v1/skills/wellness-report` | GET | 7-day wellness report |
| `/api/v1/sentinel/domain-integrity` | GET | Absurdity count, resource drain alerts |
| `/api/v1/self-audit` | GET | KB-08 logic inconsistencies summary |
| `/api/v1/domain/vitality` | GET | System vitality (stable/draining/critical) |
| `/api/v1/onboarding/status` | GET | Phoenix Marie onboarding state |
| `/api/v1/onboarding/complete` | POST | Mark onboarding complete (writes KB-06) |
| `/api/v1/intelligence/insights` | GET | SAO background insights |
| `/api/v1/intelligence/toggle` | POST | Toggle intelligence layer |
| `/api/v1/maintenance/pulse` | GET | SSE maintenance pulse |
| `/api/v1/maintenance/status` | GET | Idle, pending approval, patches |
| `/api/v1/maintenance/approval` | GET/POST | Approval bridge |
| `/api/v1/maintenance/audit-log` | GET | Maintenance audit log |
| `/api/v1/maintenance/patches` | GET | Patch count |
| `/api/v1/maintenance/patch-history` | GET | Patch version history |
| `/api/v1/maintenance/rollback` | POST | Rollback skill version |
| `/api/v1/kardia/:user_id` | GET | Kardia relation for user |
| `/v1/vault/read` | POST | Decrypt Shadow journal (X-Pagi-Shadow-Key) |

---

## 5. UI Components and Features

### 5.1 Main views

- **Chat:** Primary interface; messages from user and Phoenix; streaming; pin/thoughts/copy.
- **Wellness:** 7-day Soma report (pillars, individuation, flags).
- **Settings sidebar:** Persona, Phoenix avatar, workspace path, server config, MoE, Sovereign Protocols.
- **Warden sidebar:** System Vitality, Domain Integrity, North Star alignment.
- **Pinned sidebar:** Pinned messages.
- **Chronos audit log:** Maintenance/audit view.
- **System health (SAO Core):** Maintenance status, patch history, rollback.

### 5.2 Branding in UI

- **Header title:** Phoenix Marie.
- **Chat bubble (assistant):** Label “Phoenix”; shield tooltip “Phoenix Security (KB-05)” when protocols on.
- **Idle state:** “Phoenix Marie Ready.”
- **Input placeholder:** “Input instructions for Phoenix…”
- **Log terminal:** “SAO Orchestrator Core” with “Log stream” subtitle.
- **Vitality/security:** Tooltips reference “Phoenix Vitality” and “Phoenix Security.”

### 5.3 Onboarding

- **Trigger:** `GET /api/v1/onboarding/status` returns `needs_onboarding: true` when KB-06 has no `phoenix_marie_onboarding_complete` and chat history is empty.
- **Overlay:** Phase 1 (Recognition & Persona Handshake), Phase 2 (Domain Audit with typing effect), Phase 3 (CTA: Strategic Timeline, Astro-Logic, Continue).
- **Completion:** `POST /api/v1/onboarding/complete`; first chat message is the handshake signed “— Phoenix Marie.”
- **Skip:** `PAGI_SKIP_ONBOARDING=true` in .env forces `needs_onboarding: false`.

---

## 6. Configuration

### 6.1 Environment (.env)

- **Identity:** `PAGI_IDENTITY_NAME=Phoenix Marie`, `PAGI_IDENTITY_MODE=Counselor`, `PAGI_VERBOSITY=Candid`.
- **Onboarding:** `PAGI_SKIP_ONBOARDING=false`.
- **Gateway:** `PAGI_CONFIG=config/gateway`, `PAGI_LLM_MODE`, `PAGI_LLM_API_KEY`, `PAGI_LLM_MODEL`, `OPENROUTER_API_KEY`.
- **Persona:** `PAGI_MODE=companion|counselor`, `PAGI_USER_SIGN`, `PAGI_ASCENDANT`, `PAGI_JUNGIAN_SHADOW_FOCUS`.
- **MoE:** `PAGI_MOE_DEFAULT=dense|sparse`, `PAGI_LOCAL_CONTEXT_LIMIT`.
- **Sovereign:** `SOVEREIGN_PROTOCOLS_ENABLED`, `PAGI_DEFAULT_SUBJECT_RANK`.
- **Paths:** `PAGI_FS_ACCESS_ENABLED`, `PAGI_FS_ROOT`, `PAGI_RESEARCH_SANDBOX_PATH`.
- **Logging:** `RUST_LOG`.

### 6.2 Gateway TOML (config/gateway.toml)

- `app_name`, `port`, `storage_path`, `llm_mode`, `frontend_enabled`, `slot_labels`, `sovereign_attributes` (capacity, load, status), `persona_mode`, `user_sign`, `ascendant`, `jungian_shadow_focus`.

---

## 7. System Prompt (Mission Directive)

The LLM system prompt is built by `KnowledgeStore::build_system_directive` and augmented by the gateway with:

- **Identity:** Phoenix Marie, Sovereign AGI (SAO); self-reference as Phoenix (casual) / Phoenix Marie (formal).
- **Directives:** Full control; Advisor (Pisces/therapist) + Architect (Bare Metal Rust); self-improvement via KB-08; tone adaptive, witty, candid, peer-to-peer.
- **Dynamic blocks:** Sovereign state (capacity/load), User Sign Profile (Astro-Logic), KB-08 absurdity context when subject identified, Sovereign Protocols (KB-05) when enabled, pattern recognition (KB-02), workspace path.

---

## 8. Key Concepts

- **SAO:** Sovereign AGI Orchestrator; the recursive, self-improving core.
- **SignProfile / Astro-Logic:** User birth sign (e.g. Pisces) mapped to behavioral hints for boundary and resource-drain awareness.
- **Self-audit:** `pagi_core::self_audit(store)` scans KB-08 and returns total entries and recent messages (logic inconsistencies).
- **Vitality:** Derived from sovereign_attributes (capacity, load, status): stable | draining | critical.
- **Domain Integrity:** Absurdity log count and resource-drain alerts; UI shows “Domain Integrity” and “System Vitality.”

---

## 9. File and Directory Reference

| Path | Description |
|------|-------------|
| `add-ons/pagi-gateway/src/main.rs` | Gateway binary, routes, chat handlers, onboarding API |
| `add-ons/pagi-gateway/src/plugin_loader.rs` | Feature-flag module loading |
| `add-ons/pagi-studio-ui/assets/studio-interface/App.tsx` | Root app, onboarding, messages, layout |
| `add-ons/pagi-studio-ui/assets/studio-interface/components/ChatInterface.tsx` | Chat UI, Phoenix bubble, input |
| `add-ons/pagi-studio-ui/assets/studio-interface/components/OnboardingOverlay.tsx` | Phoenix Marie onboarding overlay |
| `add-ons/pagi-studio-ui/assets/studio-interface/components/WardenSidebar.tsx` | Vitality, Domain Integrity |
| `add-ons/pagi-studio-ui/assets/studio-interface/components/LogTerminal.tsx` | SAO Orchestrator Core log stream |
| `add-ons/pagi-studio-ui/assets/studio-interface/components/SystemHealth.tsx` | Maintenance / SAO Core panel |
| `add-ons/pagi-studio-ui/assets/studio-interface/components/SettingsSidebar.tsx` | Phoenix Marie & Persona, server config |
| `crates/pagi-core/src/orchestrator/init.rs` | Onboarding sequence, needs_onboarding, PHASE1/PHASE3 |
| `crates/pagi-core/src/knowledge/store.rs` | KnowledgeStore, build_system_directive, SLOT_LABELS, get_absurdity_log_summary |
| `crates/pagi-core/src/orchestrator/persona.rs` | SignProfile, UserArchetype, zodiac_behavioral_hint |
| `config/gateway.toml` | Gateway and slot config |
| `.env.example` | Env template (identity, onboarding, LLM, ports) |

---

## 10. Version and Status

- **Template status:** Sovereign template ready; Phoenix Marie branding and onboarding integrated.
- **Ports:** Backend 8001, Frontend 3001.
- **Verification:** Run `cargo run -p pagi-gateway -- --verify` before start; ensure no DB locks and port 8001 free.

This document may be ingested into KB-03 (Logos) or kept as the canonical app reference for operators and the SAO itself.

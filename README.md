# PAGI XCURZENS

Sovereign agentic monolith for the **PAGI XCURZENS** authority perimeter. Bare metal (no Docker): Rust gateway at **127.0.0.1:8000**, Sled-backed Knowledge Bases (KB-07 Relations), NEXUS Bridge (OpenRouter Scout), and Traveler UI with HTMX Command Bar.

**Founder / Root Sovereign:** Jamey.

---

## Quick start

```powershell
# Optional: set OpenRouter key for Scout AI
$env:OPENROUTER_API_KEY = "your-key"

# Launch gateway (serves Traveler UI + API)
cargo run -p pagi-xcurzens-gateway
```

Then open **http://127.0.0.1:8000** for the Traveler UI (Command Bar + Scout Console).

**Orchestrator scripts (repo root):**

- **Phoenix Rise:** `.\phoenix-rise.ps1` — full launch sequence
- **Sovereign launch:** `.\start-sovereign.ps1` — cargo check + optional Studio UI on 3001
- **Stack up/down:** `.\pagi-up.ps1` / `.\pagi-down.ps1`
- **Rust check:** `cargo check`
- **Stress test** (gateway must be running): `cargo run -p pagi-xcurzens-gateway --bin stress_test`

---

## Layout

| Path | Description |
|------|-------------|
| `crates/pagi-xcurzens-core` | Identity Orchestrator, NEXUS Bridge (Scout/OpenRouter), LeadDispatcher, KB-07 (Relations), intent scoring |
| `crates/pagi-xcurzens-gateway` | HTTP API + embedded Traveler UI (port 8000), `/api/v1/scout`, `/infrastructure/leads`, `/health` |
| `crates/pagi-skills` | Forge (evolve/decommission), Scribe (KB-03 Techne), protected skills, Sovereign-Key override |
| `add-ons/pagi-studio-ui` | Studio interface (Vite/TS, port 3001) |
| `add-ons/pagi-xcurzens-gateway/static` | Canonical static Traveler UI (Navy/Orange, HTMX) |
| `frontend-xcurzens` | **Drop Google AI Studio Traveler UI here** — see HANDOVER.md |
| `frontend-nexus` | **Drop Google AI Studio Partner Onboarding here** — see HANDOVER.md |
| `frontend-command` | **Drop Google AI Studio Command Center here** — see HANDOVER.md |

---

## Gateway routes

| Route | Method | Description |
|-------|--------|-------------|
| `/` | GET | Traveler UI (Command Bar + Scout Console) |
| `/health` | GET | Health check (e.g. for System Status) |
| `/api/v1/scout` | POST | Scout chat: body `{ "query", "city?", "weather?" }` → SSE (brand-filtered) |
| `/infrastructure/leads` | GET | Command Center: KB-07 leads + alerts; high-intent entries include `high_intent: true`, `highlight: "#FA921C"` |
| `/nexus/onboard` | GET | Partner Onboarding Terminal: HTMX form (Business Name, City, Service Type, Webhook URL) |
| `/nexus/register` | POST | Partner registration: writes to KB-07 (`partner_*`), runs connection test; returns Orange success or Bandwidth Error message |
| `/command` | GET | Lead Ledger Dashboard: God-View for Jamey; HTMX polls `/command/feed` every 60s; Navy/Orange table, high-intent highlight |
| `/command/feed` | GET | HTML fragment: system summary (total_leads, high_intent_count, active_partner_count) + leads table |

---

## Environment

Copy `.env.example` to `.env` and adjust. Relevant vars:

| Variable | Description |
|----------|-------------|
| `OPENROUTER_API_KEY` | OpenRouter API key for Scout (NEXUS Bridge) |
| `KB07_PATH` | Sled path for KB-07 Relations (default `./data/kbs/kb07_relations`) |
| `PARTNER_WEBHOOK_URL` | Optional webhook URL for high-intent lead notifications |
| `GATEWAY_PORT` | 8000 (backend). Frontend Studio UI: 3001. |

---

## Features

- **Identity Orchestrator** — `brand_filter` enforces Navy (#051C55) / Orange (#FA921C) and XCURZENS voice on all Scout output.
- **NEXUS Bridge** — Scout uses OpenRouter + KB-07 lead history + geo (city/weather); replies acknowledge location and conditions.
- **Geo-Context** — Browser geolocation + Nominatim (city) + Open-Meteo (weather); auto-injected into every Scout request; Orange “Scout Context” badge.
- **LeadDispatcher** — Logs leads to KB-07; on **high intent** (price, booking, availability, etc.) writes an alert and can POST to `PARTNER_WEBHOOK_URL` or to a partner’s `webhook_url` from KB-07 (`partner_*` keys).
- **Forge / Scribe** — SAM lifecycle (evolve, decommission); protected skills (forge, scribe, gateway, auth, xcurzens_core); KB-03 (Techne) archiving; Sovereign-Key force override.
- **Partner Onboarding Terminal** — `/nexus/onboard`: HTMX form (Business Name, Primary City, Service Type, Webhook URL); `/nexus/register` writes to KB-07 with `partner_*` key, runs connection test, logs to Jamey; Orange success or Bandwidth Error message.
- **Lead Ledger Dashboard** — `/command`: Command Center view; HTMX polls `/command/feed` every 60s; system summary (total_leads, high_intent_count, active_partner_count); leads table with Orange highlight for high-intent. `/infrastructure/leads` JSON now includes `system_summary` for instant infrastructure health.
- **System Stress Test** — Binary `stress_test`: 10 concurrent travelers × 5 requests (50 total) to `/api/v1/scout` with mixed high-intent and normal queries, randomized city/weather. Logs: `[STRESS TEST] Bandwidth Capacity: X% | Average Latency: Yms`. Run with gateway up; verify leads and high-intent in `/command`.

---

## Identity

This repo uses the **pagi-xcurzens** identity throughout: crate names `pagi_xcurzens_*` / `pagi-skills`, project name **PAGI XCURZENS**. Root Sovereign for auth and logs: **Jamey**. No Docker; direct host filesystem and local Rust/PowerShell orchestration only.

---

## Docs

- **HANDOVER_FINAL.md** — Creator handover: frontend folder map, endpoint list, Final Handshake steps for Google AI Studio assets.
- **FORGE_ARCHITECTURE.md** — Layers, ports, Forge protection, Sovereign-Key.
- **REBRAND_COMPLETE.md** — Rebrand checklist and folder-rename steps.
- **.cursorrules** — Port lockdown (3001 frontend, 8000 gateway), Bare Metal, Forge protection.

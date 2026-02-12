# frontend-command — Command Center / Lead Ledger

**Purpose:** Jamey's God-View (system summary + leads table, high-intent highlight).  
**Drop Google AI Studio assets here.**

## Rust endpoints (Final Handshake)

| Action | Method | URL | Body / Notes |
|--------|--------|-----|--------------|
| Dashboard page | GET | `http://127.0.0.1:8000/command` | Serves current Lead Ledger page. |
| Live feed (HTMX poll) | GET | `http://127.0.0.1:8000/command/feed` | Returns **HTML fragment**: summary (total_leads, high_intent_count, active_partner_count) + table. Poll every 60s. |
| Raw JSON | GET | `http://127.0.0.1:8000/infrastructure/leads` | JSON: `sovereign`, `source`, `system_summary`, `leads[]` (each with `id`, `payload`, `high_intent`, `highlight`). |

**Brand:** Navy `#051C55`, Orange `#FA921C`. High-intent rows use Orange tint; summary metrics in Orange.

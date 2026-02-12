# frontend-xcurzens — Traveler UI

**Purpose:** Main traveler experience (Scout Command Bar + Console).  
**Drop Google AI Studio assets here** (HTML/HTMX, Tailwind, etc.).

## Rust endpoints (Final Handshake)

| Action | Method | URL | Body / Notes |
|--------|--------|-----|--------------|
| Page load | GET | `http://127.0.0.1:8000/` | Serves current Traveler UI; replace or proxy as needed. |
| Scout query | POST | `http://127.0.0.1:8000/api/v1/scout` | JSON: `{ "query": string, "city"?: string, "weather"?: string }` → response is **SSE** (event stream). |
| Health / status | GET | `http://127.0.0.1:8000/health` | Returns `OK`. Use for "Bandwidth Stable" indicator. |

**Brand:** Navy `#051C55`, Orange `#FA921C`. Scout responses are already brand-filtered (HTML) from the gateway.

# frontend-nexus — Partner Onboarding

**Purpose:** Partner registration terminal (Business Name, City, Service Type, Webhook URL).  
**Drop Google AI Studio assets here.**

## Rust endpoints (Final Handshake)

| Action | Method | URL | Body / Notes |
|--------|--------|-----|--------------|
| Onboarding page | GET | `http://127.0.0.1:8000/nexus/onboard` | Serves current onboarding form. |
| Register partner | POST | `http://127.0.0.1:8000/nexus/register` | Form body: `business_name`, `primary_city`, `service_type`, `webhook_url`. Returns **HTML fragment** (success or Bandwidth Error message) for HTMX swap. |

**Brand:** Navy `#051C55`, Orange `#FA921C`. Success: "Infrastructure Synchronized...". Error: "Bandwidth Error: Please verify your Webhook URL."

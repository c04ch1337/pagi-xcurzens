# 🦅 Phoenix Marie: Sovereign AGI Orchestrator

**Phoenix Marie** is a high-sovereignty, bare-metal AGI orchestrator built in **Rust**. Unlike typical agentic systems, Phoenix is governed by an **Astro-Logic Engine** and a **3-Tiered Skill Firewall** that prevents unauthorized access to sensitive knowledge layers.

---

## 🛠️ System Architecture

### 1. Bare Metal Design

* **Zero Docker:** Built for direct hardware execution for maximum performance and security.
* **Rust Core:** Type-safe memory management ensures no data leaks between threads or knowledge layers.
* **Local-First:** All long-term and short-term memory resides on local disk persistence.

### 2. The 9-Layer Memory Taxonomy

Phoenix organizes its world into 9 distinct Knowledge Bases (KBs). Access is strictly gated by the **Sovereignty Firewall**.

| Layer | Domain | Description | Firewall Status |
| --- | --- | --- | --- |
| **KB-01** | **Ethos** | Core values and identity. | **CORE ONLY** |
| **KB-02** | **Technical** | Rust docs, codebase specs, and math. | Open |
| **KB-03** | **Persona** | Phoenix's conversational tone. | Open |
| **KB-04** | **Logistics** | File paths, local environment config. | Restricted |
| **KB-05** | **Sovereignty** | Social defense and Grey Rock triggers. | **RESTRICTED** |
| **KB-06** | **Creative** | Art, brainstorming, and media. | Open |
| **KB-07** | **External** | Astro-weather and news scrapers. | Restricted |
| **KB-08** | **Health** | Success metrics and "Failed Leak" audits. | **RESTRICTED** |
| **KB-09** | **Shadow** | Private PII and sensitive user data. | **CORE ONLY** |

---

## 🛡️ The 3-Tier Skill Model

Skills are the "actions" Phoenix can take. They are restricted by their **Trust Tier**:

* **Tier 1 (Core):** Signed by the User. Has full access to all KBs, including **KB-01** and **KB-09**.
* **Tier 2 (Import):** Standard normalized skills. Can access general KBs (02, 03, 06).
* **Tier 3 (Generated):** Ephemeral skills drafted by the AI. **Blocked by the Firewall** from touching sensitive layers until promoted by the Warden.

---

## 🛡️ Forge Safety Governor (Human-in-the-Loop)

Phoenix can self-modify code via the **Sovereign Operator (Forge)**. The **Forge Safety Governor** requires explicit human approval before any compile/load:

* **HITL gate:** When `PAGI_FORGE_SAFETY_ENABLED=true` (default), every proposed change is shown in the terminal and you approve with `y`/`n`.
* **Audit trail:** All approvals/denials are logged to KB-08.
* **Emergency kill switch:** Run `.\forge-kill-switch.ps1` (Windows) or `./forge-kill-switch.sh` (Linux/macOS) to re-enable safety and stop active Forge builds.
* **Sovereign Autonomy:** Runtime control of Forge safety (HITL vs autonomous), auto-revert on compile failure, and multi-layer control—see **[SOVEREIGN_AUTONOMY_SYSTEM.md](docs/SOVEREIGN_AUTONOMY_SYSTEM.md)**.

See **[FORGE_SAFETY_GOVERNOR.md](docs/FORGE_SAFETY_GOVERNOR.md)** for usage, configuration, and testing.

---

## 🪐 Astro-Logic & Defensive Toggles

Phoenix is the first AGI that uses **Celestial Transits** as a proxy for environmental volatility.

* **Astro-Logic:** Real-time planetary scraping modifies the agent's "Caution Level."
* **Social Defense:** Automatically engages "Grey Rock" or "Defensive" personas if KB-05 triggers are detected.

### Sovereign Configuration (`.env`)

You can tune the system's "Hardness" without recompiling:

* `PAGI_FORGE_SAFETY_ENABLED`: (Default: `true`) Human-in-the-loop approval for Forge/self-modification.
* `PAGI_ASTRO_LOGIC_ENABLED`: Toggles archetype directives.
* `PAGI_TRANSIT_ALERTS_ENABLED`: Toggles the background weather scraper.
* `PAGI_SKILLS_AUTO_PROMOTE_ALLOWED`: (Default: `false`) Prevents AI self-promotion.
* `PAGI_STRICT_TECHNICAL_MODE`: Forces a deterministic `0.3` temperature.

---

## 🚀 Getting Started

### ⚡ Quick Start (Recommended)

**Master Orchestrator** - One command to rule them all:

```powershell
.\start-sovereign.ps1
```

This unified script automatically handles:
- ✅ Execution policy fixes
- ✅ Environment validation (Rust, Node.js)
- ✅ Knowledge Base provisioning (8 directories)
- ✅ Port cleanup (kills zombie processes)
- ✅ Workspace build
- ✅ Frontend dependencies
- ✅ Coordinated launch (Gateway → Control Panel → Studio UI)

**See:** [`QUICK_START.md`](./QUICK_START.md) for one-liners and [`SCRIPT_ORCHESTRATION_GUIDE.md`](./SCRIPT_ORCHESTRATION_GUIDE.md) for complete documentation.

### Alternative Boot Methods

**Option 1: Phoenix Rise (Autonomous)**
For a fully automated system boot with cognitive diagnostics:
- **Windows (PowerShell)**: `.\phoenix-rise.ps1`
- **Linux/macOS (Bash)**: `./phoenix-rise.sh`

**Option 2: Quick Launch (Assumes Ready Environment)**
- **Windows (PowerShell)**: `.\pagi-up.ps1`
- **Linux/macOS (Bash)**: `./pagi-up.sh`

**Option 3: Cursor Agent**
Simply say: **"Phoenix, rise."** or **"Boot Phoenix system."**

### Manual Boot

1. **Environment:** Copy `.env.example` to `.env` and configure your toggles.
2. **Build:** From the workspace root: `cargo build --release`.
3. **Pre-flight (optional):** Run `cargo run -p pagi-gateway -- --verify` to check port 8000 and Sled DB locks.
4. **Run gateway:** From the workspace root: `cargo run -p pagi-gateway` (or `./target/release/pagi-gateway` after release build). Gateway listens on port 8000 by default.
5. **Verify:** `GET http://127.0.0.1:8000/v1/status` and `GET http://127.0.0.1:8000/api/v1/health` should return expected JSON.
6. **Warden:** Use the UI **Promote** button to elevate Tier 3 skills to Core status.

For full frontend wiring (Studio UI, drop-in UI, CORS, chat API), see **[docs/frontend-backend-integration.md](docs/frontend-backend-integration.md)**.

---

> **Audit Note:** All blocked attempts by Tier 3 skills to access KB-01/09 are logged in **KB-08**. Check your Health Report regularly to monitor for "Capability Overreach."

---

## 📚 Documentation & Resources

All detailed docs live in **[docs/](docs/README.md)** (see [docs/README.md](docs/README.md) for the full index).

| Doc | Description |
|-----|--------------|
| [**Phoenix Orchestrator**](docs/PHOENIX_ORCHESTRATOR.md) | 🔥 Master orchestrator prompt for autonomous system boot, port management, and health verification |
| [**Phoenix Post-Boot Diagnostician**](docs/PHOENIX_POST_BOOT_DIAGNOSTICIAN.md) | 🧠 Cognitive health verification for memory (Topic Indexer) and meta-cognition (Evolution Inference) |
 | [**Frontend–Backend Integration**](docs/frontend-backend-integration.md) | Gateway API, Studio/drop-in UI wiring, CORS, chat, KB status, troubleshooting |
 | [**First Mission: Operation First Rise**](docs/FIRST_MISSION_OPERATION_FIRST_RISE.md) | Beta tester end-to-end proof: sidecars healthy + Concise mode emits JSON Diagram Envelope (diagram-first) |
 | [**Forge Safety Governor**](docs/FORGE_SAFETY_GOVERNOR.md) | Human-in-the-loop approval for self-modification; kill-switch usage |
 | [**Sovereign Autonomy System**](docs/SOVEREIGN_AUTONOMY_SYSTEM.md) | Runtime Forge safety control, auto-revert on failure, HITL vs autonomous modes |
| [Deployment](docs/DEPLOYMENT.md) | Bare-metal deployment and security hardening |
| [Architecture](docs/BARE_METAL_ARCHITECTURE.md) | System architecture deep dive |
| [Project Anatomy](docs/PROJECT_ANATOMY.md) | Workspace and crate layout |
| [Vector KB Activation](docs/VECTORKB_ACTIVATION_GUIDE.md) | Optional Qdrant vector store setup |
| [Vector KB Production Hardening](docs/VECTORKB_PRODUCTION_HARDENING.md) | Vector KB production hardening and security |

[Learn how to structure Knowledge Bases for AI agents](https://www.youtube.com/watch?v=LZ0E7bjVv0s) — explains how to categorize and feed data into an AI's knowledge base, fundamental to the 9-layer memory system.

---

## 🔐 Governance Policy

**Would you like me to generate a specific "Governance Policy" document that outlines exactly what criteria Phoenix must meet before you click that 'Promote' button?**

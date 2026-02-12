# PAGI XCURZENS — Forge Architecture

## Identity

- **Project name:** PAGI XCURZENS  
- **Repository context:** pagi-xcurzens (workspace and crate names use `pagi-xcurzens` / `pagi_xcurzens`).

## Layers

1. **Rust workspace**  
   Root `Cargo.toml` defines members:
   - `crates/pagi-xcurzens-core`
   - `crates/pagi-xcurzens-gateway`

2. **Gateway**  
   Single HTTP/SSE surface at `http://127.0.0.1:8000`. All UI and external callers use this origin; no localhost:3001 or mock backends.

3. **Orchestrator**  
   - `start-sovereign.ps1` — Master entry point; runs `cargo check` and prepares perimeter.
   - `phoenix-rise.ps1` — Full launch (delegates to start-sovereign).
   - `pagi-up.ps1` / `pagi-down.ps1` — Stack up/down; paths are relative to repo root.

4. **Add-ons**  
   - `add-ons/pagi-studio-ui` — Studio interface; config in `assets/studio-interface/src/api/config.ts` points at gateway only.

5. **Forge and protected skills (kill-switch)**  
   - `crates/pagi-skills`: Forge (evolve/decommission), Scribe (KB-03 Techne archiving).  
   - **Protected registry:** `forge`, `scribe`, `gateway`, `auth`, `xcurzens_core` — SAM cannot decommission or evolve these without override.  
   - **Force override (Port 3001 / Gateway API):** Only when request includes header `Sovereign-Key` equal to env `FORGE_SOVEREIGN_KEY`. Without it, protected skills return `ForgeError::AccessDenied`. The override remains exclusively in the Architect's hands.

## Path discipline

All scripts and configs assume the repo root is the current workspace. After rebrand, the folder should be named **pagi-xcurzens** so that paths and partnership-facing metadata are consistent.

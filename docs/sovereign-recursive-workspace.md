# Sovereign Recursive System — Workspace and 8 KB Paths

## Workspace configuration

The root `Cargo.toml` includes the evolutionary engine:

```toml
[workspace]
resolver = "2"
members = [
    "crates/pagi-core",
    "crates/pagi-evolution",
    "crates/pagi-skills",
    "add-ons/pagi-gateway",
    # ... other add-ons
]
```

## 8 Knowledge Base paths (LanceDB)

Use a **single** LanceDB database directory. Recommended path:

| Path | Purpose |
|------|--------|
| `./data/pagi_lancedb` | LanceDB database (all 8 tables in one DB) |

Table names (one per slot) — see `pagi_core::knowledge::LANCEDB_TABLE_NAMES`:

| Slot | Table name   | Cognitive domain |
|------|--------------|------------------|
| 1    | kb1_pneuma   | Vision / identity |
| 2    | kb2_oikos    | Context / tasks (Oikos) |
| 3    | kb3_logos    | Knowledge / research |
| 4    | kb4_chronos  | Temporal / memory |
| 5    | kb5_techne   | Skills / blueprints |
| 6    | kb6_ethos    | Guardrails |
| 7    | kb7_kardia   | Affective / relations |
| 8    | kb8_soma     | Execution / state (Soma) |

Oikos (Task) and Soma (State) are persisted in **kb2_oikos** and **kb8_soma** when using the LanceDB semantic layer. Build `pagi-core` with `--features lancedb` to enable.

## Dynamic skill compilation (pagi-evolution)

Generated artifacts are written under:

- **Default:** `./data/pagi_evolution/<name>.so` (or `.dll` on Windows)
- **Custom:** pass `output_path` to `Compiler::compile_from_string(code, name, Some(path))`

To persist source in the repo, use:

- `crates/pagi-skills/generated/` — add a `generated/` directory and build from path with `Compiler::compile_from_path("crates/pagi-skills/generated/<crate_name>")`.

The compiled cdylib must export:

- `pagi_dynamic_skill_execute(args_json: *const c_char) -> *mut c_char`
- `pagi_dynamic_skill_free(ptr: *mut c_char)`

## OpenRouter Sovereign Bridge

- **Module:** `pagi_core::openrouter_service` (renamed from gemini_bridge). High-level reasoning only; all actions and memory stay in Rust.
- **API key:** `OPENROUTER_API_KEY` in `.env`. Chat (ModelRouter) uses `PAGI_LLM_API_KEY` with fallback to `OPENROUTER_API_KEY`.
- **Default model:** `meta-llama/llama-3.3-70b-instruct` (Bridge and ModelRouter).
- **Local priority:** The orchestrator should query the 8 local Knowledge Bases (Sled/LanceDB) *before* sending a request to OpenRouter. Attach that context to the Bridge/chat prompt to save tokens and keep the Bridge for thinking, not basic retrieval.
- **Rig (optional):** Build `pagi-core` with `--features rig` to use the `rig` crate for OpenRouter completion.

## SSE alignment

OpenRouter stream is piped through the existing Axum SSE handlers: `ModelRouter::stream_generate()` returns a channel of tokens; the gateway’s **POST /api/v1/stream** and streaming chat send them as SSE events (`event: token`, then `done`).

## Endpoints

- **POST /api/v1/stream** — SSE stream of "Inner Monologue" tokens (event: `token`, then `done`).
- **POST /api/v1/chat** — JSON chat (with optional `stream: true` for plain-text stream).
- Reflexion failures are logged to Chronos (key prefix `failure/`) for self-correction.

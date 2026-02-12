# PAGI Orchestrator — Green Check / Red X Self-Audit Report

**Role:** Senior Rust Systems Auditor  
**Scope:** Structural audit, connectivity check, logic bridge analysis for Self-Evolution readiness.  
**Date:** 2026-02-10

---

## TASK 1: Structural Audit

| Module | Status | Location | Notes |
|--------|--------|----------|--------|
| Short-term memory (session) | ✅ ALIVE | `add-ons/pagi-gateway/src/openrouter_live.rs` (in-memory `VecDeque<Message>`) | Live mode only; HTTP chat has no in-request conversation window. |
| Short-term memory (Chronos retrieval) | ⚠️ PARTIAL | `crates/pagi-core/src/knowledge/store.rs` (`get_recent_chronos_events`, `build_local_context_for_bridge`) | Chronos **events** (skill runs) are retrieved for **MoE** path only. **Conversation** blobs from `save_to_memory` use UUID keys and are **not** currently retrieved for the main chat system prompt. |
| Long-term Knowledge Base (8+1 slots) | ✅ ALIVE | `crates/pagi-core/src/knowledge/store.rs`, `mod.rs` | Sled-backed; persists to disk (`./data/pagi_knowledge`). |
| MemoryManager (pagi_vault) | ✅ ALIVE | `crates/pagi-core/src/memory.rs` | Short-term DashMap + long-term Sled at `./data/pagi_vault`. Used by gateway for vault path; not injected into LLM context in Live path. |
| Skill Registry | ✅ ALIVE | `crates/pagi-core/src/skills.rs` (`SkillRegistry`, `LiveSkillRegistry`), gateway `main.rs` ~513–556 | Gateway registers ModelRouter, FileSystem, Shell, WebSearch, Counselor, etc. |
| Skill Forge (compile at runtime) | ✅ ALIVE (isolated) | `crates/pagi-evolution/src/compiler.rs`, `loader.rs` | `Compiler::compile_from_string` / `compile_from_path` use `std::process::Command::new("cargo")`. **Not** wired into gateway. |
| SovereignOperator (Forge + Rollback) | ✅ ALIVE (not registered) | `crates/pagi-skills/src/sovereign_operator.rs` | Uses `pagi_evolution::Compiler` and `SkillLoader`. **Not** registered in gateway `SkillRegistry` or `LiveSkillRegistry`; no main-loop tie-in. |
| Governor / Ethos | ✅ ALIVE | `add-ons/pagi-gateway/src/governor.rs`, KB-05/KB-06 in core | Used for alerts and policy. |
| LLM connectivity (reqwest) | ✅ ALIVE | `add-ons/pagi-gateway/Cargo.toml`, `openrouter_live.rs`, `pagi-core/openrouter_service.rs` | reqwest 0.12 with JSON; OpenRouter used for streaming and bridge. |
| Vector KB / KB Router | ✅ ALIVE | `crates/pagi-core/src/knowledge/vector_store.rs`, `kb_router.rs`; gateway `knowledge_router.rs` | Dynamic KB selection; used in OpenRouter Live for on-demand retrieval. |
| Maintenance loop (self-patch) | ✅ ALIVE | `crates/pagi-core/src/orchestrator/maintenance.rs` | Uses `std::process::Command::new("cargo")` for validation patch compile. Separate from user-facing Forge. |
| Red Team / Rollback | ✅ ALIVE | `crates/pagi-evolution/src/red_team.rs`, `rollback.rs` | Used by SovereignOperator; not in main request path. |

**Summary:** No `todo!()` found. All critical modules exist and are implemented. Gaps are **tie-ins**: Forge not in main loop, Chronos conversation not in main chat context.

---

## TASK 2: Connectivity Check

| Check | Status | Detail |
|-------|--------|--------|
| reqwest for LLM | ✅ | Gateway and pagi-core use reqwest; OpenRouter API key via `OPENROUTER_API_KEY` / `PAGI_LLM_API_KEY`. |
| Skill Forge can run `cargo` | ✅ | `pagi-evolution/src/compiler.rs:76` and `pagi-core/src/orchestrator/maintenance.rs:537` use `Command::new("cargo")`. Forge is **not broken**; it is **not invoked** from the gateway. |
| cargo in PATH | ⚠️ | Assumed at runtime. Recommend one-time check at startup (e.g. `Command::new("cargo").arg("--version").output()`) if self-evolution is required. |

---

## TASK 3: Logic Bridge Analysis

### Multi-layer memory

- **Long-term:** `KnowledgeStore` (Sled, 8+1 trees) and `MemoryManager` (Sled + DashMap) **persist to disk**. Not in-memory-only.
- **Short-term:**  
  - **Live mode:** In-memory `history: VecDeque<Message>` in `OpenRouterLiveSession`; not backed by KB-4.  
  - **HTTP chat:** Each turn is saved to KB-4 via `save_to_memory` (conversation blobs under UUID keys). **Retrieval** of recent conversation into the **main** chat system prompt or messages is **not** implemented. `build_system_directive` (used for main chat) does **not** include recent Chronos conversation; only MoE path uses `build_local_context_for_bridge` (and that uses Chronos **events**, not the conversation blob store).

### Tie-in points for “full loop”

| Feature | File:Line / Location | Action needed to tie in |
|---------|----------------------|---------------------------|
| **Forge → Main Orchestrator** | `add-ons/pagi-gateway/src/main.rs` (chat flow after LLM response) | 1) Add `pagi-evolution` to gateway if you want direct compile in gateway, **or** 2) Register `SovereignOperatorSkill` in gateway’s `SkillRegistry`/`LiveSkillRegistry` and have the orchestrator dispatch “generate skill” goals to it. 3) In the chat/streaming path, detect “needs new skill” (e.g. from LLM tool/response or a dedicated endpoint) and call `SovereignOperator::generate_and_compile` (or equivalent) then register the new artifact with `SkillLoader` + registry. |
| **Short-term memory → LLM context (main chat)** | `crates/pagi-core/src/knowledge/store.rs` (`build_system_directive` ~1611) or gateway chat handler before building messages | Add retrieval of **recent conversation** from KB-4 (e.g. scan or dedicated key prefix for `save_to_memory`-style records), limit to last N turns, and append to system directive or as prior user/assistant messages so the model has recent dialogue context. |
| **Chronos events vs conversation** | `store.rs` (`get_recent_chronos_events` uses `event/{agent_id}` prefix) | `save_to_memory` uses UUID keys; either store conversation under a known prefix (e.g. `conversation/{agent_id}/`) and add `get_recent_conversation_turns(agent_id, limit)`, or reuse Chronos with a consistent key scheme so one retrieval path can serve both. |

---

## OUTPUT: Summary Table (Action Needed)

| Feature | Status | Location | Action Needed to 'Tie In' |
| :--- | :--- | :--- | :--- |
| Skill Forge (cargo compile) | ✅ | `crates/pagi-evolution/src/compiler.rs` | No code change; **wire into gateway**: register SovereignOperator and call from orchestrator when “new skill” is requested. |
| Skill Registry | ✅ | `crates/pagi-core/src/skills.rs`, gateway `main.rs` | Register `SovereignOperatorSkill` in gateway so the Forge can be invoked as a skill. |
| Long-term KB (disk) | ✅ | `crates/pagi-core/src/knowledge/store.rs` | None. |
| Short-term memory (conversation in context) | ⚠️ | `crates/pagi-core/src/knowledge/store.rs`, gateway chat | **Add** retrieval of recent conversation (e.g. from KB-4) and inject into `build_system_directive` or into the messages array in the chat handler. |
| LLM (reqwest) | ✅ | Gateway + pagi-core | None. |
| Governor / KB-05 | ✅ | `add-ons/pagi-gateway/src/governor.rs` | None. |
| MemoryManager (vault) | ✅ | `crates/pagi-core/src/memory.rs` | Optional: use in Live path for cross-session persistence; currently held but not read for context. |
| Vector KB / KB Router | ✅ | `crates/pagi-core/src/knowledge/vector_store.rs`, `kb_router.rs` | None for basic tie-in; already used in Live. |

---

## Red Flags (as requested)

1. **`todo!()` macro:** **None** found in the codebase.
2. **Missing `std::process`:** **No.** Forge and maintenance use `Command::new("cargo")`. The gap is **invocation** from the main loop, not capability.
3. **Isolated modules:** **Yes.** Short-term conversation is **saved** to KB-4 but **not read back** into the main HTTP chat prompt. SovereignOperator (Forge) is **not** registered in the gateway, so the main loop never triggers “generate and compile new skill.”

---

## Conceptual “Tie-In” Loop (reference)

Target shape for the orchestrator loop:

```rust
// Conceptual tie-in — not literal code
loop {
    let input = get_user_input();
    let memory_context = long_term_db.search(&input);           // Multi-layer memory tie-in
    let recent_conversation = get_recent_conversation(agent_id, 10); // ADD: from KB-4
    let response = phoenix_brain.reason(input, memory_context, recent_conversation);
    if response.requires_new_skill() {
        forge.generate_and_compile(response.code);             // Forge tie-in (SovereignOperator)
        skill_registry.register(response.new_skill_path);
    }
}
```

- **Today:** `memory_context` is partially present (system directive from KB-01/06/07/08, etc.; no recent conversation). Forge exists but is not called from this loop.
- **To close the gap:** Implement `get_recent_conversation` (or equivalent) and inject into the prompt; register and invoke SovereignOperator (or equivalent) when the agent requests a new skill.

---

## Slash-command / next steps

- **Short-term memory tie-in:** In `KnowledgeStore::build_system_directive` (or in the gateway handler that builds the system message), call a new helper e.g. `get_recent_conversation_for_prompt(agent_id, user_id, limit)` that reads from KB-4 (Chronos) the last N conversation records (either by adding a `conversation/` key prefix in `save_to_memory` or by scanning and filtering by metadata `type: "conversation"`), then append that text (or those messages) to the directive or to the messages array.
- **Forge tie-in:** In `add-ons/pagi-gateway/src/main.rs`, after building the registry (~513–556), add `registry.register(Arc::new(SovereignOperatorSkill::new(Arc::new(SovereignOperator::new()))));` (or with config). Then ensure the orchestrator can receive a goal like “generate_skill” and dispatch it to SovereignOperator (e.g. via existing skill dispatch that already runs ModelRouter, FileSystem, etc.). No change to pagi-evolution or Compiler required for basic tie-in.

Once these are done, the “Action Needed” column above can be re-checked and the Xs turned into checks.

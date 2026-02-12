# Sovereignty Drill — Master Template Verification

The **Sovereignty Drill** verifies that all three layers of the Master Template fire correctly: **FileSystemSkill** (KB-05), **Ethos** (KB-06), and **Absurdity Log** (KB-08).

## What It Does

1. **Opens KnowledgeStore** — All 8 KB slots (pagi_knowledge).
2. **KB-05 security validation** — Runs `FileSystemSkill::validate_security()` for a safe read path (`.env.example`).
3. **FileSystemSkill execute** — Reads `.env.example` via the skill (operation `read`).
4. **KB-06 Ethos alignment** — Fetches philosophical policy (`get_ethos_philosophical_policy`) or security policy presence.
5. **KB-08 success metric** — Writes a drill result message with `record_success_metric()`.
6. **KB-08 summary** — Confirms the log is visible via `get_absurdity_log_summary(5)`.

## How to Run

From the repo root:

```bash
# Full drill (FileSystem + KB-05/06/08)
cargo run -p pagi-gateway -- --sovereignty-drill

# One-shot Sovereignty Audit (weighted score, no LLM)
cargo run -p pagi-gateway -- --audit
```

**Note:** Ensure no other `pagi-gateway` process is running (e.g. stop the gateway first). If the build fails with "Access is denied" when writing the exe, close any running gateway instance and try again.

## Success Output

You should see:

```
🏛️ Sovereignty Drill — Master Template verification

  [1/5] Opening KnowledgeStore... OK
  [2/5] KB-05 security validation (FileSystemSkill)... OK
  [3/5] FileSystemSkill execute (read config)... OK (N bytes)
  [4/5] KB-06 Ethos alignment check... OK — Stoic (or policy present / no policy set)
  [5/5] KB-08 Absurdity Log (success metric)... OK

  KB-08 summary: M total entries, L recent.

✅ Sovereignty Drill PASSED — all layers (KB-05, KB-06, KB-08) fired correctly.
```

Exit code **0** on success, **1** on failure (with error message).

## One-Shot Audit (`--audit`)

Runs the **AuditSkill** once (no LLM): discovery, alignment, infra scan, ethos validation, then prints a **Sovereignty Audit Report** with weighted **sovereignty_score** (0.0–1.0).

- **Scoring:** Base 1.0; −0.5 alignment failure; −0.2 per unprotected skill; −0.025 per capability gap (Redis/Vector unset). Clamped to 0.0.
- **Compliance:** `sovereignty_compliance` is true only if `sovereignty_score > 0.9`.
- **KB-08:** If score &lt; 0.7, logs "High Risk Anomaly"; always logs "Manual CLI Audit Performed" after a run.

Example tail of output:

```
--- SOVEREIGNTY AUDIT REPORT ---
{ "sovereignty_score": 0.95, "sovereignty_compliance": true, ... }

✅ Sovereignty compliance OK (score 0.95).
```

## Relation to Master Template

- **Cognitive Core:** Drill uses the same `KnowledgeStore` and `LiveSkillRegistry` (pagi_core) as the live path.
- **Skills Registry:** Uses `FileSystemSkill` from `LiveSkillRegistry::default()` with KB-05 validation.
- **Governor & Absurdity Log:** Drill writes to KB-08 and reads back the summary; the Governor (when running) would monitor the same KB-08 and KB-06.

This drill is the definitive **verification script** that the Master Template architecture is operational end-to-end.

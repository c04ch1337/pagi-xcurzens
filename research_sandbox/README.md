# research_sandbox/

This directory is intentionally used as a **workspace hygiene sandbox** for PAGI’s autonomous monitoring loop.

The gateway’s heartbeat periodically scans `research_sandbox/` to detect issues such as:

- `TODO` markers in `*.rs` files (and optionally `todo.txt` for local verification)
- Missing `research_sandbox/README.md`

When an issue is detected, it is:

1. Persisted into **KB_OIKOS** under `workspace_guardian/active_maintenance_tasks`
2. Delegated to `DEV_BOT` via **KB_SOMA** inbox messaging
3. Later **validated** (re-scan) and **resolved** when the issue disappears

## Why this exists

This sandbox is a safe, predictable target for testing the *State-Based Resolution* lifecycle without touching production code.

## How to verify the loop

1. Create a new file like `research_sandbox/example.rs` containing `// TODO: test`.
2. Wait for the heartbeat scan to detect the TODO and open a maintenance task.
3. Remove the TODO marker (or delete the file).
4. On the next scan, the guardian should mark the issue as resolved and emit a validation message.

## Dynamic trust scaling (KB_KARDIA)

When `SAGE_BOT` validates a resolution, it updates its Kardia relation for `DEV_BOT`:

- Resolution reward: `trust_score += 0.05` (clamped to `1.0`)
- Optional deterioration: if an issue stays active beyond 50 ticks, `trust_score -= 0.02`

You can inspect this via the gateway endpoint:

- `GET /api/v1/kardia/DEV_BOT?agent_id=SAGE_BOT`

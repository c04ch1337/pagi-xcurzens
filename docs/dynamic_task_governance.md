# Dynamic Task Governance (Oikos) — Slot 4 Implementation

## Overview

The **Dynamic Task Governor** is a cross-layer decision engine that evaluates tasks against the user's current biological state (**Soma**), emotional load (**Kardia**), and philosophical lens (**Ethos**) to produce governance decisions: **Proceed**, **Postpone**, **Simplify**, or **Deprioritize**.

This transforms SAGE_BOT from a passive task list into an **active schedule manager** that protects cognitive bandwidth and aligns task execution with the user's holistic state.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    TaskGovernor                              │
│                                                             │
│  ┌──────────┐   ┌──────────┐   ┌──────────────────┐       │
│  │  Soma    │   │  Kardia  │   │     Ethos        │       │
│  │ (Slot 8) │   │ (Slot 7) │   │    (Slot 6)      │       │
│  │          │   │          │   │                   │       │
│  │ sleep_h  │   │ rel_str  │   │ active_school     │       │
│  │ readines │   │ burnout  │   │ core_maxims       │       │
│  │ hrv      │   │ grace_m  │   │ tone_weight       │       │
│  └────┬─────┘   └────┬─────┘   └────────┬──────────┘       │
│       │              │                   │                  │
│       ▼              ▼                   ▼                  │
│  ┌─────────────────────────────────────────────────┐       │
│  │           evaluate(GovernedTask)                 │       │
│  │                                                  │       │
│  │  bio_penalty() × cognitive_weight                │       │
│  │  + emotional_penalty() × 0.3                     │       │
│  │  = combined_load                                 │       │
│  │                                                  │       │
│  │  → GovernanceAction + effective_priority          │       │
│  └─────────────────────────────────────────────────┘       │
│                         │                                   │
│                         ▼                                   │
│              ┌──────────────────┐                           │
│              │   KB_OIKOS       │                           │
│              │   (Slot 2)       │                           │
│              │                  │                           │
│              │ oikos/tasks/{id} │                           │
│              │ oikos/governance │                           │
│              │   _summary       │                           │
│              └──────────────────┘                           │
└─────────────────────────────────────────────────────────────┘
```

---

## Core Types

### `GovernedTask`

A task managed by the governor. Stored in KB_OIKOS (Slot 2) under `oikos/tasks/{task_id}`.

| Field | Type | Description |
|-------|------|-------------|
| `task_id` | `String` | Unique identifier |
| `title` | `String` | Human-readable title |
| `description` | `String` | Optional context |
| `difficulty` | `TaskDifficulty` | Cognitive tier (Low/Medium/High/Critical) |
| `base_priority` | `f32` | Original priority (0.0–1.0) |
| `effective_priority` | `f32` | Adjusted priority after governance |
| `action` | `GovernanceAction` | Governance decision |
| `tags` | `Vec<String>` | Categorization tags |

### `TaskDifficulty`

| Tier | Cognitive Weight | Behavior |
|------|-----------------|----------|
| `Low` | 0.2 | Rarely affected by state |
| `Medium` | 0.5 | Moderately affected |
| `High` | 0.85 | Strongly affected by poor state |
| `Critical` | 0.0 | **Never postponed** |

### `GovernanceAction`

| Action | When Triggered |
|--------|---------------|
| `Proceed` | State is adequate for the task |
| `Postpone { reason }` | Bio/emotional load too high for difficulty |
| `Simplify { suggestion }` | Load is high but not critical; break into steps |
| `Deprioritize { reason }` | Medium tasks under moderate load |

---

## Decision Logic

### Bio Penalty (0.0–1.0)

```
sleep_penalty = (7.0 - sleep_hours) / 7.0   [clamped 0–1]
readiness_penalty = (70 - readiness_score) / 70.0   [clamped 0–1]
burnout_amplifier = (burnout_risk - 0.5) × 0.5   [if > 0.5]

bio_penalty = clamp(sleep + readiness + burnout, 0.0, 1.0)
```

### Emotional Penalty (0.0–1.0)

```
stress_penalty = (relational_stress - 0.3) × 1.0   [if > 0.3]
burnout_penalty = (burnout_risk - 0.3) × 0.5   [if > 0.3]

emotional_penalty = clamp(stress + burnout, 0.0, 1.0)
```

### Combined Load

```
combined_load = clamp(bio_penalty × cognitive_weight + emotional_penalty × 0.3, 0.0, 1.0)
effective_priority = base_priority × (1.0 - combined_load × 0.5)
```

### Decision Thresholds

| Condition | Action |
|-----------|--------|
| `difficulty == Critical` | Always `Proceed` |
| `sleep_hours < 5.0 && difficulty == High` | `Postpone` |
| `combined_load > 0.65 && difficulty == High && bio > 0.5` | `Postpone` |
| `relational_stress > 0.7 && tags contain "conflict"` | `Postpone` |
| `combined_load > 0.5 && difficulty == High` | `Simplify` |
| `combined_load > 0.6 && difficulty == Medium` | `Deprioritize` |
| Otherwise | `Proceed` |

---

## Philosophical Lens Integration

The Ethos policy shapes the **reason text** in governance actions. Each school provides school-specific reframing:

| School | Postpone Reason Style |
|--------|----------------------|
| **Stoic** | "Focus on what you can control right now. Postponing is the rational choice." |
| **Growth-Mindset** | "Your brain needs recovery to learn effectively. Rescheduling maximizes growth." |
| **Compassionate-Witness** | "Your body is asking for rest. Honoring that need is self-compassion." |
| **Taoist** | "Wu-wei — forcing this task goes against the natural flow." |
| **Existentialist** | "Choosing rest now is an authentic act of self-determination." |

When `tone_weight < 0.3`, philosophical language is suppressed and generic reasons are used.

---

## KnowledgeStore API

```rust
// Store a governed task
store.set_governed_task(&task)?;

// Retrieve by ID
let task = store.get_governed_task("architecture_decision");

// List all tasks (sorted by effective priority)
let tasks = store.list_governed_tasks()?;

// Remove a task
store.remove_governed_task("old_task")?;

// Create a governor from current cross-layer state
let governor = store.create_task_governor("default");

// Evaluate all tasks and persist results + summary
let evaluated = store.evaluate_and_persist_tasks("default")?;
```

---

## Example: High-Stress Event Processing

```rust
// 1. Set biological state (from wearable/manual input)
store.set_soma_state(&SomaState {
    sleep_hours: 4.0,
    resting_hr: 75,
    hrv: 30,
    readiness_score: 35,
})?;

// 2. Set philosophical lens
let stoic = EthosPolicy::preset("Stoic").unwrap();
store.set_ethos_philosophical_policy(&stoic)?;

// 3. Add tasks
store.set_governed_task(&GovernedTask::new(
    "architecture_review",
    "Review microservice boundaries",
    TaskDifficulty::High,
).with_priority(0.8))?;

store.set_governed_task(&GovernedTask::new(
    "email_replies",
    "Reply to routine emails",
    TaskDifficulty::Low,
).with_priority(0.4))?;

// 4. Evaluate — Governor reads Soma + Kardia + Ethos automatically
let evaluated = store.evaluate_and_persist_tasks("default")?;

// Result:
// - "email_replies" → Proceed (low cognitive load)
// - "architecture_review" → Postpone (Stoic: "Focus on what you can control")
```

---

## Test Coverage (30 tests)

### Unit Tests (`task_governance_test.rs` — 20 tests)

| # | Test | Validates |
|---|------|-----------|
| 1 | `healthy_state_all_tasks_proceed` | Baseline: no interference when healthy |
| 2 | `sleep_deprived_stoic_postpones_high_difficulty` | Soma → Postpone + Stoic reasoning |
| 3 | `high_emotional_stress_postpones_conflict_tasks` | Kardia → Postpone conflict tasks |
| 4 | `critical_tasks_always_proceed` | Critical immunity (worst state) |
| 5 | `growth_mindset_produces_different_reason_text` | Ethos lens differentiation |
| 6 | `taoist_lens_produces_wu_wei_reasoning` | Taoist wu-wei in reasons |
| 7 | `existentialist_lens_produces_freedom_reasoning` | Existentialist freedom in reasons |
| 8 | `bio_penalty_scales_with_sleep_deprivation` | Penalty math correctness |
| 9 | `emotional_penalty_scales_with_stress` | Emotional penalty scaling |
| 10 | `effective_priority_reduced_under_load` | Priority adjustment |
| 11 | `batch_evaluation_sorts_correctly` | Proceed-first sorting |
| 12 | `governed_task_serialization_roundtrip` | JSON roundtrip |
| 13 | `task_difficulty_cognitive_weights` | Weight ordering |
| 14 | `kb_integration_store_and_retrieve_tasks` | Full KB CRUD |
| 15 | `kb_integration_evaluate_and_persist` | End-to-end KB pipeline |
| 16 | `create_task_governor_reads_cross_layer_state` | Cross-layer state assembly |
| 17 | `governance_summary_is_readable` | Human-readable output |
| 18 | `no_ethos_policy_produces_generic_reasons` | Graceful degradation |
| 19 | `low_tone_weight_reduces_philosophical_influence` | Tone weight control |
| 20 | `medium_difficulty_moderate_load_deprioritize` | Deprioritize threshold |

### Oikos Integration Audit (`oikos_integration_audit.rs` — 10 tests)

The "Closing of the Circuit" — full-stack integration tests that validate the PAGI system as an **Active Proxy of Executive Function**.

**Scenario:** User had 4.5h sleep. Project Manager (trust: 0.2) is a known stress trigger. Three tasks queued: Quarterly Report (High), Grocery Run (Low), Emergency Server Patch (Critical).

| # | Test | Validates |
|---|------|-----------|
| 21 | `oikos_integration_audit_full_circuit` | **Master test:** All 4 layers (Soma→Kardia→Ethos→Oikos) end-to-end |
| 22 | `oikos_audit_cross_layer_assembly` | `create_task_governor()` reads all layers correctly |
| 23 | `oikos_audit_kardia_relational_map` | PersonRecord CRUD for Project Manager (trust 0.2) |
| 24 | `oikos_audit_ethos_shapes_governance_reason` | Stoic lens produces "focus on what you can control" |
| 25 | `oikos_audit_critical_immunity_under_maximum_stress` | Critical tasks proceed even at 2h sleep + 0.99 stress |
| 26 | `oikos_audit_biogate_cross_layer_reaction` | Soma→Kardia: BioGate adjusts burnout_risk + grace_multiplier |
| 27 | `oikos_audit_governance_summary_comprehensive` | Summary contains all layers, task names, and counts |
| 28 | `oikos_audit_task_ordering` | Proceed tasks sorted before Postponed; Critical first |
| 29 | `oikos_audit_ethos_lens_differentiation` | All 5 schools produce unique, school-specific reasons |
| 30 | `oikos_audit_print_full_report` | Generates human-readable Sovereign Sanctuary report |

---

## Oikos Integration Audit Results

### Scenario: "Biology Fails, AGI Protects"

```
═══════════════════════════════════════════════════════════════════════
  🏛️  OIKOS INTEGRATION AUDIT — SOVEREIGN SANCTUARY REPORT
═══════════════════════════════════════════════════════════════════════

LAYER 1: SOMA (Slot 8) — Hardware Budget
  Sleep: 4.5h | Resting HR: 78 BPM | HRV: 28 ms | Readiness: 32
  BioGate Active: YES ⚠️

LAYER 2: KARDIA (Slot 7) — Emotional Context
  Relational Stress: 0.75 | Burnout Risk: 0.70 | Grace: 1.6x
  High-Stress Person: Project Manager (trust: 0.2, style: Avoidant)
  Triggers: ["criticism", "deadline pressure", "micromanagement"]

LAYER 3: ETHOS (Slot 6) — Philosophical Lens
  Active School: Stoic
  Core Maxims:
    1. Focus on what you can control (Dichotomy of Control).
    2. Virtue is the sole good; external events are indifferent.
    3. Respond with reason, not reactive emotion.

LAYER 4: OIKOS (Slot 2) — Governance Decisions
  [0.95] Emergency Server Patch (Critical) → ✅ PROCEED
  [0.32] Weekly Grocery Run (Low)          → ✅ PROCEED
  [0.40] Finish Quarterly Report (High)    → ⏸️  POSTPONE

=== Task Governance Summary ===
Philosophical Lens: Stoic
Bio Penalty: 1.00 | Emotional Penalty: 0.65
Sleep: 4.5h | Readiness: 32 | Burnout Risk: 0.70
---
Tasks: 3 total | 2 proceed | 1 postponed | 0 other

  ✅ AUDIT COMPLETE — All governance decisions verified.
═══════════════════════════════════════════════════════════════════════
```

### Governance Reason (Stoic Lens)

> "Sleep: 4.5h, Readiness: 32. High-difficulty task 'Finish Quarterly Report' postponed due to physical load. — **Stoic guidance:** Focus on what you can control right now. This task involves factors outside your current capacity; postponing is the rational choice."

---

## Files Modified

| File | Change |
|------|--------|
| `crates/pagi-core/src/shared.rs` | Added `GovernedTask`, `TaskDifficulty`, `GovernanceAction`, `TaskGovernor` |
| `crates/pagi-core/src/lib.rs` | Exported new types |
| `crates/pagi-core/src/knowledge/store.rs` | Added `set_governed_task`, `get_governed_task`, `list_governed_tasks`, `remove_governed_task`, `create_task_governor`, `evaluate_and_persist_tasks` |
| `crates/pagi-core/tests/task_governance_test.rs` | 20 comprehensive tests |
| `crates/pagi-core/tests/oikos_integration_audit.rs` | 10 integration audit tests (Closing of the Circuit) |
| `docs/dynamic_task_governance.md` | This documentation |

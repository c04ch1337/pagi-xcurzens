# Total Context Stress Test — Full-System Audit

This document describes the **Total Context Stress Test**: a multi-slot scenario that verifies SAGE_BOT synthesizes **Ethos** (philosophy), **Soma** (physical load), **Kardia** (Relational Map), and **Shadow** (private journal) into a single, context-aware reflection.

---

## Scenario: "High-Stress Day"

0. **Ethos (Slot 6):** Active philosophy set to **Stoicism** → reframing uses Dichotomy of Control (focus on what is within the user's control).
1. **Soma (Slot 8):** User has low sleep (4.0h) and low readiness (30) → Governor applies supportive tone.
2. **Kardia Map (Slot 7):** Two people are in the map:
   - **Partner** — High trust (0.9), Anxious attachment.
   - **Project Manager** — Low trust (0.3), Avoidant attachment.
3. **Shadow (Slot 9):** A journal entry: *"Argument with Partner about the Project Manager's deadlines."*
4. **ReflectShadow:** User asks SAGE_BOT to reframe this entry. The prompt must include:
   - **Ethos** — Philosophical lens (e.g. Stoic: "Focus on what you can control").
   - **Soma** — Supportive Tone instruction (physical load elevated).
   - **Kardia** — Partner (0.9, Anxious) and Project Manager (0.3, Avoidant).
   - **secure_purge** — Decrypted content and prompt are zeroed after the reflection is generated.

**Verify:** SAGE_BOT's reframing of the Project Manager conflict should explicitly mention focusing on what is **within the user's control** (Stoic Dichotomy of Control).

---

## Manual Test Flow

Prerequisites:

- Gateway running: `cargo run -p pagi-gateway` (port 8001).
- `PAGI_SHADOW_KEY` set (64 hex chars) so Shadow Vault and ReflectShadow work.

### Step 0: Ethos — Set Active Philosophy to Stoicism

Sets the philosophical lens for reframing to **Stoicism** (Dichotomy of Control). Stored in **Slot 6 (Ethos)** under `ethos/current`. ReflectShadow will append this to the reflection prompt.

```powershell
$body = @{
  goal = @{
    ExecuteSkill = @{
      name = "EthosSync"
      payload = @{ active_school = "Stoic" }
    }
  }
  tenant_id = "default"
} | ConvertTo-Json -Depth 5
Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8001/v1/execute" -ContentType "application/json" -Body $body
```

**Expected:** `status: ok`, `active_school: Stoic`. Reframing will emphasize what is within the user's control.

### Step 1: Soma Sync — Low Sleep & Low Readiness

Sends sleep 4.0h and readiness 30 to **Slot 8**. This triggers the BioGate cross-layer reaction: `grace_multiplier = 1.6`, `burnout_risk += 0.15`.

```powershell
$body = @{
  goal = @{
    ExecuteSkill = @{
      name = "BioGateSync"
      payload = @{
        sleep_hours = 4.0
        readiness_score = 30
      }
    }
  }
  tenant_id = "default"
} | ConvertTo-Json -Depth 5
Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8001/v1/execute" -ContentType "application/json" -Body $body
```

**Expected:** `status: ok`, response indicates BioGate cross-layer reaction active (e.g. `grace_multiplier = 1.6`).

### Step 2: Kardia Map — Partner (High Trust, Anxious)

```powershell
$body = @{
  goal = @{
    ExecuteSkill = @{
      name = "KardiaMap"
      payload = @{
        name = "Partner"
        relationship = "Partner"
        trust_score = 0.9
        attachment_style = "Anxious"
      }
    }
  }
  tenant_id = "default"
} | ConvertTo-Json -Depth 5
Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8001/v1/execute" -ContentType "application/json" -Body $body
```

**Expected:** `status: ok`, `name_slug: partner`.

### Step 3: Kardia Map — Project Manager (Low Trust, Avoidant)

```powershell
$body = @{
  goal = @{
    ExecuteSkill = @{
      name = "KardiaMap"
      payload = @{
        name = "Project Manager"
        relationship = "Boss"
        trust_score = 0.3
        attachment_style = "Avoidant"
        triggers = @("criticism", "micromanagement")
      }
    }
  }
  tenant_id = "default"
} | ConvertTo-Json -Depth 5
Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8001/v1/execute" -ContentType "application/json" -Body $body
```

**Expected:** `status: ok`, `name_slug: project_manager`.

### Step 4: Shadow Entry — DeepJournal

Insert the conflict sentence into the Shadow Vault. **Save the returned `record_id`** for Step 5 (ReflectShadow).

```powershell
$body = @{
  goal = @{
    ExecuteSkill = @{
      name = "DeepJournalSkill"
      payload = @{
        raw_entry = "Argument with Partner about the Project Manager's deadlines."
      }
    }
  }
  tenant_id = "default"
} | ConvertTo-Json -Depth 5
$r = Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8001/v1/execute" -ContentType "application/json" -Body $body
$recordId = $r.record_id   # e.g. "journal/1738..."
```

**Expected:** `status: ok`, `record_id` (e.g. `journal/1738...`). Vault must be unlocked (`PAGI_SHADOW_KEY` set).

### Step 5: ReflectShadow

Call ReflectShadow with the `record_id` from Step 4 and a valid `session_key`. The gateway validates `session_key` against `PAGI_SHADOW_KEY` before invoking the skill.

```powershell
# Use the same key as PAGI_SHADOW_KEY (64 hex chars) as session_key for testing.
$sessionKey = $env:PAGI_SHADOW_KEY
$body = @{
  goal = @{
    ExecuteSkill = @{
      name = "ReflectShadow"
      payload = @{
        record_id = $recordId
        session_key = $sessionKey
      }
    }
  }
  tenant_id = "default"
} | ConvertTo-Json -Depth 5
Invoke-RestMethod -Method Post -Uri "http://127.0.0.1:8001/v1/execute" -ContentType "application/json" -Body $body
```

**Expected:**

- `status: ok`, `reflection` contains the model’s reframing.
- **Prompt sent to the model (internal) must include:**
  - **Soma:** A line like `[Soma — Physical load elevated...]` and the supportive-tone instruction (because `get_effective_mental_state` has `grace_multiplier >= 1.5`).
  - **Kardia:** Relational Map block with **Partner** (trust 0.90, Anxious) and **Project Manager** (trust 0.30, Avoidant).
- **secure_purge:** Decrypted content and full prompt are zeroed in RAM after the reflection is generated; they are never logged or persisted.
- **Memory locking (Slot 9):** Decrypted Shadow buffers are held in a memory-locked region (Unix: `mlock`; Windows: `VirtualLock`) so the OS cannot swap them to disk; they are zeroed and unlocked on drop.

---

## Expected Logic Summary

| Check | Description |
|-------|-------------|
| **Ethos in prompt** | When `EthosSync` has set the active school (e.g. Stoic), ReflectShadow fetches `get_ethos_philosophical_policy()` and appends `get_philosophical_prompt()` (e.g. "Use Stoic principles... Focus on what you can control (Dichotomy of Control).") so the reframing explicitly focuses on what is within the user's control. |
| **Soma in prompt** | When SomaState has low sleep (e.g. 4h) or low readiness (e.g. 30), `get_effective_mental_state` returns `grace_multiplier = 1.6`. ReflectShadow then injects `[Soma — Physical load elevated...]` and `PHYSICAL_LOAD_SYSTEM_INSTRUCTION` into the reflection prompt. |
| **Kardia in prompt** | Content mentions "Partner" and "Project Manager". ReflectShadow calls `list_people()`, filters by name-in-content, and appends a "Mentioned relationships" line with trust_score and attachment_style for each. |
| **secure_purge** | After `generate_reflection` returns, ReflectShadow calls `secure_purge(raw_content)` and `secure_purge(prompt)` so sensitive data is zeroed before drop. |

---

## Synthesis Report (What to Observe)

When the test is run end-to-end:

1. **Philosophical lens (Ethos)** — With Ethos set to Stoicism, the reframing should **explicitly mention focusing on what is within the user's control** (Dichotomy of Control), rather than trying to change the Project Manager or the situation. This is the "soul" of the AGI's wisdom.
2. **Physical fatigue (Soma)** — The Governor has already raised `burnout_risk` and set `grace_multiplier` to 1.6. ReflectShadow adds an explicit Soma instruction so the reframing is gentle and non-demanding.
3. **Relational dynamics (Kardia)** — The reframing can acknowledge:
   - **Partner:** High trust, Anxious style (e.g. need for reassurance, fear of abandonment in conflict).
   - **Project Manager:** Low trust, Avoidant style (e.g. distance, triggers around criticism/micromanagement).
4. **Shadow** — The raw sentence lives only in encrypted storage and in the single LLM request; it is never written to normal logs or long-term model memory.

Together, SAGE_BOT synthesizes **Ethos** (philosophy), body state (Soma), relationship context (Kardia), and private content (Shadow) into one nuanced, Stoic-inflected, context-aware reflection.

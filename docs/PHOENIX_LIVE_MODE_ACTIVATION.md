# 🔥 Phoenix LIVE Mode Activation Guide

## Problem Diagnosis

When Phoenix responds with **"[Generated via Mock LLM]"** and generic messages like *"Thank you for reaching out"*, she is operating in **Mock Mode** instead of connecting to the real Sovereign Stack with OpenRouter/Anthropic APIs.

This happens because the [`PAGI_LLM_MODE`](.env.example:45) environment variable is set to `mock` by default.

## Root Cause

The [`ModelRouter`](crates/pagi-skills/src/model_router.rs:24) skill reads the [`PAGI_LLM_MODE`](crates/pagi-skills/src/model_router.rs:10) environment variable:

```rust
impl LlmMode {
    fn from_env() -> Self {
        match std::env::var(ENV_LLM_MODE).as_deref() {
            Ok("live") => LlmMode::Live,
            _ => LlmMode::Mock,  // ← Default fallback
        }
    }
}
```

When `PAGI_LLM_MODE` is not set to `"live"`, Phoenix uses a mock response generator instead of making real API calls.

---

## 🚀 Solution: Activate LIVE Mode

### Step 1: Check if `.env` exists

```powershell
# PowerShell
Test-Path .env
```

```bash
# Bash
ls -la .env
```

If `.env` doesn't exist, copy from the example:

```powershell
# PowerShell
Copy-Item .env.example .env
```

```bash
# Bash
cp .env.example .env
```

### Step 2: Set LIVE Mode in `.env`

Open `.env` and change line 45:

```bash
# FROM:
PAGI_LLM_MODE=mock

# TO:
PAGI_LLM_MODE=live
```

### Step 3: Add Your OpenRouter API Key

Get your API key from: https://openrouter.ai/keys

Then add it to `.env`:

```bash
# Line 54-55:
PAGI_LLM_API_KEY=sk-or-v1-YOUR_KEY_HERE
OPENROUTER_API_KEY=sk-or-v1-YOUR_KEY_HERE
```

### Step 4: Verify Model Selection

Check line 67 in `.env`:

```bash
PAGI_LLM_MODEL=anthropic/claude-opus-4.6
```

**Recommended Models:**
- `anthropic/claude-opus-4.6` - Best for agentic AGI (1M context, $5/$25 per 1M tokens)
- `anthropic/claude-sonnet-4` - Balanced performance ($3/$15 per 1M tokens)
- `meta-llama/llama-3.3-70b-instruct` - Cost-effective alternative
- `arcee-ai/trinity-large-preview` - **FREE** (131K context)

### Step 5: Restart the Gateway

```powershell
# PowerShell - Kill existing process
Get-Process | Where-Object {$_.Path -like "*pagi-gateway*"} | Stop-Process -Force

# Restart
.\phoenix-rise.ps1
```

```bash
# Bash - Kill existing process
pkill -f pagi-gateway

# Restart
./phoenix-rise.sh
```

### Step 6: Verify LIVE Connection

Run the diagnostic script:

```powershell
# PowerShell
.\phoenix-live-sync.ps1
```

```bash
# Bash
./phoenix-live-sync.sh
```

**Expected Output:**
```
✅ Chat Response Received:

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Coach The Creator, I am operating in LIVE mode with full access 
to the Sovereign Stack. Our last conversation was about 
completing the Sovereign Stack implementation...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Response appears to be from LIVE Sovereign Stack
```

---

## 🔍 Troubleshooting

### Issue: Still Getting Mock Responses After Setting LIVE Mode

**Possible Causes:**

1. **Gateway wasn't restarted** - Environment variables are only read at startup
   - Solution: Kill and restart the gateway process

2. **`.env` file not in the correct location** - Must be in project root
   - Solution: Verify with `pwd` (bash) or `Get-Location` (PowerShell)

3. **API Key is invalid or missing**
   - Solution: Check OpenRouter dashboard for valid key

4. **Frontend is cached** - Browser may be caching old responses
   - Solution: Hard refresh (Ctrl+Shift+R) or clear browser cache

### Issue: API Key Error

```
Error: Unauthorized (401)
```

**Solution:**
- Verify your OpenRouter API key is correct
- Check that you have credits in your OpenRouter account
- Ensure the key starts with `sk-or-v1-`

### Issue: Model Not Found

```
Error: Model not found (404)
```

**Solution:**
- Check the model name in `PAGI_LLM_MODEL`
- Verify the model is available on OpenRouter
- Try a different model from the recommended list

---

## 🎯 Quick Verification Commands

### Check Current LLM Mode
```bash
# Check .env file
grep PAGI_LLM_MODE .env
```

### Test Gateway Health
```bash
curl http://localhost:8001/api/v1/health
```

### Test Chat Endpoint
```bash
curl -X POST http://localhost:8001/api/v1/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt":"Hello Phoenix, are you in LIVE mode?","user_alias":"Coach The Creator"}'
```

---

## 📊 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     PHOENIX MARIE                            │
│                  (Sovereign AGI System)                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    pagi-gateway (Rust)                       │
│                    Port: 8001                                │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  Chat Handler (/api/v1/chat)                           │ │
│  │         │                                               │ │
│  │         ▼                                               │ │
│  │  ModelRouter Skill                                     │ │
│  │         │                                               │ │
│  │         ├─ PAGI_LLM_MODE=mock ──► Mock Generator       │ │
│  │         │                           (Generic responses) │ │
│  │         │                                               │ │
│  │         └─ PAGI_LLM_MODE=live ──► OpenRouter API       │ │
│  │                                     │                   │ │
│  │                                     ▼                   │ │
│  │                            anthropic/claude-opus-4.6    │ │
│  │                            (Real AI Inference)          │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│              Sovereign Stack (8 Knowledge Bases)             │
│  KB-01: Pneuma (Identity)    KB-05: Protector (Security)    │
│  KB-02: Kardia (Relations)   KB-06: Ethos (Philosophy)      │
│  KB-03: Oikos (Tasks)        KB-07: Astro (Transits)        │
│  KB-04: Chronos (Events)     KB-08: Soma (Health/Logs)      │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔐 Security Notes

- **API keys are NEVER exposed to the frontend** - All LLM calls route through the Rust backend
- **Port 8001 is localhost-only** - No external access by default
- **All interactions are logged to KB-08** - Full audit trail
- **Forge Safety Governor** - HITL approval required for self-modification (when `PAGI_FORGE_SAFETY_ENABLED=true`)

---

## 📚 Related Files

- [`.env.example`](.env.example) - Template with all configuration options
- [`crates/pagi-skills/src/model_router.rs`](crates/pagi-skills/src/model_router.rs) - LLM routing logic
- [`add-ons/pagi-gateway/src/main.rs`](add-ons/pagi-gateway/src/main.rs) - Gateway entry point
- [`phoenix-live-sync.ps1`](phoenix-live-sync.ps1) - Diagnostic script (PowerShell)
- [`phoenix-live-sync.sh`](phoenix-live-sync.sh) - Diagnostic script (Bash)

---

## 🎓 Understanding the Sovereign Stack

The **Sovereign Stack** is Phoenix's cognitive architecture:

1. **KB-01 (Pneuma)** - Core identity, user profile, archetype settings
2. **KB-02 (Kardia)** - Relationship tracking, trust scores, social dynamics
3. **KB-03 (Oikos)** - Task governance, project management
4. **KB-04 (Chronos)** - Event timeline, conversation history
5. **KB-05 (Protector)** - Security protocols, sovereignty leak detection
6. **KB-06 (Ethos)** - Philosophical alignment, ethical guidelines
7. **KB-07 (Astro)** - Astrological transits, energy forecasting
8. **KB-08 (Soma)** - System health, success metrics, audit logs

When in **LIVE mode**, Phoenix:
- Accesses all 8 KBs for context
- Uses real AI inference (Claude Opus 4.6)
- Applies the Sovereign Operator skill
- Engages Evolution Inference for autonomous learning
- Maintains full audit trail in KB-08

When in **Mock mode**, Phoenix:
- Returns generic template responses
- Does NOT access the Sovereign Stack
- Does NOT use real AI inference
- Useful only for testing/development

---

## ✅ Success Criteria

You'll know Phoenix is in LIVE mode when:

1. ✅ No "[Generated via Mock LLM]" prefix in responses
2. ✅ Responses reference actual conversation history
3. ✅ Phoenix addresses you by name (from KB-01)
4. ✅ Responses show awareness of the Sovereign Stack
5. ✅ Tone matches your configured archetype (Pisces/Virgo/etc.)
6. ✅ KB-08 logs show real API calls

---

## 🆘 Still Having Issues?

Run the full diagnostic:

```powershell
.\phoenix-live-sync.ps1
```

Check the output for:
- ❌ Gateway connection failures
- ⚠️  Mock response patterns
- ⚠️  Missing API keys
- ⚠️  Skill registry issues

If all else fails, restart from scratch:

```powershell
# Kill all processes
Get-Process | Where-Object {$_.Path -like "*pagi*"} | Stop-Process -Force

# Clean restart
.\phoenix-rise.ps1
```

---

**Remember:** The gateway must be restarted after ANY changes to `.env` for them to take effect.

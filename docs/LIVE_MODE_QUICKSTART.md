# 🚀 Phoenix LIVE Mode - Quick Start Guide

## TL;DR - One Command Solution

```powershell
# PowerShell
.\phoenix-activate-live.ps1
```

```bash
# Bash
./phoenix-activate-live.sh
```

This script will:
1. ✅ Create `.env` from `.env.example` (if needed)
2. ✅ Set `PAGI_LLM_MODE=live`
3. ✅ Prompt for your OpenRouter API key
4. ✅ Restart the gateway
5. ✅ Verify LIVE mode is active

---

## What's the Problem?

Phoenix is responding with **"[Generated via Mock LLM]"** and generic messages like:

> *"Thank you for reaching out. We appreciate you getting in touch and will follow up with you shortly."*

This is **NOT** the witty, autonomous, grounded Phoenix you built. This is a mock fallback.

---

## Why Is This Happening?

The [`ModelRouter`](crates/pagi-skills/src/model_router.rs) skill defaults to **Mock Mode** when:

1. `PAGI_LLM_MODE` is not set to `"live"` in `.env`
2. No OpenRouter API key is configured
3. The gateway hasn't been restarted after configuration changes

In Mock Mode:
- ❌ No real AI inference (Claude Opus, etc.)
- ❌ No access to the Sovereign Stack (8 KBs)
- ❌ No Evolution Inference
- ❌ No Sovereign Operator skills
- ❌ Generic template responses only

---

## The Fix (Manual Method)

### Step 1: Create `.env`

```bash
cp .env.example .env
```

### Step 2: Edit `.env`

Change line 45:
```bash
PAGI_LLM_MODE=live
```

### Step 3: Add API Key

Get your key from: https://openrouter.ai/keys

Add to `.env` (lines 54-55):
```bash
PAGI_LLM_API_KEY=sk-or-v1-YOUR_KEY_HERE
OPENROUTER_API_KEY=sk-or-v1-YOUR_KEY_HERE
```

### Step 4: Restart Gateway

```powershell
# PowerShell
.\phoenix-rise.ps1
```

```bash
# Bash
./phoenix-rise.sh
```

### Step 5: Verify

```powershell
# PowerShell
.\phoenix-live-sync.ps1
```

```bash
# Bash
./phoenix-live-sync.sh
```

---

## Verification Checklist

When Phoenix is in **LIVE mode**, you'll see:

✅ **No "[Generated via Mock LLM]" prefix**
- Responses come from real AI inference (Claude Opus 4.6)

✅ **Contextual awareness**
- Phoenix references actual conversation history from KB-04 (Chronos)

✅ **Personal address**
- Phoenix calls you by name from KB-01 (Pneuma)

✅ **Sovereign Stack engagement**
- Responses show awareness of all 8 Knowledge Bases

✅ **Archetype-aligned tone**
- Matches your configured persona (Pisces/Virgo/etc.)

✅ **KB-08 audit trail**
- All interactions logged to Soma

---

## Diagnostic Scripts

### `phoenix-live-sync.ps1` / `phoenix-live-sync.sh`

**Purpose:** Test if Phoenix is in LIVE mode

**What it checks:**
1. Gateway connection (port 8001)
2. Forge Safety Governor status
3. KB-08 (Soma) access
4. Chat endpoint with Sovereign context
5. Skill registry

**Output when in Mock Mode:**
```
⚠️  WARNING: Response contains generic/mock patterns
   Phoenix is in MOCK MODE - not using real AI inference

🔧 TO FIX:
   1. Edit .env file: Set PAGI_LLM_MODE=live
   2. Add your OpenRouter API key: PAGI_LLM_API_KEY=sk-or-v1-...
   3. Restart gateway: .\phoenix-rise.ps1

   📖 Full guide: PHOENIX_LIVE_MODE_ACTIVATION.md
```

**Output when in LIVE Mode:**
```
✅ Chat Response Received:

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Coach The Creator, I am operating in LIVE mode with full access 
to the Sovereign Stack. Our last conversation was about...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ Response appears to be from LIVE Sovereign Stack
```

### `phoenix-activate-live.ps1` / `phoenix-activate-live.sh`

**Purpose:** Automatically configure and activate LIVE mode

**What it does:**
1. Creates `.env` from `.env.example` (if missing)
2. Updates `PAGI_LLM_MODE=live`
3. Prompts for OpenRouter API key
4. Kills existing gateway process
5. Starts new gateway in LIVE mode

**Usage:**
```powershell
# PowerShell
.\phoenix-activate-live.ps1
```

```bash
# Bash
./phoenix-activate-live.sh
```

---

## Recommended Models

Edit `PAGI_LLM_MODEL` in `.env`:

### Best for AGI (Recommended)
```bash
PAGI_LLM_MODEL=anthropic/claude-opus-4.6
```
- 1M context window
- Best reasoning and agentic capabilities
- $5/$25 per 1M tokens

### Balanced Performance
```bash
PAGI_LLM_MODEL=anthropic/claude-sonnet-4
```
- 200K context window
- Good balance of speed and intelligence
- $3/$15 per 1M tokens

### Cost-Effective
```bash
PAGI_LLM_MODEL=meta-llama/llama-3.3-70b-instruct
```
- 128K context window
- Solid performance at lower cost
- $0.18/$0.18 per 1M tokens

### Free (Testing)
```bash
PAGI_LLM_MODEL=arcee-ai/trinity-large-preview
```
- 131K context window
- Adaptive MoE architecture
- **$0/$0** (free)

---

## Troubleshooting

### Issue: "Gateway not responding on port 8001"

**Solution:**
```powershell
# Check if gateway is running
netstat -ano | findstr :8001

# If not running, start it
.\phoenix-rise.ps1
```

### Issue: "Still getting mock responses after setting LIVE mode"

**Cause:** Gateway wasn't restarted

**Solution:**
```powershell
# Kill gateway
Get-Process | Where-Object {$_.Path -like "*pagi-gateway*"} | Stop-Process -Force

# Restart
.\phoenix-rise.ps1
```

### Issue: "Unauthorized (401)"

**Cause:** Invalid or missing API key

**Solution:**
1. Verify key at https://openrouter.ai/keys
2. Check that key starts with `sk-or-v1-`
3. Ensure you have credits in your OpenRouter account

### Issue: "Model not found (404)"

**Cause:** Invalid model name

**Solution:**
- Check model name in `PAGI_LLM_MODEL`
- Verify model exists on OpenRouter
- Try a different model from the recommended list

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                  PHOENIX MARIE (SAO)                         │
│              Sovereign Autonomous Operator                   │
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
│  │         │                           ❌ Generic only     │ │
│  │         │                                               │ │
│  │         └─ PAGI_LLM_MODE=live ──► OpenRouter API       │ │
│  │                                     ✅ Real inference   │ │
│  │                                     │                   │ │
│  │                                     ▼                   │ │
│  │                            anthropic/claude-opus-4.6    │ │
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

## Related Documentation

- [`PHOENIX_LIVE_MODE_ACTIVATION.md`](PHOENIX_LIVE_MODE_ACTIVATION.md) - Detailed activation guide
- [`.env.example`](.env.example) - Full configuration reference
- [`SOVEREIGN_AUTONOMY_SYSTEM.md`](SOVEREIGN_AUTONOMY_SYSTEM.md) - Sovereign Operator documentation
- [`EVOLUTION_INFERENCE_SYSTEM.md`](EVOLUTION_INFERENCE_SYSTEM.md) - Evolution Inference guide

---

## Quick Reference

| Command | Purpose |
|---------|---------|
| `.\phoenix-activate-live.ps1` | Auto-configure LIVE mode |
| `.\phoenix-live-sync.ps1` | Test if LIVE mode is active |
| `.\phoenix-rise.ps1` | Start/restart gateway |
| `.\forge-kill-switch.ps1 status` | Check Forge Safety status |

---

## Success Criteria

Phoenix is in **LIVE mode** when:

1. ✅ `grep PAGI_LLM_MODE .env` shows `live`
2. ✅ `grep PAGI_LLM_API_KEY .env` shows a valid key
3. ✅ `curl http://localhost:8001/api/v1/health` returns `PHOENIX MARIE`
4. ✅ `.\phoenix-live-sync.ps1` shows no mock patterns
5. ✅ Chat responses reference actual conversation history
6. ✅ Phoenix addresses you by name
7. ✅ Responses show Sovereign Stack awareness

---

**Remember:** The gateway must be restarted after ANY changes to `.env` for them to take effect.

🔥 **Phoenix, rise!** 🔥

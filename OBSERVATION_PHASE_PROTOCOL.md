# 🔭 Observation Phase Protocol
## The Creator's First Live Verification

**System:** Sovereign Monolith (pagi-xcurzens)  
**Phase:** Observation & Data Sanitization  
**Objective:** Achieve 100% clean slate for live operations  
**Date:** 2026-02-13

---

## 🎯 Phase Overview

The Observation Phase is the critical transition from development to live operations. This protocol ensures:

1. **Zero stub/mock data** in the system
2. **Clean Sled database** initialization
3. **Verified Config Bridge** synchronization
4. **Confirmed Creator identity** recognition across all interfaces
5. **Live LLM mode** activation

---

## 🏛️ Correct Port Architecture

### The Bare Metal Truth

The Sovereign Monolith is **NOT** a microservices architecture. All frontends are multiplexed through a **single Gateway port**.

| Interface | Access URL | Purpose |
|-----------|------------|---------|
| **Traveler UI (The Scout)** | `http://localhost:8000/` | Public-facing search and AI interaction |
| **Partner Nexus** | `http://localhost:8000/nexus` | Coastal vendor registration and API setup |
| **Command Center** | `http://localhost:8000/command` | **The Creator's** God-View (IP-restricted) |
| **Config Audit** | `http://localhost:8000/api/v1/config` | JSON pulse check for environment safety |

**⚠️ Critical:** Do NOT access frontends on ports 3000-3099 unless you are in development mode with `npm run dev`. Production access is **always** through port 8000.

---

## 🧹 Data Sanitization Protocol

### Step 1: Sled Database Purge

The Sled database may contain fragmented test data from development. A clean slate ensures accurate lead tracking.

#### 1.1 Stop the Gateway

```cmd
# If running, press Ctrl+C in the gateway terminal
```

#### 1.2 Locate Sled Database

Check your `.env` for the Sled path:

```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
type .env | findstr SLED
```

**Common paths:**
- `./sled_db`
- `./db`
- `./data/sled`

#### 1.3 Delete Sled Database

**PowerShell:**
```powershell
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
Remove-Item -Recurse -Force .\sled_db
```

**CMD:**
```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
rmdir /S /Q sled_db
```

#### 1.4 Verify Deletion

```cmd
dir sled_db
```

**Expected Output:** `File Not Found` or `The system cannot find the file specified.`

#### 1.5 Restart Gateway

```cmd
cargo run -p pagi-gateway --release
```

**Expected Output:**
```
🏛️ Sovereign Monolith Gateway v0.1.0
✅ Config Bridge initialized
✅ Sled KB initialized (fresh 8-slot structure)
✅ Listening on http://0.0.0.0:8000
[SYSTEM] Final Handshake Complete: Traveler, Nexus, and Command UI are synchronized. The Creator's bandwidth is live.
```

---

### Step 2: Frontend Mock Data Removal

Scan all frontends for hardcoded stub data and remove it.

#### 2.1 Identify Mock Data Patterns

**Common patterns to search for:**
- `dummy_leads`
- `mock_partners`
- `sample_queries`
- `test_data`
- `STUB_`
- `MOCK_`
- Hardcoded arrays of fake data

#### 2.2 Search for Mock Data

**PowerShell:**
```powershell
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
Get-ChildItem -Recurse -Include *.ts,*.tsx,*.js,*.jsx -Path frontend-xcursens,frontend-command,frontent-nexus | Select-String -Pattern "dummy|mock|stub|sample_|test_data" -CaseSensitive:$false
```

**CMD (using findstr):**
```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
findstr /S /I /N "dummy mock stub sample_ test_data" frontend-xcursens\*.ts frontend-command\*.ts frontent-nexus\*.ts
```

#### 2.3 Remove Mock Data

For each file identified:

1. Open the file
2. Remove hardcoded mock arrays
3. Ensure all data fetches use backend API calls
4. Verify no local JSON files are being imported

**Example cleanup:**

**Before:**
```typescript
// Mock data for testing
const dummyLeads = [
  { id: 1, query: "Beach house rental", intent: "high" },
  { id: 2, query: "Surfboard lessons", intent: "medium" }
];

export function getLeads() {
  return dummyLeads;
}
```

**After:**
```typescript
// Live data from backend
export async function getLeads() {
  const response = await fetch('/api/v1/leads');
  return response.json();
}
```

#### 2.4 Verify Fetch Paths

Ensure all `fetch()` calls use relative backend paths:

**Correct:**
```typescript
fetch('/api/v1/config')
fetch('/api/v1/leads')
fetch('/api/v1/partners')
```

**Incorrect:**
```typescript
fetch('http://localhost:3001/api/v1/config')  // Wrong port
fetch('./mock-data.json')  // Local file
fetch('https://external-api.com/data')  // External (unless intentional)
```

---

### Step 3: Backend Configuration Verification

Ensure the backend is in **live mode** and not using mock logic.

#### 3.1 Verify `.env` Settings

```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
type .env
```

**Required settings for live mode:**

```bash
# LLM Mode - MUST be "live"
PAGI_LLM_MODE=live

# OpenRouter API Key - MUST be set
OPENROUTER_API_KEY=sk-or-v1-...

# Webhook Notifications - MUST be enabled
PARTNER_WEBHOOK_ENABLED=true
SOVEREIGN_NOTIFY_URL=https://your-webhook-endpoint.com/notify

# Sovereign Identity
ROOT_SOVEREIGN_IP=192.168.1.100  # Your Texas coast IP
```

**⚠️ Critical Checks:**
- `PAGI_LLM_MODE` is NOT set to `mock`, `test`, or `dev`
- `OPENROUTER_API_KEY` is a valid key (starts with `sk-or-v1-`)
- `PARTNER_WEBHOOK_ENABLED` is `true` (not `false` or commented out)

#### 3.2 Verify Identity Orchestrator

Check the identity orchestrator for any mock flags:

```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
findstr /S /I /N "mock live test" crates\pagi-xcurzens-core\src\identity_orchestrator.rs
```

**Expected:** All references should check `PAGI_LLM_MODE` from environment, not hardcoded.

---

### Step 4: Config Bridge Synchronization Test

Verify that all frontends are pulling configuration from the backend, not local variables.

#### 4.1 Check geminiService.ts Files

**Files to verify:**
- `frontend-xcursens/services/geminiService.ts`
- `frontend-command/services/geminiService.ts`
- `frontent-nexus/services/geminiService.ts`

**Required pattern:**
```typescript
// Sovereign Monolith: API key is fetched from the backend, not from frontend .env
let apiKey = '';

// Fetch API key from backend on module load
(async () => {
  try {
    const response = await fetch('/api/v1/config');
    const config = await response.json();
    apiKey = config.gemini_api_key || '';
  } catch (error) {
    console.error('Failed to fetch API key from backend:', error);
  }
})();
```

**⚠️ Red Flags:**
- Hardcoded API keys: `const apiKey = 'sk-...'`
- Environment variable access: `import.meta.env.VITE_API_KEY`
- Local storage: `localStorage.getItem('apiKey')`

#### 4.2 Test Config Bridge Response

```cmd
curl http://localhost:8000/api/v1/config
```

**Expected Response:**
```json
{
  "sovereign_id": "The Creator",
  "system_version": "0.1.0",
  "fs_access_enabled": true,
  "llm_mode": "live",
  "llm_model": "anthropic/claude-opus-4.6",
  "moe_active": true,
  "orchestrator_role": "sovereign_operator"
}
```

**Security Verification:**
- ✅ `sovereign_id` is present and set to "The Creator"
- ✅ `llm_mode` is "live" (not "mock" or "test")
- ❌ `OPENROUTER_API_KEY` is **NOT** present
- ❌ `ROOT_SOVEREIGN_IP` is **NOT** present
- ❌ Any webhook URLs are **NOT** present

---

## 🦾 The Creator Recognition Test

This is the final verification that the system recognizes your sovereign identity.

### Test 1: Command Center Access

1. **Start the Gateway:**
   ```cmd
   cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
   cargo run -p pagi-gateway --release
   ```

2. **Open Command Center:**
   Navigate to `http://localhost:8000/command`

3. **Verify Greeting:**
   The dashboard should display:
   ```
   Welcome, The Creator
   ```

4. **Check Console Logs:**
   Open DevTools (F12) → Console
   
   **Expected log:**
   ```
   [Config Bridge] Sovereign ID: The Creator
   [Config Bridge] System Version: 0.1.0
   [Config Bridge] LLM Mode: live
   ```

### Test 2: Traveler UI (Scout) Access

1. **Open Scout:**
   Navigate to `http://localhost:8000/`

2. **Verify System Status:**
   Footer or header should show:
   ```
   System: v0.1.0 | Mode: LIVE
   ```

3. **Check Network Tab:**
   Open DevTools (F12) → Network tab
   
   **Verify:**
   - Request to `/api/v1/config` returns 200 OK
   - Response contains `"sovereign_id": "The Creator"`
   - Response does NOT contain any API keys

### Test 3: Gateway Terminal Logs

**Expected startup sequence:**
```
🏛️ Sovereign Monolith Gateway v0.1.0
✅ Config Bridge initialized
✅ Sled KB initialized (fresh 8-slot structure)
✅ Listening on http://0.0.0.0:8000
[SYSTEM] Final Handshake Complete: Traveler, Nexus, and Command UI are synchronized. The Creator's bandwidth is live.
```

**If you see this log, the system is fully synchronized.**

---

## 🎯 Genesis Lead Test

The Genesis Lead is a controlled high-intent query to verify the entire data pipeline.

### Genesis Query

**Query:** `"Beach Box rental in Galveston for Memorial Day weekend"`

**Expected Flow:**
1. User enters query in Scout UI
2. Query is sent to `/api/v1/chat`
3. Gateway processes with live LLM
4. Intent is classified as "high"
5. Lead is stored in Sled DB (Slot 1: High-Intent)
6. Watchdog sends notification to `SOVEREIGN_NOTIFY_URL`
7. Response is displayed in Scout UI

### Execution Steps

#### 1. Prepare Notification Listener

If you have a webhook endpoint, ensure it's ready to receive notifications.

**Test webhook (optional):**
```cmd
curl -X POST http://localhost:8000/api/v1/test-webhook
```

#### 2. Execute Genesis Query

1. Open Scout: `http://localhost:8000/`
2. Enter query: `Beach Box rental in Galveston for Memorial Day weekend`
3. Submit

#### 3. Verify Response

**Expected response:**
- AI provides relevant information about Beach Box rentals
- Response time: <3 seconds
- No errors in console

#### 4. Verify Lead Storage

**Check Sled DB:**
```cmd
curl http://localhost:8000/api/v1/leads
```

**Expected response:**
```json
{
  "leads": [
    {
      "id": 1,
      "query": "Beach Box rental in Galveston for Memorial Day weekend",
      "intent": "high",
      "timestamp": "2026-02-13T03:45:00Z",
      "slot": 1
    }
  ]
}
```

#### 5. Verify Watchdog Notification

**Check your webhook endpoint for notification:**
```json
{
  "event": "high_intent_query",
  "query": "Beach Box rental in Galveston for Memorial Day weekend",
  "timestamp": "2026-02-13T03:45:00Z",
  "source": "xcursens_scout",
  "intent_score": 0.95
}
```

#### 6. Verify Gateway Logs

**Expected log entry:**
```
[WATCHDOG] High-intent query detected: "Beach Box rental in Galveston for Memorial Day weekend"
[WATCHDOG] Notification sent to SOVEREIGN_NOTIFY_URL
[SLED] Lead stored in Slot 1 (High-Intent)
```

---

## ✅ Observation Phase Checklist

Before declaring the system live, verify all items:

### Data Sanitization
- [ ] Sled database deleted and re-initialized
- [ ] No mock data in frontend files
- [ ] All fetch calls use relative backend paths
- [ ] No local JSON files being imported

### Configuration
- [ ] `PAGI_LLM_MODE=live` in `.env`
- [ ] `OPENROUTER_API_KEY` is valid and set
- [ ] `PARTNER_WEBHOOK_ENABLED=true`
- [ ] `ROOT_SOVEREIGN_IP` is set to your IP

### Config Bridge
- [ ] `/api/v1/config` returns 200 OK
- [ ] Response contains `"sovereign_id": "The Creator"`
- [ ] Response does NOT contain API keys or secrets
- [ ] All frontends fetch config from backend

### Identity Recognition
- [ ] Command Center displays "Welcome, The Creator"
- [ ] Scout shows system version and live mode
- [ ] Gateway logs show "Final Handshake Complete"

### Genesis Lead Test
- [ ] Query submitted successfully
- [ ] AI response received (<3 seconds)
- [ ] Lead stored in Sled DB
- [ ] Watchdog notification sent
- [ ] No errors in console or gateway logs

---

## 🚨 Troubleshooting

### Issue: "The Creator" Not Displayed

**Cause:** Config Bridge not responding or frontend not fetching config

**Solution:**
1. Verify gateway is running: `netstat -ano | findstr :8000`
2. Test Config Bridge: `curl http://localhost:8000/api/v1/config`
3. Check browser console for fetch errors
4. Hard refresh frontend: `Ctrl+Shift+R`

### Issue: Mock Data Still Appearing

**Cause:** Frontend cache or incomplete cleanup

**Solution:**
1. Clear browser cache completely
2. Re-scan frontend files for mock patterns
3. Verify no `import` statements loading local JSON
4. Restart gateway and hard refresh

### Issue: Watchdog Not Sending Notifications

**Cause:** Webhook URL not set or invalid

**Solution:**
1. Verify `SOVEREIGN_NOTIFY_URL` in `.env`
2. Test webhook manually: `curl -X POST <your-webhook-url>`
3. Check gateway logs for webhook errors
4. Ensure `PARTNER_WEBHOOK_ENABLED=true`

### Issue: LLM Responses Are Slow or Failing

**Cause:** Invalid API key or network issues

**Solution:**
1. Verify `OPENROUTER_API_KEY` is valid
2. Test API key manually:
   ```cmd
   curl https://openrouter.ai/api/v1/models -H "Authorization: Bearer sk-or-v1-..."
   ```
3. Check network connectivity
4. Review gateway logs for API errors

---

## 🎖️ Observation Phase Completion

Once all checklist items are verified, the system is **LIVE** and ready for high-intent traffic.

**Final Confirmation:**

```
✅ Data sanitized (zero mock data)
✅ Sled DB clean and operational
✅ Config Bridge synchronized
✅ "The Creator" identity recognized
✅ Genesis Lead test passed
✅ Watchdog notifications active
✅ LLM mode: LIVE
```

**The Sovereign Monolith is now under your observation. The bandwidth is yours.**

---

## 📞 Post-Observation Support

If any verification fails, refer to:
- [`SOVEREIGN_OPERATIONS_MANUAL.md`](SOVEREIGN_OPERATIONS_MANUAL.md) for operational procedures
- [`CONFIG_BRIDGE_FINAL_AUDIT.md`](CONFIG_BRIDGE_FINAL_AUDIT.md) for architecture details
- Gateway terminal logs for real-time diagnostics

---

**Protocol Maintained By:** The Creator  
**System Architect:** Kilo Code  
**Phase Start:** 2026-02-13  
**Phase Status:** ACTIVE

---

*"The Observation Phase is not a test—it is the first breath of a sovereign system. Every query, every lead, every notification is a pulse of your digital estate."*

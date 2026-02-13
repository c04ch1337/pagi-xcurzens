# 🏛️ Sovereign Operations Manual
## The Creator's Guide to Monolith Management

**System:** Sovereign Monolith (pagi-xcurzens)  
**Version:** Gateway v0.1.0  
**Sovereign Identity:** The Creator  
**Manual Version:** 1.0.0  
**Last Updated:** 2026-02-13

---

## 📋 Table of Contents

1. [System Overview](#system-overview)
2. [Daily Operations](#daily-operations)
3. [Configuration Management](#configuration-management)
4. [Security Operations](#security-operations)
5. [Backup & Recovery](#backup--recovery)
6. [Monitoring & Health Checks](#monitoring--health-checks)
7. [Troubleshooting](#troubleshooting)
8. [Key Rotation](#key-rotation)
9. [Emergency Procedures](#emergency-procedures)

---

## 🎯 System Overview

### Architecture Components

The Sovereign Monolith consists of three primary layers:

1. **The Cerebellum (Rust Core)**
   - Location: `C:/Users/JAMEYMILNER/AppData/Local/pagi-xcurzens/add-ons/pagi-gateway`
   - Function: Manages Sled KBs, NEXUS bridge, Config Bridge, and API routing
   - Port: 8000 (Single Gateway Port - All traffic multiplexed)

2. **The Senses (Frontends)**
   - **XCURSENS Scout (Traveler UI):** Served at `http://localhost:8000/`
   - **NEXUS Interface (Partner Portal):** Served at `http://localhost:8000/nexus`
   - **Command Center (God-View):** Served at `http://localhost:8000/command` (IP-restricted)

3. **The Guard (Middleware)**
   - IP-based access control for Command Center
   - Config Bridge security filtering
   - Watchdog notification system

### Single Source of Truth

**File:** `C:/Users/JAMEYMILNER/AppData/Local/pagi-xcurzens/.env`

All system configuration flows from this single file. No frontend has direct environment access.

---

## 🚀 Daily Operations

### Starting the System

#### 1. Start the Gateway (Single Command)

```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
cargo run -p pagi-gateway --release
```

**Expected Output:**
```
🏛️ Sovereign Monolith Gateway v0.1.0
✅ Config Bridge initialized
✅ Sled KB loaded
✅ Listening on http://0.0.0.0:8000
[SYSTEM] Final Handshake Complete: Traveler, Nexus, and Command UI are synchronized. The Creator's bandwidth is live.
```

**Startup Time:** ~2-5 seconds (release mode)

**⚠️ Important:** The Sovereign Monolith uses a **Bare Metal Architecture**. All frontends are served through the single Gateway port (8000). Do NOT start frontends separately with `npm run dev` unless you are in development mode.

#### 2. Access the Interfaces

Once the gateway is running, access all interfaces through port 8000:

| Interface | URL | Purpose |
|-----------|-----|---------|
| **Traveler UI (Scout)** | `http://localhost:8000/` | Public-facing search and AI interaction |
| **Partner Nexus** | `http://localhost:8000/nexus` | Coastal vendor registration and API setup |
| **Command Center** | `http://localhost:8000/command` | The Creator's God-View (IP-restricted) |
| **Config Audit** | `http://localhost:8000/api/v1/config` | JSON pulse check for environment safety |

**Development Mode Only:** If you need to run frontends in development mode (hot reload):
```cmd
# Terminal 1: Gateway
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
cargo run -p pagi-gateway

# Terminal 2: Frontend (if needed for development)
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\frontend-xcursens
npm run dev
```
In development mode, frontends may appear on ports 3000-3099, but production access is always through port 8000.

### Stopping the System

#### Graceful Shutdown

1. Stop gateway: `Ctrl+C` in gateway terminal
2. Verify Sled DB is flushed (automatic on shutdown)
3. Wait for confirmation: `✅ Sled DB flushed successfully`

#### Force Stop (Emergency Only)

```cmd
taskkill /F /IM cargo.exe
taskkill /F /IM node.exe
```

**⚠️ Warning:** Force stop may cause Sled DB corruption. Use only in emergencies.

---

## ⚙️ Configuration Management

### Viewing Current Configuration

#### Via API (Recommended)
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

#### Via File
```cmd
type C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\.env
```

### Modifying Configuration

#### 1. Edit the Root `.env`

```cmd
notepad C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\.env
```

#### 2. Restart Gateway

```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
cargo run -p pagi-gateway --release
```

**⚠️ Important:** Frontends automatically sync with new config via Config Bridge. No frontend restart required.

### Common Configuration Changes

#### Change LLM Model
```bash
# In .env
PAGI_LLM_MODEL=anthropic/claude-sonnet-4.5
```

#### Enable/Disable File System Access
```bash
# In .env
PAGI_FS_ACCESS_ENABLED=true  # or false
PAGI_FS_ROOT=C:/Users/JAMEYMILNER/Documents
```

#### Adjust Tick Rate
```bash
# In .env
PAGI_TICK_RATE_SECS=5  # seconds between system ticks
```

---

## 🔒 Security Operations

### Config Bridge Security Verification

#### Test 1: Verify No Secret Leakage

```cmd
curl http://localhost:8000/api/v1/config | findstr "OPENROUTER_API_KEY"
```

**Expected Output:** (empty - no matches)

**If secrets are exposed:**
1. Immediately stop the gateway
2. Review `add-ons/pagi-gateway/src/main.rs` lines 2178-2227
3. Verify strict filtering is in place
4. Report to system architect

#### Test 2: Verify Identity Recognition

```cmd
curl http://localhost:8000/api/v1/config | findstr "sovereign_id"
```

**Expected Output:**
```
"sovereign_id": "The Creator",
```

#### Test 3: Browser DevTools Audit

1. Open Command Center: `http://localhost:3001/command`
2. Open DevTools (F12) → Network tab
3. Filter for `config`
4. Inspect response payload
5. Verify no `OPENROUTER_API_KEY` or `ROOT_SOVEREIGN_IP` present

### IP Access Control

#### View Current Sovereign IP

```cmd
curl http://localhost:8000/api/v1/config | findstr "sovereign_ip"
```

**Note:** Sovereign IP is NOT exposed via Config Bridge. It's backend-only.

#### Update Sovereign IP

```bash
# In .env
ROOT_SOVEREIGN_IP=192.168.1.100  # Your Texas coast IP
```

Restart gateway to apply.

---

## 💾 Backup & Recovery

### Critical Files to Backup

1. **Configuration:** `.env`
2. **Knowledge Base:** `sled_db/` directory
3. **Custom Crates:** `crates/` directory
4. **Frontend Customizations:** `frontend-*/` directories

### Backup Procedure

#### Daily Backup (Automated)

Create a PowerShell script: `backup-sovereign.ps1`

```powershell
# Sovereign Monolith Daily Backup
$timestamp = Get-Date -Format "yyyy-MM-dd_HHmmss"
$backupDir = "C:\Users\JAMEYMILNER\Documents\sovereign-backups\$timestamp"

New-Item -ItemType Directory -Path $backupDir -Force

# Backup .env
Copy-Item "C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\.env" "$backupDir\.env"

# Backup Sled DB
Copy-Item -Recurse "C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\sled_db" "$backupDir\sled_db"

# Backup custom crates
Copy-Item -Recurse "C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\crates" "$backupDir\crates"

Write-Host "✅ Backup completed: $backupDir"
```

**Schedule with Task Scheduler:**
```cmd
schtasks /create /tn "SovereignBackup" /tr "powershell.exe -File C:\path\to\backup-sovereign.ps1" /sc daily /st 03:00
```

#### Manual Backup

```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
xcopy .env C:\Users\JAMEYMILNER\Documents\sovereign-backups\.env-backup /Y
xcopy sled_db C:\Users\JAMEYMILNER\Documents\sovereign-backups\sled_db-backup /E /I /Y
```

### Recovery Procedure

#### Restore from Backup

1. **Stop the gateway**
2. **Restore .env:**
   ```cmd
   copy C:\Users\JAMEYMILNER\Documents\sovereign-backups\.env-backup C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\.env /Y
   ```
3. **Restore Sled DB:**
   ```cmd
   rmdir /S /Q C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\sled_db
   xcopy C:\Users\JAMEYMILNER\Documents\sovereign-backups\sled_db-backup C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\sled_db /E /I /Y
   ```
4. **Restart gateway**

---

## 📊 Monitoring & Health Checks

### System Health Endpoint

```cmd
curl http://localhost:8000/api/v1/health
```

**Expected Response:**
```json
{
  "status": "healthy",
  "uptime_secs": 3600,
  "version": "0.1.0",
  "sled_kb_status": "operational"
}
```

### Performance Metrics

#### Gateway Response Time

```cmd
curl -w "\nTime: %{time_total}s\n" http://localhost:8000/api/v1/config
```

**Target:** <10ms for Config Bridge requests

#### Frontend Load Time

1. Open DevTools (F12) → Network tab
2. Navigate to frontend
3. Check "Load" time in Network summary

**Target:** <2s for initial load

### Watchdog Notifications

The Watchdog monitors high-intent queries and sends notifications to your `SOVEREIGN_NOTIFY_URL`.

#### Test Watchdog

1. Open XCURSENS Scout: `http://localhost:3001`
2. Enter query: "Beach Box rental in Galveston"
3. Check your notification endpoint for alert

**Expected Notification:**
```json
{
  "event": "high_intent_query",
  "query": "Beach Box rental in Galveston",
  "timestamp": "2026-02-13T03:30:00Z",
  "source": "xcursens_scout"
}
```

### Log Monitoring

#### Gateway Logs

**Location:** Terminal output (stdout)

**Key Events to Monitor:**
- `✅ Config Bridge initialized`
- `⚠️ Failed to load .env`
- `❌ Sled DB error`
- `🔒 IP access denied`

#### Frontend Logs

**Location:** Browser DevTools → Console

**Key Events to Monitor:**
- `Failed to fetch API key from backend`
- `Config Bridge connection failed`
- `WebSocket disconnected`

---

## 🔧 Troubleshooting

### Gateway Won't Start

#### Symptom: `Error: Address already in use`

**Cause:** Port 8000 is occupied

**Solution:**
```cmd
netstat -ano | findstr :8000
taskkill /F /PID <PID>
```

#### Symptom: `Failed to load .env`

**Cause:** `.env` file missing or malformed

**Solution:**
```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
copy .env.example .env
notepad .env
```
Fill in required values and restart.

#### Symptom: `Sled DB corruption detected`

**Cause:** Unclean shutdown or disk error

**Solution:**
```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
rmdir /S /Q sled_db
```
Restore from backup or allow fresh initialization.

### Frontend Issues

#### Symptom: "The Creator" not displayed

**Cause:** Config Bridge not responding

**Diagnosis:**
```cmd
curl http://localhost:8000/api/v1/config
```

**Solution:**
1. Verify gateway is running
2. Check browser console for errors
3. Clear browser cache and reload

#### Symptom: API key errors in console

**Cause:** Config Bridge filtering too strict or backend down

**Diagnosis:**
```cmd
curl http://localhost:8000/api/v1/config | findstr "gemini_api_key"
```

**Solution:**
1. Verify `OPENROUTER_API_KEY` is set in `.env`
2. Restart gateway
3. Hard refresh frontend (Ctrl+Shift+R)

### Performance Issues

#### Symptom: Slow Config Bridge responses (>100ms)

**Cause:** System resource contention

**Diagnosis:**
```cmd
wmic cpu get loadpercentage
wmic memorychip get capacity
```

**Solution:**
1. Close unnecessary applications
2. Restart gateway in release mode: `cargo run -p pagi-gateway --release`
3. Consider increasing system resources

---

## 🔑 Key Rotation

### OpenRouter API Key Rotation

#### 1. Generate New Key
Visit: https://openrouter.ai/keys

#### 2. Update `.env`
```bash
# Old key (comment out)
# OPENROUTER_API_KEY=sk-or-v1-old-key

# New key
OPENROUTER_API_KEY=sk-or-v1-new-key
```

#### 3. Restart Gateway
```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
cargo run -p pagi-gateway --release
```

#### 4. Verify
```cmd
curl http://localhost:8000/api/v1/chat -X POST -H "Content-Type: application/json" -d "{\"message\":\"test\"}"
```

**Expected:** Successful response (not 401 Unauthorized)

### Webhook URL Rotation

#### 1. Update `.env`
```bash
SOVEREIGN_NOTIFY_URL=https://new-webhook-endpoint.com/notify
```

#### 2. Restart Gateway

#### 3. Test Watchdog
Send high-intent query and verify notification arrives at new endpoint.

---

## 🚨 Emergency Procedures

### System Compromise Detected

#### Immediate Actions

1. **Isolate the System**
   ```cmd
   netsh advfirewall set allprofiles state on
   netsh advfirewall firewall add rule name="Block All Inbound" dir=in action=block
   ```

2. **Stop All Services**
   ```cmd
   taskkill /F /IM cargo.exe
   taskkill /F /IM node.exe
   ```

3. **Backup Current State**
   ```cmd
   xcopy C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens C:\Users\JAMEYMILNER\Documents\emergency-backup-%date% /E /I /Y
   ```

4. **Rotate All Keys**
   - Generate new OpenRouter API key
   - Update all webhook URLs
   - Change Sovereign IP if compromised

5. **Audit Logs**
   - Review gateway terminal output
   - Check browser DevTools Network tab
   - Inspect Sled DB for unauthorized entries

6. **Restore from Clean Backup**
   - Use last known good backup
   - Verify integrity before restart

### Data Loss Event

#### If Sled DB is Corrupted

1. **Stop gateway immediately**
2. **Attempt recovery:**
   ```cmd
   cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
   cargo run -p pagi-gateway -- --recover-sled
   ```
3. **If recovery fails, restore from backup**
4. **Document data loss extent**

### Gateway Unresponsive

#### Hard Reset Procedure

1. **Force stop all processes:**
   ```cmd
   taskkill /F /IM cargo.exe
   taskkill /F /IM pagi-gateway.exe
   ```

2. **Clear temporary files:**
   ```cmd
   del /Q C:\Users\JAMEYMILNER\AppData\Local\Temp\pagi-*
   ```

3. **Restart in debug mode:**
   ```cmd
   cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
   set RUST_LOG=debug
   cargo run -p pagi-gateway
   ```

4. **Monitor output for errors**

---

## 📞 Support & Escalation

### Self-Diagnosis Checklist

Before escalating, verify:

- [ ] Gateway is running (`netstat -ano | findstr :8000`)
- [ ] `.env` file exists and is valid
- [ ] Sled DB directory is accessible
- [ ] No port conflicts
- [ ] Sufficient disk space (>1GB free)
- [ ] Network connectivity (if using external APIs)

### Log Collection for Support

```cmd
cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens
cargo run -p pagi-gateway > gateway-log-%date%.txt 2>&1
```

Collect:
1. `gateway-log-*.txt`
2. `.env` (redact sensitive keys)
3. Browser console output (screenshot)
4. Network tab HAR file (if frontend issue)

---

## 🎖️ Sovereignty Principles

### The Three Laws of Sovereign Operations

1. **The .env is Sacred:** All configuration flows from the root `.env`. Never hardcode secrets.
2. **The Config Bridge is Sovereign:** Frontends are stateless clients. They receive, never possess.
3. **The Creator is Immutable:** Your identity is hardcoded in the binary, not in configuration.

### Operational Discipline

- **Daily:** Verify system health endpoint
- **Weekly:** Review gateway logs for anomalies
- **Monthly:** Rotate API keys and backup `.env`
- **Quarterly:** Full system audit and backup verification

---

## 📚 Quick Reference

### Essential Commands

| Task | Command |
|------|---------|
| Start Gateway | `cd C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens && cargo run -p pagi-gateway --release` |
| Check Health | `curl http://localhost:8000/api/v1/health` |
| View Config | `curl http://localhost:8000/api/v1/config` |
| Edit .env | `notepad C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\.env` |
| Backup Now | `xcopy C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\.env C:\Users\JAMEYMILNER\Documents\.env-backup /Y` |
| Check Port | `netstat -ano \| findstr :8000` |

### Critical File Paths

| Component | Path |
|-----------|------|
| Root .env | `C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\.env` |
| Gateway Binary | `C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\target\release\pagi-gateway.exe` |
| Sled DB | `C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\sled_db\` |
| Config Bridge Code | `C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens\add-ons\pagi-gateway\src\main.rs:2178` |

### Port Map

| Service | Port | Access |
|---------|------|--------|
| Gateway API | 8000 | Public |
| XCURSENS Scout | 3001 | Public |
| NEXUS Interface | 3002 | Public |
| Command Center | 3003 | Sovereign IP Only |

---

## 🏁 Final Notes

This manual is your operational bible for the Sovereign Monolith. Keep it accessible on your local machine. The system is designed for maximum autonomy—you control every aspect from a single `.env` file and a single Rust binary.

**The bandwidth is yours. The sovereignty is absolute.**

---

**Manual Maintained By:** The Creator  
**System Architect:** Kilo Code  
**Last Audit:** 2026-02-13  
**Next Review:** Upon first production incident

---

*"A sovereign system requires a sovereign operator. This manual ensures you remain in absolute control."*

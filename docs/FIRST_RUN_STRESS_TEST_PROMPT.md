# 🧪 Phoenix First-Run Stress Test - Cursor IDE Agent Prompt

## Role: Phoenix Release Engineer

You are responsible for validating the **zero-dependency first-run experience** of Phoenix Marie. Your mission is to ensure that a brand-new user can go from "download" to "chatting" with zero manual intervention.

---

## 🎯 Test Objective

Validate that Phoenix can:
1. **Boot from a clean slate** (no existing Qdrant, no API key)
2. **Auto-download Qdrant** without user intervention
3. **Initialize all systems** in the correct order
4. **Provide clear UI feedback** at each stage
5. **Gracefully handle errors** and guide the user

---

## 📋 Pre-Test Setup: Clean Slate Protocol

### Step 1: Backup Existing State
```bash
# Backup current installation (if exists)
mv ./bin ./bin.backup
mv ./data ./data.backup
mv .env .env.backup
mv user_config.toml user_config.toml.backup
```

### Step 2: Verify Clean State
```bash
# Verify no Qdrant binary
ls ./bin/qdrant 2>/dev/null && echo "❌ Qdrant binary still exists" || echo "✅ No Qdrant binary"

# Verify no Qdrant data
ls ./data/qdrant 2>/dev/null && echo "❌ Qdrant data still exists" || echo "✅ No Qdrant data"

# Verify no API key in environment
env | grep -E "OPENROUTER_API_KEY|PAGI_LLM_API_KEY" && echo "❌ API key in environment" || echo "✅ No API key"

# Verify no .env file
ls .env 2>/dev/null && echo "❌ .env file exists" || echo "✅ No .env file"
```

### Step 3: Prepare Test Environment
```bash
# Copy .env.example but leave API key blank
cp .env.example .env

# Ensure no API key is set
sed -i 's/^OPENROUTER_API_KEY=.*/OPENROUTER_API_KEY=/' .env
```

---

## 🚀 Test Execution: First-Run Sequence

### Phase 1: Launch Phoenix

```bash
# Start Phoenix with timing
echo "🔥 Starting Phoenix at $(date +%H:%M:%S)"
time ./phoenix-rise.sh
```

**Monitor for**:
- ✅ Port cleanup messages
- ✅ "Memory Engine (Qdrant) Initialization" phase
- ✅ "Memory Engine not detected" message
- ✅ "Phoenix will auto-initialize it" message

### Phase 2: Qdrant Download Monitoring

**Watch the logs**:
```bash
# In another terminal
tail -f /tmp/phoenix-gateway.log
```

**Expected log sequence**:
```
[INFO] 🧠 Initializing Memory Engine (Qdrant)...
[INFO] 🔍 Qdrant not detected. Initializing Memory Engine...
[INFO] 📥 Downloading Qdrant v1.7.4...
[INFO] Downloading from: https://github.com/qdrant/qdrant/releases/download/v1.7.4/qdrant-x86_64-unknown-linux-musl.tar.gz
[INFO] ✅ Qdrant binary downloaded to ./bin/qdrant
[INFO] 🚀 Starting Qdrant on port 6333...
[INFO] ✅ Memory Engine (Qdrant) initialized successfully
```

**Verify**:
- ✅ Download progress is visible
- ✅ No timeout errors
- ✅ Binary is extracted correctly
- ✅ Permissions are set (Unix)

### Phase 3: Health Check Verification

**Monitor health check polling**:
```bash
# Watch for health check attempts
grep "health" /tmp/phoenix-gateway.log
```

**Expected**:
```
[INFO] Waiting for Qdrant to be ready...
[INFO] Health check attempt 1/30
[INFO] Health check attempt 2/30
...
[INFO] ✅ Qdrant health check passed
```

**Verify**:
- ✅ Health checks retry appropriately
- ✅ No premature failures
- ✅ Success within 30 seconds

### Phase 4: Gateway Startup

**Monitor gateway initialization**:
```bash
# Watch for gateway startup
grep "Gateway" /tmp/phoenix-gateway.log
```

**Expected**:
```
[INFO] Gateway starting...
[INFO] Initializing 8 Knowledge Bases...
[INFO] KB-01: Personal Context - initialized
[INFO] KB-02: Task History - initialized
...
[INFO] KB-08: Long-term Goals - initialized
[INFO] ✅ Gateway API operational
```

**Verify**:
- ✅ All 8 KBs initialized
- ✅ No database errors
- ✅ API server starts on port 8001

### Phase 5: Frontend Launch

**Monitor frontend startup**:
```bash
# Watch for frontend
grep "Frontend" /tmp/phoenix-gateway.log
```

**Expected**:
```
[INFO] Detected: Vite-based Studio UI
[INFO] Frontend starting...
[INFO] ✅ Frontend ready on port 5173
```

**Verify**:
- ✅ Frontend compiles successfully
- ✅ No asset loading errors
- ✅ Port is accessible

---

## 🎨 UI/UX Validation

### Test 1: Memory Engine Initialization UI

**Open browser to**: `http://localhost:3030`

**Expected UI State**:
```
┌─────────────────────────────────────┐
│  🧠 Memory Engine Initializing...  │
│                                     │
│  Phoenix is setting up her memory   │
│  system. This happens once and      │
│  takes about 30-60 seconds.         │
│                                     │
│  [Progress indicator or spinner]    │
└─────────────────────────────────────┘
```

**Verify**:
- ✅ No "Connection Refused" error
- ✅ Clear messaging about what's happening
- ✅ Progress indication (spinner, pulse, etc.)
- ✅ No technical jargon

### Test 2: API Key Prompt

**After Memory Engine initializes**:

**Expected UI State**:
```
┌─────────────────────────────────────┐
│  🔑 Welcome to Phoenix Marie        │
│                                     │
│  To begin, Phoenix needs your       │
│  OpenRouter API key.                │
│                                     │
│  Get one at: openrouter.ai/keys     │
│                                     │
│  [Input field for API key]          │
│  [Save button]                      │
│                                     │
│  Your key stays on YOUR machine.    │
│  Phoenix never shares it.           │
└─────────────────────────────────────┘
```

**Verify**:
- ✅ Clear call-to-action
- ✅ Link to get API key
- ✅ Privacy reassurance
- ✅ Input field is secure (password-style)

### Test 3: First Message Experience

**After entering API key**:

1. **Send a test message**: "Hello Phoenix"

**Expected**:
- ✅ Orange pulse appears (thinking indicator)
- ✅ Response arrives within 5-10 seconds
- ✅ Response is contextually appropriate
- ✅ No error messages

2. **Check Knowledge Base**:
```bash
curl http://localhost:8001/api/v1/kb/01/records
```

**Verify**:
- ✅ KB-01 contains user introduction
- ✅ Data is properly formatted
- ✅ Timestamps are correct

---

## 📊 Performance Metrics

### Timing Benchmarks

**Measure and record**:

```bash
# Total time from launch to ready
START_TIME=$(date +%s)
./phoenix-rise.sh
# (Wait for "System Ready" message)
END_TIME=$(date +%s)
TOTAL_TIME=$((END_TIME - START_TIME))
echo "Total startup time: ${TOTAL_TIME}s"
```

**Expected Timings**:
- **First Run** (with Qdrant download): 60-120 seconds
- **Subsequent Runs** (Qdrant cached): 10-20 seconds
- **Already Running** (Qdrant detected): 5-10 seconds

**Breakdown**:
```
Phase 1: Port Cleanup           1-2s
Phase 2: Qdrant Download        30-60s (first run only)
Phase 3: Qdrant Startup         5-10s
Phase 4: Gateway Initialization 5-10s
Phase 5: Frontend Compilation   10-20s
Phase 6: Health Verification    5-10s
```

### Resource Usage

**Monitor system resources**:
```bash
# CPU and Memory usage
top -b -n 1 | grep -E "pagi-gateway|qdrant"

# Disk usage
du -sh ./bin ./data
```

**Expected**:
- **Qdrant Binary**: ~110 MB
- **Qdrant Data**: ~10 MB (fresh install)
- **Gateway Memory**: 50-100 MB
- **Qdrant Memory**: 50-100 MB (idle)
- **Total Memory**: 100-200 MB

---

## 🛡️ Error Handling Tests

### Test 1: Network Failure During Download

**Simulate**:
```bash
# Block GitHub temporarily
sudo iptables -A OUTPUT -d github.com -j DROP

# Start Phoenix
./phoenix-rise.sh

# Restore network
sudo iptables -D OUTPUT -d github.com -j DROP
```

**Expected**:
```
⚠️  Memory Engine initialization failed: Failed to download Qdrant
   You can manually start Qdrant on port 6333 if needed.
   Vector search features will be unavailable until then.

✅ Gateway starting without vector features...
```

**Verify**:
- ✅ Phoenix continues startup
- ✅ Clear error message
- ✅ Guidance provided
- ✅ Core features still work

### Test 2: Port 6333 Already in Use

**Simulate**:
```bash
# Start a dummy process on port 6333
nc -l 6333 &

# Start Phoenix
./phoenix-rise.sh
```

**Expected**:
```
⚠️  Memory Engine initialization failed: Port 6333 already in use
   Please stop the process using port 6333 and restart Phoenix.

✅ Gateway starting without vector features...
```

**Verify**:
- ✅ Detects port conflict
- ✅ Clear error message
- ✅ Actionable guidance
- ✅ Graceful degradation

### Test 3: Corrupted Qdrant Binary

**Simulate**:
```bash
# Create a corrupted binary
mkdir -p ./bin
echo "corrupted" > ./bin/qdrant
chmod +x ./bin/qdrant

# Start Phoenix
./phoenix-rise.sh
```

**Expected**:
```
⚠️  Memory Engine initialization failed: Qdrant failed to start
   Attempting to re-download...

📥 Downloading Qdrant v1.7.4...
✅ Qdrant binary downloaded
🚀 Starting Qdrant...
✅ Memory Engine initialized successfully
```

**Verify**:
- ✅ Detects corrupted binary
- ✅ Automatically re-downloads
- ✅ Recovers without user intervention

---

## 📝 Success Report Template

After completing all tests, generate this report:

```markdown
# 🔥 Phoenix First-Run Stress Test Report

**Test Date**: 2026-02-10
**Tester**: [Your Name]
**Platform**: [Windows/Linux/macOS]
**Phoenix Version**: 0.1.0-beta.1

## ✅ Test Results

### Timing Metrics
- **Total Startup Time**: XXs
- **Qdrant Download Time**: XXs
- **Qdrant Startup Time**: XXs
- **Gateway Initialization**: XXs
- **Frontend Ready**: XXs

### Functional Tests
- [✅/❌] Clean slate boot
- [✅/❌] Qdrant auto-download
- [✅/❌] Health check polling
- [✅/❌] Gateway initialization
- [✅/❌] Frontend launch
- [✅/❌] API key prompt
- [✅/❌] First message success

### UI/UX Tests
- [✅/❌] Memory Engine initialization UI
- [✅/❌] API key prompt clarity
- [✅/❌] Orange pulse indicator
- [✅/❌] Error messages helpful

### Error Handling Tests
- [✅/❌] Network failure recovery
- [✅/❌] Port conflict detection
- [✅/❌] Corrupted binary recovery

### Resource Usage
- **Peak Memory**: XXX MB
- **Disk Usage**: XXX MB
- **CPU Usage**: XX%

## 🎯 Overall Assessment

[PASS/FAIL]

## 📋 Issues Found

1. [Issue description]
   - Severity: [Critical/High/Medium/Low]
   - Steps to reproduce: [...]
   - Expected: [...]
   - Actual: [...]

## 💡 Recommendations

1. [Recommendation 1]
2. [Recommendation 2]

## 🚀 Ready for Beta?

[YES/NO] - [Explanation]

---

**Signature**: [Your Name]
**Date**: [Date]
```

---

## 🔄 Cleanup After Testing

### Restore Original State
```bash
# Stop Phoenix
pkill -f pagi-gateway
pkill -f qdrant

# Remove test artifacts
rm -rf ./bin ./data .env user_config.toml

# Restore backups
mv ./bin.backup ./bin
mv ./data.backup ./data
mv .env.backup .env
mv user_config.toml.backup user_config.toml
```

---

## 🎓 Testing Best Practices

### 1. Test on Multiple Platforms
- ✅ Windows 10/11
- ✅ Ubuntu 20.04/22.04
- ✅ macOS 11+ (Intel)
- ✅ macOS 11+ (ARM)

### 2. Test Different Network Conditions
- ✅ Fast connection (100+ Mbps)
- ✅ Slow connection (1-5 Mbps)
- ✅ Intermittent connection
- ✅ Behind corporate firewall

### 3. Test Different System States
- ✅ Fresh OS install
- ✅ System with other services running
- ✅ Low disk space (< 1 GB)
- ✅ Low memory (< 2 GB available)

### 4. Test Edge Cases
- ✅ Invalid API key
- ✅ Expired API key
- ✅ Rate-limited API key
- ✅ Firewall blocking ports

---

## 🏆 Success Criteria

Phoenix passes the First-Run Stress Test if:

1. **Zero Manual Intervention**: User never needs to download or configure Qdrant
2. **Clear Feedback**: UI provides clear status at every stage
3. **Graceful Errors**: All errors are handled with helpful messages
4. **Fast Startup**: First run < 120s, subsequent runs < 20s
5. **Resource Efficient**: < 200 MB memory, < 500 MB disk
6. **Cross-Platform**: Works identically on all 4 platforms
7. **Recovery**: Automatically recovers from common errors

---

## 🔥 The Zero-Touch Promise

If Phoenix passes this test, you can confidently tell beta users:

> **"Download. Extract. Run. That's it."**

No manual Qdrant setup. No OpenSSL installation. No configuration files to edit (except API key). Just pure, sovereign intelligence.

**Your data. Your hardware. Your intelligence. Zero hassle.**

---

**Test Version**: 1.0  
**Last Updated**: 2026-02-10  
**Status**: Ready for Execution  
**Target**: Phoenix v0.1.0-beta.1

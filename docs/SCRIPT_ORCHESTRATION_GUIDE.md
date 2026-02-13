# 🚀 PAGI Script Orchestration Guide

## Overview

This guide documents the standardized script execution sequence for the PAGI Sovereign ecosystem. Following this guide ensures stable local development and prevents common PowerShell errors.

---

## 📋 Script Manifest

| Script Name | Scope | Sovereign Purpose | Key Actions | Location |
|-------------|-------|-------------------|-------------|----------|
| **`start-sovereign.ps1`** | **Master** | **Unified Orchestration** | Complete environment setup, validation, build, and launch | Root |
| `pagi-up.ps1` | Runtime | Quick Launch | Fast startup (assumes environment is ready) | Root |
| `phoenix-rise.ps1` | Runtime | Phoenix Activation | Alternative startup script | Root |
| `forge-kill-switch.ps1` | Utility | Emergency Stop | Kills all PAGI processes and cleans ports | Root |
| `start-qdrant.ps1` | Infrastructure | Vector DB | Starts Qdrant sidecar for semantic search | `scripts/` |
| `audit_stress_test.ps1` | Testing | System Audit | Runs stress tests and security audits | `scripts/` |
| `deploy_beta.ps1` | Release | Beta Deployment | Builds and packages for distribution | `scripts/` |
| `sanitize_for_release.ps1` | Security | Data Sanitization | Removes personal data before release | `scripts/` |

---

## 🎯 Master Orchestrator: `start-sovereign.ps1`

### Purpose

The **Master Orchestrator** is your single entry point for the PAGI ecosystem. It handles all setup, validation, and launch steps automatically with built-in error recovery.

### Features

✅ **Automatic Execution Policy Fix** - Detects and resolves PowerShell script restrictions  
✅ **Environment Validation** - Checks for Rust, Node.js, and required tools  
✅ **Knowledge Base Provisioning** - Creates all 8 KB directories automatically  
✅ **Port Cleanup** - Clears zombie processes from previous runs  
✅ **Sequential Build** - Compiles Rust workspace with error handling  
✅ **Frontend Dependencies** - Installs npm packages if needed  
✅ **Coordinated Launch** - Starts Gateway, Control Panel, and Studio UI in order  

### Usage

#### Basic Launch (Recommended)
```powershell
.\start-sovereign.ps1
```

#### Verification Only (No Launch)
```powershell
.\start-sovereign.ps1 -VerifyOnly
```
Runs all checks and setup but doesn't start the services. Useful for CI/CD or troubleshooting.

#### Skip Build (Fast Restart)
```powershell
.\start-sovereign.ps1 -SkipBuild
```
Skips the Rust compilation step. Use when you haven't changed any Rust code.

#### Clean Build (Nuclear Option)
```powershell
.\start-sovereign.ps1 -CleanStart
```
Runs `cargo clean` before building. Use when you have build cache issues.

### Execution Sequence

The Master Orchestrator follows this 7-step sequence:

```
[0/7] Execution Policy Check
      ↓ Validates PowerShell can run scripts
      ↓ Auto-fixes if needed (sets RemoteSigned)
      
[1/7] Prerequisites Validation
      ↓ Checks for Rust (cargo)
      ↓ Checks for Node.js and npm
      ↓ Verifies running from repo root
      
[2/7] Environment Configuration
      ↓ Checks for .env file
      ↓ Creates from .env.example if missing
      ↓ Validates critical variables
      
[3/7] Knowledge Base Provisioning
      ↓ Creates storage/ directory
      ↓ Creates 8 KB subdirectories:
      ↓   • kb-01-psyche (User Profile)
      ↓   • kb-02-oikos (Social Graph)
      ↓   • kb-03-techne (Technical Knowledge)
      ↓   • kb-04-chronos (Temporal Memory)
      ↓   • kb-05-polis (Social Defense)
      ↓   • kb-06-ethos (Sovereign Config)
      ↓   • kb-07-mimir (Semantic Cache)
      ↓   • kb-08-soma (System Health)
      
[4/7] Port Cleanup
      ↓ Scans ports: 8000, 8002, 3001, 3003
      ↓ Kills zombie processes
      ↓ Ensures clean bind for new services
      
[5/7] Workspace Build
      ↓ Runs: cargo build --workspace
      ↓ Compiles all Rust crates
      ↓ Validates build success
      
[6/7] Frontend Dependencies
      ↓ Checks for node_modules in studio-interface
      ↓ Runs npm install if needed
      
[7/7] Ecosystem Launch
      ↓ Starts pagi-gateway (Port 8001) in new window
      ↓ Starts pagi-control-panel (Port 8002) in new window
      ↓ Starts pagi-studio-ui (Port 3001) in foreground
```

---

## 🔧 Solving Common PowerShell Errors

### Error 1: Execution Policy Restriction

**Symptom:**
```
...cannot be loaded because running scripts is disabled on this system.
```

**Cause:** PowerShell blocks unsigned scripts by default for security.

**Solution:** The Master Orchestrator fixes this automatically. If you need to fix manually:

```powershell
# Run as Administrator (or CurrentUser scope)
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

**Verification:**
```powershell
Get-ExecutionPolicy -Scope CurrentUser
# Should return: RemoteSigned
```

---

### Error 2: Path Not Found

**Symptom:**
```
Could not find part of the path 'C:\Users\...\storage\kb-01-psyche'
```

**Cause:** Knowledge Base directories don't exist yet.

**Solution:** The Master Orchestrator creates these automatically in Step 3. If you need to create manually:

```powershell
# Run from repository root
New-Item -ItemType Directory -Path "storage\kb-01-psyche" -Force
New-Item -ItemType Directory -Path "storage\kb-02-oikos" -Force
# ... (repeat for all 8 KBs)
```

---

### Error 3: Port Already in Use

**Symptom:**
```
Address already in use (os error 10048)
```

**Cause:** Previous PAGI process didn't shut down cleanly.

**Solution:** The Master Orchestrator cleans ports automatically in Step 4. For manual cleanup:

```powershell
# Use the kill switch
.\forge-kill-switch.ps1

# Or manually find and kill processes
Get-NetTCPConnection -LocalPort 8001 | Select-Object -ExpandProperty OwningProcess | ForEach-Object { Stop-Process -Id $_ -Force }
```

---

### Error 4: Rust Not Found

**Symptom:**
```
'cargo' is not recognized as an internal or external command
```

**Cause:** Rust toolchain not installed or not in PATH.

**Solution:**

1. Install Rust from [rustup.rs](https://rustup.rs/)
2. Restart PowerShell to refresh PATH
3. Verify: `cargo --version`

---

### Error 5: Node.js Not Found

**Symptom:**
```
'node' is not recognized as an internal or external command
```

**Cause:** Node.js not installed or not in PATH.

**Solution:**

1. Install Node.js from [nodejs.org](https://nodejs.org/) (LTS version recommended)
2. Restart PowerShell to refresh PATH
3. Verify: `node --version` and `npm --version`

---

## 📊 Port Architecture

PAGI follows a standardized port allocation:

| Port Range | Purpose | Examples |
|------------|---------|----------|
| **8001-8099** | Backend/API Services | 8001 (Gateway), 8002 (Control Panel) |
| **3001-3099** | Frontend/UI Services | 3001 (Studio UI), 3003 (Companion UI) |
| **6333** | Vector Database | Qdrant (optional) |
| **6379** | Cache | Redis (optional, future) |

---

## 🔄 Recommended Workflows

### Daily Development Workflow

```powershell
# Morning: Full startup with validation
.\start-sovereign.ps1

# During day: Quick restart after code changes
.\start-sovereign.ps1 -SkipBuild  # If only frontend changed
# OR
.\pagi-up.ps1  # Fast restart (assumes environment is ready)

# End of day: Clean shutdown
.\forge-kill-switch.ps1
```

### First-Time Setup

```powershell
# 1. Clone repository
git clone <repo-url>
cd pagi-uac-main

# 2. Run Master Orchestrator (handles everything)
.\start-sovereign.ps1

# 3. Edit .env with your API keys
notepad .env

# 4. Restart with live mode
.\start-sovereign.ps1
```

### Troubleshooting Workflow

```powershell
# 1. Verify environment without launching
.\start-sovereign.ps1 -VerifyOnly

# 2. If build issues, do clean build
.\start-sovereign.ps1 -CleanStart

# 3. If still failing, check logs
Get-Content .\vite-dev-log.txt
```

### Release Preparation

```powershell
# 1. Sanitize personal data
.\scripts\sanitize_for_release.ps1

# 2. Run stress tests
.\scripts\audit_stress_test.ps1

# 3. Build beta package
.\scripts\deploy_beta.ps1

# 4. Tag release
.\scripts\trigger-release.ps1 v0.1.0-beta.1
```

---

## 🎓 Script Execution Best Practices

### 1. Always Run from Repository Root

```powershell
# ✅ CORRECT
cd C:\Users\YourName\pagi-uac-main
.\start-sovereign.ps1

# ❌ WRONG
cd C:\Users\YourName\pagi-uac-main\scripts
..\start-sovereign.ps1  # Relative paths will break
```

### 2. Use PowerShell (Not CMD)

```powershell
# ✅ CORRECT: PowerShell
powershell.exe
.\start-sovereign.ps1

# ❌ WRONG: Command Prompt
cmd.exe
start-sovereign.ps1  # Won't work in CMD
```

### 3. Check Exit Codes

```powershell
.\start-sovereign.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Orchestrator failed. Check errors above."
}
```

### 4. Use Flags for Specific Scenarios

```powershell
# CI/CD pipeline
.\start-sovereign.ps1 -VerifyOnly

# Quick iteration
.\start-sovereign.ps1 -SkipBuild

# Nuclear option
.\start-sovereign.ps1 -CleanStart
```

---

## 🔐 Security Considerations

### Execution Policy

The Master Orchestrator sets `RemoteSigned` policy, which:
- ✅ Allows local scripts to run
- ✅ Requires remote scripts to be signed
- ✅ Balances security and usability

For maximum security in production:
```powershell
Set-ExecutionPolicy -ExecutionPolicy AllSigned -Scope CurrentUser
```

### Environment Variables

Never commit `.env` to version control:
```gitignore
# Already in .gitignore
.env
```

Sensitive variables:
- `PAGI_LLM_API_KEY` - OpenRouter API key
- `OPENROUTER_API_KEY` - Alternative key name
- `MS_GRAPH_CLIENT_SECRET` - Microsoft Graph secret

---

## 📚 Additional Resources

### Official Documentation
- [Rust Installation](https://rustup.rs/)
- [Node.js Downloads](https://nodejs.org/)
- [PowerShell Execution Policies](https://docs.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies)

### PAGI-Specific Docs
- [`README.md`](./README.md) - Project overview
- [`.env.example`](./.env.example) - Environment variable reference
- [`scripts/README.md`](./scripts/README.md) - Script directory index

### Video Tutorial
[Mastering PowerShell Scripts](https://www.youtube.com/watch?v=w2Co88TzrsQ) - Quick overview of script management

---

## 🆘 Getting Help

If you encounter issues not covered in this guide:

1. **Check the logs:**
   ```powershell
   Get-Content .\vite-dev-log.txt -Tail 50
   ```

2. **Run verification:**
   ```powershell
   .\start-sovereign.ps1 -VerifyOnly
   ```

3. **Check system status:**
   ```powershell
   cargo run -p pagi-gateway -- --verify
   ```

4. **Clean slate:**
   ```powershell
   .\forge-kill-switch.ps1
   .\start-sovereign.ps1 -CleanStart
   ```

---

## 📝 Changelog

### v1.0.0 - Master Orchestrator Release
- ✨ Created unified `start-sovereign.ps1` script
- ✨ Automatic execution policy handling
- ✨ Knowledge Base auto-provisioning
- ✨ Port cleanup automation
- ✨ Sequential validation and error handling
- 📚 Comprehensive documentation

---

**Sovereign Status:** ✅ Operational  
**Last Updated:** 2026-02-12  
**Maintained By:** Phoenix Marie Orchestrator

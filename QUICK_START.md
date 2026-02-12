# ⚡ PAGI Quick Start Guide

## 🚀 First Time Setup (5 Minutes)

```powershell
# 1. Run the Master Orchestrator
.\start-sovereign.ps1

# 2. Edit your API keys (opens in notepad)
notepad .env

# 3. Set to live mode in .env
#    Change: PAGI_LLM_MODE=mock
#    To:     PAGI_LLM_MODE=live

# 4. Restart
.\start-sovereign.ps1
```

**That's it!** The orchestrator handles everything else automatically.

---

## 🎯 Daily Commands

### Start Everything
```powershell
.\start-sovereign.ps1
```

### Quick Restart (No Build)
```powershell
.\start-sovereign.ps1 -SkipBuild
```

### Stop Everything
```powershell
.\forge-kill-switch.ps1
```

### Check Status (No Launch)
```powershell
.\start-sovereign.ps1 -VerifyOnly
```

---

## 🔧 Troubleshooting One-Liners

### "Scripts Disabled" Error
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Port Already in Use
```powershell
.\forge-kill-switch.ps1
```

### Build Issues
```powershell
.\start-sovereign.ps1 -CleanStart
```

### Missing Directories
```powershell
# The orchestrator creates these automatically, but if needed:
New-Item -ItemType Directory -Path "storage" -Force
```

---

## 📍 Access Points

Once running, access these URLs:

- **Studio UI:** http://localhost:3001
- **Gateway API:** http://localhost:8000
- **Control Panel:** http://localhost:8002

---

## 🆘 Common Issues

| Problem | Solution |
|---------|----------|
| Red text about execution policy | Run: `Set-ExecutionPolicy RemoteSigned -Scope CurrentUser` |
| Port 8000 already in use | Run: `.\forge-kill-switch.ps1` |
| Rust not found | Install from: https://rustup.rs/ |
| Node not found | Install from: https://nodejs.org/ |
| Build fails | Run: `.\start-sovereign.ps1 -CleanStart` |

---

## 📚 Full Documentation

For detailed information, see:
- [`SCRIPT_ORCHESTRATION_GUIDE.md`](./SCRIPT_ORCHESTRATION_GUIDE.md) - Complete script reference
- [`README.md`](./README.md) - Project overview
- [`.env.example`](./.env.example) - All environment variables

---

## 🎓 Pro Tips

1. **Always run from repo root** - Scripts use relative paths
2. **Use PowerShell, not CMD** - Scripts are PowerShell-specific
3. **Check .env first** - Most issues are missing API keys
4. **Use -VerifyOnly** - Test setup without launching
5. **Use -SkipBuild** - Fast iteration during development

---

**Need Help?** Run `.\start-sovereign.ps1 -VerifyOnly` to diagnose issues.

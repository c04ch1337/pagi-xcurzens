# 🔥 Phoenix Beta Tester Onboarding Guide

## Welcome to the Phoenix Evolution

You've been selected to test **Phoenix Marie** - a sovereign, bare-metal AGI system that runs entirely on YOUR hardware, with YOUR data, under YOUR control.

---

## 🎯 What Makes Phoenix Different

### Privacy-First Architecture
- **100% Local**: All your data stays on your machine
- **Your Keys**: You provide your own OpenRouter API key
- **No Telemetry**: Zero tracking, zero analytics, zero data collection
- **Bare Metal**: Optimized for your specific hardware (Windows, Linux, macOS Intel/ARM)

### Living Intelligence
- **8 Knowledge Bases**: KB-01 through KB-08 evolve with your usage
- **Auto-Updates**: Phoenix checks for "cognitive upgrades" automatically
- **Sovereign Control**: You decide when to update, what to share, how to configure

---

## 📋 Prerequisites

### Required
1. **OpenRouter API Key** - Get one at [https://openrouter.ai/keys](https://openrouter.ai/keys)
   - Free tier available for testing
   - Pay-as-you-go pricing (you control costs)
   - Supports Claude, GPT-4, and 100+ other models

2. **System Requirements**
   - **Windows**: Windows 10/11 (x64)
   - **Linux**: Ubuntu 20.04+ or equivalent (x64)
   - **macOS**: 11.0+ (Intel or Apple Silicon)
   - **RAM**: 4GB minimum, 8GB recommended
   - **Disk**: 2GB free space for installation + growth space for knowledge bases

### Optional
- **Qdrant Vector Database** (for advanced vector search)
  - Download from [https://qdrant.tech/](https://qdrant.tech/)
  - Phoenix will guide you through setup if needed

---

## 🚀 Installation

### Step 1: Download Phoenix

1. Go to [GitHub Releases](https://github.com/YOUR-USERNAME/pagi-uac-main/releases)
2. Download the appropriate archive for your platform:
   - **Windows**: `phoenix-windows-x86_64.zip`
   - **Linux**: `phoenix-linux-x86_64.tar.gz`
   - **macOS Intel**: `phoenix-macos-x86_64.tar.gz`
   - **macOS ARM**: `phoenix-macos-aarch64.tar.gz`

### Step 2: Extract & Verify

#### Windows
```powershell
# Extract the archive
Expand-Archive -Path phoenix-windows-x86_64.zip -DestinationPath C:\Phoenix

# Navigate to directory
cd C:\Phoenix\phoenix-0.1.0-beta.1

# Verify checksum (optional but recommended)
$hash = Get-FileHash pagi-gateway.exe -Algorithm SHA256
Write-Host $hash.Hash
```

#### Linux/macOS
```bash
# Extract the archive
tar -xzf phoenix-linux-x86_64.tar.gz
cd phoenix-0.1.0-beta.1

# Verify checksum (optional but recommended)
shasum -a 256 pagi-gateway
```

### Step 3: Configure Environment

```bash
# Copy the example environment file
cp .env.example .env

# Edit .env with your preferred text editor
# Add your OpenRouter API key:
# OPENROUTER_API_KEY=sk-or-v1-your-key-here
```

---

## 🔥 First Launch

### Windows
```powershell
.\phoenix-rise.ps1
```

### Linux/macOS
```bash
chmod +x phoenix-rise.sh
./phoenix-rise.sh
```

### What Happens on First Run

1. **Version Check**: Phoenix checks for updates
2. **Database Initialization**: Creates empty KB-01 through KB-08
3. **API Key Prompt**: If not in `.env`, Phoenix will ask for your OpenRouter key
4. **UI Launch**: Browser opens to `http://localhost:3030`
5. **Welcome Screen**: Onboarding overlay guides you through setup

---

## 🎨 The Phoenix Interface

### Main Dashboard
- **Chat Interface**: Interact with Phoenix Marie
- **System Health**: Monitor resource usage and KB status
- **Wellness Tab**: Track your personal metrics (optional)
- **Settings**: Configure models, API endpoints, preferences

### Knowledge Bases (KB-01 to KB-08)
Each KB serves a specific purpose:
- **KB-01**: Personal context and preferences
- **KB-02**: Task history and patterns
- **KB-03**: Domain knowledge (technical, professional)
- **KB-04**: Social intelligence and relationships
- **KB-05**: Creative projects and ideas
- **KB-06**: Health and wellness data
- **KB-07**: Financial and resource tracking
- **KB-08**: Long-term goals and reflections

As you use Phoenix, these KBs fill with YOUR data, creating a personalized intelligence layer.

---

## 🔐 Privacy & Security

### What Phoenix Knows
- **Local Only**: All KB data stored in `storage/` directory on your machine
- **Encrypted Config**: API keys stored in `user_config.toml` (local file)
- **No Cloud Sync**: Unless you explicitly enable it (future feature)

### What Leaves Your Machine
- **LLM API Calls**: Only the messages you send to the AI
- **Update Checks**: Phoenix checks GitHub for new versions (no personal data sent)
- **Nothing Else**: No telemetry, no analytics, no tracking

### Your Control
- **Delete Anytime**: Remove `storage/` to wipe all knowledge bases
- **Export Data**: Backup your KBs to external storage
- **Revoke Access**: Change or remove API key in settings

---

## 🛠️ Troubleshooting

### Phoenix Won't Start

#### Check Port Availability
```bash
# Windows
netstat -ano | findstr :3030

# Linux/macOS
lsof -i :3030
```

If port 3030 is in use, edit `config/gateway.toml`:
```toml
[server]
port = 3031  # Change to available port
```

#### Check Logs
```bash
# Logs are in the installation directory
cat pagi-gateway.log  # Linux/macOS
type pagi-gateway.log  # Windows
```

### API Key Issues

#### "Invalid API Key" Error
1. Verify key at [https://openrouter.ai/keys](https://openrouter.ai/keys)
2. Check `.env` file format:
   ```
   OPENROUTER_API_KEY=sk-or-v1-your-actual-key-here
   ```
3. Restart Phoenix after changing `.env`

#### "No API Key Configured"
- Phoenix will prompt you in the UI
- Or manually edit `user_config.toml`:
  ```toml
  api_key = "sk-or-v1-your-key-here"
  ```

### Database Issues

#### "Failed to Initialize KB"
```bash
# Delete corrupted databases and restart
rm -rf storage/  # Linux/macOS
rmdir /s storage  # Windows

# Phoenix will recreate on next start
```

### Update Issues

#### "Update Check Failed"
- Check internet connection
- Verify GitHub is accessible
- Check firewall settings

---

## 📊 Monitoring Your Phoenix

### System Health Dashboard
- **CPU/Memory**: Real-time resource usage
- **KB Status**: Size and health of each knowledge base
- **API Usage**: Track OpenRouter API calls and costs
- **Uptime**: How long Phoenix has been running

### Logs & Diagnostics
```bash
# View real-time logs
tail -f pagi-gateway.log  # Linux/macOS
Get-Content pagi-gateway.log -Wait  # Windows

# Check system status
curl http://localhost:3030/api/v1/health
```

---

## 🔄 Updates & Evolution

### Automatic Update Checks
Phoenix checks for updates on startup. You'll see:
```
🔥 A new Phoenix Evolution is available!
Current: v0.1.0-beta.1
Latest: v0.1.0-beta.2

Update now? (y/n)
```

### Manual Update Check
```bash
# Check for updates without restarting
curl http://localhost:3030/api/v1/system/version
```

### Updating Phoenix
1. Download new release from GitHub
2. Stop Phoenix (Ctrl+C in terminal)
3. Extract new version over old installation
4. Restart Phoenix
5. **Your data is preserved** - KBs remain intact

---

## 🧪 Beta Testing Guidelines

### What We Need From You

#### Bug Reports
- **Where**: GitHub Issues
- **Include**:
  - Phoenix version (`cat VERSION`)
  - Operating system and version
  - Steps to reproduce
  - Logs (sanitize any personal data!)
  - Screenshots if applicable

#### Feature Requests
- **Where**: GitHub Discussions
- **Format**:
  - Use case / problem you're solving
  - Proposed solution
  - Why it matters to you

#### General Feedback
- **Where**: GitHub Discussions or Discord (if invited)
- **Topics**:
  - User experience
  - Performance
  - Documentation clarity
  - Feature priorities

### What NOT to Share
- ❌ Your API keys
- ❌ Personal data from your KBs
- ❌ Sensitive logs (sanitize first)

---

## 🎓 Advanced Usage

### Custom Models
Edit `user_config.toml`:
```toml
llm_model = "anthropic/claude-opus-4.6"  # or any OpenRouter model
llm_api_url = "https://openrouter.ai/api/v1/chat/completions"
```

### Multiple Profiles
```bash
# Create separate Phoenix installations for different contexts
cp -r phoenix-0.1.0-beta.1 phoenix-work
cp -r phoenix-0.1.0-beta.1 phoenix-personal

# Each maintains separate KBs and config
```

### Backup & Restore
```bash
# Backup your knowledge bases
tar -czf phoenix-backup-$(date +%Y%m%d).tar.gz storage/ user_config.toml

# Restore
tar -xzf phoenix-backup-20260210.tar.gz
```

---

## 📞 Support & Community

### Getting Help
1. **Documentation**: Check this guide and `README.md`
2. **GitHub Issues**: Search existing issues first
3. **GitHub Discussions**: Ask questions, share tips
4. **Discord**: Beta tester channel (invite-only)

### Contributing
- **Bug Fixes**: Pull requests welcome
- **Documentation**: Help improve guides
- **Testing**: Try edge cases and report findings

---

## 🏆 Beta Tester Perks

### Early Access
- First to see new features
- Influence product direction
- Direct line to development team

### Recognition
- Listed in CONTRIBUTORS.md (if you want)
- Beta tester badge (future feature)
- Priority support

### Learning
- Deep dive into AGI architecture
- Rust + AI development insights
- Sovereign computing principles

---

## 📝 Quick Reference

### Essential Commands
```bash
# Start Phoenix
./phoenix-rise.sh  # or .ps1 on Windows

# Check version
cat VERSION

# View logs
tail -f pagi-gateway.log

# Check health
curl http://localhost:3030/api/v1/health

# Stop Phoenix
# Ctrl+C in terminal, or:
curl -X POST http://localhost:3030/api/v1/system/shutdown
```

### Essential Files
```
phoenix-0.1.0-beta.1/
├── pagi-gateway(.exe)      # Main binary
├── VERSION                 # Current version
├── .env                    # Your API key (create from .env.example)
├── user_config.toml        # Your preferences (auto-created)
├── storage/                # Your knowledge bases (auto-created)
├── config/                 # System configuration
└── BETA_README.md          # This guide
```

### Essential URLs
- **Dashboard**: http://localhost:3030
- **Health Check**: http://localhost:3030/api/v1/health
- **API Docs**: http://localhost:3030/api/v1/docs (future)

---

## 🎯 Next Steps

1. ✅ Complete installation
2. ✅ Configure API key
3. ✅ Launch Phoenix and complete onboarding
4. ✅ Send your first message
5. ✅ Explore the knowledge bases
6. ✅ Join the beta tester community
7. ✅ Report your first bug or feedback

---

## 🧪 First Mission (Required): Operation First Rise

This is the official end-to-end stress test for the **Architect’s View** (diagram-first Concise Mode) + **raw JSON envelope** capture + **sidecar health** verification.

- Mission brief: [`FIRST_MISSION_OPERATION_FIRST_RISE.md`](FIRST_MISSION_OPERATION_FIRST_RISE.md)

---

## 🔥 Welcome to the Phoenix Evolution

You're not just testing software - you're helping build the future of sovereign, privacy-first AGI.

**Your data. Your hardware. Your intelligence.**

Let's rise together. 🚀

---

**Version**: 0.1.0-beta.1  
**Last Updated**: 2026-02-10  
**Support**: GitHub Issues & Discussions

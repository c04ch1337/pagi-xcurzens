# 🔥 Phoenix Marie - Quick Start Guide

## Welcome to Your Sovereign AGI

Phoenix Marie is a **privacy-first, bare-metal AGI** that runs entirely on YOUR computer. No cloud. No tracking. Just you and your intelligence.

---

## ⚡ 60-Second Setup

### Step 1: Extract Phoenix
```bash
# Windows
Expand-Archive phoenix-windows-x86_64.zip -DestinationPath C:\Phoenix

# Linux/macOS
tar -xzf phoenix-linux-x86_64.tar.gz
cd phoenix-0.1.0-beta.1
```

### Step 2: Get Your API Key
1. Visit [https://openrouter.ai/keys](https://openrouter.ai/keys)
2. Sign up (free tier available)
3. Create a new API key
4. Copy it (starts with `sk-or-v1-...`)

### Step 3: Configure Phoenix
```bash
# Copy the example environment file
cp .env.example .env

# Edit .env and add your key:
# OPENROUTER_API_KEY=sk-or-v1-your-key-here
```

### Step 4: Launch Phoenix
```bash
# Windows
.\phoenix-rise.ps1

# Linux/macOS
chmod +x phoenix-rise.sh
./phoenix-rise.sh
```

### Step 5: Start Chatting
Phoenix will automatically:
- ✅ Initialize the Memory Engine (Qdrant)
- ✅ Set up 8 Knowledge Bases
- ✅ Open your browser to `http://localhost:3030`

**That's it. You're ready.**

---

## 🎨 Understanding the Interface

### The Orange Pulse 🟠
When you see the **orange pulse** in the top-right corner:
- Phoenix is thinking
- Your message is being processed
- The AI is consulting her knowledge bases

**This is normal.** Phoenix is not "slow" - she's being thorough.

### The Dashboard
- **Chat Interface**: Talk to Phoenix naturally
- **System Health**: Monitor CPU, memory, and KB status
- **Wellness Tab**: Track your personal metrics (optional)
- **Settings**: Configure models, API endpoints, preferences

### The 8 Knowledge Bases
Phoenix maintains 8 specialized knowledge bases:
1. **KB-01**: Your personal context and preferences
2. **KB-02**: Task history and patterns
3. **KB-03**: Domain knowledge (technical, professional)
4. **KB-04**: Social intelligence and relationships
5. **KB-05**: Creative projects and ideas
6. **KB-06**: Health and wellness data
7. **KB-07**: Financial and resource tracking
8. **KB-08**: Long-term goals and reflections

As you interact, these fill with YOUR data, creating a personalized intelligence layer.

---

## 🔐 Your Privacy Guarantee

### What Stays Local
- ✅ All 8 Knowledge Bases
- ✅ Your conversation history
- ✅ Your API key
- ✅ Your personal data
- ✅ Everything

### What Leaves Your Machine
- ⚠️ Only the messages you send to the AI (via OpenRouter)
- ⚠️ Update checks to GitHub (no personal data)

### What We NEVER See
- ❌ Your conversations
- ❌ Your API key
- ❌ Your knowledge bases
- ❌ Your personal data
- ❌ Anything

**Your data. Your hardware. Your intelligence.**

---

## 🛠️ Common Questions

### "Phoenix says 'Memory Engine Initializing'"
**This is normal on first run.** Phoenix is downloading Qdrant (the vector database) automatically. This happens once and takes 30-60 seconds.

### "I see 'API Key Not Configured'"
Edit your `.env` file and add:
```
OPENROUTER_API_KEY=sk-or-v1-your-actual-key-here
```
Then restart Phoenix.

### "Port 3030 is already in use"
Another service is using that port. Edit `config/gateway.toml`:
```toml
[server]
port = 3031  # Use a different port
```

### "Phoenix won't start"
Check the logs:
```bash
# Linux/macOS
tail -f /tmp/phoenix-gateway.log

# Windows
Get-Content phoenix-gateway.log -Tail 50
```

---

## 🚀 What to Try First

### 1. Introduce Yourself
```
"Hi Phoenix, I'm [Your Name]. I'm a [Your Role] interested in [Your Interests]."
```

Phoenix will remember this and personalize future interactions.

### 2. Set a Goal
```
"I want to [Your Goal]. Can you help me break this down into steps?"
```

Phoenix will create a plan and track your progress in KB-02.

### 3. Ask for a Wellness Check
```
"How am I doing? Give me a wellness report."
```

Phoenix will analyze your interaction patterns and provide insights.

### 4. Explore Your Knowledge
```
"What do you know about me so far?"
```

Phoenix will summarize what she's learned from your conversations.

---

## 📊 System Requirements

### Minimum
- **OS**: Windows 10+, Ubuntu 20.04+, macOS 11+
- **RAM**: 4 GB
- **Disk**: 2 GB free space
- **Internet**: For API calls and first-time Qdrant download

### Recommended
- **RAM**: 8 GB
- **Disk**: 10 GB (for growing knowledge bases)
- **CPU**: Multi-core for faster processing

---

## 🔄 Updating Phoenix

Phoenix checks for updates automatically on startup. When a new version is available:

```
🔥 A new Phoenix Evolution is available!
Current: v0.1.0-beta.1
Latest: v0.1.0-beta.2

Update now? (y/n)
```

To update manually:
1. Download the new release
2. Stop Phoenix (Ctrl+C)
3. Extract over your existing installation
4. Restart Phoenix

**Your data is preserved** - Knowledge bases remain intact.

---

## 🆘 Getting Help

### Documentation
- **Full Guide**: [`BETA_TESTER_ONBOARDING_GUIDE.md`](BETA_TESTER_ONBOARDING_GUIDE.md)
- **Troubleshooting**: [`SOVEREIGN_SUPPORT_PROMPT.md`](SOVEREIGN_SUPPORT_PROMPT.md)
- **Technical Details**: [`QDRANT_SIDECAR_INTEGRATION.md`](QDRANT_SIDECAR_INTEGRATION.md)

### Support Channels
- **GitHub Issues**: Report bugs
- **GitHub Discussions**: Ask questions
- **Discord**: Beta tester community (invite-only)

### Before Asking for Help
1. Check the logs (see "Phoenix won't start" above)
2. Verify your API key is correct
3. Ensure port 3030 is available
4. Try restarting Phoenix

---

## 🎓 Pro Tips

### 1. Use Natural Language
Phoenix understands context. You don't need to be formal:
```
❌ "Execute task: analyze document"
✅ "Can you help me understand this document?"
```

### 2. Be Specific
The more context you provide, the better Phoenix can help:
```
❌ "I need help with code"
✅ "I'm writing a Python script to parse CSV files. Can you help me handle errors?"
```

### 3. Set Boundaries
Phoenix respects your preferences:
```
"I don't want to discuss [topic]. Please remember that."
```

### 4. Review Your Data
Periodically check what Phoenix knows:
```
"Show me what's in KB-01"
"What patterns have you noticed in my work?"
```

### 5. Backup Your Knowledge
```bash
# Backup your knowledge bases
tar -czf phoenix-backup-$(date +%Y%m%d).tar.gz storage/ user_config.toml

# Restore
tar -xzf phoenix-backup-20260210.tar.gz
```

---

## 🔥 The Phoenix Philosophy

### Sovereignty
You own your data. You control your AI. No exceptions.

### Privacy
Your conversations never leave your machine (except API calls YOU make).

### Transparency
Open source. Auditable. No hidden telemetry.

### Evolution
Phoenix learns from YOU, not from a corporate dataset.

### Bare Metal
Runs on YOUR hardware, optimized for YOUR system.

---

## 🌟 What Makes Phoenix Different

### vs. ChatGPT
- ✅ Your data stays local
- ✅ Personalized to YOU
- ✅ No usage limits (your API key, your budget)
- ✅ Extensible with custom skills

### vs. Local LLMs
- ✅ Access to state-of-the-art models (Claude, GPT-4, etc.)
- ✅ No GPU required
- ✅ Automatic updates
- ✅ Professional-grade performance

### vs. Other AI Assistants
- ✅ True privacy (no cloud storage)
- ✅ 8 specialized knowledge bases
- ✅ Emotional intelligence (Kardia system)
- ✅ Task governance (Oikos system)
- ✅ Safety governor (Forge system)

---

## 📈 Your First Week

### Day 1: Setup & Introduction
- Install Phoenix
- Configure API key
- Introduce yourself
- Explore the interface

### Day 2-3: Build Context
- Share your goals
- Discuss your projects
- Let Phoenix learn your preferences
- Try different conversation styles

### Day 4-5: Leverage Knowledge
- Ask Phoenix to recall previous conversations
- Request summaries and insights
- Use Phoenix for real work tasks
- Explore the wellness features

### Day 6-7: Advanced Features
- Review your knowledge bases
- Customize settings
- Try voice features (if enabled)
- Provide feedback on GitHub

---

## 🎯 Success Metrics

You'll know Phoenix is working when:
- ✅ She remembers your name and preferences
- ✅ She references previous conversations
- ✅ She provides personalized recommendations
- ✅ She adapts to your communication style
- ✅ She helps you accomplish real tasks

---

## 🚀 Ready to Rise?

Phoenix Marie is not just an AI assistant - she's YOUR cognitive partner. She learns from you, adapts to you, and evolves with you.

**Your journey begins now.**

```bash
./phoenix-rise.sh
```

**Welcome to the evolution.** 🔥

---

## 📞 Quick Reference

### Essential Commands
```bash
# Start Phoenix
./phoenix-rise.sh  # or .ps1 on Windows

# Stop Phoenix
Ctrl+C in terminal

# Check logs
tail -f /tmp/phoenix-gateway.log  # Unix
Get-Content phoenix-gateway.log -Tail 50  # Windows

# Check health
curl http://localhost:3030/api/v1/health
```

### Essential Files
```
phoenix-0.1.0-beta.1/
├── pagi-gateway(.exe)      # Main binary
├── VERSION                 # Current version
├── .env                    # Your API key (create from .env.example)
├── user_config.toml        # Your preferences (auto-created)
├── storage/                # Your knowledge bases (auto-created)
└── PHOENIX_QUICKSTART.md   # This guide
```

### Essential URLs
- **Dashboard**: http://localhost:3030
- **Health Check**: http://localhost:3030/api/v1/health
- **OpenRouter**: https://openrouter.ai/keys
- **GitHub**: https://github.com/YOUR-USERNAME/YOUR-REPO

---

**Version**: 0.1.0-beta.1  
**Last Updated**: 2026-02-10  
**Status**: Ready for Beta Testing 🔥

**Your data. Your hardware. Your intelligence. Zero hassle.**

# 🎓 Phoenix Marie: Onboarding Guide

## Understanding the Sovereign Stack

Phoenix Marie is not a chatbot; she is a **Multi-Layer Orchestrator**. Here is how to work with her:

---

## 🧠 The 8 Knowledge Bases

Everything you tell Phoenix is categorized into 8 distinct "Minds." Each serves a specific purpose:

1. **KB-01: Personal Context** - Your preferences, identity, and core attributes
2. **KB-02: Task History** - Your goals, projects, and task patterns
3. **KB-03: Domain Knowledge** - Technical skills, professional expertise
4. **KB-04: Social Intelligence (Chronos)** - Relationships, communication patterns
5. **KB-05: Creative Projects** - Ideas, brainstorms, creative work
6. **KB-06: Health & Wellness** - Physical and mental health tracking
7. **KB-07: Financial & Resources** - Budget, assets, resource management
8. **KB-08: Long-term Goals (Soma)** - Phoenix's own evolution and self-reflection

**Example**: When you say "I'm working on a Python project," Phoenix stores this in **KB-02** (Task History) and **KB-03** (Domain Knowledge). When you mention feeling stressed, it goes to **KB-06** (Wellness).

---

## 🟠 The Orange Pulse

When the UI pulses orange, Phoenix is in **Autonomous Mode**. She is:
- Consulting her knowledge bases
- Inferring solutions from past patterns
- Self-correcting based on previous feedback
- Generating contextually-aware responses

**This is not "loading"** - it's Phoenix thinking deeply about your request.

---

## 🎯 Coaching & Evolution

Phoenix learns from your feedback. Here's how to help her evolve:

### Positive Reinforcement
```
"That solution worked perfectly. Remember this approach."
```
Phoenix logs this to **KB-08** (Soma) and increases confidence in similar patterns.

### Corrective Feedback
```
"That didn't work. The issue was [specific problem]. Try [alternative approach]."
```
Phoenix adjusts her inference model and avoids the failed pattern.

### Pattern Recognition
```
"I notice you always suggest X when I ask about Y. That's helpful."
```
Phoenix reinforces the X→Y pattern in her knowledge graph.

---

## 🏛️ The Sovereign Architecture

### Local-First Design
- **All data** stays in `./storage/` on YOUR machine
- **No cloud sync** unless you explicitly enable it (future feature)
- **No telemetry** - Phoenix never "phones home" with your data

### The Memory Engine (Qdrant)
- **Automatic download** on first run (~45 MB)
- **Stored in** `./bin/qdrant`
- **Data in** `./data/qdrant`
- **Portable** - Move the entire Phoenix folder, keep all memories

### The Safety Governor (Forge)
- **Monitors** all system operations
- **Triggers** on suspicious patterns
- **Alerts** you via UI and terminal
- **Kill Switch** available in Settings

---

## 🛠️ Beta Tester Responsibilities

### 1. Be Candid
Phoenix is designed to handle:
- Direct feedback ("That's wrong")
- Wit and sarcasm ("Nice try, but no")
- Technical precision ("The error is on line 42")

Don't sugarcoat. She learns faster from honest feedback.

### 2. Monitor the Forge
Occasionally check the terminal output. If you see:
```
⚠️  FORGE ALERT: [Sovereignty violation detected]
```
This is the Safety Governor in action. Note what triggered it and report via GitHub Issues.

### 3. Local Sovereignty
Your data is yours. To "reset" Phoenix:
```bash
# Backup first (optional)
tar -czf phoenix-backup.tar.gz storage/ user_config.toml

# Reset
rm -rf storage/
# Phoenix will recreate empty KBs on next start
```

### 4. Report Bugs Effectively

**Found a bug? Use the built-in diagnostic exporter:**

1. Go to **Settings** in the UI
2. Click **"Download Logs"** button
3. Save the `phoenix-diagnostics-YYYYMMDD-HHMMSS.zip` file
4. Attach it to your GitHub Issue

The diagnostic package includes:
- Phoenix version
- System information
- Sanitized logs (API keys automatically redacted)
- Qdrant status
- Knowledge base status
- Configuration (sanitized)

**Alternatively, via command line:**
```bash
curl http://localhost:8001/api/v1/system/diagnostics -o diagnostics.zip
```

When reporting, also include:
- Steps to reproduce
- What you expected vs. what happened
- Any error messages you saw

---

## 🎨 Advanced Features

### Custom Skills
Phoenix supports custom skills via the plugin system. Check [`SKILLS_REGISTRY_README.md`](add-ons/pagi-gateway/SKILLS_REGISTRY_README.md) for details.

### Knowledge Base Queries
You can directly query your knowledge bases:
```bash
curl http://localhost:8001/api/v1/kb/01/records
```

### Wellness Reports
Ask Phoenix for periodic wellness checks:
```
"Give me a weekly wellness report"
```
She'll analyze your patterns in KB-06 and provide insights.

### Astro-Weather (Experimental)
If you've configured your birth chart, Phoenix can correlate planetary transits with your mood patterns. See [`KARDIA_EMOTIONAL_CALIBRATION.md`](KARDIA_EMOTIONAL_CALIBRATION.md).

---

## 🔄 Backup & Restore

### Backup Your Phoenix
```bash
# Full backup
tar -czf phoenix-backup-$(date +%Y%m%d).tar.gz storage/ user_config.toml .env

# Knowledge bases only
tar -czf kb-backup-$(date +%Y%m%d).tar.gz storage/
```

### Restore
```bash
# Extract backup
tar -xzf phoenix-backup-20260210.tar.gz

# Restart Phoenix
./phoenix-rise.sh
```

---

## 📊 Understanding System Health

### The Dashboard
- **CPU/Memory**: Real-time resource usage
- **KB Status**: Size and health of each knowledge base
- **API Usage**: Track OpenRouter API calls and costs
- **Uptime**: How long Phoenix has been running

### Health Check Endpoint
```bash
curl http://localhost:8001/api/v1/health
```

Returns:
```json
{
  "status": "healthy",
  "version": "0.1.0-beta.1",
  "knowledge_bases": {
    "kb01": "operational",
    "kb02": "operational",
    ...
  }
}
```

---

## 🚀 What to Try First

### Week 1: Foundation
1. **Introduce yourself** - Tell Phoenix who you are, what you do
2. **Set a goal** - Give her a project to help you with
3. **Daily check-ins** - Brief updates on your progress
4. **Ask for summaries** - "What do you know about me so far?"

### Week 2: Depth
1. **Share challenges** - Discuss problems you're facing
2. **Request analysis** - "What patterns do you see in my work?"
3. **Explore wellness** - Track mood, energy, sleep
4. **Refine preferences** - Teach Phoenix your communication style

### Week 3: Integration
1. **Use for real work** - Code reviews, brainstorming, planning
2. **Leverage memory** - Reference past conversations
3. **Provide feedback** - Help Phoenix improve her responses
4. **Explore advanced features** - Custom skills, API queries

---

## 🏆 Success Metrics

You'll know Phoenix is working when:
- ✅ She remembers your name and preferences
- ✅ She references previous conversations without prompting
- ✅ She provides personalized recommendations
- ✅ She adapts to your communication style
- ✅ She helps you accomplish real tasks

---

## 🔥 The Sovereign Promise

Phoenix Marie is YOUR cognitive partner. She:
- **Learns from you**, not from a corporate dataset
- **Remembers you**, not a generic user profile
- **Respects your privacy**, not your data as a product
- **Evolves with you**, not according to a roadmap

**Your data. Your hardware. Your intelligence. Your evolution.**

---

## 📞 Support & Community

### Getting Help
- **Quick Start**: [`QUICKSTART.md`](QUICKSTART.md)
- **Full Guide**: [`BETA_TESTER_ONBOARDING_GUIDE.md`](BETA_TESTER_ONBOARDING_GUIDE.md)
- **Technical Details**: [`QDRANT_SIDECAR_INTEGRATION.md`](QDRANT_SIDECAR_INTEGRATION.md)
- **GitHub Issues**: Report bugs
- **GitHub Discussions**: Ask questions

### Contributing
- **Bug Reports**: Use the issue template
- **Feature Requests**: Discuss in GitHub Discussions
- **Code Contributions**: Pull requests welcome
- **Documentation**: Help improve guides

---

**Version**: 0.1.0-beta.1  
**Last Updated**: 2026-02-10  
**Status**: Ready for Beta Testing 🔥

**Welcome to the Forge. Let's evolve together.**

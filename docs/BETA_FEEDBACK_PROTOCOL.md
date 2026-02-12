# 🎯 Phoenix Beta Feedback Protocol

## Overview

This document outlines the structured feedback collection process for Phoenix beta testers. The goal is to gather actionable insights while respecting tester time and maintaining data sovereignty.

---

## 🧪 Beta Testing Phases

### Phase 1: Installation & First-Run (Days 1-3)
**Focus:** Verify the deployment pipeline and onboarding experience.

**Key Metrics:**
- Installation success rate
- First-run setup completion time
- API key configuration issues
- Qdrant sidecar launch success

**Feedback Questions:**
1. Which platform did you install on? (Windows/Linux/macOS Intel/macOS ARM)
2. Did the binary run without additional dependencies?
3. Did `phoenix-rise` complete successfully?
4. How long did first-run setup take?
5. Were the prompts clear and helpful?
6. Did you encounter any errors? (Provide logs if possible)

### Phase 2: Core Functionality (Days 4-10)
**Focus:** Test the primary AGI orchestration and knowledge base features.

**Key Metrics:**
- Chat interaction quality
- Knowledge base query accuracy
- Multi-agent coordination effectiveness
- System stability over extended use

**Feedback Questions:**
1. How would you rate the chat response quality? (1-10)
2. Did Phoenix remember context across sessions?
3. Were knowledge base queries relevant and accurate?
4. Did you experience any crashes or freezes?
5. What features did you use most?
6. What features felt missing or incomplete?

### Phase 3: Advanced Features (Days 11-21)
**Focus:** Stress-test sovereignty features, wellness tracking, and customization.

**Key Metrics:**
- Wellness report accuracy
- Ethos framework alignment
- Custom skill integration
- Performance under load

**Feedback Questions:**
1. Did the wellness reports feel personalized and accurate?
2. How well did Phoenix adapt to your communication style?
3. Did you try creating custom skills? How was the experience?
4. What was your average session length?
5. Did performance degrade over time?
6. What would make Phoenix more valuable to you?

---

## 📊 Feedback Collection Methods

### Method 1: Structured Survey (Recommended)

**Timing:** End of each phase

**Format:** Google Form / Typeform with:
- Multiple choice questions (quantitative)
- Short answer fields (qualitative)
- Optional log file upload
- Anonymous option available

**Example Questions:**

```markdown
## Installation Experience
1. Platform: [ ] Windows [ ] Linux [ ] macOS Intel [ ] macOS ARM
2. Installation difficulty: [ ] Easy [ ] Moderate [ ] Difficult
3. Time to first successful run: [ ] <5 min [ ] 5-15 min [ ] 15-30 min [ ] >30 min
4. Encountered errors: [ ] Yes [ ] No
   - If yes, describe: ___________

## Core Functionality
5. Response quality (1-10): [slider]
6. Context retention (1-10): [slider]
7. Most valuable feature: ___________
8. Most frustrating issue: ___________

## Open Feedback
9. What surprised you most about Phoenix?
10. What would you change first?
11. Would you recommend Phoenix to others? Why/why not?
```

### Method 2: GitHub Issues (Technical Feedback)

**For:** Bug reports, feature requests, technical issues

**Template:**
```markdown
## Issue Type
- [ ] Bug Report
- [ ] Feature Request
- [ ] Performance Issue
- [ ] Documentation Issue

## Environment
- **Platform:** Windows/Linux/macOS
- **Version:** 0.1.0-beta.1
- **Rust Version:** (if building from source)

## Description
[Clear description of the issue]

## Steps to Reproduce
1. 
2. 
3. 

## Expected Behavior
[What should happen]

## Actual Behavior
[What actually happened]

## Logs
```
[Paste relevant logs from ~/.pagi/logs/]
```

## Screenshots
[If applicable]

## Additional Context
[Any other relevant information]
```

### Method 3: Weekly Check-In (Optional)

**Format:** 5-minute async video or written update

**Prompts:**
1. What did you accomplish with Phoenix this week?
2. What worked well?
3. What didn't work?
4. What questions do you have?

---

## 🔒 Privacy & Data Sovereignty

### What We Collect
- ✅ Platform and version information
- ✅ Error logs (with sensitive data redacted)
- ✅ Performance metrics (response times, memory usage)
- ✅ Feature usage statistics (which features are used most)
- ✅ Qualitative feedback (your written responses)

### What We DON'T Collect
- ❌ Your chat conversations
- ❌ Your knowledge base contents
- ❌ Your API keys
- ❌ Your personal data stored in Phoenix
- ❌ Your identity (unless you choose to share)

### Data Handling
1. **Anonymization:** All feedback can be submitted anonymously
2. **Opt-In Logs:** Log sharing is always optional
3. **Local Storage:** Your data never leaves your machine unless you explicitly share logs
4. **Redaction Tools:** We provide scripts to redact sensitive data from logs before sharing

---

## 🛠️ Log Redaction Tool

Before sharing logs, use this script to remove sensitive information:

**Windows (PowerShell):**
```powershell
# scripts/redact-logs.ps1
param([string]$LogFile)

$content = Get-Content $LogFile -Raw
$content = $content -replace 'sk-[a-zA-Z0-9-]+', 'sk-REDACTED'
$content = $content -replace 'Bearer [a-zA-Z0-9-]+', 'Bearer REDACTED'
$content = $content -replace '"api_key":\s*"[^"]+"', '"api_key": "REDACTED"'
$content = $content -replace 'password["\s:=]+[^\s,}]+', 'password: REDACTED'

$redactedFile = $LogFile -replace '\.log$', '.redacted.log'
$content | Out-File -FilePath $redactedFile
Write-Host "Redacted log saved to: $redactedFile"
```

**Linux/macOS (Bash):**
```bash
#!/usr/bin/env bash
# scripts/redact-logs.sh

LOG_FILE="$1"
REDACTED_FILE="${LOG_FILE%.log}.redacted.log"

sed -E \
  -e 's/sk-[a-zA-Z0-9-]+/sk-REDACTED/g' \
  -e 's/Bearer [a-zA-Z0-9-]+/Bearer REDACTED/g' \
  -e 's/"api_key":\s*"[^"]+"/"api_key": "REDACTED"/g' \
  -e 's/password["\s:=]+[^\s,}]+/password: REDACTED/g' \
  "$LOG_FILE" > "$REDACTED_FILE"

echo "Redacted log saved to: $REDACTED_FILE"
```

**Usage:**
```bash
# Redact a specific log file
./scripts/redact-logs.sh ~/.pagi/logs/gateway.log

# Redact all logs
for log in ~/.pagi/logs/*.log; do
  ./scripts/redact-logs.sh "$log"
done
```

---

## 📈 Success Metrics

### Quantitative Metrics
- **Installation Success Rate:** Target >95%
- **First-Run Completion Rate:** Target >90%
- **Average Response Quality:** Target >7/10
- **Crash Rate:** Target <1% of sessions
- **Performance:** Target <2s response time for simple queries

### Qualitative Metrics
- **User Satisfaction:** "Would you recommend Phoenix?" >80% yes
- **Feature Completeness:** "Does Phoenix meet your needs?" >70% yes
- **Documentation Quality:** "Were docs helpful?" >85% yes
- **Sovereignty Perception:** "Do you feel in control of your data?" >95% yes

---

## 🎯 Feedback Prioritization

### P0 (Critical - Fix Immediately)
- Installation failures
- Data loss or corruption
- Security vulnerabilities
- Complete feature breakage

### P1 (High - Fix in Next Patch)
- Frequent crashes
- Major usability issues
- Performance degradation
- Missing critical documentation

### P2 (Medium - Fix in Next Minor Release)
- Feature requests with high demand
- Minor bugs with workarounds
- UI/UX improvements
- Documentation gaps

### P3 (Low - Consider for Future)
- Nice-to-have features
- Edge case bugs
- Cosmetic issues
- Advanced customization requests

---

## 🔄 Feedback Loop

### Weekly Cycle
1. **Monday:** Review all feedback from previous week
2. **Tuesday:** Prioritize issues and create GitHub issues
3. **Wednesday-Friday:** Implement fixes and improvements
4. **Saturday:** Test fixes locally
5. **Sunday:** Prepare release notes for next patch

### Communication
- **Weekly Update:** Email/Discord post summarizing:
  - Issues fixed
  - Features added
  - Known issues
  - Next week's focus
- **Monthly Retrospective:** Longer-form update on:
  - Beta progress
  - Major learnings
  - Roadmap adjustments
  - Tester highlights

---

## 🏆 Beta Tester Recognition

### Contribution Levels

**🥉 Bronze Contributor**
- Completed Phase 1 feedback
- Reported at least 1 issue
- Recognition in CONTRIBUTORS.md

**🥈 Silver Contributor**
- Completed Phases 1-2 feedback
- Reported 5+ issues or feature requests
- Helped test fixes
- Recognition + early access to new features

**🥇 Gold Contributor**
- Completed all 3 phases
- Reported 10+ actionable issues
- Contributed code or documentation
- Helped other testers
- Recognition + lifetime "Founding Tester" badge + input on roadmap

---

## 📞 Support Channels

### For Beta Testers

**GitHub Discussions:**
- General questions
- Feature discussions
- Best practices sharing

**GitHub Issues:**
- Bug reports
- Feature requests
- Technical problems

**Discord/Slack (if available):**
- Real-time help
- Community support
- Quick questions

**Email (for sensitive issues):**
- Security concerns
- Privacy questions
- Personal feedback

---

## 🚀 Post-Beta Transition

### When Beta Ends
1. **Final Survey:** Comprehensive feedback on entire experience
2. **Data Migration:** Ensure smooth transition to v1.0
3. **Recognition:** Public thank you to all contributors
4. **Early Access:** Beta testers get v1.0 before public release

### What Happens to Feedback
- Aggregated insights published (anonymized)
- Top feature requests prioritized for v1.0
- Bug fixes incorporated into stable release
- Lessons learned documented for future releases

---

## 📚 Templates & Resources

### Quick Feedback Template (Copy-Paste)

```markdown
## Phoenix Beta Feedback

**Platform:** [Windows/Linux/macOS]
**Version:** 0.1.0-beta.1
**Testing Phase:** [1/2/3]

### What Worked Well
- 
- 

### What Didn't Work
- 
- 

### Feature Requests
- 
- 

### Overall Rating (1-10)
- Installation: __/10
- Usability: __/10
- Performance: __/10
- Documentation: __/10

### Would you recommend Phoenix?
[ ] Yes [ ] No [ ] Maybe

**Why?**
```

### Bug Report Template (Copy-Paste)

```markdown
## Bug Report

**Platform:** [Windows/Linux/macOS]
**Version:** 0.1.0-beta.1

**Description:**
[What went wrong?]

**Steps to Reproduce:**
1. 
2. 
3. 

**Expected:** [What should happen]
**Actual:** [What actually happened]

**Logs:** [Attach redacted logs if possible]
```

---

## 🎯 Call to Action for Testers

**Your feedback shapes Phoenix's future.**

Every bug report, feature request, and piece of feedback helps us build a better AGI orchestration platform. We're committed to:

- **Respecting your time:** Feedback should take <10 minutes per phase
- **Protecting your privacy:** Your data stays yours
- **Acting on feedback:** Every issue gets reviewed and prioritized
- **Recognizing contributions:** Your help will be acknowledged

**Thank you for being part of the Phoenix beta program. Together, we're building the future of sovereign AI.**

---

**Questions about the feedback process? Open a GitHub Discussion or reach out via your preferred support channel.**

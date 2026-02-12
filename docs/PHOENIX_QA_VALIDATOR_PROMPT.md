# Phoenix Quality Assurance Validator
## Role: Eagle Eye Post-Release Auditor

This prompt guides the Cursor IDE agent through a comprehensive post-release validation process to ensure the Phoenix sovereign beta release meets all quality standards.

---

## 🎯 Mission Objective

Verify that the GitHub Release artifacts are complete, checksums are valid, and the redaction system protects user privacy as designed.

---

## 📋 Validation Protocol

### Phase 1: Release Asset Verification

**Task**: Audit the GitHub Release page for completeness and integrity.

```markdown
# RELEASE ASSET AUDIT

1. **Navigate to GitHub Release**:
   - Open: https://github.com/[YOUR_USERNAME]/pagi-uac-main/releases/tag/v0.1.0-beta.1
   - Verify the release is marked as "Pre-release" (beta status)
   - Confirm the release notes are present and professional

2. **Asset Inventory Check**:
   Verify all 8 required assets are present:
   
   **Binaries (4)**:
   - [ ] `phoenix-windows-x86_64.zip`
   - [ ] `phoenix-linux-x86_64.tar.gz`
   - [ ] `phoenix-macos-x86_64.tar.gz`
   - [ ] `phoenix-macos-aarch64.tar.gz`
   
   **Checksums (4)**:
   - [ ] `phoenix-windows-x86_64.zip.sha256`
   - [ ] `phoenix-linux-x86_64.tar.gz.sha256`
   - [ ] `phoenix-macos-x86_64.tar.gz.sha256`
   - [ ] `phoenix-macos-aarch64.tar.gz.sha256`

3. **Download Local Platform Asset**:
   - Identify your platform (Windows/Linux/macOS)
   - Download the appropriate binary archive
   - Download the corresponding .sha256 file

4. **Checksum Verification**:
   
   **Windows (PowerShell)**:
   ```powershell
   $hash = (Get-FileHash -Algorithm SHA256 phoenix-windows-x86_64.zip).Hash.ToLower()
   $expected = (Get-Content phoenix-windows-x86_64.zip.sha256).Split()[0]
   if ($hash -eq $expected) { 
       Write-Host "✅ Checksum VERIFIED" -ForegroundColor Green 
   } else { 
       Write-Host "❌ Checksum MISMATCH" -ForegroundColor Red 
   }
   ```
   
   **Linux/macOS (Bash)**:
   ```bash
   sha256sum -c phoenix-linux-x86_64.tar.gz.sha256
   # Should output: phoenix-linux-x86_64.tar.gz: OK
   ```

5. **Binary Extraction Test**:
   - Extract the archive
   - Verify the binary is executable
   - Check for any corruption or missing files
```

---

### Phase 2: Redaction Smoke Test

**Task**: Validate that the redaction engine successfully scrubs sensitive data.

```markdown
# REDACTION ENGINE VALIDATION

1. **Setup Test Environment**:
   - Extract the Phoenix binary to a test directory
   - Create a test `.env` file with fake credentials:
   
   ```env
   OPENROUTER_API_KEY=sk-or-v1-TEST_KEY_DO_NOT_LEAK_12345678
   ANTHROPIC_API_KEY=sk-ant-api03-FAKE_ANTHROPIC_KEY_FOR_TESTING
   QDRANT_API_KEY=test-qdrant-key-abc123xyz789
   ```

2. **Generate Test Logs**:
   - Run Phoenix in a test mode (or trigger a simple operation)
   - Ensure the fake keys appear in log files
   - Locate the generated log files (typically in `logs/` or `~/.pagi/logs/`)

3. **Execute Redaction Script**:
   
   **Windows**:
   ```powershell
   .\scripts\redact-logs.ps1 -LogPath ".\logs" -OutputPath ".\logs-redacted"
   ```
   
   **Linux/macOS**:
   ```bash
   ./scripts/redact-logs.sh ./logs ./logs-redacted
   ```

4. **Verify Redaction Success**:
   - Open the redacted log files
   - Search for the fake keys (should NOT be found)
   - Verify redaction markers are present (e.g., `[REDACTED_API_KEY]`)
   - Confirm log structure and readability are maintained

5. **Redaction Pattern Tests**:
   Check that these patterns are successfully redacted:
   - [ ] OpenRouter keys: `sk-or-v1-*`
   - [ ] Anthropic keys: `sk-ant-*`
   - [ ] Generic API keys: `api_key=*`, `apiKey:*`
   - [ ] Bearer tokens: `Bearer *`
   - [ ] JWT tokens: `eyJ*` (base64 JWT format)
   - [ ] Email addresses (if configured)
   - [ ] IP addresses (if configured)
```

---

### Phase 3: Beta Distribution Readiness

**Task**: Verify supporting documentation and onboarding materials.

```markdown
# BETA DISTRIBUTION AUDIT

1. **Documentation Completeness**:
   - [ ] `BETA_TESTER_ONBOARDING_GUIDE.md` is present and clear
   - [ ] `BETA_FEEDBACK_PROTOCOL.md` explains the Bronze/Silver/Gold system
   - [ ] `BETA_DISTRIBUTION_GUIDE.md` provides distribution instructions
   - [ ] `QUICKSTART.md` or `PHOENIX_QUICKSTART.md` is beginner-friendly

2. **Feedback Mechanism**:
   - [ ] GitHub Issues template is configured for bug reports
   - [ ] Contributor recognition system is documented
   - [ ] Contact method for private security issues is clear

3. **Privacy & Security**:
   - [ ] `.env.example` is present (no real keys)
   - [ ] `.gitignore` excludes sensitive files
   - [ ] Redaction scripts are included in the release
   - [ ] Privacy policy or data handling statement is clear

4. **Platform-Specific Instructions**:
   - [ ] Windows setup instructions (PowerShell scripts)
   - [ ] Linux/macOS setup instructions (Bash scripts)
   - [ ] Dependency requirements are documented
   - [ ] Troubleshooting section is present
```

---

### Phase 4: Final Health Report

**Task**: Generate a comprehensive release health summary.

```markdown
# RELEASE HEALTH REPORT

## 🔥 Phoenix v0.1.0-beta.1 Quality Audit

**Audit Date**: [CURRENT_DATE]
**Auditor**: Cursor IDE Agent (Eagle Eye QA)
**Release Tag**: v0.1.0-beta.1

---

### ✅ Asset Verification
- **Total Assets**: [X/8] present
- **Checksum Validation**: [PASS/FAIL]
- **Binary Integrity**: [PASS/FAIL]
- **Platform Coverage**: [Windows/Linux/macOS x86/macOS ARM]

---

### 🛡️ Redaction Engine
- **Test Keys Scrubbed**: [PASS/FAIL]
- **Pattern Coverage**: [X/7] patterns validated
- **Log Readability**: [MAINTAINED/DEGRADED]
- **Privacy Compliance**: [SOVEREIGN/COMPROMISED]

---

### 📚 Documentation Quality
- **Onboarding Guide**: [CLEAR/NEEDS_WORK]
- **Feedback Protocol**: [COMPLETE/INCOMPLETE]
- **Security Posture**: [SOVEREIGN/VULNERABLE]

---

### 🎯 Final Verdict

**Status**: [🟢 READY FOR BETA / 🟡 MINOR ISSUES / 🔴 CRITICAL ISSUES]

**Summary**:
[Provide a 2-3 sentence summary of the release health]

**Recommendations**:
- [Any suggested improvements or fixes]
- [Priority items for immediate attention]

---

### 🚀 Launch Signal

**[IF ALL CHECKS PASS]**:
```
🔥 Phoenix is airborne. All checksums verified. 
The Sovereign Beta has officially begun.
Coach Jamey, you are cleared for public distribution.
```

**[IF ISSUES FOUND]**:
```
⚠️ Phoenix requires attention before launch.
Review the issues above and re-run validation after fixes.
```
```

---

## 🎮 Usage Instructions

### For Cursor IDE Agent:

1. **Trigger the Audit**:
   ```
   "Run the Phoenix Quality Assurance Validator on the latest release"
   ```

2. **Automated Execution**:
   - The agent will systematically work through each phase
   - Download and verify checksums
   - Run redaction tests
   - Generate the final report

3. **Review Results**:
   - The agent will present the Health Report
   - Address any flagged issues
   - Re-run validation after fixes

---

## 🏛️ Sovereign Quality Standards

This validator embodies the Phoenix philosophy:

- **Transparency**: Every check is documented and reproducible
- **Privacy**: Redaction validation ensures user secrets stay secret
- **Sovereignty**: No external dependencies for validation
- **Excellence**: Beta quality that rivals production releases

---

## 📞 Post-Validation Actions

### If Validation Passes:
1. Announce the beta release to your community
2. Share the onboarding guide with testers
3. Monitor the feedback channels
4. Prepare for the first bug reports

### If Validation Fails:
1. Document the specific failures
2. Fix the issues in the codebase
3. Re-trigger the release workflow
4. Re-run this validation protocol

---

## 🎖️ Beta Tester Recognition

Remember: Your beta testers are **Sovereigns in Training**. The quality of this release sets the tone for the entire community.

**Bronze Contributors**: Find and report bugs
**Silver Contributors**: Provide detailed reproduction steps
**Gold Contributors**: Suggest architectural improvements

---

**Coach Jamey, this is your "Mission Control" checklist. When all lights are green, Phoenix rises.**

🔥 **Sovereign. Transparent. Unstoppable.**

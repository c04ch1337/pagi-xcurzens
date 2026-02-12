# 🚀 Phoenix Beta Distribution Guide

## Overview

This guide documents the complete beta distribution system for Phoenix, transitioning from internal research to public beta distribution while maintaining privacy and security.

---

## 🏛️ Architecture: The "Sanitization & Distribution" Strategy

### 1. **The Great Scrub** ✅
- **Purpose**: Remove all personal "biological" data while keeping the "genetic code" intact
- **Implementation**: 
  - [`scripts/sanitize_for_release.sh`](scripts/sanitize_for_release.sh) (Unix/Linux/Mac)
  - [`scripts/sanitize_for_release.ps1`](scripts/sanitize_for_release.ps1) (Windows)
- **What Gets Removed**:
  - `storage/` - Vector databases (KB-01 through KB-08)
  - `vector_db/` - Alternative vector storage
  - `data/` and `add-ons/pagi-gateway/data/` - Runtime databases
  - `.env` - Environment files with secrets
  - `user_config.toml` - User-specific configuration
  - `target/` - Build artifacts
  - `*.log` files - Log files
  - `qdrant/` and `qdrant.zip` - Qdrant binary (users download their own)

### 2. **State Management** ✅
- **First Run Detection**: [`UserConfig`](crates/pagi-core/src/config.rs) tracks first-run state
- **Empty Database Initialization**: When no database exists, Phoenix initializes fresh, empty "Phoenix Mind" for the user
- **User-Specific Data**: Each user builds their own KB-01 through KB-08 locally

### 3. **The API Vault** ✅
- **Endpoint**: `POST /api/v1/config/api-key`
- **Purpose**: UI can save user's personal OpenRouter key to local, encrypted config
- **Storage**: [`user_config.toml`](user_config.toml) (gitignored, never committed)
- **Priority Chain**: `user_config.toml` → `PAGI_LLM_API_KEY` → `OPENROUTER_API_KEY`

### 4. **Auto-Update Logic** 🚧
- **Version File**: [`VERSION`](../VERSION) tracks current release
- **Update Check**: Planned for phoenix-rise scripts
- **GitHub API Integration**: Check latest release tag against local version

---

## 📦 File Structure

### Configuration Files
```
user_config.toml          # User's personal config (gitignored)
.env                      # Environment variables (gitignored)
.env.example              # Template for users
VERSION                   # Current version (0.1.0-beta.1)
```

### Scripts
```
scripts/
├── sanitize_for_release.sh    # Unix sanitization
├── sanitize_for_release.ps1   # Windows sanitization
├── deploy_beta.sh             # Unix deployment
└── deploy_beta.ps1            # Windows deployment
```

### Startup Scripts
```
phoenix-rise.sh           # Unix startup with version check
phoenix-rise.ps1          # Windows startup with version check
pagi-up.sh               # Alternative Unix startup
pagi-up.ps1              # Alternative Windows startup
```

---

## 🔐 User Configuration System

### UserConfig Structure
```rust
pub struct UserConfig {
    pub api_key: Option<String>,        // User's OpenRouter API key
    pub llm_model: Option<String>,      // Preferred LLM model
    pub llm_api_url: Option<String>,    // Preferred API URL
    pub user_name: Option<String>,      // User identifier
    pub first_run: bool,                // First run flag
    pub version: Option<String>,        // Beta version
}
```

### API Endpoints

#### GET `/api/v1/config/api-key`
Check if API key is configured (first-run detection)
```json
{
  "configured": true,
  "first_run": false,
  "has_user_name": true
}
```

#### POST `/api/v1/config/api-key`
Save user's API key
```json
{
  "api_key": "sk-or-v1-...",
  "llm_model": "anthropic/claude-opus-4.6",
  "llm_api_url": "https://openrouter.ai/api/v1/chat/completions",
  "user_name": "Beta Tester"
}
```

#### GET `/api/v1/config/user`
Get user configuration (without exposing API key)
```json
{
  "first_run": false,
  "has_api_key": true,
  "llm_model": "anthropic/claude-opus-4.6",
  "llm_api_url": "https://openrouter.ai/api/v1/chat/completions",
  "user_name": "Beta Tester",
  "version": "0.1.0-beta.1"
}
```

---

## 🛠️ Deployment Process

### Step 1: Sanitize
```bash
# Unix/Linux/Mac
./scripts/sanitize_for_release.sh

# Windows
.\scripts\sanitize_for_release.ps1
```

### Step 2: Deploy
```bash
# Unix/Linux/Mac
./scripts/deploy_beta.sh

# Windows
.\scripts\deploy_beta.ps1
```

This will:
1. Run sanitization
2. Execute tests
3. Build release binaries
4. Create release package in `releases/phoenix-{VERSION}/`
5. Generate archives (`.tar.gz` and `.zip`)
6. Create SHA256 checksums
7. Generate beta user README

### Step 3: Create GitHub Release
```bash
gh release create v0.1.0-beta.1 \
  releases/phoenix-0.1.0-beta.1.tar.gz \
  releases/phoenix-0.1.0-beta.1.zip \
  --title "Phoenix v0.1.0-beta.1" \
  --notes "Beta release - First public distribution"
```

---

## 🚀 The "Fresh Install" Experience

### For Beta Users

1. **Download Release**
   - Download from GitHub Releases
   - Extract archive

2. **First Run**
   ```bash
   # Copy environment template
   cp .env.example .env
   
   # Edit .env and add OpenRouter API key
   # Get one at: https://openrouter.ai/keys
   
   # Start Phoenix
   ./phoenix-rise.sh  # or phoenix-rise.ps1 on Windows
   ```

3. **What Happens**
   - Orchestrator detects no database exists
   - Phoenix initializes "Blank Slate" version of 8 Knowledge Bases
   - UI prompts: "Welcome, Phoenix requires your OpenRouter API Key to begin"
   - User provides API key via UI
   - Key saved to `user_config.toml` (local, never shared)

4. **Data Capture**
   - As users interact, their own KB-01 through KB-08 fill up locally
   - All data stays on THEIR bare metal
   - No personal data sent to external services
   - Only LLM API calls go through OpenRouter (using their key)

---

## 🔒 Privacy & Security Guarantees

### What Stays Local
- ✅ All knowledge bases (KB-01 through KB-08)
- ✅ Vector databases (`storage/`, `vector_db/`)
- ✅ User configuration (`user_config.toml`)
- ✅ Runtime data (`data/`)
- ✅ Logs

### What Gets Shared
- ❌ Nothing! All data is local
- ⚠️ Only LLM API calls go to OpenRouter (using user's key)
- ⚠️ User controls what they share with the LLM

### Gitignore Protection
```gitignore
# Vector databases (personal knowledge bases)
/storage/
/vector_db/
*.sled/

# User configuration (local API keys)
user_config.toml
config/user_config.toml

# Environment files (secrets)
.env
.env.local
.env.*.local

# Runtime data
/data/
/pagi-gateway/data/
/add-ons/pagi-gateway/data/
```

---

## 📊 Version Management

### Current Version
- **File**: [`VERSION`](../VERSION)
- **Format**: Semantic versioning with beta tag
- **Example**: `0.1.0-beta.1`

### Version Bumping
```bash
# Update VERSION file
echo "0.1.0-beta.2" > VERSION

# Commit and tag
git add VERSION
git commit -m "Bump version to 0.1.0-beta.2"
git tag v0.1.0-beta.2
git push origin main --tags
```

### Auto-Update (Planned)
- Phoenix-rise scripts will check GitHub API for latest release
- Compare local VERSION against latest tag
- Prompt user: "A new Phoenix Evolution is available. Update now? (y/n)"

---

## 🧪 Testing Checklist

### Pre-Release Testing
- [ ] Run sanitization script
- [ ] Verify all personal data removed
- [ ] Test clean build: `cargo build --release`
- [ ] Run test suite: `cargo test --workspace --release`
- [ ] Test first-run experience:
  - [ ] Delete `user_config.toml`
  - [ ] Delete `storage/` and `vector_db/`
  - [ ] Start Phoenix
  - [ ] Verify first-run prompt
  - [ ] Configure API key via UI
  - [ ] Verify key saved to `user_config.toml`
  - [ ] Test chat functionality

### Post-Release Testing
- [ ] Download release archive
- [ ] Extract and test on clean machine
- [ ] Verify README instructions work
- [ ] Test on Windows, Linux, and Mac

---

## 📝 Beta User Documentation

### Quick Start (for beta users)
See [`BETA_README.md`](releases/phoenix-{VERSION}/BETA_README.md) in release package

### Support Channels
- GitHub Issues for bug reports
- GitHub Discussions for questions
- Private Discord for beta testers (optional)

---

## 🔄 Update Workflow

### For Developers
1. Make changes
2. Update VERSION file
3. Run sanitization
4. Run deployment script
5. Create GitHub release
6. Notify beta testers

### For Beta Users
1. Check for updates (manual or via phoenix-rise)
2. Download new release
3. Extract over existing installation
4. Restart Phoenix
5. User data (KB-01..KB-08) preserved

---

## 🎯 Future Enhancements

### Planned Features
- [ ] Auto-update via phoenix-rise scripts
- [ ] GitHub API integration for version checking
- [ ] In-app update notifications
- [ ] Encrypted backup/restore for user data
- [ ] Multi-profile support
- [ ] Cloud sync (optional, user-controlled)

### Security Enhancements
- [ ] API key encryption at rest
- [ ] Two-factor authentication for sensitive operations
- [ ] Audit log for all configuration changes
- [ ] Sandboxed skill execution

---

## 📞 Contact & Support

### For Beta Testers
- **Issues**: GitHub Issues
- **Questions**: GitHub Discussions
- **Security**: security@phoenix-project.ai (if applicable)

### For Contributors
- **Development**: See CONTRIBUTING.md
- **Architecture**: See this guide
- **Code Review**: Pull requests welcome

---

## 📄 License

See LICENSE file in repository.

---

**Last Updated**: 2026-02-10  
**Version**: 0.1.0-beta.1  
**Status**: Ready for Beta Distribution 🚀

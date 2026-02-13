# 🔧 Phoenix Release Configuration Guide

## Overview

Before your first release, you need to configure several files with your actual GitHub repository information. This guide walks you through each required change.

---

## ⚠️ Critical: Files That Need Configuration

### 1. GitHub Actions Workflow

**File**: [`.github/workflows/release.yml`](.github/workflows/release.yml)

**Current State**: ✅ Already configured correctly
- Uses `${{ github.repository }}` for dynamic repo detection
- No hardcoded repository names
- **Action Required**: None - workflow is ready to use

---

### 2. Updater Module

**File**: [`crates/pagi-core/src/updater.rs`](crates/pagi-core/src/updater.rs:11-12)

**Current State**: ❌ Needs configuration

**Lines to Update**:
```rust
const REPO_OWNER: &str = "your-github-username"; // TODO: Update with actual repo owner
const REPO_NAME: &str = "pagi-uac-main"; // TODO: Update with actual repo name
```

**Change To**:
```rust
const REPO_OWNER: &str = "YOUR-ACTUAL-GITHUB-USERNAME";
const REPO_NAME: &str = "YOUR-ACTUAL-REPO-NAME";
```

**Example**:
```rust
const REPO_OWNER: &str = "The Creatormilner";
const REPO_NAME: &str = "phoenix-marie";
```

---

### 3. Documentation Files

#### A. Beta Tester Onboarding Guide

**File**: [`BETA_TESTER_ONBOARDING_GUIDE.md`](BETA_TESTER_ONBOARDING_GUIDE.md)

**Lines to Update**:
- Line 51: `https://github.com/YOUR-USERNAME/pagi-uac-main/releases`
- Line 293: `https://github.com/YOUR-USERNAME/pagi-uac-main/issues/new`

**Change To**:
```markdown
https://github.com/YOUR-ACTUAL-USERNAME/YOUR-ACTUAL-REPO/releases
https://github.com/YOUR-ACTUAL-USERNAME/YOUR-ACTUAL-REPO/issues/new
```

#### B. Release Overseer Prompt

**File**: [`PHOENIX_RELEASE_OVERSEER_PROMPT.md`](PHOENIX_RELEASE_OVERSEER_PROMPT.md)

**Lines to Update**:
- Line 234: Download URL example
- Line 469: Release URL

**Change To**: Your actual repository URLs

#### C. Sovereign Support Prompt

**File**: [`SOVEREIGN_SUPPORT_PROMPT.md`](SOVEREIGN_SUPPORT_PROMPT.md)

**Line to Update**:
- Line 363: GitHub Issues link

**Change To**: Your actual repository URL

---

## 🚀 Pre-Release Configuration Checklist

### Step 1: Determine Your Repository Information

```bash
# If you haven't created a GitHub repo yet:
# 1. Go to https://github.com/new
# 2. Create a new repository (public or private)
# 3. Note the owner and repo name

# Example:
# Owner: The Creatormilner
# Repo: phoenix-marie
# Full URL: https://github.com/The Creatormilner/phoenix-marie
```

### Step 2: Update Updater Module

```bash
# Edit the file
code crates/pagi-core/src/updater.rs

# Update lines 11-12:
const REPO_OWNER: &str = "The Creatormilner";  # Your GitHub username
const REPO_NAME: &str = "phoenix-marie";  # Your repo name
```

### Step 3: Update Documentation

Use find-and-replace to update all documentation:

```bash
# Find all instances of placeholder URLs
grep -r "YOUR-USERNAME" *.md
grep -r "YOUR-ACTUAL" *.md

# Replace with your actual information
# Use your editor's find-and-replace feature
```

### Step 4: Verify Configuration

```bash
# Check updater module
grep "REPO_OWNER\|REPO_NAME" crates/pagi-core/src/updater.rs

# Should show your actual username and repo name
# NOT "your-github-username" or "YOUR-USERNAME"
```

### Step 5: Test Locally

```bash
# Build the project
cargo build --release

# Run tests
cargo test --workspace --release

# Verify updater compiles correctly
cargo check -p pagi-core
```

---

## 🔐 GitHub Token Configuration (Optional)

For **private repositories**, users need a GitHub token to check for updates.

### For Beta Testers (Private Repo)

Add to `.env.example`:
```bash
# GitHub Token (only needed for private repositories)
# Create at: https://github.com/settings/tokens
# Required scope: repo (for private repos) or public_repo (for public repos)
GITHUB_TOKEN=ghp_your_token_here
```

### For Public Repositories

No token needed! The updater will work without authentication.

---

## 📋 Complete Configuration Checklist

Before your first release, verify:

- [ ] **Repository Created**: GitHub repo exists and is accessible
- [ ] **Updater Configured**: `REPO_OWNER` and `REPO_NAME` set in `updater.rs`
- [ ] **Documentation Updated**: All `YOUR-USERNAME` placeholders replaced
- [ ] **Version File**: `VERSION` file contains correct version (e.g., `0.1.0-beta.1`)
- [ ] **Sanitization**: Run `./scripts/sanitize_for_release.sh` (if exists)
- [ ] **Tests Pass**: `cargo test --workspace --release` succeeds
- [ ] **Build Works**: `cargo build --release` succeeds
- [ ] **GitHub Actions**: Workflow file is in `.github/workflows/release.yml`
- [ ] **Secrets Configured**: `GITHUB_TOKEN` secret set in repo settings (auto-provided by GitHub)

---

## 🎯 Quick Start: First Release

Once everything is configured:

### Option 1: Tag-Based Release (Recommended)

```bash
# 1. Commit all changes
git add .
git commit -m "Configure release system for first beta"

# 2. Create and push tag
git tag v0.1.0-beta.1
git push origin main --tags

# 3. GitHub Actions will automatically:
#    - Build all 4 platform binaries
#    - Create release
#    - Upload artifacts
#    - Generate checksums
```

### Option 2: Manual Workflow Dispatch

```bash
# 1. Go to GitHub Actions tab
# 2. Select "Phoenix Release Build" workflow
# 3. Click "Run workflow"
# 4. Enter version: 0.1.0-beta.1
# 5. Click "Run workflow"
```

---

## 🔍 Verification After First Release

### 1. Check GitHub Release

```bash
# View release
gh release view v0.1.0-beta.1

# Or visit:
# https://github.com/YOUR-USERNAME/YOUR-REPO/releases/tag/v0.1.0-beta.1
```

### 2. Verify Assets

Expected assets (8 total):
- `phoenix-windows-x86_64.zip`
- `phoenix-windows-x86_64.zip.sha256`
- `phoenix-linux-x86_64.tar.gz`
- `phoenix-linux-x86_64.tar.gz.sha256`
- `phoenix-macos-x86_64.tar.gz`
- `phoenix-macos-x86_64.tar.gz.sha256`
- `phoenix-macos-aarch64.tar.gz`
- `phoenix-macos-aarch64.tar.gz.sha256`

### 3. Test Update Checker

```bash
# Build and run Phoenix
cargo run -p pagi-gateway

# In another terminal, test update endpoint
curl http://localhost:3030/api/v1/system/version

# Should return:
# {
#   "current": "0.1.0-beta.1",
#   "latest": "0.1.0-beta.1",
#   "update_available": false
# }
```

---

## 🛠️ Troubleshooting Configuration

### Issue: "Repository not found" in updater

**Cause**: `REPO_OWNER` or `REPO_NAME` incorrect

**Fix**:
```bash
# Verify your repo exists
curl -I https://api.github.com/repos/YOUR-USERNAME/YOUR-REPO

# Should return: HTTP/2 200
# If 404, check username and repo name
```

### Issue: GitHub Actions workflow not triggering

**Cause**: Workflow file not in correct location or tag format wrong

**Fix**:
```bash
# Verify workflow file location
ls -la .github/workflows/release.yml

# Verify tag format (must start with 'v')
git tag -l
# Should show: v0.1.0-beta.1 (not 0.1.0-beta.1)
```

### Issue: Build fails on specific platform

**Cause**: Platform-specific dependencies or compilation issues

**Fix**:
```bash
# Check GitHub Actions logs
gh run list --workflow=release.yml
gh run view <run-id>

# Look for platform-specific errors
# Common issues:
# - Missing system dependencies
# - Cross-compilation configuration
# - Rust toolchain version
```

---

## 📚 Additional Resources

### GitHub Actions Documentation
- [Creating Releases](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)
- [Workflow Syntax](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions)
- [Secrets](https://docs.github.com/en/actions/security-guides/encrypted-secrets)

### Rust Cross-Compilation
- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [Cross-Compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)

### Semantic Versioning
- [SemVer Specification](https://semver.org/)
- [Cargo Versioning](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)

---

## 🎓 Best Practices

### Version Numbering

```
v{MAJOR}.{MINOR}.{PATCH}-{PRERELEASE}

Examples:
- v0.1.0-beta.1    # First beta release
- v0.1.0-beta.2    # Second beta release
- v0.1.0-rc.1      # Release candidate
- v0.1.0           # First stable release
- v0.2.0           # Minor version bump (new features)
- v1.0.0           # Major version (breaking changes)
```

### Release Cadence

**Beta Phase**:
- Weekly releases for active development
- Hotfixes as needed for critical bugs
- Clear communication with beta testers

**Stable Phase**:
- Monthly minor releases (new features)
- Patch releases as needed (bug fixes)
- Major releases for breaking changes (rare)

### Changelog Management

Create `CHANGELOG.md`:
```markdown
# Changelog

## [0.1.0-beta.1] - 2026-02-10

### Added
- Initial beta release
- 8 Knowledge Base system
- Auto-update functionality
- Multi-platform support

### Security
- API key encryption
- Local-only data storage
```

---

## ✅ Configuration Complete

Once you've completed this checklist, you're ready to:

1. **Push your first tag**: `git tag v0.1.0-beta.1 && git push origin v0.1.0-beta.1`
2. **Monitor the build**: Watch GitHub Actions
3. **Verify the release**: Download and test each platform
4. **Notify beta testers**: Share the release URL

**The Phoenix is ready to rise globally.** 🔥

---

**Last Updated**: 2026-02-10  
**Version**: 1.0  
**Status**: Ready for Configuration

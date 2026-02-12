# 🎯 Phoenix Release Monitoring Guide

## Overview

This guide helps you monitor the GitHub Actions release workflow and troubleshoot common issues during the binary build process.

---

## 🚀 Triggering the Release

### Option 1: Using the Trigger Script (Recommended)

**Windows (PowerShell):**
```powershell
.\scripts\trigger-release.ps1
```

**Linux/macOS (Bash):**
```bash
chmod +x scripts/trigger-release.sh
./scripts/trigger-release.sh
```

The script will:
- Read version from `VERSION` file
- Create and push git tag `v0.1.0-beta.1`
- Trigger GitHub Actions workflow automatically

### Option 2: Manual Git Tag

```bash
git tag -a v0.1.0-beta.1 -m "Phoenix Release v0.1.0-beta.1"
git push origin v0.1.0-beta.1
```

### Option 3: GitHub UI (Manual Dispatch)

1. Go to **Actions** tab in your GitHub repo
2. Select **Phoenix Release Build** workflow
3. Click **Run workflow**
4. Enter version: `0.1.0-beta.1`
5. Click **Run workflow**

---

## 📊 Monitoring the Build

### Access the Workflow

1. Navigate to: `https://github.com/YOUR_USERNAME/YOUR_REPO/actions`
2. Click on the latest **Phoenix Release Build** run
3. You'll see 5 jobs:
   - **create-release** (Creates GitHub Release)
   - **build (Windows x86_64)**
   - **build (Linux x86_64)**
   - **build (macOS x86_64)**
   - **build (macOS ARM64)**

### Expected Timeline

| Job | Duration | Notes |
|-----|----------|-------|
| create-release | ~30 seconds | Creates the GitHub Release draft |
| Windows build | 8-12 minutes | Includes cargo cache warming |
| Linux build | 6-10 minutes | Fastest due to native toolchain |
| macOS Intel | 10-15 minutes | Cross-compilation overhead |
| macOS ARM64 | 10-15 minutes | Cross-compilation overhead |

**Total Time:** ~15-20 minutes for all platforms

---

## 🔍 What to Watch For

### ✅ Success Indicators

1. **create-release job:**
   - ✓ Release created at `https://github.com/YOUR_USERNAME/YOUR_REPO/releases/tag/v0.1.0-beta.1`
   - ✓ Release marked as "Pre-release"

2. **Each build job:**
   - ✓ Rust toolchain installed
   - ✓ Cargo cache restored (2nd run onwards)
   - ✓ `cargo build --release` completes
   - ✓ Binary copied to release directory
   - ✓ Archive created (`.zip` or `.tar.gz`)
   - ✓ SHA256 checksum generated
   - ✓ Assets uploaded to release

3. **Final Release:**
   - ✓ 8 assets total:
     - `phoenix-windows-x86_64.zip`
     - `phoenix-windows-x86_64.zip.sha256`
     - `phoenix-linux-x86_64.tar.gz`
     - `phoenix-linux-x86_64.tar.gz.sha256`
     - `phoenix-macos-x86_64.tar.gz`
     - `phoenix-macos-x86_64.tar.gz.sha256`
     - `phoenix-macos-aarch64.tar.gz`
     - `phoenix-macos-aarch64.tar.gz.sha256`

---

## ⚠️ Common Issues & Solutions

### Issue 1: `rustls` or OpenSSL Linking Errors

**Symptom:**
```
error: linking with `cc` failed
undefined reference to `SSL_*`
```

**Solution:**
The workflow uses `RUSTFLAGS: "-C target-feature=+crt-static"` to statically link dependencies. If this fails:

1. Check if `reqwest` is using `rustls-tls` feature (not `native-tls`)
2. Verify `Cargo.toml` has:
   ```toml
   [dependencies]
   reqwest = { version = "0.11", default-features = false, features = ["rustls-tls", "json"] }
   ```

### Issue 2: Missing System Dependencies (Linux)

**Symptom:**
```
error: failed to run custom build command for `openssl-sys`
```

**Solution:**
Add to workflow before build step:
```yaml
- name: Install Linux dependencies
  if: matrix.platform.os == 'ubuntu-latest'
  run: |
    sudo apt-get update
    sudo apt-get install -y pkg-config libssl-dev
```

### Issue 3: macOS Cross-Compilation Failure

**Symptom:**
```
error: linker `aarch64-apple-darwin` not found
```

**Solution:**
The workflow uses `macos-latest` which supports both targets. Ensure:
```yaml
- name: Setup Rust
  uses: dtolnay/rust-toolchain@stable
  with:
    targets: ${{ matrix.platform.target }}
```

### Issue 4: Binary Not Executable (Unix)

**Symptom:**
Users report "Permission denied" when running binary.

**Solution:**
The workflow includes:
```yaml
chmod +x release/phoenix-${{ needs.create-release.outputs.version }}/pagi-gateway
chmod +x release/phoenix-${{ needs.create-release.outputs.version }}/*.sh
```

Verify this step completes successfully.

### Issue 5: Large Binary Size

**Symptom:**
Binary is >100MB, causing slow downloads.

**Solution:**
Add to workflow:
```yaml
- name: Strip binary (Unix)
  if: matrix.platform.os != 'windows-latest'
  run: |
    strip target/${{ matrix.platform.target }}/release/pagi-gateway
```

### Issue 6: Missing Files in Archive

**Symptom:**
`QUICKSTART.md` or startup scripts missing from release archive.

**Solution:**
Verify the "Copy essential files" step includes all required files:
```yaml
- name: Copy essential files
  run: |
    cp VERSION release/phoenix-${{ needs.create-release.outputs.version }}/
    cp README.md release/phoenix-${{ needs.create-release.outputs.version }}/
    cp .env.example release/phoenix-${{ needs.create-release.outputs.version }}/
    cp QUICKSTART.md release/phoenix-${{ needs.create-release.outputs.version }}/
    cp ONBOARDING_GUIDE.md release/phoenix-${{ needs.create-release.outputs.version }}/
```

---

## 🧪 Post-Release Verification

### 1. Download and Extract

**Windows:**
```powershell
Expand-Archive phoenix-windows-x86_64.zip -DestinationPath test-release
cd test-release/phoenix-0.1.0-beta.1
```

**Linux/macOS:**
```bash
tar -xzf phoenix-linux-x86_64.tar.gz
cd phoenix-0.1.0-beta.1
```

### 2. Verify Contents

Check for:
- [ ] `pagi-gateway` (or `pagi-gateway.exe` on Windows)
- [ ] `VERSION`
- [ ] `README.md`
- [ ] `.env.example`
- [ ] `QUICKSTART.md`
- [ ] `ONBOARDING_GUIDE.md`
- [ ] `BETA_README.md`
- [ ] `BETA_TESTER_ONBOARDING_GUIDE.md`
- [ ] `phoenix-rise.sh` / `phoenix-rise.ps1`
- [ ] `pagi-up.sh` / `pagi-up.ps1`

### 3. Verify SHA256 Checksum

**Windows:**
```powershell
$hash = Get-FileHash phoenix-windows-x86_64.zip -Algorithm SHA256
$expected = Get-Content phoenix-windows-x86_64.zip.sha256
Write-Host "Match: $($hash.Hash -eq $expected.Split()[0])"
```

**Linux/macOS:**
```bash
shasum -a 256 -c phoenix-linux-x86_64.tar.gz.sha256
```

### 4. Test First-Run Experience

```bash
# Should trigger first-run setup
./phoenix-rise.sh
```

Expected behavior:
1. Detects missing `user_config.toml`
2. Prompts for OpenRouter API key
3. Creates config file
4. Starts Qdrant sidecar
5. Launches gateway

---

## 📈 Success Metrics

### Build Health
- ✅ All 4 platform builds complete successfully
- ✅ Total build time < 20 minutes
- ✅ No warnings about deprecated dependencies
- ✅ Binary sizes reasonable (<50MB stripped)

### Release Quality
- ✅ All 8 assets present in GitHub Release
- ✅ SHA256 checksums match
- ✅ Archives extract without errors
- ✅ All documentation files included

### User Experience
- ✅ First-run setup works on fresh system
- ✅ Binary runs without missing dependencies
- ✅ Startup scripts execute correctly
- ✅ Qdrant sidecar launches successfully

---

## 🔥 Emergency Rollback

If a release has critical issues:

### 1. Delete the Release
```bash
# Delete remote tag
git push origin :refs/tags/v0.1.0-beta.1

# Delete local tag
git tag -d v0.1.0-beta.1
```

### 2. Delete GitHub Release
1. Go to Releases page
2. Click on the problematic release
3. Click "Delete release"
4. Confirm deletion

### 3. Fix Issues and Re-release
1. Fix the code/workflow
2. Commit changes
3. Re-run trigger script with same or new version

---

## 📞 Getting Help

### Check Workflow Logs
1. Click on failed job
2. Expand failed step
3. Look for error messages (usually at the end)

### Common Log Locations
- **Cargo build errors:** Look in "Build release binary" step
- **File copy errors:** Look in "Copy essential files" step
- **Archive errors:** Look in "Create archive" step
- **Upload errors:** Look in "Upload Release Asset" step

### Debug Locally
Test the build locally before pushing:
```bash
# Test Windows build (on Windows)
cargo build --release --target x86_64-pc-windows-msvc -p pagi-gateway --features vector

# Test Linux build (on Linux)
cargo build --release --target x86_64-unknown-linux-gnu -p pagi-gateway --features vector

# Test macOS build (on macOS)
cargo build --release --target x86_64-apple-darwin -p pagi-gateway --features vector
cargo build --release --target aarch64-apple-darwin -p pagi-gateway --features vector
```

---

## 🎯 Next Steps After Successful Release

1. **Announce to Beta Testers:**
   - Share release URL
   - Provide platform-specific download links
   - Include SHA256 checksums for verification

2. **Monitor Feedback:**
   - Watch for installation issues
   - Track first-run experience reports
   - Collect performance metrics

3. **Prepare Hotfix Process:**
   - Keep `main` branch stable
   - Use `hotfix/` branches for urgent fixes
   - Increment patch version (e.g., `0.1.1-beta.1`)

---

## 📚 Additional Resources

- **GitHub Actions Docs:** https://docs.github.com/en/actions
- **Rust Cross-Compilation:** https://rust-lang.github.io/rustup/cross-compilation.html
- **Cargo Release Best Practices:** https://doc.rust-lang.org/cargo/reference/publishing.html

---

**🔥 Phoenix is ready to rise. Monitor, verify, and deploy with confidence.**

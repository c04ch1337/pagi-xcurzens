# ══════════════════════════════════════════════════════════════════════════════
# Phoenix Beta Deployment Script (PowerShell)
# ══════════════════════════════════════════════════════════════════════════════
# Purpose: Build release binaries and prepare for beta distribution
# Usage: .\scripts\deploy_beta.ps1
# ══════════════════════════════════════════════════════════════════════════════

$ErrorActionPreference = "Stop"

Write-Host "🚀 Phoenix Beta Deployment Pipeline" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════" -ForegroundColor Cyan

# Read version from VERSION file
if (-not (Test-Path "VERSION")) {
    Write-Host "❌ VERSION file not found!" -ForegroundColor Red
    exit 1
}

$VERSION = (Get-Content "VERSION").Trim()
Write-Host "Version: $VERSION" -ForegroundColor Blue
Write-Host ""

# Step 1: Sanitize
Write-Host "Step 1: Sanitizing repository..." -ForegroundColor Yellow
if (Test-Path "scripts\sanitize_for_release.ps1") {
    & ".\scripts\sanitize_for_release.ps1"
} else {
    Write-Host "❌ Sanitization script not found!" -ForegroundColor Red
    exit 1
}
Write-Host ""

# Step 2: Run tests
Write-Host "Step 2: Running tests..." -ForegroundColor Yellow
cargo test --workspace --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Tests failed! Fix issues before deploying." -ForegroundColor Red
    exit 1
}
Write-Host "✓ All tests passed" -ForegroundColor Green
Write-Host ""

# Step 3: Build release binaries
Write-Host "Step 3: Building release binaries..." -ForegroundColor Yellow
Write-Host "This may take several minutes..."

# Build the main gateway
cargo build --release -p pagi-gateway
if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}

# Build additional components
cargo build --release -p pagi-daemon -ErrorAction SilentlyContinue
cargo build --release -p pagi-studio-ui -ErrorAction SilentlyContinue

Write-Host "✓ Build complete" -ForegroundColor Green
Write-Host ""

# Step 4: Create release directory
Write-Host "Step 4: Preparing release package..." -ForegroundColor Yellow
$RELEASE_DIR = "releases\phoenix-$VERSION"
New-Item -ItemType Directory -Force -Path $RELEASE_DIR | Out-Null

# Copy binaries
Write-Host "Copying binaries..."
Copy-Item "target\release\pagi-gateway.exe" "$RELEASE_DIR\" -ErrorAction SilentlyContinue
Copy-Item "target\release\pagi-daemon.exe" "$RELEASE_DIR\" -ErrorAction SilentlyContinue

# Copy essential files
Write-Host "Copying documentation and configuration..."
Copy-Item "VERSION" "$RELEASE_DIR\"
Copy-Item "README.md" "$RELEASE_DIR\"
Copy-Item ".env.example" "$RELEASE_DIR\"
Copy-Item "scripts" "$RELEASE_DIR\" -Recurse -ErrorAction SilentlyContinue

# Copy startup scripts
Copy-Item "phoenix-rise.ps1" "$RELEASE_DIR\" -ErrorAction SilentlyContinue
Copy-Item "phoenix-rise.sh" "$RELEASE_DIR\" -ErrorAction SilentlyContinue
Copy-Item "pagi-up.ps1" "$RELEASE_DIR\" -ErrorAction SilentlyContinue
Copy-Item "pagi-up.sh" "$RELEASE_DIR\" -ErrorAction SilentlyContinue

# Create README for beta users
$BETA_README = @"
# Phoenix Beta Release

Welcome to the Phoenix Beta! This is your personal AI companion with sovereign intelligence.

## 🚀 Quick Start

### First Time Setup

1. **Configure your API key:**
   ``````powershell
   # Copy the example environment file
   Copy-Item .env.example .env
   
   # Edit .env and add your OpenRouter API key
   # Get one at: https://openrouter.ai/keys
   ``````

2. **Start Phoenix:**
   ``````powershell
   # On Windows:
   .\phoenix-rise.ps1
   
   # On Linux/Mac:
   ./phoenix-rise.sh
   ``````

3. **Access the UI:**
   Open your browser to ``http://localhost:3001``

### What Happens on First Run?

- Phoenix will initialize empty knowledge bases (KB-01 through KB-08)
- You'll be prompted to provide your OpenRouter API key via the UI
- Your personal data stays on YOUR machine - nothing is sent to external servers
- As you interact, Phoenix learns about YOU and builds your personal knowledge graph

## 📚 Documentation

- ``README.md`` - Full project documentation
- ``.env.example`` - Configuration options
- ``VERSION`` - Current release version

## 🔐 Privacy & Security

- All your data is stored locally in the ``storage/`` and ``vector_db/`` directories
- Your API keys are stored in ``user_config.toml`` (never committed to git)
- Phoenix never sends your personal data to external services
- Only LLM API calls go through OpenRouter (using YOUR key)

## 🆘 Support

For issues, questions, or feedback, please open an issue on GitHub.

## 📝 License

See LICENSE file in the repository.
"@

Set-Content -Path "$RELEASE_DIR\BETA_README.md" -Value $BETA_README

Write-Host "✓ Release package created: $RELEASE_DIR" -ForegroundColor Green
Write-Host ""

# Step 5: Create archive
Write-Host "Step 5: Creating release archive..." -ForegroundColor Yellow
Compress-Archive -Path $RELEASE_DIR -DestinationPath "releases\phoenix-$VERSION.zip" -Force
Write-Host "✓ Archive created" -ForegroundColor Green
Write-Host ""

# Step 6: Generate checksums
Write-Host "Step 6: Generating checksums..." -ForegroundColor Yellow
$hash = Get-FileHash "releases\phoenix-$VERSION.zip" -Algorithm SHA256
Set-Content -Path "releases\phoenix-$VERSION.zip.sha256" -Value "$($hash.Hash)  phoenix-$VERSION.zip"
Write-Host "✓ Checksums generated" -ForegroundColor Green
Write-Host ""

# Summary
Write-Host "════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "✨ Beta Deployment Complete!" -ForegroundColor Green
Write-Host ""
Write-Host "Release artifacts:"
Write-Host "  📦 releases\phoenix-$VERSION\"
Write-Host "  📦 releases\phoenix-$VERSION.zip"
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Test the release package locally"
Write-Host "  2. Create a GitHub Release with tag v$VERSION"
Write-Host "  3. Upload the archives to the release"
Write-Host "  4. Share with beta testers!"
Write-Host ""
Write-Host "GitHub Release Command:" -ForegroundColor Cyan
Write-Host "  gh release create v$VERSION releases\phoenix-$VERSION.zip --title `"Phoenix v$VERSION`" --notes `"Beta release`"" -ForegroundColor Cyan
Write-Host "════════════════════════════════════════════════════════" -ForegroundColor Cyan

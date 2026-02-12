# Phoenix Release Trigger Script (PowerShell)
# This script creates and pushes a git tag to trigger the GitHub Actions release workflow

param(
    [string]$Version = ""
)

# Read version from VERSION file if not provided
if ([string]::IsNullOrEmpty($Version)) {
    if (Test-Path "VERSION") {
        $Version = Get-Content "VERSION" -Raw
        $Version = $Version.Trim()
        Write-Host "📦 Using version from VERSION file: $Version" -ForegroundColor Cyan
    } else {
        Write-Host "❌ VERSION file not found and no version specified" -ForegroundColor Red
        Write-Host "Usage: .\trigger-release.ps1 [version]" -ForegroundColor Yellow
        exit 1
    }
}

# Validate version format
if ($Version -notmatch '^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$') {
    Write-Host "❌ Invalid version format: $Version" -ForegroundColor Red
    Write-Host "Expected format: X.Y.Z or X.Y.Z-beta.N" -ForegroundColor Yellow
    exit 1
}

$TagName = "v$Version"

Write-Host ""
Write-Host "🚀 Phoenix Release Trigger" -ForegroundColor Magenta
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  Version: $Version" -ForegroundColor White
Write-Host "  Tag:     $TagName" -ForegroundColor White
Write-Host ""

# Check if tag already exists
$ExistingTag = git tag -l $TagName 2>$null
if ($ExistingTag) {
    Write-Host "⚠️  Tag $TagName already exists locally" -ForegroundColor Yellow
    $Response = Read-Host "Delete and recreate? (y/N)"
    if ($Response -ne 'y' -and $Response -ne 'Y') {
        Write-Host "❌ Aborted" -ForegroundColor Red
        exit 1
    }
    git tag -d $TagName
    Write-Host "✓ Deleted local tag" -ForegroundColor Green
}

# Check git status
$Status = git status --porcelain
if ($Status) {
    Write-Host "⚠️  You have uncommitted changes:" -ForegroundColor Yellow
    Write-Host $Status -ForegroundColor DarkGray
    Write-Host ""
    $Response = Read-Host "Continue anyway? (y/N)"
    if ($Response -ne 'y' -and $Response -ne 'Y') {
        Write-Host "❌ Aborted" -ForegroundColor Red
        exit 1
    }
}

Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host "⚡ This will:" -ForegroundColor Cyan
Write-Host "   1. Create git tag: $TagName" -ForegroundColor White
Write-Host "   2. Push tag to origin" -ForegroundColor White
Write-Host "   3. Trigger GitHub Actions release workflow" -ForegroundColor White
Write-Host "   4. Build binaries for 4 platforms (Windows, Linux, macOS x2)" -ForegroundColor White
Write-Host "   5. Create GitHub Release with artifacts" -ForegroundColor White
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host ""

$Response = Read-Host "Proceed with release? (y/N)"
if ($Response -ne 'y' -and $Response -ne 'Y') {
    Write-Host "❌ Aborted" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "🏷️  Creating tag..." -ForegroundColor Cyan
git tag -a $TagName -m "Phoenix Release v$Version"

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Failed to create tag" -ForegroundColor Red
    exit 1
}

Write-Host "✓ Tag created" -ForegroundColor Green

Write-Host ""
Write-Host "📤 Pushing tag to origin..." -ForegroundColor Cyan
git push origin $TagName

if ($LASTEXITCODE -ne 0) {
    Write-Host "❌ Failed to push tag" -ForegroundColor Red
    Write-Host "   You may need to delete the remote tag first:" -ForegroundColor Yellow
    Write-Host "   git push origin :refs/tags/$TagName" -ForegroundColor DarkGray
    exit 1
}

Write-Host "✓ Tag pushed" -ForegroundColor Green
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host "🔥 Phoenix Release Triggered!" -ForegroundColor Magenta
Write-Host ""
Write-Host "Monitor the build at:" -ForegroundColor Cyan
Write-Host "https://github.com/YOUR_USERNAME/YOUR_REPO/actions" -ForegroundColor Blue
Write-Host ""
Write-Host "The workflow will:" -ForegroundColor White
Write-Host "  • Build for 4 platforms (~10-15 minutes)" -ForegroundColor DarkGray
Write-Host "  • Generate SHA256 checksums" -ForegroundColor DarkGray
Write-Host "  • Create GitHub Release with artifacts" -ForegroundColor DarkGray
Write-Host "  • Bundle QUICKSTART.md and ONBOARDING_GUIDE.md" -ForegroundColor DarkGray
Write-Host ""
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray

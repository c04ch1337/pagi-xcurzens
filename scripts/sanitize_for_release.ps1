# ==============================================================================
# Phoenix Release Sanitization Script (PowerShell)
# ==============================================================================
# Purpose: Remove all personal "biological" data before beta distribution
# Usage: .\scripts\sanitize_for_release.ps1
# ==============================================================================

$ErrorActionPreference = "Continue"

Write-Host "Phoenix Release Sanitization - Removing Personal Data" -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

# Function to safely remove directory
function Remove-DirectorySafely {
    param([string]$Path)
    
    if (Test-Path $Path) {
        Write-Host "Removing: $Path" -ForegroundColor Yellow
        Remove-Item -Path $Path -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "Removed" -ForegroundColor Green
    } else {
        Write-Host "Already clean: $Path" -ForegroundColor Green
    }
}

# Function to safely remove file
function Remove-FileSafely {
    param([string]$Path)
    
    if (Test-Path $Path) {
        Write-Host "Removing: $Path" -ForegroundColor Yellow
        Remove-Item -Path $Path -Force -ErrorAction SilentlyContinue
        Write-Host "Removed" -ForegroundColor Green
    } else {
        Write-Host "Already clean: $Path" -ForegroundColor Green
    }
}

Write-Host ""
Write-Host "Removing Vector Databases (KB-01 through KB-08)..." -ForegroundColor Cyan
Remove-DirectorySafely "storage"
Remove-DirectorySafely "vector_db"
Get-ChildItem -Path . -Filter "*.sled" -Recurse -Directory -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Removing Gateway Runtime Data..." -ForegroundColor Cyan
Remove-DirectorySafely "data"
Remove-DirectorySafely "pagi-gateway\data"
Remove-DirectorySafely "add-ons\pagi-gateway\data"
Remove-DirectorySafely "add-ons\pagi-studio-ui\data"

Write-Host ""
Write-Host "Removing Environment Files..." -ForegroundColor Cyan
Remove-FileSafely ".env"
Remove-FileSafely ".env.local"
Get-ChildItem -Path . -Filter ".env.*.local" -Recurse -File -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "Removing User Configuration..." -ForegroundColor Cyan
Remove-FileSafely "user_config.toml"
Remove-FileSafely "config\user_config.toml"

Write-Host ""
Write-Host "Removing Build Artifacts..." -ForegroundColor Cyan
Remove-DirectorySafely "target"

Write-Host ""
Write-Host "Removing Logs..." -ForegroundColor Cyan
Get-ChildItem -Path . -Filter "*.log" -Recurse -File -ErrorAction SilentlyContinue | Remove-Item -Force -ErrorAction SilentlyContinue
Remove-DirectorySafely "logs"

Write-Host ""
Write-Host "Removing Qdrant Binary (users will download their own)..." -ForegroundColor Cyan
Remove-DirectorySafely "qdrant"
Remove-FileSafely "qdrant.zip"

Write-Host ""
Write-Host "Removing Research Sandbox Content..." -ForegroundColor Cyan
if (Test-Path "research_sandbox") {
    # Keep the directory structure but remove user content
    Get-ChildItem -Path "research_sandbox" -Recurse -File -ErrorAction SilentlyContinue | 
        Where-Object { $_.Name -ne "README.md" -and $_.Name -ne ".gitkeep" } | 
        Remove-Item -Force -ErrorAction SilentlyContinue
    Write-Host "Cleaned research_sandbox" -ForegroundColor Green
}

Write-Host ""
Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "Sanitization Complete!" -ForegroundColor Green
Write-Host ""
Write-Host "The repository is now ready for beta distribution."
Write-Host "All personal data has been removed while preserving the genetic code."
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. Review changes: git status"
Write-Host "  2. Test clean build: cargo build --release"
Write-Host "  3. Run deploy script: .\scripts\deploy_beta.ps1"
Write-Host "========================================================" -ForegroundColor Cyan

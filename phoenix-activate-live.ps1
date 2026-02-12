#!/usr/bin/env pwsh
# Phoenix LIVE Mode Quick Activation
# Automatically configures .env for LIVE mode and restarts the gateway

$ErrorActionPreference = "Stop"

Write-Host "🔥 PHOENIX LIVE MODE ACTIVATION 🔥" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Check if .env exists
if (-not (Test-Path ".env")) {
    Write-Host "[1/4] Creating .env from .env.example..." -ForegroundColor Yellow
    Copy-Item ".env.example" ".env"
    Write-Host "✅ .env file created" -ForegroundColor Green
} else {
    Write-Host "[1/4] .env file already exists" -ForegroundColor Green
}

Write-Host ""

# Step 2: Check current LLM mode
Write-Host "[2/4] Checking current LLM mode..." -ForegroundColor Yellow
$currentMode = Select-String -Path ".env" -Pattern "^PAGI_LLM_MODE=" | ForEach-Object { $_.Line }
if ($currentMode -match "PAGI_LLM_MODE=live") {
    Write-Host "✅ Already set to LIVE mode" -ForegroundColor Green
} else {
    Write-Host "⚠️  Currently in MOCK mode: $currentMode" -ForegroundColor Yellow
    Write-Host "   Updating to LIVE mode..." -ForegroundColor Yellow
    
    # Update the file
    $content = Get-Content ".env" -Raw
    $content = $content -replace "PAGI_LLM_MODE=mock", "PAGI_LLM_MODE=live"
    Set-Content ".env" -Value $content -NoNewline
    
    Write-Host "✅ Updated to LIVE mode" -ForegroundColor Green
}

Write-Host ""

# Step 3: Check API key
Write-Host "[3/4] Checking OpenRouter API key..." -ForegroundColor Yellow
$apiKey = Select-String -Path ".env" -Pattern "^PAGI_LLM_API_KEY=" | ForEach-Object { $_.Line }
$openrouterKey = Select-String -Path ".env" -Pattern "^OPENROUTER_API_KEY=" | ForEach-Object { $_.Line }

$hasKey = $false
if ($apiKey -match "PAGI_LLM_API_KEY=sk-or-v1-" -or $openrouterKey -match "OPENROUTER_API_KEY=sk-or-v1-") {
    Write-Host "✅ API key is configured" -ForegroundColor Green
    $hasKey = $true
} else {
    Write-Host "❌ No API key found" -ForegroundColor Red
    Write-Host ""
    Write-Host "You need an OpenRouter API key to use LIVE mode." -ForegroundColor Yellow
    Write-Host "Get one at: https://openrouter.ai/keys" -ForegroundColor Cyan
    Write-Host ""
    
    $key = Read-Host "Enter your OpenRouter API key (or press Enter to skip)"
    
    if ($key -and $key.StartsWith("sk-or-v1-")) {
        Write-Host "   Adding API key to .env..." -ForegroundColor Yellow
        $content = Get-Content ".env" -Raw
        $content = $content -replace "PAGI_LLM_API_KEY=", "PAGI_LLM_API_KEY=$key"
        $content = $content -replace "OPENROUTER_API_KEY=", "OPENROUTER_API_KEY=$key"
        Set-Content ".env" -Value $content -NoNewline
        Write-Host "✅ API key added" -ForegroundColor Green
        $hasKey = $true
    } elseif ($key) {
        Write-Host "⚠️  Invalid key format (should start with sk-or-v1-)" -ForegroundColor Yellow
        Write-Host "   You can add it manually to .env later" -ForegroundColor Gray
    } else {
        Write-Host "⚠️  Skipped - you'll need to add it manually to .env" -ForegroundColor Yellow
    }
}

Write-Host ""

# Step 4: Restart gateway
Write-Host "[4/4] Restarting gateway..." -ForegroundColor Yellow

if (-not $hasKey) {
    Write-Host "⚠️  Cannot start in LIVE mode without API key" -ForegroundColor Yellow
    Write-Host "   Add your key to .env and run: .\phoenix-rise.ps1" -ForegroundColor Gray
    exit 0
}

# Kill existing gateway process
$processes = Get-Process | Where-Object {$_.Path -like "*pagi-gateway*"}
if ($processes) {
    Write-Host "   Stopping existing gateway..." -ForegroundColor Gray
    $processes | Stop-Process -Force
    Start-Sleep -Seconds 2
}

# Start new gateway
Write-Host "   Starting gateway in LIVE mode..." -ForegroundColor Gray
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PWD'; cargo run -p pagi-gateway" -WindowStyle Normal

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "✅ LIVE MODE ACTIVATED" -ForegroundColor Green
Write-Host ""
Write-Host "Gateway is starting in a new window..." -ForegroundColor Yellow
Write-Host "Wait 10-15 seconds for it to fully initialize." -ForegroundColor Yellow
Write-Host ""
Write-Host "Then test with:" -ForegroundColor Cyan
Write-Host "  .\phoenix-live-sync.ps1" -ForegroundColor White
Write-Host ""

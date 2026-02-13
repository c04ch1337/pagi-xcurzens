#!/usr/bin/env pwsh
# Phoenix LIVE Mode Force-Sync
# Forces Phoenix to bypass mock/generic responses and engage the Sovereign Stack

$ErrorActionPreference = "Stop"

Write-Host "[PHOENIX] LIVE MODE FORCE-SYNC" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# Step 1: Connection Test
Write-Host "[1/5] Testing Gateway Connection..." -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "http://localhost:8000/api/v1/health" -Method Get
    Write-Host "[OK] Gateway LIVE: $($health.identity)" -ForegroundColor Green
    Write-Host "   Message: $($health.message)" -ForegroundColor Gray
} catch {
    Write-Host "[X] FAILED: Gateway not responding on port 8000" -ForegroundColor Red
    Write-Host "   Run: .\phoenix-rise.ps1" -ForegroundColor Yellow
    exit 1
}

Write-Host ""

# Step 2: Check Forge Safety Status
Write-Host "[2/5] Checking Forge Safety Governor..." -ForegroundColor Yellow
try {
    $safety = Invoke-RestMethod -Uri "http://localhost:8000/api/v1/forge/safety-status" -Method Get -ErrorAction SilentlyContinue
    if ($safety) {
        Write-Host "[OK] Forge Status: $($safety.mode)" -ForegroundColor Green
        if ($safety.safety_enabled -eq $true) {
            Write-Host "   Safety: ENABLED (HITL Mode)" -ForegroundColor Cyan
        } else {
            Write-Host "   Safety: DISABLED (Autonomous)" -ForegroundColor Magenta
        }
    } else {
        Write-Host "[!] Forge endpoint returned empty (skill may not be registered)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "[!] Forge Safety endpoint not available" -ForegroundColor Yellow
}

Write-Host ""

# Step 3: Check KB-08 (Soma) for last interaction
Write-Host "[3/5] Accessing KB-08 (Soma) for context..." -ForegroundColor Yellow
try {
    $kb_status = Invoke-RestMethod -Uri "http://localhost:8000/api/v1/kb-status" -Method Get
    Write-Host "[OK] Knowledge Base Status:" -ForegroundColor Green
    foreach ($kb in $kb_status.PSObject.Properties) {
        $name = $kb.Name
        $count = $kb.Value
        Write-Host "   $name : $count entries" -ForegroundColor Gray
    }
} catch {
    Write-Host "[!] Could not retrieve KB status" -ForegroundColor Yellow
}

Write-Host ""

# Step 4: Test Chat Endpoint with Sovereign Context
Write-Host "[4/5] Testing Chat Endpoint with Sovereign Context..." -ForegroundColor Yellow

$chatPayload = @{
    prompt = "Phoenix, confirm you are operating in LIVE mode with full access to the Sovereign Stack. What was our last conversation topic?"
    user_alias = "Coach The Creator"
    agent_id = "phoenix"
} | ConvertTo-Json -Depth 10

try {
    $response = Invoke-RestMethod -Uri "http://localhost:8000/api/v1/chat" `
        -Method Post `
        -ContentType "application/json" `
        -Body $chatPayload
    
    Write-Host "[OK] Chat Response Received:" -ForegroundColor Green
    Write-Host ""
    Write-Host ("=" * 50) -ForegroundColor Cyan
    Write-Host $response.response -ForegroundColor White
    Write-Host ("=" * 50) -ForegroundColor Cyan
    Write-Host ""
    
    # Check if response contains generic/mock indicators
    $genericIndicators = @(
        "Thank you for reaching out",
        "I'm here to help",
        "How can I assist",
        "I don't have access to",
        "I cannot access"
    )
    
    $isMock = $false
    foreach ($indicator in $genericIndicators) {
        if ($response.response -like "*$indicator*") {
            $isMock = $true
            break
        }
    }
    
    if ($isMock) {
        Write-Host "[!] WARNING: Response contains generic/mock patterns" -ForegroundColor Yellow
        Write-Host "   Phoenix is in MOCK MODE - not using real AI inference" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "TO FIX:" -ForegroundColor Cyan
        Write-Host "   1. Edit .env file: Set PAGI_LLM_MODE=live" -ForegroundColor White
        Write-Host "   2. Add your OpenRouter API key: PAGI_LLM_API_KEY=sk-or-v1-..." -ForegroundColor White
        Write-Host "   3. Restart gateway: .\phoenix-rise.ps1" -ForegroundColor White
        Write-Host ""
        Write-Host "   Full guide: PHOENIX_LIVE_MODE_ACTIVATION.md" -ForegroundColor Gray
    } else {
        Write-Host "[OK] Response appears to be from LIVE Sovereign Stack" -ForegroundColor Green
    }
    
} catch {
    Write-Host "[X] Chat endpoint failed: $($_.Exception.Message)" -ForegroundColor Red
}

Write-Host ""

# Step 5: Verify Skill Registry
Write-Host "[5/5] Checking Skill Registry..." -ForegroundColor Yellow
try {
    $skills = Invoke-RestMethod -Uri "http://localhost:8000/api/v1/skills" -Method Get
    Write-Host "[OK] Registered Skills:" -ForegroundColor Green
    foreach ($skill in $skills) {
        Write-Host "   - $($skill.name)" -ForegroundColor Gray
    }
} catch {
    Write-Host "[!] Could not retrieve skill registry" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "[PHOENIX] LIVE MODE SYNC COMPLETE" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next Steps:" -ForegroundColor Yellow
Write-Host "1. If Phoenix is still using mock responses, check the frontend connection" -ForegroundColor Gray
Write-Host "2. Verify the UI is pointing to http://localhost:8000" -ForegroundColor Gray
Write-Host "3. Check browser console for connection errors" -ForegroundColor Gray
Write-Host "4. Try: .\forge-kill-switch.ps1 status" -ForegroundColor Gray
Write-Host ""

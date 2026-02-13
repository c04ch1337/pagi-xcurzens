# =============================================================================
# FORGE KILL SWITCH - Emergency Shutdown for Autonomous Evolution
# =============================================================================
# This script immediately:
# 1. Sets PAGI_FORGE_SAFETY_ENABLED=true in .env
# 2. Kills all active cargo build processes
# 3. Logs the emergency shutdown to KB-08
#
# Usage:
#   .\forge-kill-switch.ps1
#
# Or create a desktop shortcut for one-click emergency stop.
# =============================================================================

Write-Host "[ALERT] FORGE KILL SWITCH ACTIVATED" -ForegroundColor Red
Write-Host "===============================================================" -ForegroundColor Red

# Step 1: Update .env to re-enable safety
Write-Host ""
Write-Host "[1/3] Re-enabling Forge Safety Gate..." -ForegroundColor Yellow

$envPath = ".\.env"
if (Test-Path $envPath) {
    $envContent = Get-Content $envPath -Raw
    
    # Check if PAGI_FORGE_SAFETY_ENABLED exists
    if ($envContent -match "PAGI_FORGE_SAFETY_ENABLED\s*=\s*false") {
        # Replace false with true
        $envContent = $envContent -replace "PAGI_FORGE_SAFETY_ENABLED\s*=\s*false", "PAGI_FORGE_SAFETY_ENABLED=true"
        Set-Content -Path $envPath -Value $envContent -NoNewline
        Write-Host "[OK] PAGI_FORGE_SAFETY_ENABLED set to true in .env" -ForegroundColor Green
    }
    elseif ($envContent -match "PAGI_FORGE_SAFETY_ENABLED\s*=\s*true") {
        Write-Host "[OK] PAGI_FORGE_SAFETY_ENABLED already set to true" -ForegroundColor Green
    }
    else {
        # Add the setting if it doesn't exist
        Add-Content -Path $envPath -Value "`nPAGI_FORGE_SAFETY_ENABLED=true"
        Write-Host "[OK] PAGI_FORGE_SAFETY_ENABLED added to .env (set to true)" -ForegroundColor Green
    }
}
else {
    Write-Host "[!] .env file not found - creating with safety enabled" -ForegroundColor Yellow
    "PAGI_FORGE_SAFETY_ENABLED=true" | Out-File -FilePath $envPath -Encoding UTF8
}

# Step 2: Kill all cargo build processes
Write-Host ""
Write-Host "[2/3] Terminating active cargo build processes..." -ForegroundColor Yellow

$cargoProcesses = Get-Process -Name "cargo" -ErrorAction SilentlyContinue
if ($cargoProcesses) {
    $cargoProcesses | ForEach-Object {
        Stop-Process -Id $_.Id -Force
        Write-Host "[OK] Killed cargo process (PID: $($_.Id))" -ForegroundColor Green
    }
}
else {
    Write-Host "[OK] No active cargo processes found" -ForegroundColor Green
}

# Also kill rustc processes (compilation in progress)
$rustcProcesses = Get-Process -Name "rustc" -ErrorAction SilentlyContinue
if ($rustcProcesses) {
    $rustcProcesses | ForEach-Object {
        Stop-Process -Id $_.Id -Force
        Write-Host "[OK] Killed rustc process (PID: $($_.Id))" -ForegroundColor Green
    }
}

# Step 3: Log to KB-08 (if gateway is running)
Write-Host ""
Write-Host "[3/3] Logging emergency shutdown..." -ForegroundColor Yellow

try {
    $timestamp = Get-Date -Format "yyyy-MM-ddTHH:mm:ss.fffZ"
    $logEntry = @{
        event = "forge_kill_switch_activated"
        timestamp = $timestamp
        reason = "Emergency shutdown initiated by Coach The Creator"
        action = "PAGI_FORGE_SAFETY_ENABLED set to true, all cargo/rustc processes terminated"
    } | ConvertTo-Json

    # Try to POST to the gateway's KB-08 logging endpoint (if it exists)
    # This is a best-effort attempt; if the gateway is down, it will fail silently
    $null = Invoke-RestMethod -Uri "http://127.0.0.1:8000/api/v1/kb/soma/log" `
        -Method POST `
        -Body $logEntry `
        -ContentType "application/json" `
        -TimeoutSec 2 `
        -ErrorAction SilentlyContinue
    
    Write-Host "[OK] Emergency shutdown logged to KB-08" -ForegroundColor Green
}
catch {
    Write-Host "[!] Could not log to KB-08 (gateway may be offline)" -ForegroundColor Yellow
}

# Final status
Write-Host ""
Write-Host "===============================================================" -ForegroundColor Green
Write-Host "[OK] FORGE KILL SWITCH COMPLETE" -ForegroundColor Green
Write-Host ""
Write-Host "Status:" -ForegroundColor Cyan
Write-Host "  - Forge Safety Gate: ENABLED" -ForegroundColor Green
Write-Host "  - Autonomous Evolution: DISABLED" -ForegroundColor Green
Write-Host "  - Active Compilations: TERMINATED" -ForegroundColor Green
Write-Host ""
Write-Host "Phoenix will now require your approval for all code changes." -ForegroundColor White
Write-Host "Restart the gateway to apply the new safety setting." -ForegroundColor White
Write-Host ""

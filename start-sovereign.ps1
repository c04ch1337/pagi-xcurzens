# PAGI XCURZENS - Master Orchestrator
# Single entry point for the XCURZENS authority perimeter.
# Jamey: Run this to bring the full stack under the new identity.

$ProjectName = "PAGI XCURZENS"
$RootDir = $PSScriptRoot

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  $ProjectName - Sovereign Launch" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Root: $RootDir" -ForegroundColor Gray

# Step 1: Ensure Rust gateway is buildable
Write-Host ""
Write-Host "[Step 1] Verifying Rust workspace (pagi-xcurzens)..." -ForegroundColor Yellow
Set-Location $RootDir
cargo check 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "cargo check failed. Fix Rust crates before full launch." -ForegroundColor Red
    exit $LASTEXITCODE
}

# Step 7: Launch Studio UI on port 3001 (Architect View)
$StudioPath = Join-Path $RootDir "add-ons\pagi-studio-ui\assets\studio-interface"
if (Test-Path (Join-Path $StudioPath "package.json")) {
    Write-Host ""
    Write-Host "[Step 7] Launching Studio UI on port 3001..." -ForegroundColor Yellow
    Set-Location $StudioPath
    Start-Process -FilePath "npm" -ArgumentList "run", "dev", "--", "--port", "3001" -WorkingDirectory $StudioPath
    Set-Location $RootDir
    Write-Host "[Step 7] Studio UI starting at http://127.0.0.1:3001" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "[Step 7] Studio UI not found at $StudioPath; run npm install and npm run dev there." -ForegroundColor Gray
}

Write-Host ""
Write-Host "[Done] $ProjectName perimeter ready." -ForegroundColor Green
Write-Host "Gateway: 8000 | Architect View: 3001" -ForegroundColor Gray

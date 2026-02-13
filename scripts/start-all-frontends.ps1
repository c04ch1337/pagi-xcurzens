# 🚀 Start All Frontends Script
# Launches all three XCURZENS frontends in separate terminals
# Version: 1.0.0
# Date: 2026-02-13

$ErrorActionPreference = "Stop"
$ProjectRoot = "C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens"

# Color output functions
function Write-Success {
    param($Message)
    Write-Host "✅ $Message" -ForegroundColor Green
}

function Write-Info {
    param($Message)
    Write-Host "ℹ️  $Message" -ForegroundColor Cyan
}

function Write-Step {
    param($Message)
    Write-Host "`n🔧 $Message" -ForegroundColor Magenta
}

Write-Host "`n🏛️  SOVEREIGN MONOLITH - FRONTEND LAUNCHER" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan

# Verify project root exists
if (-not (Test-Path $ProjectRoot)) {
    Write-Host "❌ Project root not found: $ProjectRoot" -ForegroundColor Red
    exit 1
}

Set-Location $ProjectRoot

# Check if npm is available
try {
    $npmVersion = npm --version
    Write-Success "npm version: $npmVersion"
} catch {
    Write-Host "❌ npm not found. Please install Node.js" -ForegroundColor Red
    exit 1
}

# Frontend configurations
$frontends = @(
    @{
        Name = "XCURSENS Scout (Traveler UI)"
        Path = "frontend-xcursens"
        Port = 3001
        Color = "Cyan"
    },
    @{
        Name = "Partner Nexus"
        Path = "frontent-nexus"
        Port = 3002
        Color = "Yellow"
    },
    @{
        Name = "Command Center"
        Path = "frontend-command"
        Port = 3003
        Color = "Magenta"
    }
)

Write-Step "Checking frontend directories..."

foreach ($frontend in $frontends) {
    $path = Join-Path $ProjectRoot $frontend.Path
    
    if (Test-Path $path) {
        Write-Success "$($frontend.Name): Found at $($frontend.Path)"
        
        # Check if package.json exists
        $packageJson = Join-Path $path "package.json"
        if (-not (Test-Path $packageJson)) {
            Write-Host "⚠️  $($frontend.Name): package.json not found" -ForegroundColor Yellow
        }
        
        # Check if node_modules exists
        $nodeModules = Join-Path $path "node_modules"
        if (-not (Test-Path $nodeModules)) {
            Write-Host "⚠️  $($frontend.Name): node_modules not found. Run 'npm install' first." -ForegroundColor Yellow
        }
    } else {
        Write-Host "❌ $($frontend.Name): Directory not found at $($frontend.Path)" -ForegroundColor Red
    }
}

Write-Step "Starting frontends in separate terminals..."

foreach ($frontend in $frontends) {
    $path = Join-Path $ProjectRoot $frontend.Path
    
    if (Test-Path $path) {
        Write-Info "Launching $($frontend.Name) on port $($frontend.Port)..."
        
        # Start in new terminal window
        Start-Process powershell -ArgumentList @(
            "-NoExit",
            "-Command",
            "& {
                `$Host.UI.RawUI.WindowTitle = '$($frontend.Name) - Port $($frontend.Port)';
                Write-Host '🏛️  $($frontend.Name)' -ForegroundColor $($frontend.Color);
                Write-Host '=' * 60 -ForegroundColor $($frontend.Color);
                Write-Host 'Port: $($frontend.Port)' -ForegroundColor White;
                Write-Host 'Path: $($frontend.Path)' -ForegroundColor White;
                Write-Host '=' * 60 -ForegroundColor $($frontend.Color);
                Write-Host '';
                Set-Location '$path';
                npm run dev
            }"
        )
        
        Write-Success "$($frontend.Name) terminal launched"
        Start-Sleep -Milliseconds 500
    }
}

Write-Host "`n" + ("=" * 60) -ForegroundColor Cyan
Write-Host "🎯 ALL FRONTENDS LAUNCHED" -ForegroundColor Green
Write-Host ("=" * 60) -ForegroundColor Cyan

Write-Host "`nFrontend URLs (Development Mode):" -ForegroundColor Cyan
Write-Host "  • XCURSENS Scout:  http://localhost:3001" -ForegroundColor White
Write-Host "  • Partner Nexus:   http://localhost:3002" -ForegroundColor White
Write-Host "  • Command Center:  http://localhost:3003" -ForegroundColor White

Write-Host "`nProduction URLs (via Gateway):" -ForegroundColor Cyan
Write-Host "  • XCURSENS Scout:  http://localhost:8000/" -ForegroundColor White
Write-Host "  • Partner Nexus:   http://localhost:8000/nexus" -ForegroundColor White
Write-Host "  • Command Center:  http://localhost:8000/command" -ForegroundColor White

Write-Host "`n⚠️  Note: In production, access all frontends through port 8000" -ForegroundColor Yellow
Write-Host "Development mode (ports 3001-3003) is for hot reload only.`n" -ForegroundColor Yellow

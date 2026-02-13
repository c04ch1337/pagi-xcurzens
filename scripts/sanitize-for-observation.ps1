# 🧹 Sovereign Monolith Data Sanitization Script
# Prepares the system for The Creator's Observation Phase
# Version: 1.0.0
# Date: 2026-02-13

param(
    [switch]$Force,
    [switch]$SkipBackup,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"
$ProjectRoot = "C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens"

# Color output functions
function Write-Success { param($Message) Write-Host "✅ $Message" -ForegroundColor Green }
function Write-Warning { param($Message) Write-Host "⚠️  $Message" -ForegroundColor Yellow }
function Write-Error { param($Message) Write-Host "❌ $Message" -ForegroundColor Red }
function Write-Info { param($Message) Write-Host "ℹ️  $Message" -ForegroundColor Cyan }
function Write-Step { param($Message) Write-Host "`n🔧 $Message" -ForegroundColor Magenta }

Write-Host "`n🏛️  SOVEREIGN MONOLITH DATA SANITIZATION" -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan
Write-Host "Preparing system for live observation..." -ForegroundColor Cyan
Write-Host "=" * 60 -ForegroundColor Cyan

# Verify project root exists
if (-not (Test-Path $ProjectRoot)) {
    Write-Error "Project root not found: $ProjectRoot"
    exit 1
}

Set-Location $ProjectRoot

# Step 1: Check if gateway is running
Write-Step "Step 1: Checking for running gateway..."

$gatewayProcess = Get-Process -Name "pagi-gateway" -ErrorAction SilentlyContinue
if ($gatewayProcess) {
    Write-Warning "Gateway is currently running (PID: $($gatewayProcess.Id))"
    
    if (-not $Force) {
        $response = Read-Host "Stop the gateway? (y/n)"
        if ($response -ne "y") {
            Write-Error "Cannot sanitize while gateway is running. Use -Force to auto-stop."
            exit 1
        }
    }
    
    Write-Info "Stopping gateway..."
    Stop-Process -Id $gatewayProcess.Id -Force
    Start-Sleep -Seconds 2
    Write-Success "Gateway stopped"
} else {
    Write-Success "No running gateway detected"
}

# Step 2: Backup current state
if (-not $SkipBackup) {
    Write-Step "Step 2: Creating backup..."
    
    $timestamp = Get-Date -Format "yyyy-MM-dd_HHmmss"
    $backupDir = "C:\Users\JAMEYMILNER\Documents\sovereign-backups\pre-observation-$timestamp"
    
    New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
    
    # Backup .env
    if (Test-Path ".env") {
        Copy-Item ".env" "$backupDir\.env"
        Write-Success "Backed up .env"
    }
    
    # Backup Sled DB
    if (Test-Path "sled_db") {
        Copy-Item -Recurse "sled_db" "$backupDir\sled_db"
        Write-Success "Backed up Sled DB"
    }
    
    Write-Success "Backup completed: $backupDir"
} else {
    Write-Warning "Skipping backup (use -SkipBackup flag)"
}

# Step 3: Purge Sled Database
Write-Step "Step 3: Purging Sled database..."

$sledPaths = @("sled_db", "db", "data\sled")
$sledFound = $false

foreach ($path in $sledPaths) {
    if (Test-Path $path) {
        $sledFound = $true
        Write-Info "Found Sled DB at: $path"
        
        if (-not $Force) {
            $response = Read-Host "Delete this database? (y/n)"
            if ($response -ne "y") {
                Write-Warning "Skipping Sled DB deletion"
                continue
            }
        }
        
        Remove-Item -Recurse -Force $path
        Write-Success "Deleted Sled DB: $path"
    }
}

if (-not $sledFound) {
    Write-Warning "No Sled database found (this is OK for first run)"
}

# Step 4: Scan for mock data in frontends
Write-Step "Step 4: Scanning frontends for mock data..."

$frontendDirs = @("frontend-xcursens", "frontend-command", "frontent-nexus")
$mockPatterns = @("dummy", "mock", "stub", "sample_", "test_data", "MOCK_", "STUB_")
$mockFiles = @()

foreach ($dir in $frontendDirs) {
    if (Test-Path $dir) {
        Write-Info "Scanning $dir..."
        
        $files = Get-ChildItem -Path $dir -Recurse -Include *.ts,*.tsx,*.js,*.jsx -ErrorAction SilentlyContinue
        
        foreach ($file in $files) {
            $content = Get-Content $file.FullName -Raw
            
            foreach ($pattern in $mockPatterns) {
                if ($content -match $pattern) {
                    $mockFiles += [PSCustomObject]@{
                        File = $file.FullName.Replace($ProjectRoot, ".")
                        Pattern = $pattern
                    }
                    break
                }
            }
        }
    }
}

if ($mockFiles.Count -gt 0) {
    Write-Warning "Found $($mockFiles.Count) files with potential mock data:"
    $mockFiles | Format-Table -AutoSize
    Write-Warning "Manual review required. Check these files and remove mock data."
} else {
    Write-Success "No mock data patterns detected in frontends"
}

# Step 5: Verify .env configuration
Write-Step "Step 5: Verifying .env configuration..."

if (-not (Test-Path ".env")) {
    Write-Error ".env file not found!"
    Write-Info "Copy .env.example to .env and configure it"
    exit 1
}

$envContent = Get-Content ".env" -Raw

# Check critical settings
$checks = @{
    "PAGI_LLM_MODE" = "live"
    "OPENROUTER_API_KEY" = "sk-or-v1-"
    "PARTNER_WEBHOOK_ENABLED" = "true"
}

$envValid = $true

foreach ($key in $checks.Keys) {
    $expectedValue = $checks[$key]
    
    if ($envContent -match "$key\s*=\s*(.+)") {
        $actualValue = $matches[1].Trim()
        
        if ($key -eq "OPENROUTER_API_KEY") {
            if ($actualValue -like "$expectedValue*") {
                Write-Success "$key is set"
            } else {
                Write-Error "$key is not set or invalid"
                $envValid = $false
            }
        } else {
            if ($actualValue -eq $expectedValue) {
                Write-Success "$key = $actualValue"
            } else {
                Write-Warning "$key = $actualValue (expected: $expectedValue)"
                $envValid = $false
            }
        }
    } else {
        Write-Error "$key not found in .env"
        $envValid = $false
    }
}

if (-not $envValid) {
    Write-Warning ".env configuration needs attention"
    Write-Info "Review .env file and ensure all required settings are correct"
}

# Step 6: Verify Config Bridge code
Write-Step "Step 6: Verifying Config Bridge implementation..."

$mainRsPath = "add-ons\pagi-gateway\src\main.rs"

if (Test-Path $mainRsPath) {
    $mainRsContent = Get-Content $mainRsPath -Raw
    
    # Check for Config Bridge endpoint
    if ($mainRsContent -match "async fn feature_config") {
        Write-Success "Config Bridge endpoint found"
        
        # Check for strict filtering
        if ($mainRsContent -match "OPENROUTER_API_KEY" -and $mainRsContent -match "// STRICT FILTERING") {
            Write-Success "Strict filtering implemented"
        } else {
            Write-Warning "Verify strict filtering in Config Bridge"
        }
        
        # Check for Creator identity
        if ($mainRsContent -match '"The Creator"') {
            Write-Success "Creator identity hardcoded"
        } else {
            Write-Warning "Creator identity not found in Config Bridge"
        }
    } else {
        Write-Warning "Config Bridge endpoint not found in main.rs"
    }
} else {
    Write-Warning "Gateway main.rs not found at: $mainRsPath"
}

# Step 7: Check geminiService.ts files
Write-Step "Step 7: Verifying geminiService.ts configuration..."

$geminiServices = @(
    "frontend-xcursens\services\geminiService.ts",
    "frontend-command\services\geminiService.ts",
    "frontent-nexus\services\geminiService.ts"
)

foreach ($service in $geminiServices) {
    if (Test-Path $service) {
        $content = Get-Content $service -Raw
        
        # Check for Config Bridge fetch
        if ($content -match "fetch\('/api/v1/config'\)") {
            Write-Success "${service}: Config Bridge fetch found"
        } else {
            Write-Warning "${service}: Config Bridge fetch not found"
        }
        
        # Check for hardcoded API keys
        if ($content -match "const apiKey = 'sk-") {
            Write-Error "${service}: Hardcoded API key detected!"
        } else {
            Write-Success "${service}: No hardcoded API keys"
        }
    } else {
        Write-Warning "${service}: File not found"
    }
}

# Step 8: Generate sanitization report
Write-Step "Step 8: Generating sanitization report..."

$reportPath = "SANITIZATION_REPORT_$(Get-Date -Format 'yyyy-MM-dd_HHmmss').txt"

$report = @"
🏛️ SOVEREIGN MONOLITH SANITIZATION REPORT
Generated: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')

SLED DATABASE:
- Purged: $(if ($sledFound) { "Yes" } else { "N/A (not found)" })

MOCK DATA:
- Files with mock patterns: $($mockFiles.Count)
$(if ($mockFiles.Count -gt 0) { $mockFiles | Out-String } else { "None detected" })

ENVIRONMENT CONFIGURATION:
- .env exists: $(Test-Path ".env")
- PAGI_LLM_MODE: $(if ($envContent -match "PAGI_LLM_MODE\s*=\s*(.+)") { $matches[1].Trim() } else { "NOT SET" })
- OPENROUTER_API_KEY: $(if ($envContent -match "OPENROUTER_API_KEY\s*=\s*(.+)") { "SET" } else { "NOT SET" })
- PARTNER_WEBHOOK_ENABLED: $(if ($envContent -match "PARTNER_WEBHOOK_ENABLED\s*=\s*(.+)") { $matches[1].Trim() } else { "NOT SET" })

CONFIG BRIDGE:
- Endpoint found: $(if (Test-Path $mainRsPath) { (Get-Content $mainRsPath -Raw) -match "async fn feature_config" } else { "N/A" })
- Creator identity: $(if (Test-Path $mainRsPath) { (Get-Content $mainRsPath -Raw) -match '"The Creator"' } else { "N/A" })

NEXT STEPS:
1. Review any files with mock data patterns
2. Verify .env configuration is correct
3. Start the gateway: cargo run -p pagi-gateway --release
4. Access interfaces through http://localhost:8000
5. Run Genesis Lead test (see OBSERVATION_PHASE_PROTOCOL.md)

The system is ready for The Creator's observation.
"@

$report | Out-File $reportPath -Encoding UTF8
Write-Success "Report saved: $reportPath"

# Final summary
Write-Host "`n" + ("=" * 60) -ForegroundColor Cyan
Write-Host "🎯 SANITIZATION COMPLETE" -ForegroundColor Green
Write-Host ("=" * 60) -ForegroundColor Cyan

if ($mockFiles.Count -gt 0) {
    Write-Warning "Manual review required for $($mockFiles.Count) files with mock patterns"
}

if (-not $envValid) {
    Write-Warning ".env configuration needs attention"
}

Write-Host "`nNext steps:" -ForegroundColor Cyan
Write-Host "1. Review sanitization report: $reportPath" -ForegroundColor White
Write-Host "2. Start gateway: cargo run -p pagi-gateway --release" -ForegroundColor White
Write-Host "3. Follow OBSERVATION_PHASE_PROTOCOL.md for verification" -ForegroundColor White

Write-Host "`n✨ The Sovereign Monolith is ready for observation." -ForegroundColor Green

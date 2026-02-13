# =============================================================================
# PAGI SOVEREIGN MASTER ORCHESTRATOR
# =============================================================================
# This script provides a unified entry point for the PAGI ecosystem with:
# - Automatic execution policy fixes
# - Environment validation
# - Knowledge Base provisioning
# - Port cleanup
# - Sequential build and launch with error handling
# =============================================================================

param(
    [switch]$SkipBuild,
    [switch]$CleanStart,
    [switch]$VerifyOnly
)

$ErrorActionPreference = "Stop"
$script:HasErrors = $false

# -----------------------------------------------------------------------------
# UTILITY FUNCTIONS
# -----------------------------------------------------------------------------

function Write-SovereignHeader {
    Write-Host ""
    Write-Host "+====================================================================+" -ForegroundColor Cyan
    Write-Host "|                                                                    |" -ForegroundColor Cyan
    Write-Host "|              PAGI SOVEREIGN MASTER ORCHESTRATOR                    |" -ForegroundColor Cyan
    Write-Host "|                    Phoenix Marie v0.1.0                            |" -ForegroundColor Cyan
    Write-Host "|                                                                    |" -ForegroundColor Cyan
    Write-Host "+====================================================================+" -ForegroundColor Cyan
    Write-Host ""
}

function Write-StepHeader {
    param([string]$Step, [string]$Description)
    Write-Host ""
    Write-Host "[$Step] $Description" -ForegroundColor Yellow
    Write-Host ("-" * 70) -ForegroundColor DarkGray
}

function Write-Success {
    param([string]$Message)
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Write-SovereignWarning {
    param([string]$Message)
    Write-Host "[!] $Message" -ForegroundColor Yellow
}

function Write-SovereignError {
    param([string]$Message)
    Write-Host "[X] $Message" -ForegroundColor Red
    $script:HasErrors = $true
}

function Write-Info {
    param([string]$Message)
    Write-Host "  $Message" -ForegroundColor Gray
}

# -----------------------------------------------------------------------------
# STEP 0: EXECUTION POLICY CHECK & FIX
# -----------------------------------------------------------------------------

function Test-ExecutionPolicy {
    Write-StepHeader "0/7" "Checking PowerShell Execution Policy"
    
    $currentPolicy = Get-ExecutionPolicy -Scope CurrentUser
    Write-Info "Current policy: $currentPolicy"
    
    if ($currentPolicy -eq "Restricted" -or $currentPolicy -eq "Undefined") {
        Write-SovereignWarning "Execution policy is too restrictive for script execution"
        Write-Info "Attempting to set policy to RemoteSigned for CurrentUser..."
        
        try {
            Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force
            Write-Success "Execution policy updated to RemoteSigned"
        }
        catch {
            Write-SovereignError "Failed to update execution policy. Please run as Administrator:"
            Write-Info "  Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser"
            return $false
        }
    }
    else {
        Write-Success "Execution policy is compatible: $currentPolicy"
    }
    
    return $true
}

# -----------------------------------------------------------------------------
# STEP 1: ENVIRONMENT VALIDATION
# -----------------------------------------------------------------------------

function Test-Prerequisites {
    Write-StepHeader "1/7" "Validating System Prerequisites"
    
    $allGood = $true
    
    # Check Rust
    Write-Info "Checking for Rust toolchain..."
    $rustVersion = & cargo --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Rust found: $rustVersion"
    }
    else {
        Write-SovereignError "Rust not found. Install from: https://rustup.rs/"
        $allGood = $false
    }
    
    # Check Node.js
    Write-Info "Checking for Node.js..."
    $nodeVersion = & node --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Success "Node.js found: $nodeVersion"
    }
    else {
        Write-SovereignError "Node.js not found. Install from: https://nodejs.org/"
        $allGood = $false
    }
    
    # Check npm
    Write-Info "Checking for npm..."
    $npmVersion = & npm --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Success "npm found: v$npmVersion"
    }
    else {
        Write-SovereignError "npm not found. Install Node.js from: https://nodejs.org/"
        $allGood = $false
    }
    
    # Check working directory
    Write-Info "Verifying working directory..."
    $cargoToml = Join-Path $PSScriptRoot "Cargo.toml"
    if (Test-Path $cargoToml) {
        Write-Success "Running from repository root"
    }
    else {
        Write-SovereignError "Not running from repository root. Please run from: $PSScriptRoot"
        $allGood = $false
    }
    
    return $allGood
}

# -----------------------------------------------------------------------------
# STEP 2: ENVIRONMENT FILE CHECK
# -----------------------------------------------------------------------------

function Test-EnvironmentFile {
    Write-StepHeader "2/7" "Checking Environment Configuration"
    
    $envPath = Join-Path $PSScriptRoot ".env"
    $envExamplePath = Join-Path $PSScriptRoot ".env.example"
    
    if (Test-Path $envPath) {
        Write-Success "Environment file exists: .env"
        
        # Check for critical variables
        $envContent = Get-Content $envPath -Raw
        
        $warnings = @()
        if ($envContent -notmatch "PAGI_LLM_API_KEY=.+") {
            $warnings += "PAGI_LLM_API_KEY is not set"
        }
        if ($envContent -notmatch "PAGI_LLM_MODE=") {
            $warnings += "PAGI_LLM_MODE is not set (defaults to 'mock')"
        }
        
        if ($warnings.Count -gt 0) {
            Write-SovereignWarning "Environment configuration warnings:"
            foreach ($warning in $warnings) {
                Write-Info "  - $warning"
            }
            Write-Info "System will run in mock mode without API keys"
        }
        else {
            Write-Success "Environment configuration looks good"
        }
    }
    else {
        Write-SovereignWarning "No .env file found"
        if (Test-Path $envExamplePath) {
            Write-Info "Creating .env from .env.example..."
            Copy-Item $envExamplePath $envPath
            Write-Success "Created .env file"
            Write-SovereignWarning "Please edit .env and add your API keys before running in live mode"
        }
        else {
            Write-SovereignError ".env.example not found. Cannot create environment file."
            return $false
        }
    }
    
    return $true
}

# -----------------------------------------------------------------------------
# STEP 3: KNOWLEDGE BASE PROVISIONING
# -----------------------------------------------------------------------------

function Initialize-KnowledgeBases {
    Write-StepHeader "3/7" "Provisioning Knowledge Base Directories"
    
    $storagePath = Join-Path $PSScriptRoot "storage"
    
    # Create main storage directory
    if (-not (Test-Path $storagePath)) {
        New-Item -ItemType Directory -Path $storagePath -Force | Out-Null
        Write-Success "Created storage directory"
    }
    
    # Define the 8 Knowledge Bases
    $knowledgeBases = @(
        @{ Name = "kb-01-psyche"; Description = "Psyche (User Profile & Preferences)" },
        @{ Name = "kb-02-oikos"; Description = "Oikos (Social Graph & Relationships)" },
        @{ Name = "kb-03-techne"; Description = "Techne (Technical Knowledge & Skills)" },
        @{ Name = "kb-04-chronos"; Description = "Chronos (Temporal Memory & Events)" },
        @{ Name = "kb-05-polis"; Description = "Polis (Social Defense & Sovereignty)" },
        @{ Name = "kb-06-ethos"; Description = "Ethos (Values & Sovereign Config)" },
        @{ Name = "kb-07-mimir"; Description = "Mimir (Semantic Cache & Embeddings)" },
        @{ Name = "kb-08-soma"; Description = "Soma (System Health & Audit Log)" }
    )
    
    $created = 0
    $existing = 0
    
    foreach ($kb in $knowledgeBases) {
        $kbPath = Join-Path $storagePath $kb.Name
        if (-not (Test-Path $kbPath)) {
            New-Item -ItemType Directory -Path $kbPath -Force | Out-Null
            Write-Info "Created: $($kb.Name) - $($kb.Description)"
            $created++
        }
        else {
            $existing++
        }
    }
    
    if ($created -gt 0) {
        Write-Success "Created $created new Knowledge Base directories"
    }
    if ($existing -gt 0) {
        Write-Info "$existing Knowledge Base directories already exist"
    }
    
    Write-Success "Knowledge Base provisioning complete"
    return $true
}

# -----------------------------------------------------------------------------
# STEP 4: PORT CLEANUP
# -----------------------------------------------------------------------------

function Clear-SovereignPorts {
    Write-StepHeader "4/7" "Cleaning Sovereign Ports"
    
    # PAGI port ranges: Backend 8000-8099, Frontend 3001-3099, Qdrant 6333
    $ports = @(8000, 8002, 3001, 3002, 3003, 6333)
    $cleaned = 0
    
    foreach ($port in $ports) {
        try {
            $conn = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
            if ($conn) {
                $procId = $conn.OwningProcess | Select-Object -First 1
                if ($procId) {
                    $proc = Get-Process -Id $procId -ErrorAction SilentlyContinue
                    if ($proc) {
                        Write-Info "Cleaning port $port (PID: $procId, Process: $($proc.Name))..."
                        Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
                        Start-Sleep -Milliseconds 500
                        $cleaned++
                    }
                }
            }
        }
        catch {
            # Port not in use or already cleaned
        }
    }
    
    if ($cleaned -gt 0) {
        Write-Success "Cleaned $cleaned port(s)"
    }
    else {
        Write-Success "All ports are clear"
    }
    
    return $true
}

# -----------------------------------------------------------------------------
# STEP 5: WORKSPACE BUILD
# -----------------------------------------------------------------------------

function Build-Workspace {
    Write-StepHeader "5/7" "Building Rust Workspace"
    
    if ($SkipBuild) {
        Write-SovereignWarning "Skipping build (--SkipBuild flag set)"
        return $true
    }
    
    Write-Info "Running: cargo build --workspace --features bridge-ms,vector"
    Write-Info "This may take several minutes on first run..."
    
    Set-Location $PSScriptRoot
    
    if ($CleanStart) {
        Write-Info "Performing clean build..."
        & cargo clean
    }
    
    & cargo build --workspace --features "bridge-ms,vector"
    
    if ($LASTEXITCODE -ne 0) {
        Write-SovereignError "Workspace build failed with exit code $LASTEXITCODE"
        Write-Info "Try running: cargo clean && cargo build --workspace --features bridge-ms,vector"
        return $false
    }
    
    Write-Success "Workspace build completed successfully"
    return $true
}

# -----------------------------------------------------------------------------
# STEP 6: FRONTEND DEPENDENCIES
# -----------------------------------------------------------------------------

function Install-FrontendDependencies {
    Write-StepHeader "6/7" "Checking Frontend Dependencies"
    
    $studioInterfacePath = Join-Path $PSScriptRoot "add-ons\pagi-studio-ui\assets\studio-interface"
    
    if (-not (Test-Path $studioInterfacePath)) {
        Write-SovereignWarning "Studio interface directory not found, skipping npm install"
        return $true
    }
    
    $nodeModulesPath = Join-Path $studioInterfacePath "node_modules"
    
    if (Test-Path $nodeModulesPath) {
        Write-Success "Frontend dependencies already installed"
        return $true
    }
    
    Write-Info "Installing frontend dependencies..."
    Push-Location $studioInterfacePath
    
    & npm install
    $npmResult = $LASTEXITCODE
    
    Pop-Location
    
    if ($npmResult -ne 0) {
        Write-SovereignError "npm install failed with exit code $npmResult"
        return $false
    }
    
    Write-Success "Frontend dependencies installed"
    return $true
}

# -----------------------------------------------------------------------------
# STEP 7: LAUNCH SOVEREIGN ECOSYSTEM
# -----------------------------------------------------------------------------

function Start-SovereignEcosystem {
    Write-StepHeader "7/7" "Launching Sovereign Ecosystem"
    
    if ($VerifyOnly) {
        Write-Success "Verification complete. Skipping launch (--VerifyOnly flag set)"
        return $true
    }
    
    Write-Info "Starting components in sequence..."
    Write-Info ""
    
    # Component counter for dynamic numbering
    $componentNum = 1
    $totalComponents = 4
    
    # Launch Gateway (Backend) with vector support in new window
    Write-Info "[$componentNum/$totalComponents] Launching pagi-gateway (Backend API + Qdrant - Port 8000)..."
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PSScriptRoot'; Write-Host 'PAGI Gateway Starting with Vector Memory...' -ForegroundColor Cyan; cargo run -p pagi-gateway --features bridge-ms,vector"
    Start-Sleep -Seconds 3
    $componentNum++
    
    # Launch Control Panel in new window
    Write-Info "[$componentNum/$totalComponents] Launching pagi-control-panel (System Toggles - Port 8002)..."
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PSScriptRoot'; Write-Host 'PAGI Control Panel Starting...' -ForegroundColor Cyan; cargo run -p pagi-control-panel"
    Start-Sleep -Seconds 2
    $componentNum++
    
    # Defensive Launch: Companion UI (only if directory exists)
    $companionPath = Join-Path $PSScriptRoot "add-ons\pagi-companion-ui"
    if (Test-Path $companionPath) {
        Write-Info "[$componentNum/$totalComponents] Launching pagi-companion-ui (Companion Interface - Port 3002)..."
        Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PSScriptRoot'; Write-Host 'PAGI Companion UI Starting...' -ForegroundColor Cyan; cargo run -p pagi-companion-ui"
        Start-Sleep -Seconds 2
    }
    else {
        Write-SovereignWarning "[$componentNum/$totalComponents] Companion UI not found at: $companionPath"
        Write-Info "  Skipping launch. Port 3002 reserved for future deployment."
    }
    $componentNum++
    
    # Defensive Launch: XCURZENS UI (only if directory exists)
    $xcurzensPath = Join-Path $PSScriptRoot "apps\pagi-xcurzens-ui"
    if (Test-Path $xcurzensPath) {
        Write-Info "[$componentNum/$totalComponents] Launching pagi-xcurzens-ui (Auditor Interface - Port 3003)..."
        Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PSScriptRoot'; Write-Host 'PAGI XCURZENS UI Starting...' -ForegroundColor Cyan; cargo run -p pagi-xcurzens-ui"
        Start-Sleep -Seconds 2
    }
    else {
        Write-SovereignWarning "[$componentNum/$totalComponents] XCURZENS UI not found at: $xcurzensPath"
        Write-Info "  Skipping launch. Port 3003 reserved for future deployment."
    }
    
    # Launch Studio UI in foreground
    Write-Info ""
    Write-Info "Launching pagi-studio-ui (Main Interface - Port 3001) in foreground..."
    Write-Info ""
    Write-Success "Ecosystem launch initiated"
    Write-Info ""
    Write-Host "+====================================================================+" -ForegroundColor Green
    Write-Host "|                                                                    |" -ForegroundColor Green
    Write-Host "|  PAGI Sovereign Ecosystem is starting...                          |" -ForegroundColor Green
    Write-Host "|                                                                    |" -ForegroundColor Green
    Write-Host "|  Gateway:       http://localhost:8000 (+ Qdrant on 6333)          |" -ForegroundColor Green
    Write-Host "|  Control Panel: http://localhost:8002                             |" -ForegroundColor Green
    
    # Dynamic UI status display
    if (Test-Path $companionPath) {
        Write-Host "|  Companion UI:  http://localhost:3002                             |" -ForegroundColor Green
    }
    else {
        Write-Host "|  Companion UI:  [Reserved - Port 3002]                            |" -ForegroundColor DarkGray
    }
    
    if (Test-Path $xcurzensPath) {
        Write-Host "|  XCURZENS UI:   http://localhost:3003                             |" -ForegroundColor Green
    }
    else {
        Write-Host "|  XCURZENS UI:   [Reserved - Port 3003]                            |" -ForegroundColor DarkGray
    }
    
    Write-Host "|  Studio UI:     http://localhost:3001                             |" -ForegroundColor Green
    Write-Host "|                                                                    |" -ForegroundColor Green
    Write-Host "|  Qdrant Dashboard: http://localhost:6333/dashboard                |" -ForegroundColor Green
    Write-Host "|                                                                    |" -ForegroundColor Green
    Write-Host "|  Close this window to stop the Studio UI.                         |" -ForegroundColor Green
    Write-Host "|  Other components will continue in their own windows.             |" -ForegroundColor Green
    Write-Host "|                                                                    |" -ForegroundColor Green
    Write-Host "+====================================================================+" -ForegroundColor Green
    Write-Info ""
    
    # Run Studio UI in foreground
    cargo run -p pagi-studio-ui --bin pagi-studio-ui
    
    Write-Info ""
    Write-Host "Studio UI closed. Other components are still running in their windows." -ForegroundColor Cyan
    Write-Host "Close their windows manually when done." -ForegroundColor Cyan
    
    return $true
}

# -----------------------------------------------------------------------------
# MAIN EXECUTION
# -----------------------------------------------------------------------------

function Main {
    Write-SovereignHeader
    
    # Execute all steps in sequence
    $steps = @(
        { Test-ExecutionPolicy },
        { Test-Prerequisites },
        { Test-EnvironmentFile },
        { Initialize-KnowledgeBases },
        { Clear-SovereignPorts },
        { Build-Workspace },
        { Install-FrontendDependencies },
        { Start-SovereignEcosystem }
    )
    
    foreach ($step in $steps) {
        $result = & $step
        if (-not $result) {
            Write-Host ""
            Write-Host "+====================================================================+" -ForegroundColor Red
            Write-Host "|                                                                    |" -ForegroundColor Red
            Write-Host "|  ORCHESTRATOR HALTED: Critical error encountered                  |" -ForegroundColor Red
            Write-Host "|                                                                    |" -ForegroundColor Red
            Write-Host "|  Please resolve the errors above and try again.                   |" -ForegroundColor Red
            Write-Host "|                                                                    |" -ForegroundColor Red
            Write-Host "+====================================================================+" -ForegroundColor Red
            Write-Host ""
            exit 1
        }
    }
    
    Write-Host ""
    Write-Host "+====================================================================+" -ForegroundColor Green
    Write-Host "|                                                                    |" -ForegroundColor Green
    Write-Host "|  SOVEREIGN ORCHESTRATION COMPLETE                                  |" -ForegroundColor Green
    Write-Host "|                                                                    |" -ForegroundColor Green
    Write-Host "+====================================================================+" -ForegroundColor Green
    Write-Host ""
}

# Run main function
Main

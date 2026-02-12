# PAGI Ecosystem Startup Script (Windows) — "Phoenix Rise"
# Order: 1) Port sanitization, 2) Build (workspace + gateway with vector), 3) Gateway (Qdrant + Bridge-MS), 4) Control Panel, 5) Studio UI
# The gateway's `vector` feature enables QdrantSidecar (Memory Engine on port 6333).

$ErrorActionPreference = "Stop"

Write-Host "--- Starting PAGI Master Orchestrator Ecosystem ---" -ForegroundColor Cyan

# 0. Port sanitization: clear PAGI ports and Qdrant so no zombie process blocks bind
$ports = @(8000, 8002, 3001, 6333)
foreach ($port in $ports) {
    $conn = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
    if ($conn) {
        $procId = $conn.OwningProcess | Select-Object -First 1
        if ($procId) {
            Write-Host "Clearing port $port (PID: $procId)..." -ForegroundColor Yellow
            Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
        }
    }
}

# 1. Workspace synthesis: build all, then gateway with Memory (vector) + Bridge-MS
Write-Host "[1/4] Auditing infrastructure and building workspace..." -ForegroundColor Yellow
Set-Location $PSScriptRoot
cargo build --workspace
if ($LASTEXITCODE -ne 0) { Write-Host "Build failed. Aborting." -ForegroundColor Red; exit 1 }
cargo build -p pagi-gateway --features "bridge-ms,vector"
if ($LASTEXITCODE -ne 0) { Write-Host "Gateway (vector) build failed. Aborting." -ForegroundColor Red; exit 1 }

# 2. Ignition: Gateway with Memory Engine (Qdrant) and Bridge-MS
Write-Host "[2/4] Phoenix Rising — launching pagi-gateway (Qdrant + bridge-ms)..." -ForegroundColor Green
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PSScriptRoot'; cargo run -p pagi-gateway --features 'bridge-ms,vector'"

# 3. Control Panel (management layer)
Write-Host "[3/4] Launching pagi-control-panel..." -ForegroundColor Green
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$PSScriptRoot'; cargo run -p pagi-control-panel"

# 4. Studio UI (frontend — port 3001) in this window
Write-Host "[4/4] Launching pagi-studio-ui..." -ForegroundColor Green
cargo run -p pagi-studio-ui --bin pagi-studio-ui

Write-Host "--- Sovereign Perimeter Active. Gateway and Control Panel are still running in their own windows. Close those when done. ---" -ForegroundColor Cyan

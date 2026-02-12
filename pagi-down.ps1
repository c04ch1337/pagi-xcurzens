# PAGI Ecosystem Shutdown Script (Windows) — "Phoenix Set"
# Clears PAGI ports (Gateway, Control Panel, Studio UI, Qdrant) so the next pagi-up.ps1
# can bind without "Address already in use". Use after closing the Gateway/Control Panel
# windows, or when a process didn't exit cleanly (e.g. Qdrant left on 6333).

$ErrorActionPreference = "Stop"

Write-Host "--- Stopping PAGI Master Orchestrator Ecosystem ---" -ForegroundColor Cyan

$ports = @(8000, 8002, 3001, 6333)
$cleared = 0
foreach ($port in $ports) {
    $conn = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue
    if ($conn) {
        $procId = $conn.OwningProcess | Select-Object -First 1
        if ($procId) {
            Write-Host "Clearing port $port (PID: $procId)..." -ForegroundColor Yellow
            Stop-Process -Id $procId -Force -ErrorAction SilentlyContinue
            $cleared++
        }
    }
}

if ($cleared -eq 0) {
    Write-Host "No processes were bound to PAGI ports (8000, 8002, 3001, 6333)." -ForegroundColor Gray
} else {
    Write-Host "Cleared $cleared process(es). Sovereign perimeter closed." -ForegroundColor Green
}

Write-Host "--- Run .\pagi-up.ps1 to rise again. ---" -ForegroundColor Cyan

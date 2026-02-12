# Start Qdrant Vector Database for Phoenix VectorKB
# This script starts Qdrant in the background for semantic search capabilities

$qdrantPath = Join-Path $PSScriptRoot "qdrant\qdrant.exe"
$workingDir = $PSScriptRoot

Write-Host "🚀 Starting Qdrant Vector Database..." -ForegroundColor Cyan
Write-Host "   Location: $qdrantPath" -ForegroundColor Gray
Write-Host "   HTTP API: http://localhost:6333" -ForegroundColor Gray
Write-Host "   Dashboard: http://localhost:6333/dashboard" -ForegroundColor Gray
Write-Host ""

if (Test-Path $qdrantPath) {
    Start-Process -FilePath $qdrantPath -WorkingDirectory $workingDir -NoNewWindow
    Write-Host "✓ Qdrant started successfully" -ForegroundColor Green
    Write-Host ""
    Write-Host "To verify Qdrant is running:" -ForegroundColor Yellow
    Write-Host "  curl http://localhost:6333/health" -ForegroundColor Gray
    Write-Host ""
    Write-Host "To stop Qdrant:" -ForegroundColor Yellow
    Write-Host "  Get-Process qdrant | Stop-Process" -ForegroundColor Gray
} else {
    Write-Host "✗ Error: Qdrant executable not found at $qdrantPath" -ForegroundColor Red
    Write-Host "  Run the installation first or check the path." -ForegroundColor Yellow
    exit 1
}

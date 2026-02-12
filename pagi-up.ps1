# PAGI XCURZENS — Stack Up
# Brings up the pagi-xcurzens stack (gateway, UI, etc.).

$ProjectName = "PAGI XCURZENS"
$RootDir = $PSScriptRoot

Write-Host "[$ProjectName] Stack UP — $RootDir" -ForegroundColor Cyan
# Add docker-compose, process starts, or cargo run as needed.
# Paths reference this repo root (pagi-xcurzens when folder is renamed).
Set-Location $RootDir
cargo build --release 2>&1
Write-Host "[$ProjectName] Build complete. Start gateway with: cargo run -p pagi-xcurzens-gateway" -ForegroundColor Green

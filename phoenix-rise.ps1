# PAGI XCURZENS - Phoenix Rise
# Full launch sequence for the XCURZENS authority (alias for sovereign start).

$ProjectName = "PAGI XCURZENS"
$RootDir = $PSScriptRoot

Write-Host "Phoenix Rise - $ProjectName" -ForegroundColor Magenta
$script = Join-Path $RootDir "start-sovereign.ps1"
& $script

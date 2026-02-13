# Start All Frontends - Simple Version
# Launches all three XCURZENS frontends in separate terminals

$ProjectRoot = "C:\Users\JAMEYMILNER\AppData\Local\pagi-xcurzens"

Write-Host "`n🏛️  Starting All Frontends..." -ForegroundColor Cyan

# Start XCURSENS Scout (Port 3001)
Write-Host "Starting XCURSENS Scout on port 3001..." -ForegroundColor Yellow
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$ProjectRoot\frontend-xcursens'; Write-Host '🏛️  XCURSENS Scout - Port 3001' -ForegroundColor Cyan; npm run dev"

Start-Sleep -Seconds 1

# Start Partner Nexus (Port 3002)
Write-Host "Starting Partner Nexus on port 3002..." -ForegroundColor Yellow
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$ProjectRoot\frontent-nexus'; Write-Host '🏛️  Partner Nexus - Port 3002' -ForegroundColor Yellow; npm run dev"

Start-Sleep -Seconds 1

# Start Command Center (Port 3003)
Write-Host "Starting Command Center on port 3003..." -ForegroundColor Yellow
Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$ProjectRoot\frontend-command'; Write-Host '🏛️  Command Center - Port 3003' -ForegroundColor Magenta; npm run dev"

Write-Host "`n✅ All frontends launched in separate terminals!" -ForegroundColor Green
Write-Host "`nDevelopment URLs:" -ForegroundColor Cyan
Write-Host "  • XCURSENS Scout:  http://localhost:3001" -ForegroundColor White
Write-Host "  • Partner Nexus:   http://localhost:3002" -ForegroundColor White
Write-Host "  • Command Center:  http://localhost:3003" -ForegroundColor White
Write-Host "`nProduction URLs (via Gateway on port 8000):" -ForegroundColor Cyan
Write-Host "  • XCURSENS Scout:  http://localhost:8000/" -ForegroundColor White
Write-Host "  • Partner Nexus:   http://localhost:8000/nexus" -ForegroundColor White
Write-Host "  • Command Center:  http://localhost:8000/command" -ForegroundColor White
Write-Host ""

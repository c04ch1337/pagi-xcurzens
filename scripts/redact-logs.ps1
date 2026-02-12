# Phoenix Log Redaction Script (PowerShell)
# Removes sensitive information from log files before sharing

param(
    [Parameter(Mandatory=$false)]
    [string]$LogFile = "",
    
    [Parameter(Mandatory=$false)]
    [switch]$All = $false
)

function Invoke-LogRedaction {
    param([string]$FilePath)
    
    if (-not (Test-Path $FilePath)) {
        Write-Host "❌ File not found: $FilePath" -ForegroundColor Red
        return $false
    }
    
    Write-Host "🔒 Redacting: $FilePath" -ForegroundColor Cyan
    
    $content = Get-Content $FilePath -Raw
    
    # Redact API keys
    $content = $content -replace 'sk-[a-zA-Z0-9_-]{20,}', 'sk-REDACTED'
    $content = $content -replace 'sk_[a-zA-Z0-9_-]{20,}', 'sk_REDACTED'
    
    # Redact Bearer tokens
    $content = $content -replace 'Bearer [a-zA-Z0-9_-]+', 'Bearer REDACTED'
    
    # Redact API key fields in JSON/TOML
    $content = $content -replace '"api_key":\s*"[^"]+"', '"api_key": "REDACTED"'
    $content = $content -replace 'api_key\s*=\s*"[^"]+"', 'api_key = "REDACTED"'
    $content = $content -replace 'openrouter_api_key\s*=\s*"[^"]+"', 'openrouter_api_key = "REDACTED"'
    
    # Redact passwords
    $content = $content -replace 'password["\s:=]+[^\s,}]+', 'password: REDACTED'
    $content = $content -replace '"password":\s*"[^"]+"', '"password": "REDACTED"'
    
    # Redact email addresses
    $content = $content -replace '\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b', 'user@REDACTED.com'
    
    # Redact IP addresses (keep localhost)
    $content = $content -replace '\b(?!127\.0\.0\.1|localhost)(\d{1,3}\.){3}\d{1,3}\b', 'XXX.XXX.XXX.XXX'
    
    # Redact file paths with usernames
    $content = $content -replace 'C:\\Users\\[^\\]+', 'C:\Users\REDACTED'
    $content = $content -replace '/home/[^/]+', '/home/REDACTED'
    $content = $content -replace '/Users/[^/]+', '/Users/REDACTED'
    
    # Save redacted version
    $redactedFile = $FilePath -replace '\.log$', '.redacted.log'
    $content | Out-File -FilePath $redactedFile -Encoding UTF8
    
    Write-Host "✓ Saved to: $redactedFile" -ForegroundColor Green
    return $true
}

# Main execution
Write-Host ""
Write-Host "🔒 Phoenix Log Redaction Tool" -ForegroundColor Magenta
Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host ""

if ($All) {
    # Redact all logs in ~/.pagi/logs/
    $pagiDir = Join-Path $env:USERPROFILE ".pagi"
    $logsDir = Join-Path $pagiDir "logs"
    
    if (-not (Test-Path $logsDir)) {
        Write-Host "❌ Logs directory not found: $logsDir" -ForegroundColor Red
        Write-Host "   Have you run Phoenix yet?" -ForegroundColor Yellow
        exit 1
    }
    
    $logFiles = Get-ChildItem -Path $logsDir -Filter "*.log" -File
    
    if ($logFiles.Count -eq 0) {
        Write-Host "⚠️  No log files found in $logsDir" -ForegroundColor Yellow
        exit 0
    }
    
    Write-Host "Found $($logFiles.Count) log file(s)" -ForegroundColor Cyan
    Write-Host ""
    
    $successCount = 0
    foreach ($file in $logFiles) {
        if (Invoke-LogRedaction -FilePath $file.FullName) {
            $successCount++
        }
    }
    
    Write-Host ""
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
    Write-Host "✓ Redacted $successCount/$($logFiles.Count) log files" -ForegroundColor Green
    Write-Host ""
    Write-Host "Redacted logs saved with .redacted.log extension" -ForegroundColor Cyan
    Write-Host "You can now safely share these files for debugging" -ForegroundColor Cyan
    
} elseif ([string]::IsNullOrEmpty($LogFile)) {
    # No file specified, show usage
    Write-Host "Usage:" -ForegroundColor Yellow
    Write-Host "  .\redact-logs.ps1 -LogFile <path-to-log>" -ForegroundColor White
    Write-Host "  .\redact-logs.ps1 -All" -ForegroundColor White
    Write-Host ""
    Write-Host "Examples:" -ForegroundColor Yellow
    Write-Host "  .\redact-logs.ps1 -LogFile C:\Users\YourName\.pagi\logs\gateway.log" -ForegroundColor DarkGray
    Write-Host "  .\redact-logs.ps1 -All" -ForegroundColor DarkGray
    Write-Host ""
    
} else {
    # Redact single file
    if (Invoke-LogRedaction -FilePath $LogFile) {
        Write-Host ""
        Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
        Write-Host "✓ Log redaction complete" -ForegroundColor Green
        Write-Host ""
        Write-Host "You can now safely share the .redacted.log file" -ForegroundColor Cyan
    }
}

Write-Host ""

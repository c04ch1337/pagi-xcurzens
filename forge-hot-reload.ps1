<#
.SYNOPSIS
    Forge Hot-Reload Orchestrator - Enables dynamic skill activation without Gateway restart

.DESCRIPTION
    This script orchestrates the hot-reload process for Forge-generated skills:
    1. Creates a new skill via the Forge API
    2. Triggers incremental compilation
    3. Activates the skill without full Gateway restart
    
    This is the "Self-Evolving" capability that allows PAGI to write new skills
    and use them immediately, even on thin satellite connections.

.PARAMETER SkillName
    Name of the skill to create (snake_case)

.PARAMETER Description
    Human-readable description of the skill

.PARAMETER Params
    JSON array of parameters (optional)

.PARAMETER GatewayUrl
    Gateway URL (default: http://localhost:8000)

.PARAMETER EnableHotReload
    Enable hot-reload before creating the skill

.PARAMETER DisableHotReload
    Disable hot-reload after creating the skill

.EXAMPLE
    .\forge-hot-reload.ps1 -SkillName "salesforce_sentinel" -Description "Scans Salesforce for security issues"

.EXAMPLE
    .\forge-hot-reload.ps1 -SkillName "weather_sentinel" -Description "Fetches weather data" -Params '[{"name":"location","type":"string","required":true}]'

.NOTES
    Author: The Forge (PAGI Self-Synthesis Engine)
    Version: 1.0.0
    
    This script is part of the Sovereign Infrastructure and operates on bare metal.
#>

param(
    [Parameter(Mandatory=$true)]
    [string]$SkillName,
    
    [Parameter(Mandatory=$false)]
    [string]$Description = "",
    
    [Parameter(Mandatory=$false)]
    [string]$Params = "[]",
    
    [Parameter(Mandatory=$false)]
    [string]$GatewayUrl = "http://localhost:8000",
    
    [Parameter(Mandatory=$false)]
    [switch]$EnableHotReload,
    
    [Parameter(Mandatory=$false)]
    [switch]$DisableHotReload
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-Forge {
    param([string]$Message, [string]$Color = "Cyan")
    Write-Host "🔥 Forge: $Message" -ForegroundColor $Color
}

function Write-ForgeSuccess {
    param([string]$Message)
    Write-Forge $Message -Color "Green"
}

function Write-ForgeError {
    param([string]$Message)
    Write-Forge $Message -Color "Red"
}

function Write-ForgeWarning {
    param([string]$Message)
    Write-Forge $Message -Color "Yellow"
}

# Check if Gateway is running
Write-Forge "Checking Gateway status..."
try {
    $statusResponse = Invoke-RestMethod -Uri "$GatewayUrl/v1/status" -Method Get -ErrorAction Stop
    Write-ForgeSuccess "Gateway is running on port $($statusResponse.port)"
} catch {
    Write-ForgeError "Gateway is not running. Start it with .\pagi-up.ps1"
    exit 1
}

# Enable hot-reload if requested
if ($EnableHotReload) {
    Write-Forge "Enabling hot-reload..."
    try {
        $enableResponse = Invoke-RestMethod -Uri "$GatewayUrl/api/v1/forge/hot-reload/enable" -Method Post -ErrorAction Stop
        Write-ForgeSuccess $enableResponse.message
    } catch {
        Write-ForgeError "Failed to enable hot-reload: $_"
        exit 1
    }
}

# Check hot-reload status
Write-Forge "Checking hot-reload status..."
try {
    $hotReloadStatus = Invoke-RestMethod -Uri "$GatewayUrl/api/v1/forge/hot-reload/status" -Method Get -ErrorAction Stop
    if ($hotReloadStatus.enabled) {
        Write-ForgeSuccess "Hot-reload is enabled"
    } else {
        Write-ForgeWarning "Hot-reload is disabled. Skill will require manual Gateway restart."
        Write-ForgeWarning "Enable with: .\forge-hot-reload.ps1 -SkillName $SkillName -EnableHotReload"
    }
} catch {
    Write-ForgeError "Failed to check hot-reload status: $_"
    exit 1
}

# Create the skill
Write-Forge "Creating skill '$SkillName'..."

$toolSpec = @{
    name = $SkillName
    description = $Description
    params = $Params | ConvertFrom-Json
} | ConvertTo-Json -Depth 10

try {
    $createResponse = Invoke-RestMethod `
        -Uri "$GatewayUrl/api/v1/forge/create" `
        -Method Post `
        -Body $toolSpec `
        -ContentType "application/json" `
        -ErrorAction Stop
    
    if ($createResponse.success -or $createResponse.cargo_check_ok) {
        Write-ForgeSuccess "Skill created successfully!"
        Write-Host ""
        Write-Host "📋 Skill Details:" -ForegroundColor Cyan
        Write-Host "  Name:        $($createResponse.skill_name)" -ForegroundColor White
        Write-Host "  Module:      $($createResponse.module_name)" -ForegroundColor White
        Write-Host "  File:        $($createResponse.file_path)" -ForegroundColor White
        
        if ($createResponse.hot_reloaded) {
            Write-Host "  Hot-Reload:  ✓ Activated" -ForegroundColor Green
            Write-Host "  Compile Time: $($createResponse.compilation_time_ms)ms" -ForegroundColor White
            Write-Host ""
            Write-ForgeSuccess "Skill is ready for immediate use!"
        } else {
            Write-Host "  Hot-Reload:  ✗ Not activated" -ForegroundColor Yellow
            Write-Host ""
            Write-ForgeWarning "Restart Gateway to activate: .\pagi-down.ps1 && .\pagi-up.ps1"
        }
    } else {
        Write-ForgeError "Skill creation failed!"
        Write-Host ""
        Write-Host "Error: $($createResponse.message)" -ForegroundColor Red
        if ($createResponse.cargo_stderr) {
            Write-Host ""
            Write-Host "Cargo Output:" -ForegroundColor Yellow
            Write-Host $createResponse.cargo_stderr -ForegroundColor Gray
        }
        exit 1
    }
} catch {
    Write-ForgeError "Failed to create skill: $_"
    exit 1
}

# Disable hot-reload if requested
if ($DisableHotReload) {
    Write-Forge "Disabling hot-reload..."
    try {
        $disableResponse = Invoke-RestMethod -Uri "$GatewayUrl/api/v1/forge/hot-reload/disable" -Method Post -ErrorAction Stop
        Write-ForgeSuccess $disableResponse.message
    } catch {
        Write-ForgeError "Failed to disable hot-reload: $_"
        exit 1
    }
}

# List all hot-reloaded skills
Write-Host ""
Write-Forge "Listing all hot-reloaded skills..."
try {
    $listResponse = Invoke-RestMethod -Uri "$GatewayUrl/api/v1/forge/hot-reload/list" -Method Get -ErrorAction Stop
    if ($listResponse.count -gt 0) {
        Write-Host ""
        Write-Host "🔥 Hot-Reloaded Skills ($($listResponse.count)):" -ForegroundColor Cyan
        foreach ($skill in $listResponse.skills) {
            $loadedTime = [DateTimeOffset]::FromUnixTimeSeconds($skill.loaded_at).LocalDateTime
            Write-Host "  • $($skill.skill_name)" -ForegroundColor White
            Write-Host "    Module: $($skill.module_name)" -ForegroundColor Gray
            Write-Host "    Loaded: $loadedTime" -ForegroundColor Gray
        }
    } else {
        Write-Host "  No hot-reloaded skills yet." -ForegroundColor Gray
    }
} catch {
    Write-ForgeWarning "Failed to list hot-reloaded skills: $_"
}

Write-Host ""
Write-ForgeSuccess "Forge operation complete!"

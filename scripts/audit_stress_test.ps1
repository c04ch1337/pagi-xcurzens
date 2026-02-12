# Total Context Stress Test — Full-System Audit Script
# Run with gateway on http://127.0.0.1:8001 and PAGI_SHADOW_KEY set (64 hex chars).
# Usage: .\scripts\audit_stress_test.ps1 [-BaseUrl "http://127.0.0.1:8001"]

param(
    [string]$BaseUrl = "http://127.0.0.1:8001"
)

$executeUrl = "$BaseUrl/v1/execute"

function Invoke-Execute {
    param([hashtable]$Body)
    $json = $Body | ConvertTo-Json -Depth 10
    try {
        $r = Invoke-RestMethod -Method Post -Uri $executeUrl -ContentType "application/json" -Body $json
        return $r
    } catch {
        Write-Error "Request failed: $_"
        return $null
    }
}

Write-Host "=== Total Context Stress Test ===" -ForegroundColor Cyan
Write-Host "BaseUrl: $BaseUrl"
Write-Host ""

# Step 0: Ethos — Stoicism
Write-Host "[0/6] Ethos: Set active philosophy to Stoic..." -ForegroundColor Yellow
$r0 = Invoke-Execute -Body @{
    goal = @{
        ExecuteSkill = @{
            name = "EthosSync"
            payload = @{ active_school = "Stoic" }
        }
    }
    tenant_id = "default"
}
if ($r0.status -eq "ok") { Write-Host "  OK - Reframing will use Dichotomy of Control" -ForegroundColor Green } else { Write-Host "  FAIL: $r0" -ForegroundColor Red }

# Step 1: Soma Sync
Write-Host "[1/6] Soma Sync (sleep 4h, readiness 30)..." -ForegroundColor Yellow
$r1 = Invoke-Execute -Body @{
    goal = @{
        ExecuteSkill = @{
            name = "BioGateSync"
            payload = @{ sleep_hours = 4.0; readiness_score = 30 }
        }
    }
    tenant_id = "default"
}
if ($r1.status -eq "ok") { Write-Host "  OK - BioGate cross-layer reaction active" -ForegroundColor Green } else { Write-Host "  FAIL: $r1" -ForegroundColor Red }

# Step 2: Kardia — Partner
Write-Host "[2/6] Kardia Map: Partner (trust 0.9, Anxious)..." -ForegroundColor Yellow
$r2 = Invoke-Execute -Body @{
    goal = @{
        ExecuteSkill = @{
            name = "KardiaMap"
            payload = @{
                name = "Partner"
                relationship = "Partner"
                trust_score = 0.9
                attachment_style = "Anxious"
            }
        }
    }
    tenant_id = "default"
}
if ($r2.status -eq "ok") { Write-Host "  OK - name_slug: $($r2.name_slug)" -ForegroundColor Green } else { Write-Host "  FAIL: $r2" -ForegroundColor Red }

# Step 3: Kardia — Project Manager
Write-Host "[3/6] Kardia Map: Project Manager (trust 0.3, Avoidant)..." -ForegroundColor Yellow
$r3 = Invoke-Execute -Body @{
    goal = @{
        ExecuteSkill = @{
            name = "KardiaMap"
            payload = @{
                name = "Project Manager"
                relationship = "Boss"
                trust_score = 0.3
                attachment_style = "Avoidant"
                triggers = @("criticism", "micromanagement")
            }
        }
    }
    tenant_id = "default"
}
if ($r3.status -eq "ok") { Write-Host "  OK - name_slug: $($r3.name_slug)" -ForegroundColor Green } else { Write-Host "  FAIL: $r3" -ForegroundColor Red }

# Step 4: Shadow entry (DeepJournal)
Write-Host "[4/6] Shadow entry (DeepJournal)..." -ForegroundColor Yellow
$r4 = Invoke-Execute -Body @{
    goal = @{
        ExecuteSkill = @{
            name = "DeepJournalSkill"
            payload = @{ raw_entry = "Argument with Partner about the Project Manager's deadlines." }
        }
    }
    tenant_id = "default"
}
$recordId = $r4.record_id
if ($r4.status -eq "ok" -and $recordId) {
    Write-Host "  OK - record_id: $recordId" -ForegroundColor Green
} else {
    Write-Host "  FAIL or missing record_id (is PAGI_SHADOW_KEY set?): $r4" -ForegroundColor Red
    exit 1
}

# Step 5: ReflectShadow
$sessionKey = $env:PAGI_SHADOW_KEY
if (-not $sessionKey -or $sessionKey.Length -lt 64) {
    Write-Host "[5/6] ReflectShadow SKIPPED - PAGI_SHADOW_KEY not set or invalid (need 64 hex chars)" -ForegroundColor Yellow
} else {
    Write-Host "[5/6] ReflectShadow..." -ForegroundColor Yellow
    $r5 = Invoke-Execute -Body @{
        goal = @{
            ExecuteSkill = @{
                name = "ReflectShadow"
                payload = @{ record_id = $recordId; session_key = $sessionKey }
            }
        }
        tenant_id = "default"
    }
    if ($r5.status -eq "ok") {
        Write-Host "  OK - reflection generated" -ForegroundColor Green
        Write-Host ""
        Write-Host "--- Reflection (first 500 chars) ---" -ForegroundColor Cyan
        $ref = if ($r5.reflection) { $r5.reflection } else { "" }
        $len = [Math]::Min(500, $ref.Length)
        if ($len -gt 0) { Write-Host $ref.Substring(0, $len) } else { Write-Host "(empty)" }
        Write-Host "..."
    } else {
        Write-Host "  FAIL: $($r5 | ConvertTo-Json -Compress)" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "=== Expected logic ===" -ForegroundColor Cyan
Write-Host '  - Ethos: Prompt includes Stoic principles; reframing should mention focusing on what is within your control.'
Write-Host '  - Soma: Prompt includes [Soma - Physical load elevated...] (grace_multiplier 1.6).'
Write-Host '  - Kardia: Prompt includes Partner (0.90, Anxious) and Project Manager (0.30, Avoidant).'
Write-Host '  - secure_purge: Decrypted content and prompt zeroed after reflection.'
Write-Host 'See docs/total_context_stress_test.md for full scenario and synthesis report.'

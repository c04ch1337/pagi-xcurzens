# 🧾 Mission Submission Validator (Operation First Rise)

Phoenix includes a machine-checkable validator for beta tester submissions.

It validates:

- **JSON Diagram Envelope**: `type=diagram`, `format=mermaid`, non-empty `content`
- **Mermaid dark theme init** present in `content`: `%%{init: {'theme': 'dark'}}%%` (or equivalent)
- **Sidecar evidence**: looks for port `6333` and a “healthy/ready” signal in logs (PID optional)
- **Concise density**: expects `density_mode=concise` for the Architect’s View proof

---

## 1) Endpoint

- `POST /api/v1/mission/validate`

This is a thin wrapper over the orchestrator skill [`MissionValidator`](crates/pagi-skills/src/mission_validator.rs:1).

---

## 2) Request Schema

```json
{
  "tenant_id": "mission-review",
  "agent_id": "default",
  "density_mode": "concise",
  "json_envelope": {
    "type": "diagram",
    "format": "mermaid",
    "content": "%%{init: {'theme': 'dark'}}%%\ngraph TD; A-->B;",
    "metadata": {"title": "Firewall unauthorized API call"}
  },
  "sidecar_logs": "...Port 6333...Healthy...PID 1234...",
  "diagramviewer_screenshot_description": "Diagram rendered in dark mode with 2 bullets below."
}
```

Notes:

- `json_envelope` may be either the envelope object **or** a string containing JSON.
- You can also paste a full gateway response object if it has `response` containing the envelope.

---

## 3) Response

```json
{
  "status": "ok",
  "report": {
    "status": "verified",
    "rank": "Gold",
    "checks": {
      "density_concise_ok": true,
      "json_schema_ok": true,
      "mermaid_dark_theme_ok": true,
      "sidecar_qdrant_ok": true,
      "qdrant_pid": 1234,
      "qdrant_port_6333_seen": true
    },
    "notes": []
  }
}
```

Ranks:

- **Gold**: envelope valid + dark theme ok + sidecar healthy + concise density + screenshot description present
- **Silver**: sidecar healthy + envelope valid, but missing one secondary proof (often dark theme or screenshot description)
- **Bronze**: envelope valid but sidecar proof weak/missing
- **Retry**: envelope invalid/missing

---

## 4) cURL Example

Windows PowerShell (escaping may vary by shell):

```powershell
$body = @'
{
  "density_mode": "concise",
  "json_envelope": {
    "type": "diagram",
    "format": "mermaid",
    "content": "%%{init: {'theme': 'dark'}}%%\ngraph TD; A-->B;"
  },
  "sidecar_logs": "Port 6333: Active & Healthy (PID 1234)",
  "diagramviewer_screenshot_description": "Rendered diagram in dark mode; 2 bullets below."
}
'@

Invoke-RestMethod -Method Post -Uri http://127.0.0.1:8001/api/v1/mission/validate -ContentType application/json -Body $body
```


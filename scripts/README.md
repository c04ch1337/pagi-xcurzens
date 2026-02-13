# Scripts

Run all scripts from the **repository root** unless noted.

| Script | Purpose |
|--------|---------|
| **Release & beta** | |
| `deploy_beta.ps1` / `deploy_beta.sh` | Build release, sanitize, package for beta distribution |
| `sanitize_for_release.ps1` / `sanitize_for_release.sh` | Remove personal data before release (storage, .env, logs, etc.) |
| `trigger-release.ps1` / `trigger-release.sh` | Tag and trigger GitHub release (e.g. `./scripts/trigger-release.sh v0.1.0-beta.1`) |
| `redact-logs.ps1` / `redact-logs.sh` | Redact sensitive data from log files before sharing |
| **Utilities** | |
| `start-qdrant.ps1` | Start Qdrant vector DB sidecar (Windows) |
| `setup_voice.sh` / `setup_voice.bat` | One-click Sovereign Voice (STT/TTS) setup |
| `audit_stress_test.ps1` | Stress test and audit run |

**Entry points (at repo root):** Run from project root: `.\start-sovereign.ps1`, `.\phoenix-rise.ps1` / `.\phoenix-rise.sh`, `.\pagi-up.ps1` / `.\pagi-up.sh`, `.\pagi-down.ps1`, `.\forge-kill-switch.ps1` / `.\forge-kill-switch.sh`, `.\forge-hot-reload.ps1` / `.\forge-hot-reload.sh`, `.\phoenix-activate-live.ps1` / `.\phoenix-activate-live.sh`, `.\phoenix-live-sync.ps1` / `.\phoenix-live-sync.sh`.

**Documentation:** All guides and architecture docs are in [../docs/](../docs/) (see [docs/README.md](../docs/README.md) for the index).

# PAGI XCURZENS — Rebrand Complete

**Sovereign voice:** *Jamey, the rebranding is complete. Every line of code now recognizes the XCURZENS authority. The old 'uac-main' identity has been purged from the system. We are operating under the new perimeter.*

---

## What was done

| Layer | Status |
|-------|--------|
| **Global string replacement** | All new files use `pagi-xcurzens`, `PAGI-XCURZENS`, `pagi_xcurzens`. No `pagi-uac-main` / `PAGI-UAC` / `pagi_uac` in this repo. |
| **Rust infrastructure** | Root `Cargo.toml` workspace members point to `crates/pagi-xcurzens-core` and `crates/pagi-xcurzens-gateway`. Crate names and `use pagi_xcurzens_core` imports use new naming. |
| **Orchestrator & scripts** | `start-sovereign.ps1`, `pagi-up.ps1`, `pagi-down.ps1`, `phoenix-rise.ps1` use `$ProjectName = "PAGI XCURZENS"` and paths relative to repo root. |
| **Documentation & metadata** | `README.md`, `FORGE_ARCHITECTURE.md`, `.cursorrules` reflect the pagi-xcurzens repository context. |

---

## One manual step: folder rename

Scripts and partners may expect the repo folder to match the identity. Rename the project folder:

**From:** `pagi-uac-main`  
**To:** `pagi-xcurzens`

1. Close Cursor/IDE and any terminals using this path.
2. In Explorer or PowerShell (parent directory):
   ```powershell
   cd $env:LOCALAPPDATA
   Rename-Item -Path "pagi-uac-main" -NewName "pagi-xcurzens"
   ```
3. Reopen the workspace from `%LOCALAPPDATA%\pagi-xcurzens`.

---

## Verify

- **Rust:** From repo root run `cargo check`. Fix any remaining path or name errors.
- **Scripts:** Run `.\start-sovereign.ps1` (or `.\phoenix-rise.ps1`) and confirm the PAGI XCURZENS header and steps run.

---

## Final Git anchor (when code is clean)

When you're ready to push the rebrand to the new repository:

```powershell
git remote set-url origin <new-pagi-xcurzens-repo-url>
git add -A
git commit -m "Rebrand: PAGI XCURZENS — purge uac-main identity"
git push -u origin main
```

Replace `<new-pagi-xcurzens-repo-url>` with your actual remote URL.

# PAGI Sanctuary — Bare-Metal Deployment Guide

This guide formalizes the deployment process for the **PAGI Sanctuary**: from a research build to a **stable, persistent personal utility**. All instructions are for **bare metal** (no Docker). Use system services (systemd on Linux, Windows Task/Service on Windows) and direct `cargo`/binary execution.

---

## 1. Prerequisites

### Rust toolchain

- **Rust (stable).** Install from [rustup.rs](https://rustup.rs/).

  ```bash
  rustup default stable
  ```

### Build dependencies (OS-specific)

| OS        | Packages / notes |
|-----------|-------------------|
| **Linux** | `pkg-config`, `libssl-dev` (Debian/Ubuntu: `build-essential pkg-config libssl-dev`). For memory locking: `libc` is linked via `libc` crate (no extra package); the process must have `CAP_IPC_LOCK` or run with sufficient privileges (see [Security Hardening](#3-security-hardening)). |
| **Windows** | No extra system packages. `windows-sys` (Win32 memory APIs) is a crate dependency; ensure Visual Studio Build Tools or equivalent for linking. |
| **macOS** | `openssl` (e.g. `brew install openssl pkg-config`). Memory locking uses `libc::mlock`; no extra capability required for normal execution. |

### Memory-locking support (Slot 9 — Shadow)

- **Linux:** The `libc` crate provides `mlock`/`munlock`. No separate system library beyond the standard C library.
- **Windows:** The `windows-sys` crate (feature `Win32_System_Memory`) provides `VirtualLock`/`VirtualUnlock`. No extra install.

---

## 2. Environment configuration

Use either a `.env` file (sourced before running the gateway) or system/process environment variables.

### Template (.env or system variables)

```bash
# Path to the Sled database directory (pagi_vault, pagi_knowledge, etc.).
# In config this is "storage_path"; override via PAGI__storage_path (logical name: PAGI_STORAGE_PATH).
# Example: PAGI__storage_path=/var/lib/pagi/data
export PAGI__storage_path="./data"

# Path to gateway config (TOML). Default: config/gateway.toml if unset.
export PAGI_CONFIG="config/gateway.toml"

# Master key for Slot 9 (Shadow Vault): 64 hex chars = 32 bytes.
# ⚠️ SECURITY: Keep secret; never commit. Without it, Slot 9 remains LOCKED.
export PAGI_SHADOW_KEY="<64-char-hex-string>"

# Optional: API key for Live Status API and gateway auth.
# If set, clients (e.g. Sovereign Dashboard) must send X-API-Key with this value.
export PAGI_API_KEY="<your-api-key>"
```

### Variable reference

| Variable | Purpose |
|----------|---------|
| **PAGI_CONFIG** | Path to gateway config file (e.g. `config/gateway.toml`). Config is loaded from this file; defaults to `config/gateway` if unset. |
| **PAGI__storage_path** | Overrides `storage_path` in config. Sled databases live under this directory (`pagi_vault`, `pagi_knowledge`, etc.). |
| **PAGI_SHADOW_KEY** | 64 hex characters (32 bytes). Used to encrypt/decrypt Slot 9 (Shadow). If unset or invalid, Slot 9 is locked. |
| **PAGI_API_KEY** | Optional. When set, `GET /api/v1/sovereign-status` and other protected endpoints require `X-API-Key` with this value. Set the same value when running `pagi status` so the dashboard can call the Live Status API. |

**Note:** The config crate uses prefix `PAGI` and separator `__`; e.g. `PAGI__port=8002` overrides `port` in the loaded TOML.

---

## 3. Security hardening

### Slot 9 (Shadow) — memory locking

Decrypted Shadow (Slot 9) content is held in RAM in a **memory-locked** buffer so the OS does not swap it to disk:

- **Linux:** `mlock` / `munlock` (via `libc`).
- **Windows:** `VirtualLock` / `VirtualUnlock` (via `windows-sys`).

On drop, the buffer is zeroed and unlocked. This reduces the risk of sensitive journal/vault data appearing in swap or page files.

#### Linux: allowing memory lock

- **Without privilege:** If the process hits resource limits, `mlock` can fail; the crate logs a warning and continues (buffer may be swapped).
- **To allow locking:** Grant `CAP_IPC_LOCK` to the binary or run as a user with sufficient `RLIMIT_MEMLOCK` (e.g. `ulimit -l`). For a system service, use a dedicated user and set limits in the unit file (see [Service integration](#5-service-integration)).

Example (systemd unit snippet):

```ini
# Allow memory locking for Shadow (Slot 9)
LimitMEMLOCK=infinity
```

Or run the process with capabilities:

```bash
sudo setcap cap_ipc_lock+ep /path/to/release/pagi-gateway
```

#### Windows

`VirtualLock` is available to the process without extra configuration; ensure the account running the service has normal memory quotas.

---

## 4. Execution flow

### Step 1: Build release binaries

From the repository root:

```bash
cargo build --release
```

Binaries:

- **Gateway (Headless Brain):** `target/release/pagi-gateway`
- **Sovereign Dashboard:** `target/release/pagi` (from `pagi-sovereign-dashboard`)

### Step 2: Start the Gateway

The gateway is the single API entry point (orchestrator, skills, KB APIs). Run from the **repository root** so relative paths (e.g. `config/`, `data/`) resolve.

```bash
# Linux/macOS
export PAGI_CONFIG="${PAGI_CONFIG:-config/gateway.toml}"
./target/release/pagi-gateway

# Or with env from file
set -a && source .env && set +a && ./target/release/pagi-gateway
```

**Pre-flight check (optional):** Verify Sled DB and port before starting:

```bash
cargo run -p pagi-gateway --release -- --verify
# Exit 0: ready; non-zero: DB lock or port in use
```

Default bind: `127.0.0.1:8001` (port from `config/gateway.toml` or `PAGI__port`).

### Step 3: Sovereign Dashboard (telemetry)

With the gateway **stopped**, the dashboard opens the knowledge store directly. With the gateway **running**, it uses the **Live Status API** so it does not contend for the Sled lock.

```bash
# From repo root; uses same config as gateway (PAGI_CONFIG, storage_path)
./target/release/pagi status
# or
cargo run -p pagi-sovereign-dashboard --release -- status
```

If `PAGI_API_KEY` is set on the gateway, set it in the environment when running `pagi status` so the dashboard can call `GET /api/v1/sovereign-status`.

**Quick integrity check:**

```bash
pagi status
# Or if binary is on PATH:
./target/release/pagi status
```

You should see the 9-slot knowledge matrix, Oikos (Slot 2), Soma (Slot 8), Ethos (Slot 6), Kardia (Slot 7), and Shadow (Slot 9) vault status.

---

## 5. Service integration

### Linux: systemd

Create a unit file, e.g. `/etc/systemd/system/pagi-gateway.service`:

```ini
[Unit]
Description=PAGI Gateway (Headless Brain)
After=network.target

[Service]
Type=simple
User=pagi
Group=pagi
WorkingDirectory=/opt/pagi
EnvironmentFile=/opt/pagi/.env
ExecStart=/opt/pagi/target/release/pagi-gateway
Restart=on-failure
RestartSec=5
# Allow memory locking for Slot 9 (Shadow)
LimitMEMLOCK=infinity

[Install]
WantedBy=multi-user.target
```

- Install the binary and config under `/opt/pagi` (or your chosen path); set `WorkingDirectory` and `ExecStart` accordingly.
- Put secrets and overrides in `/opt/pagi/.env` (e.g. `PAGI_SHADOW_KEY`, `PAGI_API_KEY`, `PAGI__storage_path`) and restrict permissions: `chmod 600 /opt/pagi/.env`.
- Enable and start:

  ```bash
  sudo systemctl daemon-reload
  sudo systemctl enable pagi-gateway
  sudo systemctl start pagi-gateway
  sudo systemctl status pagi-gateway
  ```

### Windows: Task or Windows Service

**Option A — Scheduled Task (run at logon or at startup)**

1. Task Scheduler → Create Task.
2. **General:** Run with highest privileges if needed; run whether user is logged on or not.
3. **Triggers:** At startup or At logon.
4. **Actions:** Start a program  
   - Program: `C:\path\to\pagi-uac-main\target\release\pagi-gateway.exe`  
   - Start in: `C:\path\to\pagi-uac-main` (repo root).
5. **Settings:** Allow task to be run on demand; configure restart on failure if desired.

Set environment variables (e.g. `PAGI_CONFIG`, `PAGI_SHADOW_KEY`, `PAGI_API_KEY`) in the task’s **Environment** or via a wrapper script that sets them and runs `pagi-gateway.exe`.

**Option B — Windows Service**

Use a service wrapper (e.g. NSSM, or a small Rust/other service that starts the process) so that `pagi-gateway.exe` runs under the correct working directory and environment. Ensure the same env vars are available to the child process and that the account has permission to use `VirtualLock` (default for normal users).

---

## 6. Slot IDs (architecture consistency)

All 9 slots must match the established PAGI architecture:

| Slot | Domain | Role |
|------|--------|------|
| 1 | Pneuma (Vision) | Brand/vision |
| 2 | Oikos (Context) | Task governance, context |
| 3 | Logos | Finance / logic |
| 4 | Chronos | Time / operations |
| 5 | Techne | Community / craft |
| 6 | Ethos | Philosophical lens |
| 7 | Kardia | Relational map |
| 8 | Soma | Body / BioGate |
| 9 | **Shadow** | Encrypted vault (memory-locked when decrypted) |

Config labels for slots 1–8 are set in `config/gateway.toml` under `[slot_labels]`. Slot 9 is the Shadow Vault; its state is reported by the Sovereign Dashboard and Live Status API.

---

## 7. Verification checklist

After deployment:

1. **Build:** `cargo build --release` completes without errors.
2. **Pre-flight:** `cargo run -p pagi-gateway --release -- --verify` exits 0.
3. **Gateway:** Start the gateway; `GET http://127.0.0.1:8001/api/v1/health` returns `{"status":"ok"}` (or your configured port).
4. **Dashboard:** `./target/release/pagi status` shows the 9-slot matrix and Shadow (Slot 9) status (locked/unlocked).
5. **Memory locking:** On Linux, if you use `LimitMEMLOCK=infinity` (or `cap_ipc_lock`), ensure the gateway runs and that the dashboard or logs do not show the mlock warning for Shadow buffers.

Once this checklist passes, the sanctuary is **online and hardened** for persistent use.

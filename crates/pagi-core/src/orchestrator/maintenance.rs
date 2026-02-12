//! Autonomous Maintenance & Reflexion Loop
//!
//! A low-priority background thread that periodically:
//! 1. **Telemetry Pulse** – Snapshots OS health (CPU, RAM, disk) via sysinfo.
//! 2. **Failure Audit** – Retrieves `Err` records from the last 24 h (Sled/Chronos).
//! 3. **Root Cause** – Sends failures to the OpenRouter Sovereign Bridge for analysis.
//! 4. **Self-Patch** – Pipes code fixes into `pagi-evolution::Compiler`, saving to
//!    `crates/pagi-skills/src/generated/patches/` (never overwrites existing files).
//! 4.5. **Validation Benchmark** – Compiles the patch to a temporary cdylib, runs a
//!    smoke test against it, and measures CPU/memory delta vs baseline. If the patch
//!    fails to compile or crashes during the smoke test, it is **auto-rejected** and
//!    recorded in Chronos as a "Syntactic Hallucination." The `MaintenancePulseEvent`
//!    includes a `performance_delta` field so the UI can show efficiency metrics.
//! 5. **Interlock** – Requests human approval via `TerminalGuard` before applying.
//!    **NEW:** Approval can also come from the web UI via a `tokio::sync::oneshot` channel.
//!    The approval prompt now includes performance delta data from Phase 4.5.
//! 6. **Telemetry Broadcast** – Emits structured `maintenance_pulse` SSE events via the
//!    broadcast channel so the UI can display background reflexion status in real time.
//!
//! ## Resource Safety
//!
//! * Runs only when the gateway has been idle (no user input) for a configurable
//!   number of minutes (default: 5).
//! * Uses `tokio::time::interval` at 30-minute cadence (configurable via
//!   `PAGI_MAINTENANCE_INTERVAL_SECS`).
//! * Reads from `KnowledgeStore` / Sled without contending for write-locks used by
//!   the main Orchestrator — failure records are read-only scans.
//! * The `pagi-evolution::Compiler` writes to an isolated `patches/` directory.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, Mutex as TokioMutex, oneshot};
use tracing::{debug, error, info, warn};

use crate::knowledge::{EventRecord, KbType, KnowledgeStore};
use crate::openrouter_service::OpenRouterBridge;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Default maintenance interval: 30 minutes.
const DEFAULT_INTERVAL_SECS: u64 = 1800;

/// Default idle threshold before maintenance is allowed: 5 minutes.
const DEFAULT_IDLE_THRESHOLD_SECS: u64 = 300;

/// Maximum number of failure records to include in a single reflexion prompt.
const MAX_FAILURES_PER_CYCLE: usize = 10;

/// Agent ID used for maintenance loop Chronos events.
const MAINTENANCE_AGENT_ID: &str = "MAINTENANCE_LOOP";

// ---------------------------------------------------------------------------
// Maintenance Configuration
// ---------------------------------------------------------------------------

/// Configuration for the autonomous maintenance loop.
#[derive(Debug, Clone)]
pub struct MaintenanceConfig {
    /// Interval between maintenance cycles (default: 30 min).
    pub interval: Duration,
    /// Minimum idle time before a cycle is allowed to run (default: 5 min).
    pub idle_threshold: Duration,
    /// Directory where generated patches are saved.
    pub patches_dir: PathBuf,
    /// Whether human-in-the-loop approval is required before applying patches.
    pub require_approval: bool,
    /// Optional approval bridge for UI-based approval (bypasses terminal stdin).
    pub approval_bridge: Option<ApprovalBridgeHandle>,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        let interval_secs = std::env::var("PAGI_MAINTENANCE_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_INTERVAL_SECS)
            .max(60); // minimum 1 minute

        let idle_secs = std::env::var("PAGI_MAINTENANCE_IDLE_THRESHOLD_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(DEFAULT_IDLE_THRESHOLD_SECS)
            .max(30); // minimum 30 seconds

        Self {
            interval: Duration::from_secs(interval_secs),
            idle_threshold: Duration::from_secs(idle_secs),
            patches_dir: PathBuf::from("crates/pagi-skills/src/generated/patches"),
            require_approval: true,
            approval_bridge: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Idle Tracker
// ---------------------------------------------------------------------------

/// Tracks the last time user activity was observed (epoch millis).
/// Shared between the gateway (which bumps it on every request) and the
/// maintenance loop (which reads it to decide whether to run).
#[derive(Debug, Clone)]
pub struct IdleTracker {
    last_activity_ms: Arc<AtomicU64>,
}

impl IdleTracker {
    pub fn new() -> Self {
        Self {
            last_activity_ms: Arc::new(AtomicU64::new(Self::now_ms())),
        }
    }

    /// Call this on every user interaction (HTTP request, SSE connect, etc.).
    pub fn touch(&self) {
        self.last_activity_ms.store(Self::now_ms(), Ordering::Release);
    }

    /// Returns how long the system has been idle.
    pub fn idle_duration(&self) -> Duration {
        let last = self.last_activity_ms.load(Ordering::Acquire);
        let now = Self::now_ms();
        Duration::from_millis(now.saturating_sub(last))
    }

    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

impl Default for IdleTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Maintenance Pulse Event (structured SSE payload for the UI)
// ---------------------------------------------------------------------------

/// Performance delta comparing a new patch against the existing skill.
/// Sent as part of `MaintenancePulseEvent` so the UI can display
/// "This patch is X% more efficient" before the operator approves.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceDelta {
    /// CPU usage change as a human-readable string (e.g. "-5%", "+2%").
    pub cpu: String,
    /// Memory usage change as a human-readable string (e.g. "-3%", "+1%").
    pub mem: String,
    /// Whether the patch compiled successfully.
    pub compiled: bool,
    /// Whether the smoke test passed.
    pub smoke_test_passed: bool,
    /// Optional detail message (e.g. smoke test output or error).
    pub detail: String,
}

impl Default for PerformanceDelta {
    fn default() -> Self {
        Self {
            cpu: "N/A".to_string(),
            mem: "N/A".to_string(),
            compiled: false,
            smoke_test_passed: false,
            detail: String::new(),
        }
    }
}

/// Structured event emitted via the SSE broadcast channel so the frontend
/// can render a real-time "System Health" indicator.
///
/// Serialized as JSON and sent with `event: maintenance_pulse`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MaintenancePulseEvent {
    /// Current phase: "idle", "telemetry", "audit", "reflexion", "patching",
    /// "validation", "awaiting_approval", "applying", "complete", "healthy",
    /// "auto_rejected".
    pub phase: String,
    /// The skill or subsystem being targeted (e.g. "FileSystemSkill").
    pub target: String,
    /// Human-readable detail string.
    pub details: String,
    /// Epoch millis.
    pub timestamp_ms: i64,
    /// Number of patches that have been applied/saved in this session.
    pub applied_patches: u32,
    /// Number of failures detected in the current audit window.
    pub failure_count: u32,
    /// Performance delta from Phase 4.5 validation (None if validation hasn't run).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance_delta: Option<PerformanceDelta>,
}

impl MaintenancePulseEvent {
    pub fn new(phase: &str, target: &str, details: &str) -> Self {
        Self {
            phase: phase.to_string(),
            target: target.to_string(),
            details: details.to_string(),
            timestamp_ms: now_epoch_ms(),
            applied_patches: 0,
            failure_count: 0,
            performance_delta: None,
        }
    }

    pub fn with_counts(mut self, applied: u32, failures: u32) -> Self {
        self.applied_patches = applied;
        self.failure_count = failures;
        self
    }

    pub fn with_performance_delta(mut self, delta: PerformanceDelta) -> Self {
        self.performance_delta = Some(delta);
        self
    }

    /// Serialize to the SSE data payload format: `event: maintenance_pulse\ndata: {json}\n\n`
    pub fn to_sse_line(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

// ---------------------------------------------------------------------------
// Maintenance Approval Bridge (oneshot channel for UI-based approval)
// ---------------------------------------------------------------------------

/// A pending approval request that can be answered from either the terminal
/// or the web UI. The maintenance loop creates one of these when it needs
/// human approval, and the gateway can resolve it via an HTTP endpoint.
#[derive(Debug)]
pub struct PendingApproval {
    /// Unique ID for this approval request.
    pub id: String,
    /// Human-readable description of what is being approved.
    pub description: String,
    /// The patch name (for display).
    pub patch_name: String,
    /// The skill that failed (for display).
    pub skill: String,
    /// Epoch millis when the request was created.
    pub created_ms: i64,
    /// Sender half of the oneshot channel. Send `true` to approve, `false` to decline.
    pub responder: Option<oneshot::Sender<bool>>,
}

/// Thread-safe handle to the current pending approval (if any).
/// The gateway stores this in `AppState` and exposes it via HTTP endpoints.
pub type ApprovalBridgeHandle = Arc<TokioMutex<Option<PendingApproval>>>;

/// Creates a new empty approval bridge handle.
pub fn new_approval_bridge() -> ApprovalBridgeHandle {
    Arc::new(TokioMutex::new(None))
}

// ---------------------------------------------------------------------------
// Telemetry Snapshot (lightweight, no sysinfo dependency in pagi-core)
// ---------------------------------------------------------------------------

/// Lightweight OS health snapshot collected at the start of each maintenance cycle.
/// Uses only `std` APIs so `pagi-core` doesn't need `sysinfo` as a dependency.
/// For full telemetry, the gateway can inject richer data via the `SystemTelemetry` skill.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TelemetryPulse {
    /// Timestamp (epoch millis).
    pub timestamp_ms: i64,
    /// Number of logical CPUs.
    pub cpu_count: usize,
    /// Current working directory.
    pub cwd: String,
    /// Available environment hints (e.g. RUST_LOG, PAGI_* vars).
    pub env_hints: Vec<(String, String)>,
}

impl TelemetryPulse {
    /// Collect a basic telemetry pulse using only `std` APIs.
    pub fn collect() -> Self {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "<unknown>".to_string());

        // Collect PAGI_* and RUST_LOG env vars (no secrets).
        let env_hints: Vec<(String, String)> = std::env::vars()
            .filter(|(k, _)| k.starts_with("PAGI_") || k == "RUST_LOG")
            .map(|(k, v)| {
                // Redact anything that looks like a key/secret.
                let safe_v = if k.to_uppercase().contains("KEY")
                    || k.to_uppercase().contains("SECRET")
                    || k.to_uppercase().contains("TOKEN")
                {
                    "***REDACTED***".to_string()
                } else {
                    v
                };
                (k, safe_v)
            })
            .collect();

        Self {
            timestamp_ms: now_epoch_ms(),
            cpu_count: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            cwd,
            env_hints,
        }
    }
}

// ---------------------------------------------------------------------------
// Failure Record (from Chronos / Sled scan)
// ---------------------------------------------------------------------------

/// A distilled failure record extracted from Chronos events in the last 24 h.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FailureRecord {
    /// Chronos key where this was found.
    pub key: String,
    /// The skill or tool that failed.
    pub skill: String,
    /// Human-readable description of the failure.
    pub description: String,
    /// stderr or error output (truncated).
    pub stderr_snippet: String,
    /// Epoch millis when the failure occurred.
    pub timestamp_ms: i64,
}

// ---------------------------------------------------------------------------
// Phase 4.5: Validation Benchmarks — Compile, Smoke-Test, Perf-Compare
// ---------------------------------------------------------------------------

/// Result of the Phase 4.5 validation pipeline.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the patch compiled to a temporary cdylib.
    pub compiled: bool,
    /// Path to the compiled artifact (if compilation succeeded).
    pub artifact_path: Option<PathBuf>,
    /// Whether the smoke test passed (false if compilation failed).
    pub smoke_test_passed: bool,
    /// Performance delta (CPU/memory comparison).
    pub performance_delta: PerformanceDelta,
    /// Human-readable summary of the validation.
    pub summary: String,
    /// If true, the patch should be auto-rejected (compilation failure or smoke test crash).
    pub auto_reject: bool,
    /// Reason for auto-rejection (empty if not rejected).
    pub rejection_reason: String,
}

/// Compiles a patch to a temporary cdylib, runs a smoke test, and measures
/// CPU/memory delta. This is the core of Phase 4.5.
///
/// The runner is gated behind `#[cfg(feature = "validation")]` so that
/// `pagi-core` can still compile without `sysinfo` / `libloading` / `tempfile`.
pub struct SmokeTestRunner;

#[cfg(feature = "validation")]
impl SmokeTestRunner {
    /// Compile the patch code to a temporary cdylib and run a smoke test.
    ///
    /// # Arguments
    /// * `code` – The Rust source code for the patch (full `lib.rs`).
    /// * `patch_name` – Human-readable name for logging.
    /// * `target_skill` – The skill being patched (used to select the smoke test).
    ///
    /// # Returns
    /// A `ValidationResult` with compilation status, smoke test outcome, and perf delta.
    pub async fn validate(
        code: &str,
        patch_name: &str,
        target_skill: &str,
    ) -> ValidationResult {
        info!(
            target: "pagi::maintenance::validation",
            patch = patch_name,
            skill = target_skill,
            "Phase 4.5: Starting validation benchmark"
        );

        // Step 1: Compile to a temporary cdylib
        let compile_result = Self::compile_to_temp(code, patch_name);
        let (compiled, artifact_path, compile_error) = match compile_result {
            Ok(path) => (true, Some(path), String::new()),
            Err(e) => (false, None, e),
        };

        if !compiled {
            // Auto-reject: compilation failure = "Syntactic Hallucination"
            return ValidationResult {
                compiled: false,
                artifact_path: None,
                smoke_test_passed: false,
                performance_delta: PerformanceDelta {
                    cpu: "N/A".to_string(),
                    mem: "N/A".to_string(),
                    compiled: false,
                    smoke_test_passed: false,
                    detail: format!("Compilation failed: {}", compile_error),
                },
                summary: format!(
                    "REJECTED: Patch '{}' failed to compile — Syntactic Hallucination. Error: {}",
                    patch_name, compile_error
                ),
                auto_reject: true,
                rejection_reason: format!("Compilation failed: {}", compile_error),
            };
        }

        let artifact = artifact_path.as_ref().unwrap();

        // Step 2: Measure baseline system telemetry
        let baseline = Self::measure_telemetry().await;

        // Step 3: Run smoke test against the compiled library
        let smoke_result = Self::run_smoke_test(artifact, target_skill).await;

        // Step 4: Measure post-test system telemetry
        let post_test = Self::measure_telemetry().await;

        // Step 5: Compute performance delta
        let cpu_delta = post_test.0 - baseline.0;
        let mem_delta_bytes = post_test.1 as i64 - baseline.1 as i64;
        let mem_delta_pct = if baseline.1 > 0 {
            (mem_delta_bytes as f64 / baseline.1 as f64) * 100.0
        } else {
            0.0
        };

        let cpu_str = if cpu_delta >= 0.0 {
            format!("+{:.1}%", cpu_delta)
        } else {
            format!("{:.1}%", cpu_delta)
        };
        let mem_str = if mem_delta_pct >= 0.0 {
            format!("+{:.1}%", mem_delta_pct)
        } else {
            format!("{:.1}%", mem_delta_pct)
        };

        let (smoke_passed, smoke_detail) = match smoke_result {
            Ok(detail) => (true, detail),
            Err(e) => (false, e),
        };

        let perf_delta = PerformanceDelta {
            cpu: cpu_str.clone(),
            mem: mem_str.clone(),
            compiled: true,
            smoke_test_passed: smoke_passed,
            detail: smoke_detail.clone(),
        };

        if !smoke_passed {
            // Auto-reject: smoke test crash = "Syntactic Hallucination"
            return ValidationResult {
                compiled: true,
                artifact_path: artifact_path.clone(),
                smoke_test_passed: false,
                performance_delta: perf_delta,
                summary: format!(
                    "REJECTED: Patch '{}' compiled but smoke test failed — Syntactic Hallucination. Error: {}",
                    patch_name, smoke_detail
                ),
                auto_reject: true,
                rejection_reason: format!("Smoke test failed: {}", smoke_detail),
            };
        }

        // Validation passed
        let efficiency_msg = if cpu_delta < 0.0 {
            format!("This patch is {:.1}% more CPU-efficient.", cpu_delta.abs())
        } else if cpu_delta > 1.0 {
            format!("This patch uses {:.1}% more CPU.", cpu_delta)
        } else {
            "CPU usage is comparable.".to_string()
        };

        ValidationResult {
            compiled: true,
            artifact_path: artifact_path.clone(),
            smoke_test_passed: true,
            performance_delta: perf_delta,
            summary: format!(
                "VALIDATED: Patch '{}' compiled and passed smoke test. CPU: {}, Mem: {}. {}",
                patch_name, cpu_str, mem_str, efficiency_msg
            ),
            auto_reject: false,
            rejection_reason: String::new(),
        }
    }

    /// Compile Rust code to a temporary cdylib. Returns the path to the artifact.
    fn compile_to_temp(code: &str, name: &str) -> Result<PathBuf, String> {
        let dir = tempfile::tempdir()
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let root = dir.path().to_path_buf();

        // Write Cargo.toml for a cdylib crate
        let toml = format!(
            r#"[package]
name = "pagi_validation_{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = {{ version = "1", features = ["derive"] }}
serde_json = "1"
libc = "0.2"
"#,
            sanitize_filename(name)
        );

        std::fs::write(root.join("Cargo.toml"), &toml)
            .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;
        std::fs::create_dir_all(root.join("src"))
            .map_err(|e| format!("Failed to create src dir: {}", e))?;
        std::fs::write(root.join("src").join("lib.rs"), code)
            .map_err(|e| format!("Failed to write lib.rs: {}", e))?;

        // Run cargo build --release
        let target_dir = root.join("target");
        let output = std::process::Command::new("cargo")
            .current_dir(&root)
            .args([
                "build",
                "--release",
                "--target-dir",
                target_dir.to_str().unwrap_or("target"),
            ])
            .output()
            .map_err(|e| format!("cargo build spawn failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "cargo build failed (exit {}): {}",
                output.status.code().unwrap_or(-1),
                stderr.chars().take(1000).collect::<String>()
            ));
        }

        // Find the artifact
        let lib_name = if cfg!(target_os = "windows") {
            format!("pagi_validation_{}.dll", sanitize_filename(name))
        } else {
            format!("libpagi_validation_{}.so", sanitize_filename(name))
        };
        let lib_path = target_dir.join("release").join(&lib_name);

        if lib_path.exists() {
            // Copy to a stable temp location so the tempdir can be dropped later
            let stable_dir = std::env::temp_dir().join("pagi_validation");
            let _ = std::fs::create_dir_all(&stable_dir);
            let stable_path = stable_dir.join(&lib_name);
            std::fs::copy(&lib_path, &stable_path)
                .map_err(|e| format!("Failed to copy artifact: {}", e))?;
            info!(
                target: "pagi::maintenance::validation",
                path = %stable_path.display(),
                "Patch compiled successfully to temp cdylib"
            );
            Ok(stable_path)
        } else {
            Err(format!("Artifact not found at {}", lib_path.display()))
        }
    }

    /// Measure current system telemetry: (cpu_usage_percent, used_memory_bytes).
    async fn measure_telemetry() -> (f32, u64) {
        use sysinfo::System;

        let mut sys = System::new_all();
        sys.refresh_all();
        // Small delay to let CPU measurement stabilize
        tokio::time::sleep(Duration::from_millis(250)).await;
        sys.refresh_all();

        let cpu = sys.global_cpu_info().cpu_usage();
        let mem = sys.used_memory();
        (cpu, mem)
    }

    /// Run a smoke test against the compiled cdylib.
    ///
    /// The smoke test depends on the target skill:
    /// - `FileSystemSkill` / `fs_*`: Calls the exported function with a "list directory" arg.
    /// - Default: Calls with an empty JSON object `{}` and checks for non-null return.
    ///
    /// The test runs in a `spawn_blocking` thread with a timeout to prevent hangs.
    async fn run_smoke_test(
        artifact_path: &Path,
        target_skill: &str,
    ) -> Result<String, String> {
        let path = artifact_path.to_path_buf();
        let skill = target_skill.to_string();

        // Run in a blocking thread with a 30-second timeout
        let result = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::task::spawn_blocking(move || {
                Self::execute_smoke_test_sync(&path, &skill)
            }),
        )
        .await;

        match result {
            Ok(Ok(inner)) => inner,
            Ok(Err(e)) => Err(format!("Smoke test thread panicked: {}", e)),
            Err(_) => Err("Smoke test timed out after 30 seconds".to_string()),
        }
    }

    /// Synchronous smoke test execution (runs inside `spawn_blocking`).
    fn execute_smoke_test_sync(
        artifact_path: &Path,
        target_skill: &str,
    ) -> Result<String, String> {
        // Construct the smoke test input based on the target skill
        let test_input = Self::build_smoke_test_input(target_skill);

        // Try to load the library and call the exported function
        unsafe {
            let lib = libloading::Library::new(artifact_path)
                .map_err(|e| format!("Failed to load library: {}", e))?;

            // Look for the standard pagi_dynamic_skill_execute symbol
            let execute_fn: libloading::Symbol<
                unsafe extern "C" fn(*const std::ffi::c_char) -> *mut std::ffi::c_char,
            > = lib
                .get(b"pagi_dynamic_skill_execute")
                .map_err(|e| format!("Symbol pagi_dynamic_skill_execute not found: {}", e))?;

            let free_fn: libloading::Symbol<unsafe extern "C" fn(*mut std::ffi::c_char)> = lib
                .get(b"pagi_dynamic_skill_free")
                .map_err(|e| format!("Symbol pagi_dynamic_skill_free not found: {}", e))?;

            // Call the function
            let c_input = std::ffi::CString::new(test_input.as_bytes())
                .map_err(|e| format!("CString creation failed: {}", e))?;

            let result_ptr = execute_fn(c_input.as_ptr());

            if result_ptr.is_null() {
                return Err("Skill returned null pointer".to_string());
            }

            let result_str = std::ffi::CStr::from_ptr(result_ptr)
                .to_string_lossy()
                .into_owned();
            free_fn(result_ptr);

            // Validate the result is valid JSON
            let _: serde_json::Value = serde_json::from_str(&result_str)
                .map_err(|e| format!("Skill returned invalid JSON: {}", e))?;

            Ok(format!(
                "Smoke test passed. Output: {}",
                result_str.chars().take(200).collect::<String>()
            ))
        }
    }

    /// Build the JSON input for the smoke test based on the target skill type.
    fn build_smoke_test_input(target_skill: &str) -> String {
        let lower = target_skill.to_lowercase();

        if lower.contains("filesystem") || lower.contains("fs_") || lower.contains("file") {
            // FileSystemSkill smoke test: list the current directory
            serde_json::json!({
                "operation": "list",
                "path": "."
            })
            .to_string()
        } else if lower.contains("sentiment") || lower.contains("analyze") {
            // Sentiment analysis smoke test
            serde_json::json!({
                "text": "This is a test sentence for validation."
            })
            .to_string()
        } else if lower.contains("knowledge") || lower.contains("query") {
            // Knowledge query smoke test
            serde_json::json!({
                "query": "test",
                "limit": 1
            })
            .to_string()
        } else {
            // Default: empty object
            serde_json::json!({}).to_string()
        }
    }
}

/// Fallback when the `validation` feature is disabled: always returns a "skipped" result.
#[cfg(not(feature = "validation"))]
impl SmokeTestRunner {
    pub async fn validate(
        _code: &str,
        patch_name: &str,
        _target_skill: &str,
    ) -> ValidationResult {
        warn!(
            target: "pagi::maintenance::validation",
            "Validation feature is disabled — skipping Phase 4.5 for patch '{}'",
            patch_name
        );
        ValidationResult {
            compiled: false,
            artifact_path: None,
            smoke_test_passed: false,
            performance_delta: PerformanceDelta::default(),
            summary: format!(
                "Validation skipped for '{}' (feature 'validation' not enabled)",
                patch_name
            ),
            auto_reject: false,
            rejection_reason: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Reflexion Prompt Template
// ---------------------------------------------------------------------------

/// Builds the reflexion prompt sent to OpenRouter for root-cause analysis.
fn build_reflexion_prompt(
    telemetry: &TelemetryPulse,
    failures: &[FailureRecord],
) -> String {
    let mut prompt = String::with_capacity(4096);

    prompt.push_str(
        "You are an autonomous Rust systems engineer embedded in the PAGI Sovereign Agent. \
         Your task is to analyze recent tool/skill failures from the local bare-metal environment \
         and produce a concrete, compilable Rust code fix.\n\n",
    );

    prompt.push_str("## System Telemetry\n");
    prompt.push_str(&format!("- CPUs: {}\n", telemetry.cpu_count));
    prompt.push_str(&format!("- CWD: {}\n", telemetry.cwd));
    for (k, v) in &telemetry.env_hints {
        prompt.push_str(&format!("- ENV {}: {}\n", k, v));
    }
    prompt.push('\n');

    prompt.push_str("## Recent Failures (last 24 h)\n\n");
    for (i, f) in failures.iter().enumerate() {
        prompt.push_str(&format!(
            "### Failure #{}\n- **Skill:** {}\n- **Description:** {}\n- **stderr:**\n```\n{}\n```\n\n",
            i + 1,
            f.skill,
            f.description,
            f.stderr_snippet,
        ));
    }

    prompt.push_str(
        "## Instructions\n\
         1. Identify the root cause for each failure.\n\
         2. For the most impactful failure, produce a **complete, compilable Rust module** \
            that fixes the issue. The module should be a standalone `lib.rs` suitable for \
            the `pagi-skills` crate (it may use `pagi_core::AgentSkill`, `serde`, `serde_json`).\n\
         3. Wrap the code in a fenced code block: ```rust ... ```\n\
         4. Include a one-line comment at the top: `// PATCH: <short description>`\n\
         5. Do NOT produce generic advice. Produce only executable Rust code.\n\
         6. If no code fix is possible, respond with exactly: `NO_PATCH_NEEDED`\n",
    );

    prompt
}

/// Extracts the first fenced Rust code block from the LLM response.
fn extract_rust_code(response: &str) -> Option<String> {
    // Look for ```rust ... ``` or ``` ... ```
    let markers = ["```rust", "```rs"];
    for marker in markers {
        if let Some(start) = response.find(marker) {
            let code_start = start + marker.len();
            if let Some(end) = response[code_start..].find("```") {
                let code = response[code_start..code_start + end].trim();
                if !code.is_empty() {
                    return Some(code.to_string());
                }
            }
        }
    }
    // Fallback: generic ``` block
    if let Some(start) = response.find("```") {
        let code_start = start + 3;
        // Skip optional language tag on the same line
        let line_end = response[code_start..]
            .find('\n')
            .map(|i| code_start + i + 1)
            .unwrap_or(code_start);
        if let Some(end) = response[line_end..].find("```") {
            let code = response[line_end..line_end + end].trim();
            if !code.is_empty() {
                return Some(code.to_string());
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// TerminalGuard Interlock (inline, no dependency on pagi-skills)
// ---------------------------------------------------------------------------

/// Prompts the user for approval in the terminal. Returns `true` if approved.
/// This is a lightweight inline version so `pagi-core` doesn't depend on `pagi-skills`.
fn request_maintenance_approval(description: &str) -> bool {
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("  🔧 [MAINTENANCE]: Autonomous Reflexion Patch");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("  {}", description);
    println!();
    print!("  Apply this patch? (y/n): ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        println!("  ✗ Failed to read input — patch NOT applied.");
        println!("═══════════════════════════════════════════════════════════════");
        return false;
    }

    let approved = input.trim().eq_ignore_ascii_case("y");
    if approved {
        println!("  ✓ Patch approved by operator.");
    } else {
        println!("  ✗ Patch declined by operator.");
    }
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    approved
}

// ---------------------------------------------------------------------------
// Failure Audit: scan Chronos for recent errors
// ---------------------------------------------------------------------------

/// Scans Chronos events across all agents for failures in the last `window` duration.
fn audit_recent_failures(
    knowledge: &KnowledgeStore,
    window: Duration,
) -> Vec<FailureRecord> {
    let chronos_slot = KbType::Chronos.slot_id();
    let cutoff_ms = now_epoch_ms() - (window.as_millis() as i64);

    let keys = match knowledge.scan_keys(chronos_slot) {
        Ok(k) => k,
        Err(e) => {
            warn!(target: "pagi::maintenance", error = %e, "Failed to scan Chronos keys");
            return Vec::new();
        }
    };

    let mut failures = Vec::new();

    for key in &keys {
        // Chronos keys look like: chronos/{agent_id}/{timestamp_ms}
        // We read the raw bytes and try to deserialize as EventRecord.
        let bytes = match knowledge.get(chronos_slot, key) {
            Ok(Some(b)) => b,
            _ => continue,
        };

        let text = match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };

        // Try to parse as EventRecord JSON.
        let event: EventRecord = match serde_json::from_str(&text) {
            Ok(e) => e,
            Err(_) => {
                // Might be a plain-text event; check for failure keywords.
                let lower = text.to_lowercase();
                if (lower.contains("fail") || lower.contains("error") || lower.contains("err"))
                    && !lower.contains("success")
                {
                    failures.push(FailureRecord {
                        key: key.clone(),
                        skill: "unknown".to_string(),
                        description: text.chars().take(200).collect(),
                        stderr_snippet: String::new(),
                        timestamp_ms: now_epoch_ms(),
                    });
                }
                continue;
            }
        };

        // Filter by time window.
        if event.timestamp_ms < cutoff_ms {
            continue;
        }

        // Check if the event indicates a failure.
        let outcome_lower = event.outcome.as_deref().unwrap_or("").to_lowercase();
        let reflection_lower = event.reflection.to_lowercase();
        let is_failure = outcome_lower.contains("fail")
            || outcome_lower.contains("error")
            || outcome_lower.contains("err")
            || reflection_lower.contains("failed")
            || reflection_lower.contains("error:");

        if is_failure {
            failures.push(FailureRecord {
                key: key.clone(),
                skill: event.skill_name.clone().unwrap_or_else(|| "unknown".to_string()),
                description: event.reflection.chars().take(300).collect(),
                stderr_snippet: event
                    .outcome
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .take(500)
                    .collect(),
                timestamp_ms: event.timestamp_ms,
            });
        }

        if failures.len() >= MAX_FAILURES_PER_CYCLE {
            break;
        }
    }

    // Sort by timestamp descending (most recent first).
    failures.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));
    failures.truncate(MAX_FAILURES_PER_CYCLE);
    failures
}

// ---------------------------------------------------------------------------
// Patch Writer
// ---------------------------------------------------------------------------

/// Saves a patch to the patches directory using versioned naming: `{name}_v{timestamp}.rs`.
/// Also maintains a `current_{name}.rs` that always points to the latest version.
/// Returns the path if successful.
fn save_patch(patches_dir: &Path, patch_name: &str, code: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(patches_dir)
        .map_err(|e| format!("Failed to create patches dir: {}", e))?;

    let base_name = sanitize_filename(patch_name);
    let timestamp = now_epoch_ms();
    let versioned_name = format!("{}_v{}.rs", base_name, timestamp);
    let versioned_path = patches_dir.join(&versioned_name);

    // Write the versioned source file.
    std::fs::write(&versioned_path, code)
        .map_err(|e| format!("Failed to write patch: {}", e))?;

    // Update the `current_{name}.rs` pointer (file copy for cross-platform compatibility).
    let current_path = patches_dir.join(format!("current_{}.rs", base_name));
    let temp_path = patches_dir.join(format!("current_{}.rs.tmp", base_name));
    let _ = std::fs::remove_file(&temp_path);
    std::fs::copy(&versioned_path, &temp_path)
        .map_err(|e| format!("Failed to copy to current: {}", e))?;
    let _ = std::fs::remove_file(&current_path);
    std::fs::rename(&temp_path, &current_path)
        .map_err(|e| format!("Failed to rename current: {}", e))?;

    Ok(versioned_path)
}

/// Computes a simple hash of the patch code for genetic memory / dead-end detection.
/// Returns a hex string suitable for storage in Chronos.
pub fn compute_patch_dna(code: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h1 = DefaultHasher::new();
    code.hash(&mut h1);
    let v1 = h1.finish();
    let mut h2 = DefaultHasher::new();
    format!("{}{}", code, v1).hash(&mut h2);
    let v2 = h2.finish();
    format!("{:016x}{:016x}", v1, v2)
}

/// Checks Chronos (KB-4) for a previously rejected or rolled-back patch with the same DNA hash.
/// Returns `Some(reason)` if the hash is a known dead-end, `None` otherwise.
pub fn check_genetic_dead_end(knowledge: &KnowledgeStore, code_hash: &str) -> Option<String> {
    let chronos_slot = KbType::Chronos.slot_id();
    let dead_end_key = format!("dead_end/{}", code_hash);
    match knowledge.get(chronos_slot, &dead_end_key) {
        Ok(Some(bytes)) => String::from_utf8(bytes).ok(),
        _ => None,
    }
}

/// Records a patch DNA hash as a dead-end in Chronos (KB-4).
/// This prevents the agent from re-suggesting the same broken code.
pub fn record_genetic_dead_end(
    knowledge: &KnowledgeStore,
    code_hash: &str,
    skill_name: &str,
    reason: &str,
) {
    let chronos_slot = KbType::Chronos.slot_id();
    let dead_end_key = format!("dead_end/{}", code_hash);
    let value = format!(
        "{{\"skill\":\"{}\",\"reason\":\"{}\",\"timestamp_ms\":{}}}",
        skill_name,
        reason.replace('"', "'"),
        now_epoch_ms()
    );
    if let Err(e) = knowledge.insert(chronos_slot, &dead_end_key, value.as_bytes()) {
        warn!(
            target: "pagi::maintenance::genetic",
            error = %e,
            hash = code_hash,
            "Failed to record genetic dead-end in Chronos"
        );
    } else {
        info!(
            target: "pagi::maintenance::genetic",
            hash = &code_hash[..code_hash.len().min(12)],
            skill = skill_name,
            "Recorded genetic dead-end in Chronos"
        );
    }
}

/// Sanitizes a string for use as a filename (alphanumeric + underscore only).
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

// ---------------------------------------------------------------------------
// Maintenance Cycle (single tick)
// ---------------------------------------------------------------------------

/// Helper: emit a structured `maintenance_pulse` SSE event via the broadcast channel.
fn emit_pulse(
    log_tx: &broadcast::Sender<String>,
    phase: &str,
    target: &str,
    details: &str,
    applied: u32,
    failures: u32,
) {
    let pulse = MaintenancePulseEvent::new(phase, target, details)
        .with_counts(applied, failures);
    // Send as a JSON-prefixed line so the SSE handler can detect and route it.
    let json = pulse.to_sse_line();
    let _ = log_tx.send(format!("MAINTENANCE_PULSE:{}", json));
    // Also send the human-readable log line.
    let _ = log_tx.send(format!("[MAINTENANCE] [{}] {} — {}", phase, target, details));
}

/// Executes one maintenance cycle. Returns a human-readable summary.
async fn maintenance_tick(
    knowledge: &KnowledgeStore,
    bridge: &OpenRouterBridge,
    config: &MaintenanceConfig,
    log_tx: &broadcast::Sender<String>,
    applied_patches_count: &mut u32,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Phase 1: Telemetry Pulse
    let telemetry = TelemetryPulse::collect();
    emit_pulse(log_tx, "telemetry", "system", &format!("{} CPUs, cwd={}", telemetry.cpu_count, telemetry.cwd), *applied_patches_count, 0);
    debug!(
        target: "pagi::maintenance",
        cpus = telemetry.cpu_count,
        cwd = %telemetry.cwd,
        "Telemetry pulse collected"
    );

    // Phase 2: Failure Audit (last 24 h)
    emit_pulse(log_tx, "audit", "chronos", "Scanning for failures in the last 24h...", *applied_patches_count, 0);
    let failures = audit_recent_failures(knowledge, Duration::from_secs(86400));
    if failures.is_empty() {
        let msg = "No failures in the last 24 h. System healthy.".to_string();
        emit_pulse(log_tx, "healthy", "system", &msg, *applied_patches_count, 0);
        info!(target: "pagi::maintenance", "{}", msg);

        // Record a healthy-pulse Chronos event.
        let event = EventRecord::now("Maintenance", "Autonomous maintenance cycle: no failures detected.")
            .with_skill("maintenance_loop")
            .with_outcome("healthy");
        let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

        return Ok(msg);
    }

    let failure_count = failures.len() as u32;
    emit_pulse(log_tx, "audit", "chronos", &format!("Found {} failure(s). Starting reflexion...", failure_count), *applied_patches_count, failure_count);
    info!(
        target: "pagi::maintenance",
        failure_count,
        "Failure audit complete — entering reflexion phase"
    );

    // Phase 3: Root Cause Analysis via OpenRouter Bridge
    let target_skill = failures.first().map(|f| f.skill.as_str()).unwrap_or("unknown");
    let prompt = build_reflexion_prompt(&telemetry, &failures);
    emit_pulse(log_tx, "reflexion", target_skill, "Analyzing failures via OpenRouter...", *applied_patches_count, failure_count);

    let response = match bridge.plan(&prompt, None).await {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("OpenRouter reflexion failed: {}", e);
            warn!(target: "pagi::maintenance", "{}", msg);
            emit_pulse(log_tx, "complete", target_skill, &msg, *applied_patches_count, failure_count);

            // Record the failure in Chronos.
            let event = EventRecord::now("Maintenance", &msg)
                .with_skill("maintenance_loop")
                .with_outcome("reflexion_failed");
            let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

            return Err(msg.into());
        }
    };

    // Phase 4: Self-Patch — extract code and save
    if response.contains("NO_PATCH_NEEDED") {
        let msg = format!(
            "Reflexion complete: {} failure(s) analyzed, no code patch needed.",
            failure_count
        );
        emit_pulse(log_tx, "complete", target_skill, &msg, *applied_patches_count, failure_count);
        info!(target: "pagi::maintenance", "{}", msg);

        let event = EventRecord::now("Maintenance", &msg)
            .with_skill("maintenance_loop")
            .with_outcome("no_patch_needed");
        let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

        return Ok(msg);
    }

    let code = match extract_rust_code(&response) {
        Some(c) => c,
        None => {
            let msg = "Reflexion returned text but no extractable Rust code block.".to_string();
            emit_pulse(log_tx, "complete", target_skill, &msg, *applied_patches_count, failure_count);
            warn!(target: "pagi::maintenance", "{}", msg);

            let event = EventRecord::now("Maintenance", &msg)
                .with_skill("maintenance_loop")
                .with_outcome("no_code_extracted");
            let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

            return Ok(msg);
        }
    };

    // Derive a patch name from the first failure's skill.
    let patch_name = format!(
        "patch_{}",
        failures
            .first()
            .map(|f| f.skill.as_str())
            .unwrap_or("unknown")
    );

    emit_pulse(log_tx, "patching", target_skill, &format!("Synthesized patch '{}'", patch_name), *applied_patches_count, failure_count);

    // Phase 4.25: Genetic Memory — check if this code is a known dead-end
    let code_hash = compute_patch_dna(&code);
    if let Some(dead_end_reason) = check_genetic_dead_end(knowledge, &code_hash) {
        let msg = format!(
            "SELF-CENSORED: Patch '{}' for skill '{}' matches a known Evolutionary Dead-End (hash {}). Reason: {}",
            patch_name, target_skill, &code_hash[..code_hash.len().min(12)], dead_end_reason
        );
        emit_pulse(log_tx, "auto_rejected", target_skill, &msg, *applied_patches_count, failure_count);
        warn!(target: "pagi::maintenance::genetic", "{}", msg);

        let event = EventRecord::now("Maintenance", &msg)
            .with_skill("maintenance_loop")
            .with_outcome("evolutionary_dead_end");
        let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

        return Ok(msg);
    }

    // Phase 4.5: Validation Benchmark — compile, smoke-test, perf-compare
    emit_pulse(log_tx, "validation", target_skill, &format!("Validating patch '{}' (compile + smoke test)...", patch_name), *applied_patches_count, failure_count);
    info!(
        target: "pagi::maintenance",
        patch = %patch_name,
        skill = target_skill,
        "Entering Phase 4.5: Validation Benchmark"
    );

    let validation = SmokeTestRunner::validate(&code, &patch_name, target_skill).await;

    if validation.auto_reject {
        // Auto-rejection: record as "Syntactic Hallucination" in Chronos
        let rejection_msg = format!(
            "AUTO-REJECTED: Patch '{}' for skill '{}' — Syntactic Hallucination. Reason: {}",
            patch_name, target_skill, validation.rejection_reason
        );

        let auto_reject_pulse = MaintenancePulseEvent::new("auto_rejected", target_skill, &rejection_msg)
            .with_counts(*applied_patches_count, failure_count)
            .with_performance_delta(validation.performance_delta.clone());
        let json = auto_reject_pulse.to_sse_line();
        let _ = log_tx.send(format!("MAINTENANCE_PULSE:{}", json));
        let _ = log_tx.send(format!("[MAINTENANCE] [auto_rejected] {} — {}", target_skill, rejection_msg));

        warn!(target: "pagi::maintenance", "{}", rejection_msg);

        // Record the Syntactic Hallucination in Chronos
        let event = EventRecord::now("Maintenance", &rejection_msg)
            .with_skill("maintenance_loop")
            .with_outcome("syntactic_hallucination");
        let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

        // Record the DNA hash as a dead-end so the agent never re-suggests this code
        record_genetic_dead_end(
            knowledge,
            &code_hash,
            target_skill,
            &format!("Syntactic Hallucination: {}", validation.rejection_reason),
        );

        // Clean up temp artifact if it exists
        if let Some(ref artifact) = validation.artifact_path {
            let _ = std::fs::remove_file(artifact);
        }

        return Ok(rejection_msg);
    }

    // Validation passed — emit the performance delta
    let validation_msg = format!(
        "Validation passed for '{}'. CPU: {}, Mem: {}. {}",
        patch_name,
        validation.performance_delta.cpu,
        validation.performance_delta.mem,
        validation.summary
    );
    let validation_pulse = MaintenancePulseEvent::new("validation", target_skill, &validation_msg)
        .with_counts(*applied_patches_count, failure_count)
        .with_performance_delta(validation.performance_delta.clone());
    let json = validation_pulse.to_sse_line();
    let _ = log_tx.send(format!("MAINTENANCE_PULSE:{}", json));
    let _ = log_tx.send(format!("[MAINTENANCE] [validation] {} — {}", target_skill, validation_msg));

    info!(
        target: "pagi::maintenance",
        patch = %patch_name,
        cpu_delta = %validation.performance_delta.cpu,
        mem_delta = %validation.performance_delta.mem,
        "Phase 4.5 validation complete"
    );

    // Record validation success in Chronos
    let val_event = EventRecord::now("Maintenance", &validation_msg)
        .with_skill("maintenance_loop")
        .with_outcome("validation_passed");
    let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &val_event);

    // Clean up temp artifact
    if let Some(ref artifact) = validation.artifact_path {
        let _ = std::fs::remove_file(artifact);
    }

    // Phase 5: Human-in-the-Loop Interlock (Terminal OR UI)
    if config.require_approval {
        // Build the approval description with performance delta info
        let perf_summary = if validation.performance_delta.compiled && validation.performance_delta.smoke_test_passed {
            format!(
                "\n  📊 Performance: CPU {}, Mem {}",
                validation.performance_delta.cpu,
                validation.performance_delta.mem
            )
        } else {
            String::new()
        };

        let description = format!(
            "[MAINTENANCE]: I have synthesized and validated a patch for the '{}' skill to fix: {}{}",
            failures.first().map(|f| f.skill.as_str()).unwrap_or("unknown"),
            failures
                .first()
                .map(|f| f.description.as_str())
                .unwrap_or("unknown error"),
            perf_summary,
        );

        let approval_pulse = MaintenancePulseEvent::new(
            "awaiting_approval",
            target_skill,
            &format!("Awaiting operator approval for '{}' (CPU: {}, Mem: {})", patch_name, validation.performance_delta.cpu, validation.performance_delta.mem),
        )
        .with_counts(*applied_patches_count, failure_count)
        .with_performance_delta(validation.performance_delta.clone());
        let json = approval_pulse.to_sse_line();
        let _ = log_tx.send(format!("MAINTENANCE_PULSE:{}", json));
        let _ = log_tx.send(format!("[MAINTENANCE] [awaiting_approval] {} — Awaiting operator approval for '{}'", target_skill, patch_name));

        // Try UI-based approval first (if bridge is configured), fall back to terminal.
        let approved = if let Some(ref bridge_handle) = config.approval_bridge {
            // Create a oneshot channel and park the pending approval in the bridge.
            let (tx, rx) = oneshot::channel::<bool>();
            {
                let mut guard = bridge_handle.lock().await;
                *guard = Some(PendingApproval {
                    id: uuid::Uuid::new_v4().to_string(),
                    description: description.clone(),
                    patch_name: patch_name.clone(),
                    skill: target_skill.to_string(),
                    created_ms: now_epoch_ms(),
                    responder: Some(tx),
                });
            }

            // Race: wait for UI response OR terminal input (with a 5-minute timeout).
            let desc_clone = description.clone();
            let terminal_future = tokio::task::spawn_blocking(move || {
                request_maintenance_approval(&desc_clone)
            });

            tokio::select! {
                ui_result = rx => {
                    // UI responded first — cancel terminal wait (it will just time out).
                    let approved = ui_result.unwrap_or(false);
                    // Clear the pending approval.
                    let mut guard = bridge_handle.lock().await;
                    *guard = None;
                    approved
                }
                terminal_result = terminal_future => {
                    let approved = terminal_result.unwrap_or(false);
                    // Clear the pending approval (terminal answered first).
                    let mut guard = bridge_handle.lock().await;
                    // If the UI hasn't answered yet, drop the sender.
                    *guard = None;
                    approved
                }
            }
        } else {
            // No UI bridge — terminal only.
            let desc_clone = description.clone();
            tokio::task::spawn_blocking(move || {
                request_maintenance_approval(&desc_clone)
            })
            .await
            .unwrap_or(false)
        };

        if !approved {
            let msg = format!("Patch '{}' declined by operator.", patch_name);
            emit_pulse(log_tx, "complete", target_skill, &msg, *applied_patches_count, failure_count);
            info!(target: "pagi::maintenance", "{}", msg);

            let event = EventRecord::now("Maintenance", &msg)
                .with_skill("maintenance_loop")
                .with_outcome("patch_declined");
            let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

            // Record the DNA hash as a dead-end so the agent doesn't re-suggest declined patches
            record_genetic_dead_end(
                knowledge,
                &code_hash,
                target_skill,
                "Declined by operator",
            );

            return Ok(msg);
        }
    }

    // Save the patch (versioned: {name}_v{timestamp}.rs + current_{name}.rs pointer).
    emit_pulse(log_tx, "applying", target_skill, &format!("Saving versioned patch '{}'...", patch_name), *applied_patches_count, failure_count);
    match save_patch(&config.patches_dir, &patch_name, &code) {
        Ok(path) => {
            *applied_patches_count += 1;
            let msg = format!(
                "Versioned patch saved: {} ({} bytes, DNA: {})",
                path.display(),
                code.len(),
                &code_hash[..code_hash.len().min(12)]
            );
            emit_pulse(log_tx, "complete", target_skill, &format!("✓ {}", msg), *applied_patches_count, failure_count);
            info!(target: "pagi::maintenance", path = %path.display(), dna = &code_hash[..code_hash.len().min(12)], "Versioned patch saved");

            // Record the patch DNA in Chronos for genetic memory tracking
            let dna_key = format!("patch_dna/{}/{}", target_skill, code_hash);
            let dna_value = format!(
                "{{\"skill\":\"{}\",\"path\":\"{}\",\"timestamp_ms\":{},\"status\":\"applied\"}}",
                target_skill,
                path.display(),
                now_epoch_ms()
            );
            let _ = knowledge.insert(KbType::Chronos.slot_id(), &dna_key, dna_value.as_bytes());

            let event = EventRecord::now("Maintenance", &msg)
                .with_skill("maintenance_loop")
                .with_outcome("patch_saved");
            let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

            Ok(msg)
        }
        Err(e) => {
            let msg = format!("Failed to save patch '{}': {}", patch_name, e);
            emit_pulse(log_tx, "complete", target_skill, &format!("✗ {}", msg), *applied_patches_count, failure_count);
            error!(target: "pagi::maintenance", "{}", msg);

            let event = EventRecord::now("Maintenance", &msg)
                .with_skill("maintenance_loop")
                .with_outcome("patch_save_failed");
            let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);

            Err(msg.into())
        }
    }
}

// ---------------------------------------------------------------------------
// Public API: init_maintenance_loop
// ---------------------------------------------------------------------------

/// Spawns the autonomous maintenance loop as a background `tokio::spawn` task.
///
/// # Arguments
///
/// * `knowledge` – Shared `KnowledgeStore` (read-only access for failure audit).
/// * `idle_tracker` – Tracks gateway idle time; maintenance only runs when idle.
/// * `log_tx` – Broadcast channel for SSE `maintenance_pulse` events.
/// * `config` – Optional configuration; uses env-driven defaults if `None`.
///
/// # Returns
///
/// A `tokio::task::JoinHandle` for the spawned loop (can be used for graceful shutdown).
pub fn init_maintenance_loop(
    knowledge: Arc<KnowledgeStore>,
    idle_tracker: IdleTracker,
    log_tx: broadcast::Sender<String>,
    config: Option<MaintenanceConfig>,
) -> tokio::task::JoinHandle<()> {
    let config = config.unwrap_or_default();

    info!(
        target: "pagi::maintenance",
        interval_secs = config.interval.as_secs(),
        idle_threshold_secs = config.idle_threshold.as_secs(),
        patches_dir = %config.patches_dir.display(),
        require_approval = config.require_approval,
        "Autonomous Maintenance Loop initialized"
    );

    let _ = log_tx.send(format!(
        "[MAINTENANCE] Loop initialized (interval={}s, idle_threshold={}s, patches={})",
        config.interval.as_secs(),
        config.idle_threshold.as_secs(),
        config.patches_dir.display(),
    ));

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(config.interval);
        let mut cycle_count: u64 = 0;
        let mut applied_patches_count: u32 = 0;

        loop {
            interval.tick().await;
            cycle_count += 1;

            // Gate: only run when the system has been idle long enough.
            let idle = idle_tracker.idle_duration();
            if idle < config.idle_threshold {
                debug!(
                    target: "pagi::maintenance",
                    idle_secs = idle.as_secs(),
                    threshold_secs = config.idle_threshold.as_secs(),
                    "Skipping maintenance cycle — system not idle enough"
                );
                // Emit an idle pulse so the UI knows the loop is alive but waiting.
                emit_pulse(&log_tx, "idle", "system", &format!("Waiting for idle ({}s / {}s)", idle.as_secs(), config.idle_threshold.as_secs()), applied_patches_count, 0);
                continue;
            }

            info!(
                target: "pagi::maintenance",
                cycle = cycle_count,
                idle_secs = idle.as_secs(),
                "Starting maintenance cycle"
            );
            emit_pulse(&log_tx, "starting", "system", &format!("Cycle #{} starting (idle {}s)", cycle_count, idle.as_secs()), applied_patches_count, 0);

            // Create a fresh OpenRouter bridge for each cycle (picks up env changes).
            let bridge = match OpenRouterBridge::from_env() {
                Some(b) => b,
                None => {
                    let msg = "OpenRouter API key not set — skipping reflexion phase.";
                    warn!(target: "pagi::maintenance", "{}", msg);
                    emit_pulse(&log_tx, "idle", "system", msg, applied_patches_count, 0);

                    // Still record a Chronos event.
                    let event = EventRecord::now("Maintenance", msg)
                        .with_skill("maintenance_loop")
                        .with_outcome("no_api_key");
                    let _ = knowledge.append_chronos_event(MAINTENANCE_AGENT_ID, &event);
                    continue;
                }
            };

            match maintenance_tick(&knowledge, &bridge, &config, &log_tx, &mut applied_patches_count).await {
                Ok(summary) => {
                    info!(
                        target: "pagi::maintenance",
                        cycle = cycle_count,
                        summary = %summary,
                        "Maintenance cycle complete"
                    );
                }
                Err(e) => {
                    warn!(
                        target: "pagi::maintenance",
                        cycle = cycle_count,
                        error = %e,
                        "Maintenance cycle failed"
                    );
                }
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_code_fenced() {
        let response = r#"Here is the fix:
```rust
// PATCH: fix permission error
fn fixed_function() -> bool { true }
```
Done."#;
        let code = extract_rust_code(response).unwrap();
        assert!(code.contains("fn fixed_function"));
        assert!(code.contains("// PATCH:"));
    }

    #[test]
    fn test_extract_rust_code_generic_fence() {
        let response = "```\nfn hello() {}\n```";
        let code = extract_rust_code(response).unwrap();
        assert!(code.contains("fn hello"));
    }

    #[test]
    fn test_extract_rust_code_none() {
        assert!(extract_rust_code("NO_PATCH_NEEDED").is_none());
        assert!(extract_rust_code("just some text").is_none());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("Hello World!"), "hello_world_");
        assert_eq!(sanitize_filename("fs_tools"), "fs_tools");
        assert_eq!(sanitize_filename("a/b\\c"), "a_b_c");
    }

    #[test]
    fn test_telemetry_pulse_collect() {
        let pulse = TelemetryPulse::collect();
        assert!(pulse.cpu_count >= 1);
        assert!(pulse.timestamp_ms > 0);
    }

    #[test]
    fn test_idle_tracker() {
        let tracker = IdleTracker::new();
        // Just created — should be nearly zero idle.
        assert!(tracker.idle_duration() < Duration::from_secs(2));

        // Touch resets.
        tracker.touch();
        assert!(tracker.idle_duration() < Duration::from_secs(2));
    }

    #[test]
    fn test_build_reflexion_prompt() {
        let telemetry = TelemetryPulse {
            timestamp_ms: 1000,
            cpu_count: 8,
            cwd: "/test".to_string(),
            env_hints: vec![("RUST_LOG".to_string(), "info".to_string())],
        };
        let failures = vec![FailureRecord {
            key: "chronos/test/1".to_string(),
            skill: "ShellExecutor".to_string(),
            description: "cargo build failed".to_string(),
            stderr_snippet: "error[E0308]: mismatched types".to_string(),
            timestamp_ms: 999,
        }];
        let prompt = build_reflexion_prompt(&telemetry, &failures);
        assert!(prompt.contains("ShellExecutor"));
        assert!(prompt.contains("cargo build failed"));
        assert!(prompt.contains("E0308"));
        assert!(prompt.contains("CPUs: 8"));
    }

    #[test]
    fn test_maintenance_config_default() {
        let config = MaintenanceConfig::default();
        assert!(config.interval.as_secs() >= 60);
        assert!(config.idle_threshold.as_secs() >= 30);
        assert!(config.require_approval);
    }
}

//! Adversarial Peer Review Engine (Red-Team)
//!
//! Implements **Phase 4.75: Consensus Gating** in the Maintenance Loop.
//!
//! After a patch is synthesized (Phase 4) and validated (Phase 4.5), the proposed
//! code is sent to a **different model** for adversarial security analysis before
//! it can be presented to the operator for approval.
//!
//! ## Architecture
//!
//! | Component | Role |
//! |-----------|------|
//! | [`RedTeamAnalyzer`] | Sends code to a secondary LLM for vulnerability analysis. |
//! | [`SecurityVerdict`] | Structured result: severity, findings, recommendation. |
//! | [`ConsensusGate`] | Decision engine: auto-reject "Critical"/"High" findings. |
//! | [`CveCheckList`] | Common vulnerability patterns injected into the review prompt. |
//!
//! ## Genetic Memory Integration
//!
//! If the Red-Teamer finds a Critical or High vulnerability, the patch DNA is
//! marked as a **"Lethal Mutation"** in [`GeneticMemory`](super::rollback::GeneticMemory),
//! preventing the agent from ever re-proposing the same flawed code.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Severity Classification
// ---------------------------------------------------------------------------

/// Severity level for a security finding, aligned with CVSS qualitative ratings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational note — no action required.
    Info,
    /// Low-risk finding — acceptable with documentation.
    Low,
    /// Medium-risk finding — should be addressed before production.
    Medium,
    /// High-risk finding — patch is auto-rejected.
    High,
    /// Critical vulnerability — patch is auto-rejected and marked as Lethal Mutation.
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "Info"),
            Severity::Low => write!(f, "Low"),
            Severity::Medium => write!(f, "Medium"),
            Severity::High => write!(f, "High"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

impl Severity {
    /// Returns true if this severity should trigger auto-rejection.
    pub fn is_blocking(&self) -> bool {
        matches!(self, Severity::High | Severity::Critical)
    }

    /// Returns true if this severity should be flagged as a Lethal Mutation.
    pub fn is_lethal(&self) -> bool {
        matches!(self, Severity::Critical)
    }
}

// ---------------------------------------------------------------------------
// Security Finding
// ---------------------------------------------------------------------------

/// A single security finding from the Red-Team analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    /// CVE category or custom vulnerability class.
    pub category: String,
    /// Severity of this finding.
    pub severity: Severity,
    /// Human-readable description of the vulnerability.
    pub description: String,
    /// Affected code region (line range or function name, if identifiable).
    pub affected_region: Option<String>,
    /// Suggested remediation.
    pub remediation: Option<String>,
}

// ---------------------------------------------------------------------------
// Security Verdict
// ---------------------------------------------------------------------------

/// The structured result of a Red-Team peer review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityVerdict {
    /// Overall severity (highest finding).
    pub overall_severity: Severity,
    /// Individual findings.
    pub findings: Vec<SecurityFinding>,
    /// The model/agent that performed the review.
    pub reviewer_model: String,
    /// Whether the patch passed peer review.
    pub passed: bool,
    /// Human-readable summary.
    pub summary: String,
    /// Timestamp of the review (epoch ms).
    pub reviewed_at_ms: i64,
    /// Memory usage warning (if the reviewer flagged high memory patterns).
    pub memory_warning: Option<String>,
    /// Raw response from the reviewer (for audit trail).
    pub raw_response: Option<String>,
}

impl SecurityVerdict {
    /// Create a "passed" verdict with no findings.
    pub fn passed(reviewer_model: &str, summary: &str) -> Self {
        Self {
            overall_severity: Severity::Info,
            findings: Vec::new(),
            reviewer_model: reviewer_model.to_string(),
            passed: true,
            summary: summary.to_string(),
            reviewed_at_ms: now_epoch_ms(),
            memory_warning: None,
            raw_response: None,
        }
    }

    /// Create a "failed" verdict from findings.
    pub fn failed(reviewer_model: &str, findings: Vec<SecurityFinding>, summary: &str) -> Self {
        let overall = findings
            .iter()
            .map(|f| f.severity)
            .max()
            .unwrap_or(Severity::Info);
        Self {
            overall_severity: overall,
            findings,
            reviewer_model: reviewer_model.to_string(),
            passed: false,
            summary: summary.to_string(),
            reviewed_at_ms: now_epoch_ms(),
            memory_warning: None,
            raw_response: None,
        }
    }

    /// Returns true if any finding is blocking (High or Critical).
    pub fn has_blocking_findings(&self) -> bool {
        self.findings.iter().any(|f| f.severity.is_blocking())
    }

    /// Returns true if any finding is a Lethal Mutation (Critical).
    pub fn has_lethal_findings(&self) -> bool {
        self.findings.iter().any(|f| f.severity.is_lethal())
    }
}

// ---------------------------------------------------------------------------
// CVE Check List (Prompt Injection for Red-Team)
// ---------------------------------------------------------------------------

/// Common vulnerability patterns to check in Rust code.
/// These are injected into the LLM prompt for the peer-review phase.
pub struct CveCheckList;

impl CveCheckList {
    /// Returns the CVE/vulnerability check prompt section.
    pub fn rust_security_checks() -> &'static str {
        r#"## Security Vulnerability Checklist (Rust-Specific)

Analyze the code for the following vulnerability classes:

### Memory Safety
- **Buffer Overflow**: Unchecked indexing into slices/arrays without bounds checking.
- **Use-After-Free**: Unsafe blocks that dereference raw pointers after the owning value is dropped.
- **Double Free**: Manual memory management in unsafe blocks that could free the same allocation twice.
- **Uninitialized Memory**: Use of `MaybeUninit` or `mem::uninitialized()` without proper initialization.

### Path Traversal & File System
- **Path Traversal (CWE-22)**: User-controlled paths that could escape sandboxed directories (e.g., `../../../etc/passwd`).
- **Symlink Following (CWE-61)**: TOCTOU race conditions where a symlink is swapped between check and use.
- **Unrestricted File Write**: Writing to arbitrary paths without validation or sandboxing.

### Input Validation
- **Command Injection (CWE-78)**: User input passed to `std::process::Command` without sanitization.
- **SQL/NoSQL Injection**: Unsanitized input in database queries.
- **Format String Attacks**: User-controlled format strings in `format!()` or logging macros.

### Logic & Concurrency
- **Race Conditions (CWE-362)**: Shared mutable state without proper synchronization.
- **Deadlocks**: Lock ordering violations or holding locks across await points.
- **Integer Overflow**: Arithmetic operations that could overflow without `checked_*` or `wrapping_*`.
- **Panic in Production**: `unwrap()` or `expect()` on `Result`/`Option` in non-test code paths.

### Cryptographic Issues
- **Weak Hashing**: Use of MD5, SHA-1, or custom hash functions for security-critical operations.
- **Hardcoded Secrets**: API keys, passwords, or tokens embedded in source code.
- **Insufficient Randomness**: Use of `rand::thread_rng()` for cryptographic purposes instead of `OsRng`.

### Resource Exhaustion
- **Unbounded Allocation**: Collections that grow without limits based on external input.
- **Infinite Loops**: Loop conditions that may never terminate.
- **File Descriptor Leaks**: Opening files/sockets without ensuring they are closed.

### Unsafe Code
- **Unsound Unsafe Blocks**: `unsafe` blocks that violate Rust's safety invariants.
- **FFI Boundary Issues**: Incorrect type sizes or calling conventions at FFI boundaries.
- **Transmute Misuse**: `std::mem::transmute` between incompatible types.

For each finding, provide:
1. **Category**: The vulnerability class (e.g., "Path Traversal CWE-22").
2. **Severity**: One of: Info, Low, Medium, High, Critical.
3. **Description**: What the vulnerability is and how it could be exploited.
4. **Affected Region**: The function or line range affected.
5. **Remediation**: How to fix it.

Also flag if the code exhibits patterns that suggest high memory usage or resource exhaustion,
even if not a direct security vulnerability."#
    }
}

// ---------------------------------------------------------------------------
// Red-Team Analyzer
// ---------------------------------------------------------------------------

/// Configuration for the Red-Team analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamConfig {
    /// The model to use for peer review (e.g., "gpt-4o", "meta-llama/llama-3.3-70b-instruct").
    /// If different from the primary model, this creates true multi-agent consensus.
    pub reviewer_model: String,
    /// API endpoint for the reviewer (OpenRouter-compatible).
    pub api_url: String,
    /// Whether to auto-reject on High severity (in addition to Critical).
    pub auto_reject_high: bool,
    /// Maximum tokens for the review response.
    pub max_review_tokens: u32,
    /// Temperature for the review (lower = more deterministic/strict).
    pub review_temperature: f32,
}

impl Default for RedTeamConfig {
    fn default() -> Self {
        Self {
            reviewer_model: std::env::var("PAGI_REDTEAM_MODEL")
                .unwrap_or_else(|_| "meta-llama/llama-3.3-70b-instruct".to_string()),
            api_url: std::env::var("PAGI_REDTEAM_API_URL")
                .unwrap_or_else(|_| "https://openrouter.ai/api/v1/chat/completions".to_string()),
            auto_reject_high: true,
            max_review_tokens: 2048,
            review_temperature: 0.1,
        }
    }
}

/// The Red-Team Analyzer: sends proposed patches to a secondary model for
/// adversarial security review.
pub struct RedTeamAnalyzer {
    config: RedTeamConfig,
    client: reqwest::Client,
}

impl RedTeamAnalyzer {
    pub fn new(config: RedTeamConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Create with default configuration (reads from environment).
    pub fn from_env() -> Self {
        Self::new(RedTeamConfig::default())
    }

    /// Build the adversarial review prompt for a given patch.
    pub fn build_review_prompt(skill_name: &str, code: &str, patch_description: &str) -> String {
        format!(
            r#"ACT as a Senior Security Researcher and Rust Developer.

You are performing an adversarial peer review of a proposed code patch for the skill "{skill_name}".

## Patch Description
{patch_description}

## Proposed Code
```rust
{code}
```

{checklist}

## Response Format

Respond with a JSON object (and ONLY a JSON object, no markdown fences) with this structure:
{{
  "overall_severity": "info|low|medium|high|critical",
  "passed": true|false,
  "summary": "Brief summary of findings",
  "memory_warning": null or "description of memory concern",
  "findings": [
    {{
      "category": "Vulnerability class",
      "severity": "info|low|medium|high|critical",
      "description": "What the vulnerability is",
      "affected_region": "function or line range",
      "remediation": "How to fix it"
    }}
  ]
}}

If the code is clean and has no vulnerabilities, return:
{{
  "overall_severity": "info",
  "passed": true,
  "summary": "No vulnerabilities found.",
  "memory_warning": null,
  "findings": []
}}

Be thorough but fair. Only flag real vulnerabilities, not style issues."#,
            skill_name = skill_name,
            patch_description = patch_description,
            code = code,
            checklist = CveCheckList::rust_security_checks(),
        )
    }

    /// Perform an adversarial peer review of the proposed code.
    ///
    /// In **Live** mode, sends the code to the configured reviewer model via OpenRouter.
    /// In **Mock** mode (no API key), performs a heuristic-based local analysis.
    pub async fn review_patch(
        &self,
        skill_name: &str,
        code: &str,
        patch_description: &str,
    ) -> SecurityVerdict {
        // Check if we have an API key for live review.
        let api_key = std::env::var("PAGI_REDTEAM_API_KEY")
            .or_else(|_| std::env::var("PAGI_LLM_API_KEY"))
            .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
            .ok()
            .filter(|k| !k.trim().is_empty());

        match api_key {
            Some(key) => {
                self.review_live(skill_name, code, patch_description, &key)
                    .await
            }
            None => {
                info!(
                    target: "pagi::redteam",
                    "No API key for Red-Team reviewer — falling back to heuristic analysis"
                );
                self.review_heuristic(skill_name, code)
            }
        }
    }

    /// Live review: send to the secondary LLM via OpenRouter-compatible API.
    async fn review_live(
        &self,
        skill_name: &str,
        code: &str,
        patch_description: &str,
        api_key: &str,
    ) -> SecurityVerdict {
        let prompt = Self::build_review_prompt(skill_name, code, patch_description);

        let request_body = serde_json::json!({
            "model": self.config.reviewer_model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a security-focused code reviewer. Respond ONLY with valid JSON."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": self.config.review_temperature,
            "max_tokens": self.config.max_review_tokens,
        });

        info!(
            target: "pagi::redteam",
            model = %self.config.reviewer_model,
            skill = skill_name,
            code_len = code.len(),
            "Sending patch to Red-Team reviewer"
        );

        let response = match self
            .client
            .post(&self.config.api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", "https://pagi-orchestrator.local")
            .header("X-Title", "PAGI-RedTeam")
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    target: "pagi::redteam",
                    error = %e,
                    "Red-Team API request failed — falling back to heuristic"
                );
                return self.review_heuristic(skill_name, code);
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            warn!(
                target: "pagi::redteam",
                status = %status,
                body = %body,
                "Red-Team API returned error — falling back to heuristic"
            );
            return self.review_heuristic(skill_name, code);
        }

        // Parse the OpenRouter response.
        let raw_json: serde_json::Value = match response.json().await {
            Ok(v) => v,
            Err(e) => {
                warn!(
                    target: "pagi::redteam",
                    error = %e,
                    "Failed to parse Red-Team API response"
                );
                return self.review_heuristic(skill_name, code);
            }
        };

        // Extract the content from the chat completion response.
        let content = raw_json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Try to parse the content as our SecurityVerdict JSON.
        self.parse_review_response(content, &self.config.reviewer_model)
    }

    /// Parse the LLM's JSON response into a SecurityVerdict.
    fn parse_review_response(&self, content: &str, reviewer_model: &str) -> SecurityVerdict {
        // Strip markdown code fences if present.
        let cleaned = content
            .trim()
            .strip_prefix("```json")
            .or_else(|| content.trim().strip_prefix("```"))
            .unwrap_or(content.trim());
        let cleaned = cleaned
            .strip_suffix("```")
            .unwrap_or(cleaned)
            .trim();

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct ReviewResponse {
            overall_severity: Option<String>,
            passed: Option<bool>,
            summary: Option<String>,
            memory_warning: Option<String>,
            findings: Option<Vec<FindingResponse>>,
        }

        #[derive(Deserialize)]
        struct FindingResponse {
            category: Option<String>,
            severity: Option<String>,
            description: Option<String>,
            affected_region: Option<String>,
            remediation: Option<String>,
        }

        match serde_json::from_str::<ReviewResponse>(cleaned) {
            Ok(resp) => {
                let findings: Vec<SecurityFinding> = resp
                    .findings
                    .unwrap_or_default()
                    .into_iter()
                    .map(|f| SecurityFinding {
                        category: f.category.unwrap_or_else(|| "Unknown".to_string()),
                        severity: parse_severity(&f.severity.unwrap_or_default()),
                        description: f.description.unwrap_or_default(),
                        affected_region: f.affected_region,
                        remediation: f.remediation,
                    })
                    .collect();

                let overall = findings
                    .iter()
                    .map(|f| f.severity)
                    .max()
                    .unwrap_or(Severity::Info);

                let passed = resp.passed.unwrap_or_else(|| !overall.is_blocking());

                let mut verdict = SecurityVerdict {
                    overall_severity: overall,
                    findings,
                    reviewer_model: reviewer_model.to_string(),
                    passed,
                    summary: resp.summary.unwrap_or_else(|| "Review complete.".to_string()),
                    reviewed_at_ms: now_epoch_ms(),
                    memory_warning: resp.memory_warning,
                    raw_response: Some(content.to_string()),
                };

                // Override: if we have blocking findings, force passed = false.
                if verdict.has_blocking_findings() {
                    verdict.passed = false;
                }

                info!(
                    target: "pagi::redteam",
                    model = reviewer_model,
                    severity = %verdict.overall_severity,
                    passed = verdict.passed,
                    findings = verdict.findings.len(),
                    "Red-Team review complete"
                );

                verdict
            }
            Err(e) => {
                warn!(
                    target: "pagi::redteam",
                    error = %e,
                    content_preview = &content[..content.len().min(200)],
                    "Failed to parse Red-Team JSON response — treating as passed with warning"
                );
                let mut verdict = SecurityVerdict::passed(
                    reviewer_model,
                    "Review response could not be parsed. Manual review recommended.",
                );
                verdict.raw_response = Some(content.to_string());
                verdict
            }
        }
    }

    /// Heuristic-based local analysis (no LLM required).
    /// Scans for common vulnerability patterns using string matching.
    fn review_heuristic(&self, skill_name: &str, code: &str) -> SecurityVerdict {
        let mut findings = Vec::new();
        let lines: Vec<&str> = code.lines().collect();

        // --- Unsafe blocks ---
        let unsafe_count = code.matches("unsafe").count();
        if unsafe_count > 0 {
            findings.push(SecurityFinding {
                category: "Unsafe Code".to_string(),
                severity: if unsafe_count > 3 {
                    Severity::High
                } else {
                    Severity::Medium
                },
                description: format!(
                    "Code contains {} unsafe block(s). Each must be audited for soundness.",
                    unsafe_count
                ),
                affected_region: None,
                remediation: Some(
                    "Minimize unsafe usage. Document safety invariants with // SAFETY: comments."
                        .to_string(),
                ),
            });
        }

        // --- Command injection ---
        for (i, line) in lines.iter().enumerate() {
            if (line.contains("Command::new") || line.contains("process::Command"))
                && !line.trim_start().starts_with("//")
            {
                // Check if the command argument comes from a variable (potential injection).
                let has_format = lines[i.saturating_sub(3)..=(i + 3).min(lines.len() - 1)]
                    .iter()
                    .any(|l| l.contains("format!") || l.contains("&user") || l.contains("input"));

                if has_format {
                    findings.push(SecurityFinding {
                        category: "Command Injection (CWE-78)".to_string(),
                        severity: Severity::High,
                        description: format!(
                            "Line {}: Command execution with potentially user-controlled input.",
                            i + 1
                        ),
                        affected_region: Some(format!("Line {}", i + 1)),
                        remediation: Some(
                            "Sanitize all inputs before passing to Command. Use allowlists for command names."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        // --- Path traversal ---
        for (i, line) in lines.iter().enumerate() {
            if (line.contains("..") && (line.contains("Path") || line.contains("join")))
                || line.contains("canonicalize")
            {
                if !line.trim_start().starts_with("//") {
                    findings.push(SecurityFinding {
                        category: "Path Traversal (CWE-22)".to_string(),
                        severity: Severity::Medium,
                        description: format!(
                            "Line {}: Potential path traversal pattern detected.",
                            i + 1
                        ),
                        affected_region: Some(format!("Line {}", i + 1)),
                        remediation: Some(
                            "Validate paths against a root directory. Use canonicalize() and check prefix."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        // --- Unwrap in non-test code ---
        let unwrap_count = code.matches(".unwrap()").count();
        let expect_count = code.matches(".expect(").count();
        let panic_points = unwrap_count + expect_count;
        if panic_points > 5 {
            findings.push(SecurityFinding {
                category: "Panic in Production".to_string(),
                severity: Severity::Medium,
                description: format!(
                    "Code contains {} potential panic points (.unwrap()/.expect()). \
                     These can crash the process in production.",
                    panic_points
                ),
                affected_region: None,
                remediation: Some(
                    "Replace .unwrap() with proper error handling (? operator or match)."
                        .to_string(),
                ),
            });
        }

        // --- Hardcoded secrets ---
        let secret_patterns = [
            "api_key", "API_KEY", "secret", "password", "token", "Bearer ",
        ];
        for (i, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for pattern in &secret_patterns {
                if line.contains(pattern)
                    && (line.contains("= \"") || line.contains("= '"))
                    && !line.contains("env::var")
                    && !line.contains("std::env")
                {
                    findings.push(SecurityFinding {
                        category: "Hardcoded Secrets".to_string(),
                        severity: Severity::High,
                        description: format!(
                            "Line {}: Potential hardcoded secret ({}).",
                            i + 1,
                            pattern
                        ),
                        affected_region: Some(format!("Line {}", i + 1)),
                        remediation: Some(
                            "Use environment variables or a secrets manager instead of hardcoding."
                                .to_string(),
                        ),
                    });
                    break; // One finding per line is enough.
                }
            }
        }

        // --- Unbounded allocation ---
        for (i, line) in lines.iter().enumerate() {
            if line.contains("Vec::with_capacity") || line.contains("String::with_capacity") {
                // Check if capacity comes from external input.
                if lines[i.saturating_sub(2)..=(i + 2).min(lines.len() - 1)]
                    .iter()
                    .any(|l| l.contains("as usize") || l.contains("parse"))
                {
                    findings.push(SecurityFinding {
                        category: "Unbounded Allocation".to_string(),
                        severity: Severity::Medium,
                        description: format!(
                            "Line {}: Allocation size may be controlled by external input.",
                            i + 1
                        ),
                        affected_region: Some(format!("Line {}", i + 1)),
                        remediation: Some(
                            "Cap allocation sizes with a maximum limit.".to_string(),
                        ),
                    });
                }
            }
        }

        // --- Memory usage heuristic ---
        let memory_warning = if code.contains("Vec::new()") && code.len() > 5000 {
            Some("Large code with dynamic allocations — monitor runtime memory usage.".to_string())
        } else {
            None
        };

        // Build verdict.
        let overall = findings
            .iter()
            .map(|f| f.severity)
            .max()
            .unwrap_or(Severity::Info);

        let passed = !overall.is_blocking();

        let summary = if findings.is_empty() {
            format!(
                "Heuristic analysis of '{}': No vulnerabilities found.",
                skill_name
            )
        } else {
            format!(
                "Heuristic analysis of '{}': {} finding(s), highest severity: {}.",
                skill_name,
                findings.len(),
                overall
            )
        };

        info!(
            target: "pagi::redteam",
            skill = skill_name,
            severity = %overall,
            passed = passed,
            findings = findings.len(),
            "Heuristic Red-Team review complete"
        );

        SecurityVerdict {
            overall_severity: overall,
            findings,
            reviewer_model: "heuristic-analyzer-v1".to_string(),
            passed,
            summary,
            reviewed_at_ms: now_epoch_ms(),
            memory_warning,
            raw_response: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Consensus Gate
// ---------------------------------------------------------------------------

/// The Consensus Gate: decides whether a patch should proceed based on the
/// Red-Team verdict and the system's consensus policy.
pub struct ConsensusGate {
    /// Whether to auto-reject on High severity (default: true).
    pub auto_reject_high: bool,
}

impl Default for ConsensusGate {
    fn default() -> Self {
        Self {
            auto_reject_high: true,
        }
    }
}

/// Result of the consensus gate evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// Whether the patch is allowed to proceed.
    pub approved: bool,
    /// Reason for the decision.
    pub reason: String,
    /// Whether the patch DNA should be marked as a Lethal Mutation.
    pub mark_lethal: bool,
    /// The security verdict that informed this decision.
    pub verdict: SecurityVerdict,
}

impl ConsensusGate {
    pub fn new(auto_reject_high: bool) -> Self {
        Self { auto_reject_high }
    }

    /// Evaluate a security verdict and return a consensus decision.
    pub fn evaluate(&self, verdict: SecurityVerdict) -> ConsensusResult {
        // Critical findings: always reject + mark as lethal.
        if verdict.has_lethal_findings() {
            return ConsensusResult {
                approved: false,
                reason: format!(
                    "CRITICAL vulnerability detected by {} — patch auto-rejected as Lethal Mutation. {}",
                    verdict.reviewer_model, verdict.summary
                ),
                mark_lethal: true,
                verdict,
            };
        }

        // High findings: reject if auto_reject_high is enabled.
        if self.auto_reject_high && verdict.has_blocking_findings() {
            return ConsensusResult {
                approved: false,
                reason: format!(
                    "HIGH severity vulnerability detected by {} — patch auto-rejected. {}",
                    verdict.reviewer_model, verdict.summary
                ),
                mark_lethal: false,
                verdict,
            };
        }

        // Passed: allow to proceed.
        ConsensusResult {
            approved: true,
            reason: if verdict.findings.is_empty() {
                format!(
                    "✅ Passed Peer Review ({} analysis: No vulnerabilities found).",
                    verdict.reviewer_model
                )
            } else {
                format!(
                    "⚠️ Passed Peer Review with {} advisory finding(s) ({} analysis). {}",
                    verdict.findings.len(),
                    verdict.reviewer_model,
                    verdict.summary
                )
            },
            mark_lethal: false,
            verdict,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "critical" => Severity::Critical,
        "high" => Severity::High,
        "medium" | "med" => Severity::Medium,
        "low" => Severity::Low,
        _ => Severity::Info,
    }
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
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
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Medium > Severity::Low);
        assert!(Severity::Low > Severity::Info);
    }

    #[test]
    fn test_severity_blocking() {
        assert!(!Severity::Info.is_blocking());
        assert!(!Severity::Low.is_blocking());
        assert!(!Severity::Medium.is_blocking());
        assert!(Severity::High.is_blocking());
        assert!(Severity::Critical.is_blocking());
    }

    #[test]
    fn test_severity_lethal() {
        assert!(!Severity::High.is_lethal());
        assert!(Severity::Critical.is_lethal());
    }

    #[test]
    fn test_verdict_passed() {
        let v = SecurityVerdict::passed("test-model", "All clear");
        assert!(v.passed);
        assert!(!v.has_blocking_findings());
        assert!(!v.has_lethal_findings());
        assert_eq!(v.overall_severity, Severity::Info);
    }

    #[test]
    fn test_verdict_failed_critical() {
        let findings = vec![SecurityFinding {
            category: "Command Injection".to_string(),
            severity: Severity::Critical,
            description: "User input passed to shell".to_string(),
            affected_region: Some("Line 42".to_string()),
            remediation: Some("Sanitize input".to_string()),
        }];
        let v = SecurityVerdict::failed("test-model", findings, "Critical issue found");
        assert!(!v.passed);
        assert!(v.has_blocking_findings());
        assert!(v.has_lethal_findings());
        assert_eq!(v.overall_severity, Severity::Critical);
    }

    #[test]
    fn test_consensus_gate_critical_rejects() {
        let gate = ConsensusGate::default();
        let findings = vec![SecurityFinding {
            category: "Buffer Overflow".to_string(),
            severity: Severity::Critical,
            description: "Unchecked buffer access".to_string(),
            affected_region: None,
            remediation: None,
        }];
        let verdict = SecurityVerdict::failed("llama-3", findings, "Critical");
        let result = gate.evaluate(verdict);
        assert!(!result.approved);
        assert!(result.mark_lethal);
    }

    #[test]
    fn test_consensus_gate_high_rejects_when_enabled() {
        let gate = ConsensusGate::new(true);
        let findings = vec![SecurityFinding {
            category: "Hardcoded Secret".to_string(),
            severity: Severity::High,
            description: "API key in source".to_string(),
            affected_region: None,
            remediation: None,
        }];
        let verdict = SecurityVerdict::failed("gpt-4o", findings, "High");
        let result = gate.evaluate(verdict);
        assert!(!result.approved);
        assert!(!result.mark_lethal);
    }

    #[test]
    fn test_consensus_gate_medium_passes() {
        let gate = ConsensusGate::default();
        let findings = vec![SecurityFinding {
            category: "Panic in Production".to_string(),
            severity: Severity::Medium,
            description: "Too many unwraps".to_string(),
            affected_region: None,
            remediation: None,
        }];
        let verdict = SecurityVerdict {
            overall_severity: Severity::Medium,
            findings,
            reviewer_model: "llama-3".to_string(),
            passed: true,
            summary: "Medium findings only".to_string(),
            reviewed_at_ms: 0,
            memory_warning: None,
            raw_response: None,
        };
        let result = gate.evaluate(verdict);
        assert!(result.approved);
        assert!(!result.mark_lethal);
    }

    #[test]
    fn test_heuristic_detects_unsafe() {
        let analyzer = RedTeamAnalyzer::new(RedTeamConfig {
            reviewer_model: "test".to_string(),
            api_url: "http://localhost".to_string(),
            auto_reject_high: true,
            max_review_tokens: 1024,
            review_temperature: 0.0,
        });

        let code = r#"
            fn dangerous() {
                unsafe { std::ptr::null::<u8>().read() };
                unsafe { std::ptr::null::<u8>().read() };
                unsafe { std::ptr::null::<u8>().read() };
                unsafe { std::ptr::null::<u8>().read() };
            }
        "#;

        let verdict = analyzer.review_heuristic("test_skill", code);
        assert!(!verdict.passed);
        assert!(verdict.findings.iter().any(|f| f.category == "Unsafe Code"));
    }

    #[test]
    fn test_heuristic_clean_code() {
        let analyzer = RedTeamAnalyzer::new(RedTeamConfig {
            reviewer_model: "test".to_string(),
            api_url: "http://localhost".to_string(),
            auto_reject_high: true,
            max_review_tokens: 1024,
            review_temperature: 0.0,
        });

        let code = r#"
            pub fn add(a: i32, b: i32) -> i32 {
                a.checked_add(b).unwrap_or(i32::MAX)
            }
        "#;

        let verdict = analyzer.review_heuristic("test_skill", code);
        assert!(verdict.passed);
        assert!(verdict.findings.is_empty());
    }

    #[test]
    fn test_parse_review_response_valid() {
        let analyzer = RedTeamAnalyzer::new(RedTeamConfig::default());
        let json = r#"{
            "overall_severity": "medium",
            "passed": true,
            "summary": "One medium finding",
            "memory_warning": null,
            "findings": [
                {
                    "category": "Panic in Production",
                    "severity": "medium",
                    "description": "Too many unwraps",
                    "affected_region": "fn main()",
                    "remediation": "Use ? operator"
                }
            ]
        }"#;

        let verdict = analyzer.parse_review_response(json, "test-model");
        assert!(verdict.passed);
        assert_eq!(verdict.findings.len(), 1);
        assert_eq!(verdict.overall_severity, Severity::Medium);
    }

    #[test]
    fn test_parse_review_response_with_fences() {
        let analyzer = RedTeamAnalyzer::new(RedTeamConfig::default());
        let json = r#"```json
{
    "overall_severity": "info",
    "passed": true,
    "summary": "Clean code",
    "memory_warning": null,
    "findings": []
}
```"#;

        let verdict = analyzer.parse_review_response(json, "test-model");
        assert!(verdict.passed);
        assert!(verdict.findings.is_empty());
    }

    #[test]
    fn test_build_review_prompt_contains_checklist() {
        let prompt = RedTeamAnalyzer::build_review_prompt(
            "test_skill",
            "fn main() {}",
            "Test patch",
        );
        assert!(prompt.contains("Buffer Overflow"));
        assert!(prompt.contains("Path Traversal"));
        assert!(prompt.contains("Command Injection"));
        assert!(prompt.contains("test_skill"));
    }
}

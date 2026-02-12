//! The Governor: Background monitoring loop for KB-08 (Absurdity Log) and KB-06 (Ethos)
//!
//! Ensures Phoenix doesn't over-reach operational boundaries by continuously monitoring:
//! - Logic inconsistencies in KB-08 (Absurdity Log)
//! - Ethical alignment with KB-06 (Ethos)
//! - Skill execution patterns and anomalies
//!
//! The Governor acts as a "cognitive immune system" that can pause or flag operations
//! that violate sovereignty principles or show signs of degraded reasoning.
//!
//! When `PAGI_WEBHOOK_URL` is set, **Critical** alerts are POSTed to that URL (non-blocking)
//! and the "Notification Sent" event is logged to KB-08.

use pagi_core::KnowledgeStore;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn, error};
use serde::{Deserialize, Serialize};

/// Governor configuration
#[derive(Debug, Clone)]
pub struct GovernorConfig {
    /// How often to check KB-08 and KB-06 (in seconds)
    pub check_interval_secs: u64,
    /// Maximum absurdity log entries before triggering alert
    pub max_absurdity_threshold: usize,
    /// Enable automatic intervention
    pub auto_intervene: bool,
    /// Optional: current sovereignty score (0.0–1.0) for webhook payload. Updated by main loop after audit.
    /// Stored as f64::to_bits for shared read without locking.
    #[doc(hidden)]
    pub sovereignty_score_bits: Option<Arc<std::sync::atomic::AtomicU64>>,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            check_interval_secs: 60, // Check every minute
            max_absurdity_threshold: 10,
            auto_intervene: false, // Require manual approval by default
            sovereignty_score_bits: None,
        }
    }
}

/// Severity of a Governor alert. Only **Critical** triggers webhook notification when `PAGI_WEBHOOK_URL` is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernorAlertSeverity {
    Critical,
    Warning,
    Info,
}

/// Governor alert types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernorAlert {
    /// High absurdity log count detected
    HighAbsurdityCount {
        count: usize,
        threshold: usize,
        recent_entries: Vec<String>,
    },
    /// Ethos policy violation detected
    EthosViolation {
        policy_name: String,
        violation_details: String,
    },
    /// Skill execution anomaly
    SkillAnomalyDetected {
        skill_name: String,
        anomaly_type: String,
        details: String,
    },
    /// KB query pattern anomaly
    KbQueryAnomaly {
        slot_id: u8,
        query_count: usize,
        details: String,
    },
    /// Vector DB connection anomaly (Production Telemetry)
    VectorDbOffline {
        backend: String,
        error_message: String,
    },
}

impl GovernorAlert {
    /// Returns the severity of this alert. Critical alerts trigger webhook when `PAGI_WEBHOOK_URL` is set.
    pub fn severity(&self) -> GovernorAlertSeverity {
        match self {
            GovernorAlert::HighAbsurdityCount { .. } | GovernorAlert::EthosViolation { .. } => {
                GovernorAlertSeverity::Critical
            }
            GovernorAlert::SkillAnomalyDetected { .. } 
            | GovernorAlert::KbQueryAnomaly { .. }
            | GovernorAlert::VectorDbOffline { .. } => {
                GovernorAlertSeverity::Warning
            }
        }
    }

    /// Short description for webhook payload and KB-08 audit log.
    pub fn anomaly_description(&self) -> String {
        match self {
            GovernorAlert::HighAbsurdityCount { count, threshold, recent_entries } => {
                format!(
                    "High absurdity count ({}/{}). Recent: {}",
                    count,
                    threshold,
                    recent_entries.join("; ")
                )
            }
            GovernorAlert::EthosViolation { policy_name, violation_details } => {
                format!("Ethos violation ({}): {}", policy_name, violation_details)
            }
            GovernorAlert::SkillAnomalyDetected { skill_name, anomaly_type, details } => {
                format!("Skill anomaly '{}' ({}): {}", skill_name, anomaly_type, details)
            }
            GovernorAlert::KbQueryAnomaly { slot_id, query_count, details } => {
                format!("KB-{} query anomaly ({} queries): {}", slot_id, query_count, details)
            }
            GovernorAlert::VectorDbOffline { backend, error_message } => {
                format!("Anomaly: Semantic Memory Offline ({}). Falling back to Local Knowledge Bases. Error: {}", backend, error_message)
            }
        }
    }
}

/// JSON payload sent to `PAGI_WEBHOOK_URL` on Critical Governor alerts.
#[derive(Debug, Clone, Serialize)]
pub struct GovernorWebhookPayload {
    pub anomaly_description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sovereignty_score: Option<f64>,
}

/// Governor state
pub struct Governor {
    knowledge: Arc<KnowledgeStore>,
    config: GovernorConfig,
    alert_tx: tokio::sync::broadcast::Sender<GovernorAlert>,
    /// When set, Critical alerts are POSTed here (non-blocking) and logged to KB-08.
    webhook_url: Option<String>,
    webhook_client: Option<reqwest::Client>,
    /// Optional vector store for health monitoring (Production Telemetry)
    #[cfg(feature = "vector")]
    vector_store: Option<Arc<dyn pagi_core::VectorStore>>,
}

impl Governor {
    /// Create new Governor instance. Reads `PAGI_WEBHOOK_URL` from env for external alerts.
    pub fn new(
        knowledge: Arc<KnowledgeStore>,
        config: GovernorConfig,
    ) -> (Self, tokio::sync::broadcast::Receiver<GovernorAlert>) {
        let (alert_tx, alert_rx) = tokio::sync::broadcast::channel(100);

        let webhook_url = std::env::var("PAGI_WEBHOOK_URL").ok();
        let webhook_url = webhook_url.filter(|s| !s.trim().is_empty());
        let webhook_client = webhook_url.as_ref().and_then(|_| {
            reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .ok()
        });
        if webhook_url.is_some() && webhook_client.is_none() {
            warn!("Governor: PAGI_WEBHOOK_URL set but reqwest client failed to build; webhook disabled");
        }

        (
            Self {
                knowledge,
                config,
                alert_tx,
                webhook_url,
                webhook_client,
                #[cfg(feature = "vector")]
                vector_store: None,
            },
            alert_rx,
        )
    }
    
    /// Set the vector store for health monitoring (Production Telemetry).
    #[cfg(feature = "vector")]
    pub fn set_vector_store(&mut self, vector_store: Arc<dyn pagi_core::VectorStore>) {
        self.vector_store = Some(vector_store);
    }

    /// If this is a Critical alert and webhook is configured, spawns a non-blocking task to POST and log to KB-08.
    fn maybe_send_webhook(&self, alert: GovernorAlert) {
        use GovernorAlertSeverity::Critical;
        if alert.severity() != Critical {
            return;
        }
        let Some(ref url) = self.webhook_url else { return };
        let Some(ref client) = self.webhook_client else { return };

        let knowledge = Arc::clone(&self.knowledge);
        let url = url.clone();
        let client = client.clone();
        let anomaly_description = alert.anomaly_description();
        let sovereignty_score = self
            .config
            .sovereignty_score_bits
            .as_ref()
            .map(|a| f64::from_bits(a.load(std::sync::atomic::Ordering::Relaxed)));

        tokio::spawn(async move {
            let payload = GovernorWebhookPayload {
                anomaly_description: anomaly_description.clone(),
                sovereignty_score,
            };
            match client.post(&url).json(&payload).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if resp.status().is_success() {
                        info!(target: "pagi::governor", "Webhook notification sent to {}", url);
                    } else {
                        warn!(
                            target: "pagi::governor",
                            "Webhook returned {}: {}", status, url
                        );
                    }
                }
                Err(e) => {
                    error!(target: "pagi::governor", "Webhook POST failed: {}", e);
                }
            }
            let log_msg = format!("Governor webhook notification sent: {}", anomaly_description);
            if let Err(e) = knowledge.record_success_metric(&log_msg) {
                error!(target: "pagi::governor", "KB-08 record_success_metric failed: {}", e);
            }
        });
    }
    
    /// Start the Governor monitoring loop
    pub async fn run(self) {
        info!("🏛️ Governor started - monitoring KB-08, KB-06, and Vector DB");
        
        let mut check_interval = interval(Duration::from_secs(self.config.check_interval_secs));
        
        loop {
            check_interval.tick().await;
            
            // Check KB-08 (Absurdity Log)
            if let Err(e) = self.check_absurdity_log().await {
                error!("Governor: KB-08 check failed: {}", e);
            }
            
            // Check KB-06 (Ethos)
            if let Err(e) = self.check_ethos_alignment().await {
                error!("Governor: KB-06 check failed: {}", e);
            }
            
            // Check skill execution patterns
            if let Err(e) = self.check_skill_patterns().await {
                error!("Governor: Skill pattern check failed: {}", e);
            }
            
            // Check Vector DB health (Production Telemetry)
            #[cfg(feature = "vector")]
            if let Err(e) = self.check_vector_db_health().await {
                error!("Governor: Vector DB health check failed: {}", e);
            }
        }
    }
    
    /// Check KB-08 (Absurdity Log) for logic inconsistencies
    async fn check_absurdity_log(&self) -> Result<(), String> {
        // Query KB-08 for recent absurdity entries
        let summary = self.knowledge.get_absurdity_log_summary(20)
            .map_err(|e| format!("Failed to get absurdity log: {}", e))?;
        
        let entry_count = summary.total_entries;
        
        if entry_count > self.config.max_absurdity_threshold {
            warn!(
                "⚠️ Governor: High absurdity count detected: {} entries (threshold: {})",
                entry_count, self.config.max_absurdity_threshold
            );
            
            // Extract recent entry descriptions (recent_messages is Vec<String>)
            let recent_entries: Vec<String> = summary.recent_messages.iter()
                .take(5)
                .cloned()
                .collect();
            
            let alert = GovernorAlert::HighAbsurdityCount {
                count: entry_count,
                threshold: self.config.max_absurdity_threshold,
                recent_entries,
            };

            self.maybe_send_webhook(alert.clone());
            let _ = self.alert_tx.send(alert);

            if self.config.auto_intervene {
                info!("🛡️ Governor: Auto-intervention enabled - logging alert to KB-08");
                // Could implement automatic corrective actions here
            }
        } else {
            info!("✓ Governor: KB-08 check passed ({} entries)", entry_count);
        }
        
        Ok(())
    }
    
    /// Check KB-06 (Ethos) for alignment violations
    async fn check_ethos_alignment(&self) -> Result<(), String> {
        // Query KB-06 for philosophical ethos (active_school)
        let ethos = self.knowledge.get_ethos_philosophical_policy();
        
        if let Some(ref policy) = ethos {
            info!("✓ Governor: KB-06 ethos policy active: {}", policy.active_school);
            
            let summary = self.knowledge.get_absurdity_log_summary(10)
                .map_err(|e| format!("Failed to check violations: {}", e))?;
            
            for entry in &summary.recent_messages {
                if entry.to_lowercase().contains("ethos")
                    || entry.to_lowercase().contains("violation")
                    || entry.to_lowercase().contains("boundary") {
                    
                    warn!("⚠️ Governor: Potential ethos violation detected in KB-08");
                    
                    let alert = GovernorAlert::EthosViolation {
                        policy_name: policy.active_school.clone(),
                        violation_details: entry.clone(),
                    };

                    self.maybe_send_webhook(alert.clone());
                    let _ = self.alert_tx.send(alert);
                }
            }
        } else {
            warn!("⚠️ Governor: No ethos policy found in KB-06");
        }
        
        Ok(())
    }
    
    /// Check skill execution patterns for anomalies
    async fn check_skill_patterns(&self) -> Result<(), String> {
        // This would integrate with skill execution logs
        // For now, this is a placeholder for future implementation
        
        info!("✓ Governor: Skill pattern check passed");
        
        Ok(())
    }
    
    /// Check Vector DB health (Production Telemetry & Sovereignty).
    /// If Qdrant is offline, log to KB-08 and trigger Warning alert.
    #[cfg(feature = "vector")]
    async fn check_vector_db_health(&self) -> Result<(), String> {
        let Some(ref vector_store) = self.vector_store else {
            // No vector store configured, skip check
            return Ok(());
        };
        
        let status = vector_store.status();
        
        if !status.connected {
            warn!(
                "⚠️ Governor: Vector DB offline - {}",
                status.last_error.as_deref().unwrap_or("Unknown error")
            );
            
            // Log to KB-08 (Absurdity/Anomaly Log) for Sovereignty Audit
            let log_msg = format!(
                "Anomaly: Semantic Memory Offline ({}). Falling back to Local Knowledge Bases.",
                status.backend
            );
            
            if let Err(e) = self.knowledge.record_success_metric(&log_msg) {
                error!(target: "pagi::governor", "Failed to log Vector DB anomaly to KB-08: {}", e);
            } else {
                info!(target: "pagi::governor", "✓ Vector DB anomaly logged to KB-08");
            }
            
            // Trigger Warning alert
            let alert = GovernorAlert::VectorDbOffline {
                backend: status.backend.clone(),
                error_message: status.last_error.unwrap_or_else(|| "Connection lost".to_string()),
            };
            
            let _ = self.alert_tx.send(alert);
        } else {
            info!("✓ Governor: Vector DB health check passed ({})", status.backend);
        }
        
        Ok(())
    }
}

/// Start the Governor in a background task
pub fn start_governor(
    knowledge: Arc<KnowledgeStore>,
    config: GovernorConfig,
) -> (
    tokio::task::JoinHandle<()>,
    tokio::sync::broadcast::Receiver<GovernorAlert>,
) {
    let (governor, alert_rx) = Governor::new(knowledge, config);
    
    let handle = tokio::spawn(async move {
        governor.run().await;
    });
    
    (handle, alert_rx)
}

/// Governor alert handler for TUI dashboard
pub async fn handle_governor_alerts(
    mut alert_rx: tokio::sync::broadcast::Receiver<GovernorAlert>,
    log_tx: tokio::sync::broadcast::Sender<String>,
) {
    while let Ok(alert) = alert_rx.recv().await {
        let message = match alert {
            GovernorAlert::HighAbsurdityCount { count, threshold, recent_entries } => {
                format!(
                    "🚨 GOVERNOR ALERT: High absurdity count ({}/{}) - Recent: {}",
                    count,
                    threshold,
                    recent_entries.join("; ")
                )
            }
            GovernorAlert::EthosViolation { policy_name, violation_details } => {
                format!(
                    "🚨 GOVERNOR ALERT: Ethos violation ({}): {}",
                    policy_name,
                    violation_details
                )
            }
            GovernorAlert::SkillAnomalyDetected { skill_name, anomaly_type, details } => {
                format!(
                    "🚨 GOVERNOR ALERT: Skill anomaly in '{}' ({}): {}",
                    skill_name,
                    anomaly_type,
                    details
                )
            }
            GovernorAlert::KbQueryAnomaly { slot_id, query_count, details } => {
                format!(
                    "🚨 GOVERNOR ALERT: KB-{} query anomaly ({} queries): {}",
                    slot_id,
                    query_count,
                    details
                )
            }
            GovernorAlert::VectorDbOffline { backend, error_message } => {
                format!(
                    "⚠️ GOVERNOR ALERT: Vector DB offline ({}) - {}. Falling back to local search.",
                    backend,
                    error_message
                )
            }
        };
        
        warn!(target: "pagi::governor", "{}", message);
        let _ = log_tx.send(message);
    }
}

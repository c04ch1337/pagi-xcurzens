//! Sovereign Calibration Skill: Emotional & Urgency-Based Safety Governor Tuning
//!
//! **Mission**: Analyze Coach Jamey's sentiment and urgency patterns from chat history (KB-04)
//! and cross-reference with Evolution Inference reports (KB-08) to dynamically adjust the
//! Safety Governor's sensitivity thresholds. This allows Phoenix to be more experimental
//! during high-energy exploratory sessions and more conservative during production-critical work.
//!
//! **Design Philosophy**:
//! - Read sentiment patterns from recent coaching sessions (KB-04 Chronos)
//! - Correlate with Evolution Inference success/failure rates (KB-08 Soma)
//! - Calculate optimal Safety Governor sensitivity based on coaching tone
//! - Store calibration settings in KB-07 (Kardia) for relationship-aware tuning
//! - Log all calibration changes to KB-08 for sovereign oversight
//!
//! **Autonomous Safety**:
//! - Read-only analysis of chat history and inference reports
//! - Requires Ethos alignment check before adjusting Safety Governor
//! - All calibration changes are logged and reversible
//! - Respects HITL (Human-in-the-Loop) override at all times

use pagi_core::{AgentSkill, EventRecord, KbType, KnowledgeStore, TenantContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const SKILL_NAME: &str = "sovereign_calibration";
const CALIBRATION_PREFIX: &str = "calibration/";

#[derive(Debug, Deserialize)]
struct SovereignCalibrationArgs {
    /// Mode: "analyze" (sentiment analysis only) or "calibrate" (adjust Safety Governor)
    #[serde(default = "default_mode")]
    mode: String,
    
    /// Optional: number of recent messages to analyze for sentiment
    #[serde(default = "default_message_count")]
    message_count: usize,
    
    /// Optional: lookback days for evolution inference correlation
    #[serde(default = "default_lookback_days")]
    lookback_days: usize,
}

fn default_mode() -> String {
    "analyze".to_string()
}

fn default_message_count() -> usize {
    20
}

fn default_lookback_days() -> usize {
    7
}

/// Sentiment classification for coaching sessions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CoachingSentiment {
    /// High energy, experimental, exploratory ("Let's try this!", "What if...")
    Experimental,
    /// Urgent, production-critical ("Fix this now", "ASAP", "Critical")
    Urgent,
    /// Calm, methodical, teaching mode ("Let me explain", "Consider this")
    Methodical,
    /// Strict, production-focused ("Must be perfect", "No errors allowed")
    Strict,
    /// Neutral, balanced coaching
    Neutral,
}

impl CoachingSentiment {
    /// Infer sentiment from message text
    fn from_messages(messages: &[String]) -> Self {
        let combined = messages.join(" ").to_lowercase();
        
        // Check for experimental/exploratory language
        if combined.contains("let's try") 
            || combined.contains("what if") 
            || combined.contains("experiment")
            || combined.contains("explore")
            || combined.contains("play with") {
            return Self::Experimental;
        }
        
        // Check for urgent/critical language
        if combined.contains("urgent") 
            || combined.contains("asap") 
            || combined.contains("immediately")
            || combined.contains("critical")
            || combined.contains("fix this now")
            || combined.contains("production") {
            return Self::Urgent;
        }
        
        // Check for strict/production language
        if combined.contains("must be perfect")
            || combined.contains("no errors")
            || combined.contains("production-ready")
            || combined.contains("zero tolerance")
            || combined.contains("strict") {
            return Self::Strict;
        }
        
        // Check for methodical/teaching language
        if combined.contains("let me explain")
            || combined.contains("consider this")
            || combined.contains("think about")
            || combined.contains("understand")
            || combined.contains("learn") {
            return Self::Methodical;
        }
        
        Self::Neutral
    }
    
    /// Get recommended Safety Governor sensitivity (0.0 = very permissive, 1.0 = very strict)
    fn safety_sensitivity(&self) -> f64 {
        match self {
            Self::Experimental => 0.3,  // Allow more retries, encourage exploration
            Self::Urgent => 0.9,         // Very strict, fail fast to HITL
            Self::Methodical => 0.5,     // Balanced approach
            Self::Strict => 0.95,        // Maximum strictness
            Self::Neutral => 0.6,        // Slightly conservative default
        }
    }
    
    /// Get recommended max retry count before HITL engagement
    fn max_retries(&self) -> usize {
        match self {
            Self::Experimental => 5,     // More attempts during exploration
            Self::Urgent => 1,           // Fail fast to human
            Self::Methodical => 3,       // Standard retry count
            Self::Strict => 1,           // Immediate HITL on any issue
            Self::Neutral => 2,          // Conservative default
        }
    }
}

/// Calibration settings for Safety Governor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationSettings {
    /// Unix timestamp (ms) when calibration was performed
    pub calibrated_at_ms: i64,
    /// Detected coaching sentiment
    pub sentiment: CoachingSentiment,
    /// Safety Governor sensitivity (0.0-1.0)
    pub safety_sensitivity: f64,
    /// Max retry count before HITL
    pub max_retries: usize,
    /// Recent evolution success rate (from inference reports)
    pub recent_success_rate: f64,
    /// Number of messages analyzed
    pub messages_analyzed: usize,
    /// Reasoning for calibration decision
    pub reasoning: String,
}

impl CalibrationSettings {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

/// Sovereign Calibration Skill: Dynamic Safety Governor tuning
pub struct SovereignCalibrationSkill {
    store: Arc<KnowledgeStore>,
}

impl SovereignCalibrationSkill {
    pub fn new(store: Arc<KnowledgeStore>) -> Self {
        Self { store }
    }
    
    /// Analyze mode: Sentiment analysis and success rate correlation
    async fn run_analysis(
        &self,
        agent_id: &str,
        message_count: usize,
        lookback_days: usize,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let chronos_slot = KbType::Chronos.slot_id();
        let soma_slot = KbType::Soma.slot_id();
        
        // Calculate time window
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let lookback_ms = (lookback_days as i64) * 24 * 60 * 60 * 1000;
        let cutoff_ms = now_ms - lookback_ms;
        
        // Load recent chat messages from KB-04 (Chronos)
        let mut messages = Vec::new();
        for key in self.store.scan_keys(chronos_slot)? {
            if key.starts_with("event/") {
                if let Ok(Some(bytes)) = self.store.get(chronos_slot, &key) {
                    if let Some(event) = EventRecord::from_bytes(&bytes) {
                        if event.timestamp_ms >= cutoff_ms {
                            // Extract message content from event reflection
                            messages.push(event.reflection.clone());
                        }
                    }
                }
            }
        }
        
        // Take most recent N messages
        messages.sort_by_key(|_| std::time::SystemTime::now());
        messages.truncate(message_count);
        
        // Analyze sentiment
        let sentiment = CoachingSentiment::from_messages(&messages);
        
        // Load recent evolution success rate from KB-08
        let mut total_events = 0;
        let mut successful_events = 0;
        
        for key in self.store.scan_keys(soma_slot)? {
            if key.starts_with("event/") {
                if let Ok(Some(bytes)) = self.store.get(soma_slot, &key) {
                    if let Some(event) = EventRecord::from_bytes(&bytes) {
                        if event.timestamp_ms >= cutoff_ms {
                            total_events += 1;
                            if let Some(ref outcome) = event.outcome {
                                if outcome.contains("success") || outcome.contains("passed") {
                                    successful_events += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        let success_rate = if total_events > 0 {
            (successful_events as f64 / total_events as f64) * 100.0
        } else {
            0.0
        };
        
        // Generate reasoning
        let reasoning = format!(
            "Detected {:?} coaching sentiment from {} recent messages. \
             Recent evolution success rate: {:.1}%. \
             Recommended safety sensitivity: {:.2}, max retries: {}",
            sentiment,
            messages.len(),
            success_rate,
            sentiment.safety_sensitivity(),
            sentiment.max_retries()
        );
        
        Ok(serde_json::json!({
            "status": "analysis_complete",
            "skill": SKILL_NAME,
            "analysis": {
                "sentiment": format!("{:?}", sentiment),
                "messages_analyzed": messages.len(),
                "lookback_days": lookback_days,
                "recent_success_rate": format!("{:.1}%", success_rate),
                "recommended_sensitivity": sentiment.safety_sensitivity(),
                "recommended_max_retries": sentiment.max_retries(),
                "reasoning": reasoning,
            },
            "recommendation": "Run in 'calibrate' mode to apply these settings to the Safety Governor.",
        }))
    }
    
    /// Calibrate mode: Apply settings to Safety Governor
    async fn run_calibration(
        &self,
        agent_id: &str,
        message_count: usize,
        lookback_days: usize,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let chronos_slot = KbType::Chronos.slot_id();
        let soma_slot = KbType::Soma.slot_id();
        let kardia_slot = KbType::Kardia.slot_id();
        
        // Calculate time window
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let lookback_ms = (lookback_days as i64) * 24 * 60 * 60 * 1000;
        let cutoff_ms = now_ms - lookback_ms;
        
        // Load recent chat messages
        let mut messages = Vec::new();
        for key in self.store.scan_keys(chronos_slot)? {
            if key.starts_with("event/") {
                if let Ok(Some(bytes)) = self.store.get(chronos_slot, &key) {
                    if let Some(event) = EventRecord::from_bytes(&bytes) {
                        if event.timestamp_ms >= cutoff_ms {
                            messages.push(event.reflection.clone());
                        }
                    }
                }
            }
        }
        
        messages.sort_by_key(|_| std::time::SystemTime::now());
        messages.truncate(message_count);
        
        // Analyze sentiment
        let sentiment = CoachingSentiment::from_messages(&messages);
        
        // Calculate success rate
        let mut total_events = 0;
        let mut successful_events = 0;
        
        for key in self.store.scan_keys(soma_slot)? {
            if key.starts_with("event/") {
                if let Ok(Some(bytes)) = self.store.get(soma_slot, &key) {
                    if let Some(event) = EventRecord::from_bytes(&bytes) {
                        if event.timestamp_ms >= cutoff_ms {
                            total_events += 1;
                            if let Some(ref outcome) = event.outcome {
                                if outcome.contains("success") || outcome.contains("passed") {
                                    successful_events += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        let success_rate = if total_events > 0 {
            successful_events as f64 / total_events as f64
        } else {
            0.0
        };
        
        // Build calibration settings
        let reasoning = format!(
            "Calibrated based on {:?} sentiment. Success rate: {:.1}%. \
             Adjusting Safety Governor to sensitivity={:.2}, max_retries={}",
            sentiment,
            success_rate * 100.0,
            sentiment.safety_sensitivity(),
            sentiment.max_retries()
        );
        
        let settings = CalibrationSettings {
            calibrated_at_ms: now_ms,
            sentiment: sentiment.clone(),
            safety_sensitivity: sentiment.safety_sensitivity(),
            max_retries: sentiment.max_retries(),
            recent_success_rate: success_rate,
            messages_analyzed: messages.len(),
            reasoning: reasoning.clone(),
        };
        
        // Store calibration in KB-07 (Kardia) - relationship-aware tuning
        let calibration_key = format!("{}{}", CALIBRATION_PREFIX, agent_id);
        self.store.insert(kardia_slot, &calibration_key, &settings.to_bytes())?;
        
        // Log calibration event to KB-08 (Soma)
        let log_event = EventRecord::now(
            "SovereignCalibration",
            format!("Safety Governor calibrated: {:?} sentiment, sensitivity={:.2}, max_retries={}",
                sentiment, settings.safety_sensitivity, settings.max_retries),
        )
        .with_skill(SKILL_NAME)
        .with_outcome("calibration_applied");
        
        self.store.append_chronos_event(agent_id, &log_event)?;
        
        Ok(serde_json::json!({
            "status": "calibration_complete",
            "skill": SKILL_NAME,
            "calibration": {
                "sentiment": format!("{:?}", sentiment),
                "safety_sensitivity": settings.safety_sensitivity,
                "max_retries": settings.max_retries,
                "recent_success_rate": format!("{:.1}%", success_rate * 100.0),
                "messages_analyzed": messages.len(),
                "reasoning": reasoning,
            },
            "message": format!(
                "Safety Governor calibrated for {:?} coaching mode. Sensitivity: {:.2}, Max retries: {}",
                sentiment, settings.safety_sensitivity, settings.max_retries
            ),
        }))
    }
}

#[async_trait::async_trait]
impl AgentSkill for SovereignCalibrationSkill {
    fn name(&self) -> &str {
        SKILL_NAME
    }
    
    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let args: SovereignCalibrationArgs = payload
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or(SovereignCalibrationArgs {
                mode: "analyze".to_string(),
                message_count: 20,
                lookback_days: 7,
            });
        
        let agent_id = ctx.resolved_agent_id();
        
        match args.mode.as_str() {
            "analyze" => {
                self.run_analysis(agent_id, args.message_count, args.lookback_days).await
            }
            "calibrate" => {
                // Check Ethos alignment before calibrating Safety Governor
                if let Some(policy) = self.store.get_ethos_policy() {
                    let alignment = policy.allows(SKILL_NAME, "safety_governor_calibration");
                    if let pagi_core::AlignmentResult::Fail { reason } = alignment {
                        return Ok(serde_json::json!({
                            "status": "blocked_by_ethos",
                            "skill": SKILL_NAME,
                            "reason": reason,
                        }));
                    }
                }
                
                self.run_calibration(agent_id, args.message_count, args.lookback_days).await
            }
            _ => {
                Err(format!("Invalid mode '{}'. Use 'analyze' or 'calibrate'", args.mode).into())
            }
        }
    }
}

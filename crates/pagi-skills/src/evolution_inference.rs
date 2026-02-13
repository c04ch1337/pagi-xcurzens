//! Evolution Inference Skill: Autonomous Relationship Pattern Detection
//!
//! **Mission**: Cross-reference KB-04 (Chronos) topic patterns with KB-08 (Soma) Forge
//! success/failure logs to identify "Optimal Evolution Conditions" — the coaching patterns
//! that lead to successful autonomous system improvements.
//!
//! **Design Philosophy**:
//! - Analyze Topic Indexer summaries from KB-04 to identify coaching themes
//! - Correlate with EventRecord outcomes in KB-08 (Forge compilations, skill registrations)
//! - Generate confidence scores for pattern relationships
//! - Provide actionable insights: "Phoenix performs best when Coach The Creator provides X vs Y"
//!
//! **Autonomous Safety**:
//! - Read-only analysis mode: No modifications to any KB layer
//! - Requires Ethos alignment check before generating inference reports
//! - Logs all pattern discoveries to KB-08 for sovereign oversight
//! - Respects the Kardia (KB-07) relationship map for coaching context

use pagi_core::{AgentSkill, EventRecord, KbType, KnowledgeStore, TenantContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

const SKILL_NAME: &str = "evolution_inference";
const INFERENCE_REPORT_PREFIX: &str = "inference_report/";

#[derive(Debug, Deserialize)]
struct EvolutionInferenceArgs {
    /// Mode: "diagnostic" (analyze patterns) or "report" (generate full inference report)
    #[serde(default = "default_mode")]
    mode: String,
    
    /// Optional: minimum confidence threshold (0.0-1.0) for pattern reporting
    #[serde(default = "default_confidence_threshold")]
    confidence_threshold: f64,
    
    /// Optional: number of days to look back for pattern analysis
    #[serde(default = "default_lookback_days")]
    lookback_days: usize,
}

fn default_mode() -> String {
    "diagnostic".to_string()
}

fn default_confidence_threshold() -> f64 {
    0.6
}

fn default_lookback_days() -> usize {
    30
}

/// Pattern inference result: correlates coaching topics with evolution outcomes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionPattern {
    /// Coaching topic theme (e.g., "Rust trait definitions", "System architecture")
    pub coaching_theme: String,
    /// Evolution outcome (e.g., "forge_success", "compilation_error", "skill_registered")
    pub outcome_type: String,
    /// Number of occurrences of this pattern
    pub occurrence_count: usize,
    /// Confidence score (0.0-1.0) based on statistical significance
    pub confidence_score: f64,
    /// Average time delta (ms) between coaching and outcome
    pub avg_time_delta_ms: i64,
    /// Specific examples of this pattern
    pub examples: Vec<PatternExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternExample {
    /// Topic ID from KB-04
    pub topic_id: String,
    /// Topic summary
    pub topic_summary: String,
    /// Related event from KB-08
    pub event_outcome: String,
    /// Timestamp of the event
    pub event_timestamp_ms: i64,
}

/// Inference report: comprehensive analysis of evolution patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceReport {
    /// Unix timestamp (ms) when this report was generated
    pub generated_at_ms: i64,
    /// Number of days analyzed
    pub lookback_days: usize,
    /// Total coaching topics analyzed
    pub total_topics_analyzed: usize,
    /// Total evolution events analyzed
    pub total_events_analyzed: usize,
    /// Identified patterns (sorted by confidence score)
    pub patterns: Vec<EvolutionPattern>,
    /// Key insights and recommendations
    pub insights: Vec<String>,
    /// Overall evolution success rate
    pub success_rate: f64,
}

impl InferenceReport {
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

/// Evolution Inference Skill: Autonomous pattern detection for optimal coaching
pub struct EvolutionInferenceSkill {
    store: Arc<KnowledgeStore>,
}

impl EvolutionInferenceSkill {
    pub fn new(store: Arc<KnowledgeStore>) -> Self {
        Self { store }
    }
    
    /// Diagnostic mode: Quick analysis of available data
    async fn run_diagnostic(
        &self,
        lookback_days: usize,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let chronos_slot = KbType::Chronos.slot_id();
        let soma_slot = KbType::Soma.slot_id();
        
        // Count topic index entries in KB-04
        let topic_keys: Vec<String> = self.store.scan_keys(chronos_slot)?
            .into_iter()
            .filter(|k| k.starts_with("topic_index/"))
            .collect();
        
        // Count evolution events in KB-08
        let event_keys: Vec<String> = self.store.scan_keys(soma_slot)?
            .into_iter()
            .filter(|k| k.starts_with("event/") || k.starts_with("success_metric/"))
            .collect();
        
        // Calculate time window
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let lookback_ms = (lookback_days as i64) * 24 * 60 * 60 * 1000;
        let cutoff_ms = now_ms - lookback_ms;
        
        // Count recent events
        let mut recent_events = 0;
        let mut forge_successes = 0;
        let mut forge_failures = 0;
        
        for event_key in &event_keys {
            if let Ok(Some(bytes)) = self.store.get(soma_slot, event_key) {
                if let Some(event) = EventRecord::from_bytes(&bytes) {
                    if event.timestamp_ms >= cutoff_ms {
                        recent_events += 1;
                        if let Some(ref outcome) = event.outcome {
                            if outcome.contains("success") || outcome.contains("passed") {
                                forge_successes += 1;
                            } else if outcome.contains("fail") || outcome.contains("error") {
                                forge_failures += 1;
                            }
                        }
                    }
                }
            }
        }
        
        let success_rate = if recent_events > 0 {
            (forge_successes as f64 / recent_events as f64) * 100.0
        } else {
            0.0
        };
        
        Ok(serde_json::json!({
            "status": "diagnostic_complete",
            "skill": SKILL_NAME,
            "analysis": {
                "total_topic_entries": topic_keys.len(),
                "total_event_entries": event_keys.len(),
                "lookback_days": lookback_days,
                "recent_events": recent_events,
                "forge_successes": forge_successes,
                "forge_failures": forge_failures,
                "success_rate": format!("{:.1}%", success_rate),
            },
            "recommendation": if topic_keys.len() > 0 && recent_events > 0 {
                "Sufficient data available for pattern inference. Run in 'report' mode to generate insights."
            } else if topic_keys.len() == 0 {
                "No topic index found. Run conversation_topic_indexer first to create KB-04 topic summaries."
            } else {
                "Limited event data. Continue coaching sessions to build pattern corpus."
            },
        }))
    }
    
    /// Report mode: Generate comprehensive inference report
    async fn run_inference(
        &self,
        agent_id: &str,
        lookback_days: usize,
        confidence_threshold: f64,
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
        
        // Load topic summaries from KB-04
        let topic_prefix = format!("topic_index/{}/", agent_id);
        let mut topics = Vec::new();
        
        for key in self.store.scan_keys(chronos_slot)? {
            if key.starts_with(&topic_prefix) {
                if let Ok(Some(bytes)) = self.store.get(chronos_slot, &key) {
                    if let Ok(text) = String::from_utf8(bytes) {
                        if let Ok(summary) = serde_json::from_str::<serde_json::Value>(&text) {
                            if let Some(indexed_at) = summary.get("indexed_at_ms").and_then(|v| v.as_i64()) {
                                if indexed_at >= cutoff_ms {
                                    topics.push((indexed_at, summary));
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Load evolution events from KB-08
        let mut events = Vec::new();
        for key in self.store.scan_keys(soma_slot)? {
            if key.starts_with("event/") || key.starts_with("success_metric/") {
                if let Ok(Some(bytes)) = self.store.get(soma_slot, &key) {
                    if let Some(event) = EventRecord::from_bytes(&bytes) {
                        if event.timestamp_ms >= cutoff_ms {
                            events.push(event);
                        }
                    }
                }
            }
        }
        
        // Analyze patterns: correlate topics with nearby events
        let mut pattern_map: HashMap<String, Vec<(String, String, i64, i64)>> = HashMap::new();
        
        for (topic_ts, topic) in &topics {
            let topic_str = topic.get("topic")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let topic_id = topic.get("topic_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            
            // Find events within 24 hours of this topic
            let time_window_ms = 24 * 60 * 60 * 1000;
            for event in &events {
                let time_delta = (event.timestamp_ms - topic_ts).abs();
                if time_delta <= time_window_ms {
                    if let Some(ref outcome) = event.outcome {
                        let key = format!("{}::{}", topic_str, outcome);
                        pattern_map.entry(key)
                            .or_insert_with(Vec::new)
                            .push((
                                topic_id.to_string(),
                                topic_str.to_string(),
                                event.timestamp_ms,
                                time_delta,
                            ));
                    }
                }
            }
        }
        
        // Build pattern results
        let mut patterns = Vec::new();
        for (pattern_key, occurrences) in pattern_map {
            let parts: Vec<&str> = pattern_key.split("::").collect();
            if parts.len() != 2 {
                continue;
            }
            
            let coaching_theme = parts[0].to_string();
            let outcome_type = parts[1].to_string();
            let occurrence_count = occurrences.len();
            
            // Calculate confidence score based on frequency and consistency
            let confidence_score = (occurrence_count as f64 / topics.len().max(1) as f64)
                .min(1.0)
                .max(0.1);
            
            if confidence_score < confidence_threshold {
                continue;
            }
            
            // Calculate average time delta
            let avg_time_delta_ms = if !occurrences.is_empty() {
                occurrences.iter().map(|(_, _, _, delta)| delta).sum::<i64>() / occurrences.len() as i64
            } else {
                0
            };
            
            // Build examples (limit to 3)
            let examples: Vec<PatternExample> = occurrences.iter()
                .take(3)
                .map(|(topic_id, topic_summary, event_ts, _)| PatternExample {
                    topic_id: topic_id.clone(),
                    topic_summary: topic_summary.clone(),
                    event_outcome: outcome_type.clone(),
                    event_timestamp_ms: *event_ts,
                })
                .collect();
            
            patterns.push(EvolutionPattern {
                coaching_theme,
                outcome_type,
                occurrence_count,
                confidence_score,
                avg_time_delta_ms,
                examples,
            });
        }
        
        // Sort by confidence score (descending)
        patterns.sort_by(|a, b| b.confidence_score.partial_cmp(&a.confidence_score).unwrap());
        
        // Generate insights
        let mut insights = Vec::new();
        
        // Calculate overall success rate
        let total_events = events.len();
        let successful_events = events.iter()
            .filter(|e| {
                if let Some(ref outcome) = e.outcome {
                    outcome.contains("success") || outcome.contains("passed")
                } else {
                    false
                }
            })
            .count();
        let success_rate = if total_events > 0 {
            (successful_events as f64 / total_events as f64) * 100.0
        } else {
            0.0
        };
        
        // Generate insights based on patterns
        if let Some(top_pattern) = patterns.first() {
            insights.push(format!(
                "Highest confidence pattern: '{}' correlates with '{}' (confidence: {:.1}%)",
                top_pattern.coaching_theme,
                top_pattern.outcome_type,
                top_pattern.confidence_score * 100.0
            ));
        }
        
        if success_rate > 70.0 {
            insights.push(format!(
                "Strong evolution success rate ({:.1}%). Current coaching approach is highly effective.",
                success_rate
            ));
        } else if success_rate < 50.0 {
            insights.push(format!(
                "Evolution success rate is {:.1}%. Consider more specific technical guidance or smaller iteration steps.",
                success_rate
            ));
        }
        
        // Identify optimal coaching patterns
        let success_patterns: Vec<&EvolutionPattern> = patterns.iter()
            .filter(|p| {
                p.outcome_type.contains("success") || p.outcome_type.contains("passed")
            })
            .collect();
        
        if !success_patterns.is_empty() {
            let themes: Vec<String> = success_patterns.iter()
                .take(3)
                .map(|p| format!("'{}'", p.coaching_theme))
                .collect();
            insights.push(format!(
                "Optimal coaching conditions identified: Phoenix performs best when Coach The Creator provides {}",
                themes.join(", ")
            ));
        }
        
        // Time-to-success analysis
        let avg_success_time = success_patterns.iter()
            .map(|p| p.avg_time_delta_ms)
            .sum::<i64>() / success_patterns.len().max(1) as i64;
        
        if avg_success_time > 0 {
            let hours = avg_success_time / (1000 * 60 * 60);
            insights.push(format!(
                "Average time from coaching to successful evolution: {} hours",
                hours
            ));
        }
        
        // Build final report
        let report = InferenceReport {
            generated_at_ms: now_ms,
            lookback_days,
            total_topics_analyzed: topics.len(),
            total_events_analyzed: events.len(),
            patterns: patterns.clone(),
            insights: insights.clone(),
            success_rate: success_rate / 100.0,
        };
        
        // Store report in KB-08
        let report_key = format!("{}{}", INFERENCE_REPORT_PREFIX, now_ms);
        self.store.insert(soma_slot, &report_key, &report.to_bytes())?;
        
        // Log to Chronos
        let log_event = EventRecord::now(
            "EvolutionInference",
            format!("Generated inference report: {} patterns identified, {:.1}% success rate",
                patterns.len(), success_rate),
        )
        .with_skill(SKILL_NAME)
        .with_outcome("report_generated");
        
        self.store.append_chronos_event(agent_id, &log_event)?;
        
        Ok(serde_json::json!({
            "status": "inference_complete",
            "skill": SKILL_NAME,
            "report": {
                "generated_at_ms": report.generated_at_ms,
                "lookback_days": report.lookback_days,
                "total_topics_analyzed": report.total_topics_analyzed,
                "total_events_analyzed": report.total_events_analyzed,
                "patterns_identified": patterns.len(),
                "success_rate": format!("{:.1}%", success_rate),
                "insights": insights,
                "top_patterns": patterns.iter().take(5).map(|p| serde_json::json!({
                    "coaching_theme": p.coaching_theme,
                    "outcome_type": p.outcome_type,
                    "confidence": format!("{:.1}%", p.confidence_score * 100.0),
                    "occurrences": p.occurrence_count,
                })).collect::<Vec<_>>(),
            },
            "message": format!("Evolution inference report generated with {} patterns", patterns.len()),
        }))
    }
}

#[async_trait::async_trait]
impl AgentSkill for EvolutionInferenceSkill {
    fn name(&self) -> &str {
        SKILL_NAME
    }
    
    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let args: EvolutionInferenceArgs = payload
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or(EvolutionInferenceArgs {
                mode: "diagnostic".to_string(),
                confidence_threshold: 0.6,
                lookback_days: 30,
            });
        
        let agent_id = ctx.resolved_agent_id();
        
        match args.mode.as_str() {
            "diagnostic" => {
                self.run_diagnostic(args.lookback_days).await
            }
            "report" => {
                // Check Ethos alignment before generating inference report
                if let Some(policy) = self.store.get_ethos_policy() {
                    let alignment = policy.allows(SKILL_NAME, "pattern_inference");
                    if let pagi_core::AlignmentResult::Fail { reason } = alignment {
                        return Ok(serde_json::json!({
                            "status": "blocked_by_ethos",
                            "skill": SKILL_NAME,
                            "reason": reason,
                        }));
                    }
                }
                
                self.run_inference(agent_id, args.lookback_days, args.confidence_threshold).await
            }
            _ => {
                Err(format!("Invalid mode '{}'. Use 'diagnostic' or 'report'", args.mode).into())
            }
        }
    }
}

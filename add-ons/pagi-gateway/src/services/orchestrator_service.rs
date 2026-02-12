//! OrchestratorService: Background SAO intelligence layer
//!
//! Runs pattern matching (Manipulation Library / KB-2) and heuristic analysis
//! in the background. Results are cached and made available via the IntelligenceLayer API.
//! This decouples SAO logic from the main chat flow, making it a value-add service
//! that can be toggled on/off.

use pagi_core::{KnowledgeStore, HeuristicProcessor, SovereignDomain, ThreatContext, KbType};
use pagi_skills::{pattern_match_analyze, StrategicTimeline};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Intelligence layer insights from SAO background analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceInsights {
    /// Pattern matching results (manipulation detection)
    pub pattern_result: PatternInsights,
    /// Heuristic analysis (ROI, resource drain detection)
    pub heuristic_result: HeuristicInsights,
    /// Strategic alignment score (KB-06)
    pub strategic_alignment: StrategicAlignmentInsights,
    /// Domain integrity score (0.0-1.0)
    pub domain_integrity: f32,
    /// Soma balance summary (Spirit/Mind/Body)
    pub soma_balance: SomaBalanceSummary,
    /// Timestamp of last analysis
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternInsights {
    pub detected: bool,
    pub categories: Vec<String>,
    pub root_cause: String,
    pub counter_measure: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeuristicInsights {
    pub roi_score: f32,
    pub is_resource_drain: bool,
    pub threat_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicAlignmentInsights {
    pub score: f64,
    pub level: String, // "High", "Medium", "Low"
    pub matched_goals: Vec<String>,
    pub divergence_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SomaBalanceSummary {
    pub spirit: f32,
    pub mind: f32,
    pub body: f32,
    pub is_critical: bool,
}

/// Background service that runs SAO intelligence analysis
pub struct OrchestratorService {
    knowledge: Arc<KnowledgeStore>,
    heuristic_processor: HeuristicProcessor,
    strategic_timeline: Arc<RwLock<StrategicTimeline>>,
    cached_insights: Arc<RwLock<Option<IntelligenceInsights>>>,
    enabled: Arc<RwLock<bool>>,
}

impl OrchestratorService {
    pub fn new(knowledge: Arc<KnowledgeStore>) -> Self {
        // Try to load strategic timeline from KB-06, otherwise use default
        let timeline = Self::load_strategic_timeline(&knowledge)
            .unwrap_or_else(|| StrategicTimeline::default_timeline());
        
        Self {
            knowledge,
            heuristic_processor: HeuristicProcessor::new(SovereignDomain::default()),
            strategic_timeline: Arc::new(RwLock::new(timeline)),
            cached_insights: Arc::new(RwLock::new(None)),
            enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// Load strategic timeline from KB-06 (Ethos slot)
    fn load_strategic_timeline(knowledge: &KnowledgeStore) -> Option<StrategicTimeline> {
        let slot_id = KbType::Ethos.slot_id();
        let bytes = knowledge.get(slot_id, "strategic_goals_timeline").ok().flatten()?;
        let json = String::from_utf8(bytes).ok()?;
        if let Ok(timeline) = StrategicTimeline::from_json(&json) {
            tracing::info!(target: "pagi::strategic", "Loaded strategic timeline from KB-06");
            return Some(timeline);
        }
        None
    }

    /// Update strategic timeline and persist to KB-06
    pub async fn update_strategic_timeline(&self, timeline: StrategicTimeline) -> Result<(), String> {
        // Persist to KB-06
        let json = timeline.to_json()
            .map_err(|e| format!("Failed to serialize timeline: {}", e))?;
        
        let slot_id = KbType::Ethos.slot_id();
        self.knowledge.insert(slot_id, "strategic_goals_timeline", json.as_bytes())
            .map_err(|e| format!("Failed to persist to KB-06: {}", e))?;
        
        // Update in-memory cache
        *self.strategic_timeline.write().await = timeline;
        
        tracing::info!(target: "pagi::strategic", "Updated strategic timeline in KB-06");
        Ok(())
    }

    /// Get current strategic timeline
    pub async fn get_strategic_timeline(&self) -> StrategicTimeline {
        self.strategic_timeline.read().await.clone()
    }

    /// Toggle the intelligence layer on/off
    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
        tracing::info!(target: "pagi::intelligence", enabled, "Intelligence layer toggled");
    }

    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    /// Analyze user input in the background and cache results
    pub async fn analyze_input(&self, user_input: &str) -> Option<IntelligenceInsights> {
        if !self.is_enabled().await {
            return None;
        }

        // Pattern matching (Manipulation Library / KB-2)
        let pattern_result = pattern_match_analyze(user_input);
        
        // Heuristic analysis
        let context = ThreatContext {
            situation: user_input.to_string(),
            subject_id: None,
            emotional_valence: None,
        };
        let heuristic_result = self.heuristic_processor.process(&context);

        // Strategic alignment analysis (KB-06)
        let timeline = self.strategic_timeline.read().await;
        let alignment = timeline.calculate_alignment(user_input);
        let strategic_alignment = StrategicAlignmentInsights {
            score: alignment.overall_score,
            level: format!("{:?}", alignment.level),
            matched_goals: alignment.matched_goals,
            divergence_warning: alignment.divergence_warning,
        };
        drop(timeline); // Release lock

        // Get Soma balance from KB-8
        let soma_state = self.knowledge.get_soma_state();
        let soma_balance = SomaBalanceSummary {
            spirit: 0.0, // Would need to calculate from balance_check entries
            mind: 0.0,
            body: 0.0,
            is_critical: soma_state.needs_biogate_adjustment(),
        };

        // Calculate domain integrity (factoring in strategic alignment)
        let domain_integrity = if pattern_result.detected {
            0.3 // Low integrity if manipulation detected
        } else if heuristic_result.roi.is_low_roi {
            0.6 // Medium integrity if resource drain
        } else if strategic_alignment.score < 30.0 {
            0.5 // Medium-low integrity if poor strategic alignment
        } else {
            0.9 // High integrity otherwise
        };

        let insights = IntelligenceInsights {
            pattern_result: PatternInsights {
                detected: pattern_result.detected,
                categories: pattern_result.categories,
                root_cause: pattern_result.root_cause,
                counter_measure: pattern_result.sao_counter_measure,
            },
            heuristic_result: HeuristicInsights {
                roi_score: heuristic_result.roi.score as f32,
                is_resource_drain: heuristic_result.roi.is_low_roi,
                threat_level: if pattern_result.detected {
                    "high".to_string()
                } else if heuristic_result.roi.is_low_roi {
                    "medium".to_string()
                } else {
                    "low".to_string()
                },
            },
            strategic_alignment,
            domain_integrity,
            soma_balance,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };

        // Cache the insights
        *self.cached_insights.write().await = Some(insights.clone());

        Some(insights)
    }

    /// Build context injection string for LLM system prompt
    pub async fn build_strategic_context(&self) -> String {
        let timeline = self.strategic_timeline.read().await;
        let mut context = String::from("\n\n## Strategic North Star (KB-06)\n");
        context.push_str("Your user has defined long-term strategic goals. Consider alignment when responding:\n\n");
        context.push_str(&timeline.get_summary());
        context
    }

    /// Get cached insights (for status bar display)
    pub async fn get_cached_insights(&self) -> Option<IntelligenceInsights> {
        self.cached_insights.read().await.clone()
    }

    /// Clear cached insights
    pub async fn clear_cache(&self) {
        *self.cached_insights.write().await = None;
    }
}

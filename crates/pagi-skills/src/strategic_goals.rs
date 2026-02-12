use serde::{Deserialize, Serialize};

/// Strategic timeline horizons for goal planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicTimeline {
    /// Short-term goals (1-5 years)
    pub short_term: Vec<StrategicGoal>,
    /// Mid-term goals (5-10 years)
    pub mid_term: Vec<StrategicGoal>,
    /// Long-term goals (20+ years)
    pub long_term: Vec<StrategicGoal>,
}

/// Individual strategic goal with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicGoal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub weight: f64, // 0.0 to 1.0, importance multiplier
    pub category: GoalCategory,
}

/// Categories for strategic goals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalCategory {
    Sovereignty,
    Financial,
    Technical,
    Health,
    Relationships,
    Learning,
    Legacy,
}

/// Alignment score result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignmentScore {
    pub overall_score: f64, // 0.0 to 100.0
    pub level: AlignmentLevel,
    pub matched_goals: Vec<String>,
    pub divergence_warning: Option<String>,
}

/// Alignment level classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlignmentLevel {
    High,    // 70-100%
    Medium,  // 30-70%
    Low,     // 0-30%
}

impl StrategicTimeline {
    /// Create a default timeline with placeholder goals
    pub fn default_timeline() -> Self {
        Self {
            short_term: vec![
                StrategicGoal {
                    id: "st_001".to_string(),
                    title: "Establish Sovereign Infrastructure".to_string(),
                    description: "Build self-hosted, privacy-first systems for all critical operations".to_string(),
                    keywords: vec!["sovereignty".to_string(), "privacy".to_string(), "infrastructure".to_string(), "self-hosted".to_string()],
                    weight: 1.0,
                    category: GoalCategory::Sovereignty,
                },
                StrategicGoal {
                    id: "st_002".to_string(),
                    title: "Financial Independence".to_string(),
                    description: "Achieve sustainable income streams independent of traditional employment".to_string(),
                    keywords: vec!["financial".to_string(), "independence".to_string(), "income".to_string(), "business".to_string()],
                    weight: 0.9,
                    category: GoalCategory::Financial,
                },
                StrategicGoal {
                    id: "st_003".to_string(),
                    title: "Master Core Technologies".to_string(),
                    description: "Deep expertise in Rust, AI/ML, distributed systems, and security".to_string(),
                    keywords: vec!["rust".to_string(), "ai".to_string(), "ml".to_string(), "security".to_string(), "distributed".to_string()],
                    weight: 0.8,
                    category: GoalCategory::Technical,
                },
            ],
            mid_term: vec![
                StrategicGoal {
                    id: "mt_001".to_string(),
                    title: "Build Sustainable AGI Systems".to_string(),
                    description: "Create production-grade AGI orchestration platforms that serve real users".to_string(),
                    keywords: vec!["agi".to_string(), "production".to_string(), "platform".to_string(), "users".to_string()],
                    weight: 1.0,
                    category: GoalCategory::Technical,
                },
                StrategicGoal {
                    id: "mt_002".to_string(),
                    title: "Establish Technical Authority".to_string(),
                    description: "Become recognized expert in sovereign AI and privacy-preserving systems".to_string(),
                    keywords: vec!["authority".to_string(), "expert".to_string(), "recognition".to_string(), "thought-leader".to_string()],
                    weight: 0.7,
                    category: GoalCategory::Learning,
                },
            ],
            long_term: vec![
                StrategicGoal {
                    id: "lt_001".to_string(),
                    title: "Legacy: Democratize Sovereign AI".to_string(),
                    description: "Make sovereign, privacy-first AI accessible to individuals and small organizations".to_string(),
                    keywords: vec!["legacy".to_string(), "democratize".to_string(), "accessible".to_string(), "sovereignty".to_string()],
                    weight: 1.0,
                    category: GoalCategory::Legacy,
                },
                StrategicGoal {
                    id: "lt_002".to_string(),
                    title: "Mentor Next Generation".to_string(),
                    description: "Train and empower others to build sovereign systems and resist digital feudalism".to_string(),
                    keywords: vec!["mentor".to_string(), "teach".to_string(), "empower".to_string(), "next-generation".to_string()],
                    weight: 0.8,
                    category: GoalCategory::Legacy,
                },
            ],
        }
    }

    /// Calculate alignment score for given input context
    pub fn calculate_alignment(&self, input_context: &str) -> AlignmentScore {
        let context_lower = input_context.to_lowercase();
        let mut matched_goal_scores = Vec::new();
        let mut matched_goals = Vec::new();

        // Check all goals across all timelines
        let all_goals: Vec<&StrategicGoal> = self.short_term.iter()
            .chain(self.mid_term.iter())
            .chain(self.long_term.iter())
            .collect();

        for goal in all_goals {
            // Check for keyword matches
            let mut match_count = 0;
            for keyword in &goal.keywords {
                if context_lower.contains(&keyword.to_lowercase()) {
                    match_count += 1;
                }
            }

            if match_count > 0 {
                // Score based on percentage of keywords matched, weighted by goal importance
                let match_ratio = match_count as f64 / goal.keywords.len() as f64;
                let goal_score = match_ratio * goal.weight * 100.0;
                matched_goal_scores.push(goal_score);
                matched_goals.push(goal.id.clone());
            }
        }

        // Calculate overall score as average of matched goals (not all goals)
        // This makes it possible to score high by deeply aligning with a few goals
        let overall_score = if !matched_goal_scores.is_empty() {
            matched_goal_scores.iter().sum::<f64>() / matched_goal_scores.len() as f64
        } else {
            0.0
        };

        // Determine alignment level
        let level = if overall_score >= 70.0 {
            AlignmentLevel::High
        } else if overall_score >= 30.0 {
            AlignmentLevel::Medium
        } else {
            AlignmentLevel::Low
        };

        // Generate divergence warning for low alignment
        let divergence_warning = if overall_score < 10.0 && !matched_goals.is_empty() {
            Some("⚠️ Strategic Divergence: This interaction shows minimal alignment with your long-term sovereignty goals.".to_string())
        } else if overall_score == 0.0 {
            Some("🚨 Zero Strategic Value: This interaction provides no measurable progress toward your North Star objectives.".to_string())
        } else {
            None
        };

        AlignmentScore {
            overall_score,
            level,
            matched_goals,
            divergence_warning,
        }
    }

    /// Get a summary of all goals for context injection
    pub fn get_summary(&self) -> String {
        let mut summary = String::from("# Strategic North Star (KB-06)\n\n");
        
        summary.push_str("## Short-Term Goals (1-5 years)\n");
        for goal in &self.short_term {
            summary.push_str(&format!("- **{}**: {}\n", goal.title, goal.description));
        }
        
        summary.push_str("\n## Mid-Term Goals (5-10 years)\n");
        for goal in &self.mid_term {
            summary.push_str(&format!("- **{}**: {}\n", goal.title, goal.description));
        }
        
        summary.push_str("\n## Long-Term Goals (20+ years)\n");
        for goal in &self.long_term {
            summary.push_str(&format!("- **{}**: {}\n", goal.title, goal.description));
        }
        
        summary
    }

    /// Export timeline to JSON for KB-06 storage
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import timeline from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_high_alignment() {
        let timeline = StrategicTimeline::default_timeline();
        let context = "Working on Rust-based sovereign AI infrastructure with privacy-first design and distributed systems";
        let score = timeline.calculate_alignment(context);
        
        // Should match multiple goals with high keyword overlap
        assert!(score.overall_score > 0.0, "Score should be greater than 0, got {}", score.overall_score);
        assert!(!score.matched_goals.is_empty(), "Should match at least one goal");
        // With good keyword matches, should be at least Medium level
        assert!(matches!(score.level, AlignmentLevel::Medium | AlignmentLevel::High),
                "Expected Medium or High, got {:?}", score.level);
    }

    #[test]
    fn test_low_alignment() {
        let timeline = StrategicTimeline::default_timeline();
        let context = "Watching random YouTube videos about celebrity gossip";
        let score = timeline.calculate_alignment(context);
        
        assert!(score.overall_score < 30.0);
        assert_eq!(score.level, AlignmentLevel::Low);
        assert!(score.divergence_warning.is_some());
    }

    #[test]
    fn test_zero_alignment() {
        let timeline = StrategicTimeline::default_timeline();
        let context = "xyz abc qwerty nonsense";
        let score = timeline.calculate_alignment(context);
        
        assert_eq!(score.overall_score, 0.0);
        assert_eq!(score.level, AlignmentLevel::Low);
        assert!(score.divergence_warning.is_some());
    }

    #[test]
    fn test_json_serialization() {
        let timeline = StrategicTimeline::default_timeline();
        let json = timeline.to_json().unwrap();
        let restored = StrategicTimeline::from_json(&json).unwrap();
        
        assert_eq!(timeline.short_term.len(), restored.short_term.len());
        assert_eq!(timeline.mid_term.len(), restored.mid_term.len());
        assert_eq!(timeline.long_term.len(), restored.long_term.len());
    }
}

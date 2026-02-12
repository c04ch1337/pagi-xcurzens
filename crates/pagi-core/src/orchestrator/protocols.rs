//! KB-05: Sovereign Security Protocols
//!
//! Gated Social Protection - applies communication strategies based on subject rank
//! to protect the user's domain from manipulation and emotional exploitation.
//! Integrates with sovereignty_leak_triggers (KB-05) to auto-rank subjects when
//! traits or initial interaction match user-defined leaks.
//! Gated by PAGI_SOVEREIGNTY_AUTO_RANK_ENABLED (SovereignConfig).

use crate::shared::SovereignConfig;
use std::env;

/// Gray Rock rank threshold: subjects matching sovereignty leaks get this rank (8 = Gray Rock).
pub const SOVEREIGNTY_LEAK_AUTO_RANK: u8 = 8;

/// Communication protocol styles based on subject threat level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolStyle {
    /// Gray Rock: Minimal emotional engagement, boring responses
    GrayRock,
    /// Professional: Polite but distant, business-like
    Professional,
    /// Open: Normal, warm communication (low/no threat)
    Open,
}

impl ProtocolStyle {
    /// Determine protocol style based on subject rank (0-10)
    /// Higher rank = higher threat = more defensive protocol
    pub fn from_rank(rank: u8) -> Self {
        match rank {
            8..=10 => ProtocolStyle::GrayRock,      // High threat: Gray Rock
            4..=7 => ProtocolStyle::Professional,    // Medium threat: Professional
            _ => ProtocolStyle::Open,                // Low/no threat: Open
        }
    }

    /// Get the strategic advice for this protocol style
    pub fn get_advice(&self) -> &'static str {
        match self {
            ProtocolStyle::GrayRock => {
                "GRAY ROCK PROTOCOL: Keep responses minimal, emotionally flat, and uninteresting. \
                 Avoid sharing personal details or emotional reactions. Be boring and unrewarding \
                 to manipulative probing. Refer to KB-02 for identified manipulation patterns."
            }
            ProtocolStyle::Professional => {
                "PROFESSIONAL PROTOCOL: Maintain polite but distant communication. Keep boundaries \
                 clear and responses business-like. Avoid emotional vulnerability. Monitor for \
                 manipulation attempts per KB-02."
            }
            ProtocolStyle::Open => {
                "OPEN PROTOCOL: Normal, warm communication is appropriate. Subject shows low \
                 manipulation risk. Maintain awareness but engage naturally."
            }
        }
    }
}

/// Protocol Engine - manages sovereign security protocols
pub struct ProtocolEngine {
    enabled: bool,
}

impl ProtocolEngine {
    /// Create a new ProtocolEngine, checking environment configuration
    pub fn new() -> Self {
        let enabled = env::var("SOVEREIGN_PROTOCOLS_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase()
            == "true";

        Self { enabled }
    }

    /// Check if protocols are enabled in the environment
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Apply protocol to input based on subject rank
    /// Returns strategic advice to inject into LLM context
    /// 
    /// # Arguments
    /// * `subject_rank` - Threat rank from KB-02 (0-10, higher = more dangerous)
    /// * `input` - The user's input message
    /// 
    /// # Returns
    /// Strategic advice string to prepend to system prompt, or empty if disabled
    pub fn apply_protocol(&self, subject_rank: u8, input: String) -> String {
        if !self.enabled {
            return String::new();
        }

        let style = ProtocolStyle::from_rank(subject_rank);
        let advice = style.get_advice();

        format!(
            "PROTOCOL ACTIVE: Subject is Rank {}. {}\n\nUser Input: {}",
            subject_rank, advice, input
        )
    }

    /// Get protocol advice without applying to input (for system prompt injection)
    /// 
    /// # Arguments
    /// * `subject_rank` - Threat rank from KB-02 (0-10)
    /// 
    /// # Returns
    /// Protocol advice string, or empty if disabled
    pub fn get_protocol_advice(&self, subject_rank: u8) -> String {
        if !self.enabled {
            return String::new();
        }

        let style = ProtocolStyle::from_rank(subject_rank);
        format!(
            "PROTOCOL ACTIVE: Subject is Rank {}. {}",
            subject_rank,
            style.get_advice()
        )
    }

    /// Get the current protocol style for a given rank
    pub fn get_protocol_style(&self, subject_rank: u8) -> Option<ProtocolStyle> {
        if !self.enabled {
            return None;
        }
        Some(ProtocolStyle::from_rank(subject_rank))
    }
}

/// Cross-reference subject traits or initial interaction text against sovereignty_leak_triggers.
/// If any trigger keyword appears (case-insensitive) in the text, returns a rank that triggers
/// Gray Rock (8) so the ProtocolEngine applies defensive protocols.
///
/// Call when a new subject is introduced in chat; use the returned rank (or override) for
/// `get_protocol_advice` so Phoenix Marie structurally adjusts to protect the user.
///
/// When PAGI_SOVEREIGNTY_AUTO_RANK_ENABLED is false, returns None (no auto-ranking).
pub fn rank_subject_from_sovereignty_triggers(
    triggers: &[String],
    traits_or_interaction: &str,
) -> Option<u8> {
    if !SovereignConfig::from_env().sovereignty_auto_rank_enabled {
        return None;
    }
    if triggers.is_empty() || traits_or_interaction.trim().is_empty() {
        return None;
    }
    let text_lower = traits_or_interaction.to_lowercase();
    let matched = triggers
        .iter()
        .any(|t| !t.trim().is_empty() && text_lower.contains(&t.trim().to_lowercase()));
    if matched {
        Some(SOVEREIGNTY_LEAK_AUTO_RANK)
    } else {
        None
    }
}

/// Returns which triggers matched (for logging). Empty if none.
pub fn matched_sovereignty_triggers(triggers: &[String], traits_or_interaction: &str) -> Vec<String> {
    if triggers.is_empty() || traits_or_interaction.trim().is_empty() {
        return Vec::new();
    }
    let text_lower = traits_or_interaction.to_lowercase();
    triggers
        .iter()
        .filter(|t| !t.trim().is_empty() && text_lower.contains(&t.trim().to_lowercase()))
        .map(|t| t.trim().to_string())
        .collect()
}

impl Default for ProtocolEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_style_from_rank() {
        assert_eq!(ProtocolStyle::from_rank(10), ProtocolStyle::GrayRock);
        assert_eq!(ProtocolStyle::from_rank(8), ProtocolStyle::GrayRock);
        assert_eq!(ProtocolStyle::from_rank(7), ProtocolStyle::Professional);
        assert_eq!(ProtocolStyle::from_rank(4), ProtocolStyle::Professional);
        assert_eq!(ProtocolStyle::from_rank(3), ProtocolStyle::Open);
        assert_eq!(ProtocolStyle::from_rank(0), ProtocolStyle::Open);
    }

    #[test]
    fn test_protocol_engine_disabled_by_default() {
        // Without env var set, should be disabled
        let engine = ProtocolEngine::new();
        assert!(!engine.is_enabled());
        assert_eq!(engine.apply_protocol(10, "test".to_string()), "");
        assert_eq!(engine.get_protocol_advice(10), "");
        assert!(engine.get_protocol_style(10).is_none());
    }

    #[test]
    fn test_protocol_advice_content() {
        let gray_rock = ProtocolStyle::GrayRock.get_advice();
        assert!(gray_rock.contains("GRAY ROCK"));
        assert!(gray_rock.contains("minimal"));

        let professional = ProtocolStyle::Professional.get_advice();
        assert!(professional.contains("PROFESSIONAL"));
        assert!(professional.contains("polite"));

        let open = ProtocolStyle::Open.get_advice();
        assert!(open.contains("OPEN"));
        assert!(open.contains("warm"));
    }
}

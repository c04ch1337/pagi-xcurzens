//! KB-05 Social Protocols: Subject Profile System
//!
//! This module implements the "Sovereign Subject Schema" - a relational ledger
//! that tracks entities (people, partners, livestock) through the lens of loyalty,
//! history, and strategic value.
//!
//! ## Architecture
//!
//! The Subject Profile is not just a contact card; it's a comprehensive tracking
//! system that combines:
//! - Vital Statistics (Name, DOB, Astrological data)
//! - Relationship DNA (Family, Key Dates, History)
//! - Strategic Weight (Ally/Neutral/Drain classification)
//! - Pattern Recognition (Manipulation detection, recurring issues)
//!
//! ## Integration Points
//!
//! - **KB-05 (Social Protocols)**: Primary storage for subject profiles
//! - **KB-08 (Absurdity Log)**: Automatic logging of disrespect events
//! - **KB-07 (Kardia)**: User preferences and compatibility data
//! - **Email/Calendar**: Automatic tagging and context enrichment

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Astrological sign enumeration for compatibility calculations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZodiacSign {
    Aries,
    Taurus,
    Gemini,
    Cancer,
    Leo,
    Virgo,
    Libra,
    Scorpio,
    Sagittarius,
    Capricorn,
    Aquarius,
    Pisces,
}

impl ZodiacSign {
    /// Calculate zodiac sign from birth date
    pub fn from_date(date: NaiveDate) -> Self {
        let month = date.month();
        let day = date.day();

        match (month, day) {
            (3, 21..=31) | (4, 1..=19) => ZodiacSign::Aries,
            (4, 20..=30) | (5, 1..=20) => ZodiacSign::Taurus,
            (5, 21..=31) | (6, 1..=20) => ZodiacSign::Gemini,
            (6, 21..=30) | (7, 1..=22) => ZodiacSign::Cancer,
            (7, 23..=31) | (8, 1..=22) => ZodiacSign::Leo,
            (8, 23..=31) | (9, 1..=22) => ZodiacSign::Virgo,
            (9, 23..=30) | (10, 1..=22) => ZodiacSign::Libra,
            (10, 23..=31) | (11, 1..=21) => ZodiacSign::Scorpio,
            (11, 22..=30) | (12, 1..=21) => ZodiacSign::Sagittarius,
            (12, 22..=31) | (1, 1..=19) => ZodiacSign::Capricorn,
            (1, 20..=31) | (2, 1..=18) => ZodiacSign::Aquarius,
            (2, 19..=29) | (3, 1..=20) => ZodiacSign::Pisces,
            _ => ZodiacSign::Pisces, // Default fallback
        }
    }

    /// Get element for this sign (Fire, Earth, Air, Water)
    pub fn element(&self) -> Element {
        match self {
            ZodiacSign::Aries | ZodiacSign::Leo | ZodiacSign::Sagittarius => Element::Fire,
            ZodiacSign::Taurus | ZodiacSign::Virgo | ZodiacSign::Capricorn => Element::Earth,
            ZodiacSign::Gemini | ZodiacSign::Libra | ZodiacSign::Aquarius => Element::Air,
            ZodiacSign::Cancer | ZodiacSign::Scorpio | ZodiacSign::Pisces => Element::Water,
        }
    }

    /// Get modality for this sign (Cardinal, Fixed, Mutable)
    pub fn modality(&self) -> Modality {
        match self {
            ZodiacSign::Aries | ZodiacSign::Cancer | ZodiacSign::Libra | ZodiacSign::Capricorn => {
                Modality::Cardinal
            }
            ZodiacSign::Taurus | ZodiacSign::Leo | ZodiacSign::Scorpio | ZodiacSign::Aquarius => {
                Modality::Fixed
            }
            ZodiacSign::Gemini | ZodiacSign::Virgo | ZodiacSign::Sagittarius | ZodiacSign::Pisces => {
                Modality::Mutable
            }
        }
    }
}

/// Elemental classification for astrological compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Element {
    Fire,
    Earth,
    Air,
    Water,
}

/// Modality classification for astrological compatibility
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Modality {
    Cardinal, // Initiators
    Fixed,    // Stabilizers
    Mutable,  // Adapters
}

/// Birth metadata including astrological information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirthMetadata {
    /// Date of birth
    pub date: NaiveDate,
    /// Optional time of birth (for more precise calculations)
    pub time: Option<chrono::NaiveTime>,
    /// Zodiac sign (calculated from date)
    pub sign: ZodiacSign,
    /// Optional birth location for advanced calculations
    pub location: Option<String>,
}

impl BirthMetadata {
    /// Create new birth metadata from date
    pub fn new(date: NaiveDate) -> Self {
        Self {
            sign: ZodiacSign::from_date(date),
            date,
            time: None,
            location: None,
        }
    }

    /// Create with full details
    pub fn with_details(
        date: NaiveDate,
        time: Option<chrono::NaiveTime>,
        location: Option<String>,
    ) -> Self {
        Self {
            sign: ZodiacSign::from_date(date),
            date,
            time,
            location,
        }
    }
}

/// Social connection node (family member, associate, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialNode {
    /// Name of the connected person
    pub name: String,
    /// Relationship type (e.g., "Spouse", "Child", "Business Partner")
    pub relationship: String,
    /// Optional additional context
    pub notes: Option<String>,
}

/// Strategic rank classification (0-10 scale)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StrategicRank(u8);

impl StrategicRank {
    /// Create a new strategic rank (clamped to 0-10)
    pub fn new(value: u8) -> Self {
        Self(value.min(10))
    }

    /// Get the raw value
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Check if this is a high-priority subject (rank >= 8)
    pub fn is_high_priority(&self) -> bool {
        self.0 >= 8
    }

    /// Check if this is a low-priority subject (rank <= 3)
    pub fn is_low_priority(&self) -> bool {
        self.0 <= 3
    }

    /// Get communication strategy based on rank
    pub fn communication_strategy(&self) -> CommunicationStrategy {
        match self.0 {
            0..=2 => CommunicationStrategy::GrayRock,
            3..=5 => CommunicationStrategy::Professional,
            6..=7 => CommunicationStrategy::Friendly,
            8..=9 => CommunicationStrategy::Transparent,
            10 => CommunicationStrategy::FullSovereign,
            _ => CommunicationStrategy::Professional,
        }
    }
}

/// Communication strategy based on strategic rank
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommunicationStrategy {
    /// Minimal, non-engaging responses (Rank 0-2)
    GrayRock,
    /// Polite but bounded (Rank 3-5)
    Professional,
    /// Warm and engaging (Rank 6-7)
    Friendly,
    /// Open and honest (Rank 8-9)
    Transparent,
    /// Complete vulnerability and trust (Rank 10)
    FullSovereign,
}

/// Disrespect event logged to KB-08
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisrespectEvent {
    /// Unique event ID
    pub id: Uuid,
    /// Subject who committed the disrespect
    pub subject_id: Uuid,
    /// Timestamp of the event
    pub timestamp: DateTime<Utc>,
    /// Description of what happened
    pub description: String,
    /// Severity (0.0 = minor, 1.0 = severe)
    pub severity: f32,
    /// Pattern tags (e.g., "manipulation", "gaslighting", "boundary-violation")
    pub pattern_tags: Vec<String>,
}

/// The core Subject Profile structure
///
/// This is the "Relational Ledger" that tracks all entities in the user's life.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectProfile {
    /// Unique identifier
    pub id: Uuid,
    /// Display name or alias
    pub alias: String,
    /// Birth metadata (optional for privacy)
    pub birth_metadata: Option<BirthMetadata>,
    /// Social connections (family, associates)
    pub social_nodes: Vec<SocialNode>,
    /// Strategic importance (0-10, where 10 = Sovereign Family)
    pub strategic_rank: StrategicRank,
    /// Vulnerability score: how much the "Savior" wants to help them (0.0-1.0)
    pub vulnerability_score: f32,
    /// Classification tags (e.g., "Family", "Business", "High-Maintenance")
    pub tags: Vec<String>,
    /// When this profile was created
    pub created_at: DateTime<Utc>,
    /// Last interaction timestamp
    pub last_interaction: Option<DateTime<Utc>>,
    /// Custom metadata for extensibility
    pub metadata: HashMap<String, String>,
}

impl SubjectProfile {
    /// Create a new subject profile with minimal information
    pub fn new(alias: String, strategic_rank: u8) -> Self {
        Self {
            id: Uuid::new_v4(),
            alias,
            birth_metadata: None,
            social_nodes: Vec::new(),
            strategic_rank: StrategicRank::new(strategic_rank),
            vulnerability_score: 0.0,
            tags: Vec::new(),
            created_at: Utc::now(),
            last_interaction: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a comprehensive profile
    pub fn with_details(
        alias: String,
        birth_metadata: Option<BirthMetadata>,
        strategic_rank: u8,
        tags: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            alias,
            birth_metadata,
            social_nodes: Vec::new(),
            strategic_rank: StrategicRank::new(strategic_rank),
            vulnerability_score: 0.0,
            tags,
            created_at: Utc::now(),
            last_interaction: None,
            metadata: HashMap::new(),
        }
    }

    /// Add a social connection
    pub fn add_social_node(&mut self, name: String, relationship: String, notes: Option<String>) {
        self.social_nodes.push(SocialNode {
            name,
            relationship,
            notes,
        });
    }

    /// Update last interaction timestamp
    pub fn record_interaction(&mut self) {
        self.last_interaction = Some(Utc::now());
    }

    /// Get communication strategy for this subject
    pub fn communication_strategy(&self) -> CommunicationStrategy {
        self.strategic_rank.communication_strategy()
    }

    /// Check if this subject has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }

    /// Add a tag if it doesn't exist
    pub fn add_tag(&mut self, tag: String) {
        if !self.has_tag(&tag) {
            self.tags.push(tag);
        }
    }

    /// Get zodiac sign if birth metadata exists
    pub fn zodiac_sign(&self) -> Option<ZodiacSign> {
        self.birth_metadata.as_ref().map(|bm| bm.sign)
    }

    /// Calculate days until next birthday
    pub fn days_until_birthday(&self) -> Option<i64> {
        self.birth_metadata.as_ref().map(|bm| {
            let today = Utc::now().date_naive();
            let this_year_birthday = NaiveDate::from_ymd_opt(
                today.year(),
                bm.date.month(),
                bm.date.day(),
            ).unwrap_or(today);

            let next_birthday = if this_year_birthday < today {
                NaiveDate::from_ymd_opt(
                    today.year() + 1,
                    bm.date.month(),
                    bm.date.day(),
                ).unwrap_or(today)
            } else {
                this_year_birthday
            };

            (next_birthday - today).num_days()
        })
    }
}

/// Compatibility calculator for astrological analysis
pub struct CompatibilityCalculator;

impl CompatibilityCalculator {
    /// Calculate base compatibility between two zodiac signs (0.0-1.0)
    pub fn calculate_compatibility(sign1: ZodiacSign, sign2: ZodiacSign) -> f32 {
        if sign1 == sign2 {
            return 0.7; // Same sign: moderate compatibility
        }

        let element1 = sign1.element();
        let element2 = sign2.element();

        // Element compatibility
        let element_score = match (element1, element2) {
            // Same element: high compatibility
            (Element::Fire, Element::Fire) => 0.8,
            (Element::Earth, Element::Earth) => 0.8,
            (Element::Air, Element::Air) => 0.8,
            (Element::Water, Element::Water) => 0.8,
            // Compatible elements
            (Element::Fire, Element::Air) | (Element::Air, Element::Fire) => 0.9,
            (Element::Earth, Element::Water) | (Element::Water, Element::Earth) => 0.9,
            // Challenging combinations
            (Element::Fire, Element::Water) | (Element::Water, Element::Fire) => 0.3,
            (Element::Earth, Element::Air) | (Element::Air, Element::Earth) => 0.4,
            // Neutral combinations
            (Element::Fire, Element::Earth) | (Element::Earth, Element::Fire) => 0.5,
            (Element::Air, Element::Water) | (Element::Water, Element::Air) => 0.5,
        };

        element_score
    }

    /// Calculate if a user is statistically more likely to over-give to a subject
    /// (Pisces users are particularly vulnerable to certain signs)
    pub fn calculate_savior_vulnerability(user_sign: ZodiacSign, subject_sign: ZodiacSign) -> f32 {
        let base_compatibility = Self::calculate_compatibility(user_sign, subject_sign);

        // Pisces-specific vulnerability patterns
        if user_sign == ZodiacSign::Pisces {
            match subject_sign {
                // Pisces tends to over-give to fire signs who need "saving"
                ZodiacSign::Aries | ZodiacSign::Leo | ZodiacSign::Sagittarius => {
                    base_compatibility * 1.5
                }
                // And to other water signs in crisis
                ZodiacSign::Cancer | ZodiacSign::Scorpio => base_compatibility * 1.3,
                _ => base_compatibility,
            }
        } else if user_sign == ZodiacSign::Cancer {
            // Cancer also has savior tendencies
            match subject_sign {
                ZodiacSign::Pisces | ZodiacSign::Scorpio => base_compatibility * 1.4,
                _ => base_compatibility,
            }
        } else {
            base_compatibility
        }
    }

    /// Get a compatibility modifier for interactions (-1.0 to 1.0)
    pub fn get_compatibility_modifier(user_sign: ZodiacSign, subject_sign: ZodiacSign) -> f32 {
        let compatibility = Self::calculate_compatibility(user_sign, subject_sign);
        // Convert 0.0-1.0 scale to -1.0 to 1.0 modifier
        (compatibility - 0.5) * 2.0
    }
}

/// Subject Profile Manager for KB-05 operations
pub struct SubjectProfileManager {
    profiles: HashMap<Uuid, SubjectProfile>,
}

impl SubjectProfileManager {
    /// Create a new manager
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Add or update a profile
    pub fn upsert_profile(&mut self, profile: SubjectProfile) {
        self.profiles.insert(profile.id, profile);
    }

    /// Get a profile by ID
    pub fn get_profile(&self, id: &Uuid) -> Option<&SubjectProfile> {
        self.profiles.get(id)
    }

    /// Get a mutable profile by ID
    pub fn get_profile_mut(&mut self, id: &Uuid) -> Option<&mut SubjectProfile> {
        self.profiles.get_mut(id)
    }

    /// Find profiles by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<&SubjectProfile> {
        self.profiles
            .values()
            .filter(|p| p.has_tag(tag))
            .collect()
    }

    /// Find profiles by strategic rank range
    pub fn find_by_rank_range(&self, min_rank: u8, max_rank: u8) -> Vec<&SubjectProfile> {
        self.profiles
            .values()
            .filter(|p| {
                let rank = p.strategic_rank.value();
                rank >= min_rank && rank <= max_rank
            })
            .collect()
    }

    /// Get high-priority subjects (rank >= 8)
    pub fn get_high_priority(&self) -> Vec<&SubjectProfile> {
        self.profiles
            .values()
            .filter(|p| p.strategic_rank.is_high_priority())
            .collect()
    }

    /// Register a disrespect event for a subject
    pub fn register_disrespect(
        &mut self,
        subject_id: Uuid,
        description: String,
        severity: f32,
        pattern_tags: Vec<String>,
    ) -> DisrespectEvent {
        let event = DisrespectEvent {
            id: Uuid::new_v4(),
            subject_id,
            timestamp: Utc::now(),
            description,
            severity: severity.clamp(0.0, 1.0),
            pattern_tags,
        };

        // Update the subject's vulnerability score based on disrespect
        if let Some(profile) = self.profiles.get_mut(&subject_id) {
            // Decrease vulnerability score when disrespect occurs
            profile.vulnerability_score = (profile.vulnerability_score - severity * 0.1).max(0.0);
            
            // Potentially downgrade strategic rank for severe/repeated disrespect
            if severity > 0.7 && profile.strategic_rank.value() > 0 {
                profile.strategic_rank = StrategicRank::new(profile.strategic_rank.value() - 1);
            }
        }

        event
    }

    /// Get upcoming birthdays (within next N days)
    pub fn get_upcoming_birthdays(&self, days: i64) -> Vec<(&SubjectProfile, i64)> {
        self.profiles
            .values()
            .filter_map(|p| {
                p.days_until_birthday()
                    .filter(|d| *d <= days)
                    .map(|d| (p, d))
            })
            .collect()
    }

    /// Serialize all profiles to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.profiles)
    }

    /// Deserialize profiles from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let profiles: HashMap<Uuid, SubjectProfile> = serde_json::from_str(json)?;
        Ok(Self { profiles })
    }
}

impl Default for SubjectProfileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zodiac_sign_calculation() {
        let date = NaiveDate::from_ymd_opt(1990, 3, 21).unwrap();
        assert_eq!(ZodiacSign::from_date(date), ZodiacSign::Aries);

        let date = NaiveDate::from_ymd_opt(1990, 12, 25).unwrap();
        assert_eq!(ZodiacSign::from_date(date), ZodiacSign::Capricorn);
    }

    #[test]
    fn test_subject_profile_creation() {
        let profile = SubjectProfile::new("John Doe".to_string(), 8);
        assert_eq!(profile.alias, "John Doe");
        assert_eq!(profile.strategic_rank.value(), 8);
        assert!(profile.strategic_rank.is_high_priority());
    }

    #[test]
    fn test_communication_strategy() {
        let low_rank = StrategicRank::new(1);
        assert_eq!(
            low_rank.communication_strategy(),
            CommunicationStrategy::GrayRock
        );

        let high_rank = StrategicRank::new(10);
        assert_eq!(
            high_rank.communication_strategy(),
            CommunicationStrategy::FullSovereign
        );
    }

    #[test]
    fn test_compatibility_calculation() {
        let compat = CompatibilityCalculator::calculate_compatibility(
            ZodiacSign::Pisces,
            ZodiacSign::Cancer,
        );
        assert!(compat > 0.7); // Water signs are compatible

        let compat = CompatibilityCalculator::calculate_compatibility(
            ZodiacSign::Pisces,
            ZodiacSign::Aries,
        );
        assert!(compat < 0.5); // Water and Fire are challenging
    }

    #[test]
    fn test_savior_vulnerability() {
        let vuln = CompatibilityCalculator::calculate_savior_vulnerability(
            ZodiacSign::Pisces,
            ZodiacSign::Aries,
        );
        // Pisces should have elevated vulnerability to Aries
        let base = CompatibilityCalculator::calculate_compatibility(
            ZodiacSign::Pisces,
            ZodiacSign::Aries,
        );
        assert!(vuln > base);
    }

    #[test]
    fn test_profile_manager() {
        let mut manager = SubjectProfileManager::new();
        let profile = SubjectProfile::new("Test Subject".to_string(), 5);
        let id = profile.id;

        manager.upsert_profile(profile);
        assert!(manager.get_profile(&id).is_some());

        let high_priority = manager.get_high_priority();
        assert_eq!(high_priority.len(), 0); // Rank 5 is not high priority
    }

    #[test]
    fn test_disrespect_event() {
        let mut manager = SubjectProfileManager::new();
        let mut profile = SubjectProfile::new("Problem Person".to_string(), 7);
        let id = profile.id;
        profile.vulnerability_score = 0.8;

        manager.upsert_profile(profile);

        let event = manager.register_disrespect(
            id,
            "Boundary violation".to_string(),
            0.8,
            vec!["manipulation".to_string()],
        );

        assert_eq!(event.subject_id, id);
        assert_eq!(event.severity, 0.8);

        // Check that vulnerability score decreased
        let updated_profile = manager.get_profile(&id).unwrap();
        assert!(updated_profile.vulnerability_score < 0.8);
    }
}

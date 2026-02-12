//! Social Intelligence Layer — KB-05 (Techne) & KB-07 (Kardia) Integration
//!
//! This module provides the **Subject Profile** system for tracking contacts, relationships,
//! and strategic importance within the GSI Framework. It extends the base PersonRecord with:
//! - Birthday tracking and reminder logic
//! - Astrological metadata (birth signs, compatibility)
//! - Strategic importance scoring (Resource Drain vs. Domain Asset)
//! - Children/dependents tracking
//! - Multi-vertical contact management (Personal, Ranch, Finance, etc.)
//!
//! ## Architecture Alignment
//!
//! | Component | KB Slot | Purpose |
//! |-----------|---------|---------|
//! | SubjectProfile | KB-07 (Kardia) | Core relationship data, trust, attachment |
//! | StrategicValue | KB-02 (Oikos) | Manipulation detection, resource drain analysis |
//! | AstralContext | KB-01 (Pneuma) | Archetype alignment, compatibility scoring |
//! | ContactReminders | KB-04 (Chronos) | Birthday alerts, interaction cadence |

use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// -----------------------------------------------------------------------------
// Astrological Context (KB-01 Pneuma Integration)
// -----------------------------------------------------------------------------

/// Zodiac signs for astrological profiling and compatibility analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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
    /// Determine zodiac sign from birth date (month/day).
    pub fn from_date(month: u32, day: u32) -> Option<Self> {
        match (month, day) {
            (3, 21..=31) | (4, 1..=19) => Some(Self::Aries),
            (4, 20..=30) | (5, 1..=20) => Some(Self::Taurus),
            (5, 21..=31) | (6, 1..=20) => Some(Self::Gemini),
            (6, 21..=30) | (7, 1..=22) => Some(Self::Cancer),
            (7, 23..=31) | (8, 1..=22) => Some(Self::Leo),
            (8, 23..=31) | (9, 1..=22) => Some(Self::Virgo),
            (9, 23..=30) | (10, 1..=22) => Some(Self::Libra),
            (10, 23..=31) | (11, 1..=21) => Some(Self::Scorpio),
            (11, 22..=30) | (12, 1..=21) => Some(Self::Sagittarius),
            (12, 22..=31) | (1, 1..=19) => Some(Self::Capricorn),
            (1, 20..=31) | (2, 1..=18) => Some(Self::Aquarius),
            (2, 19..=29) | (3, 1..=20) => Some(Self::Pisces),
            _ => None,
        }
    }

    /// Archetype vulnerability mapping (e.g., Pisces = "Provider/Savior Override").
    pub fn archetype_vulnerability(&self) -> &'static str {
        match self {
            Self::Pisces => "Provider/Savior Override — High empathy, prone to resource drain",
            Self::Cancer => "Nurturer — Emotional investment risk, boundary challenges",
            Self::Virgo => "Perfectionist — Over-responsibility, burnout from fixing others",
            Self::Libra => "Peacemaker — Conflict avoidance, people-pleasing drain",
            Self::Sagittarius => "Optimist — Over-commitment, scattered energy",
            Self::Capricorn => "Builder — Workaholic tendencies, neglecting self-care",
            Self::Aries => "Warrior — Impulsive decisions, conflict escalation",
            Self::Taurus => "Stabilizer — Resistance to change, stubbornness",
            Self::Gemini => "Communicator — Scattered focus, over-commitment",
            Self::Leo => "Leader — Ego-driven decisions, validation-seeking",
            Self::Scorpio => "Transformer — Intensity, trust issues",
            Self::Aquarius => "Visionary — Detachment, neglecting emotional needs",
        }
    }

    /// Compatibility score with another sign (0.0 = incompatible, 1.0 = highly compatible).
    /// Simplified model; can be expanded with full astrological logic.
    pub fn compatibility_score(&self, other: &Self) -> f32 {
        // Same element = high compatibility
        let self_element = self.element();
        let other_element = other.element();
        
        if self_element == other_element {
            0.85
        } else if self.is_complementary(other) {
            0.75
        } else if self.is_challenging(other) {
            0.35
        } else {
            0.55 // Neutral
        }
    }

    fn element(&self) -> &'static str {
        match self {
            Self::Aries | Self::Leo | Self::Sagittarius => "Fire",
            Self::Taurus | Self::Virgo | Self::Capricorn => "Earth",
            Self::Gemini | Self::Libra | Self::Aquarius => "Air",
            Self::Cancer | Self::Scorpio | Self::Pisces => "Water",
        }
    }

    fn is_complementary(&self, other: &Self) -> bool {
        matches!(
            (self.element(), other.element()),
            ("Fire", "Air") | ("Air", "Fire") | ("Earth", "Water") | ("Water", "Earth")
        )
    }

    fn is_challenging(&self, other: &Self) -> bool {
        matches!(
            (self.element(), other.element()),
            ("Fire", "Water") | ("Water", "Fire") | ("Earth", "Air") | ("Air", "Earth")
        )
    }
}

/// Astrological metadata for a subject (stored in SubjectProfile).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstralContext {
    /// Zodiac sign (derived from birthday).
    pub sign: ZodiacSign,
    /// Archetype vulnerability notes (auto-populated from sign).
    pub archetype_notes: String,
    /// Compatibility score with the user's sign (if user sign is known).
    #[serde(default)]
    pub compatibility_with_user: Option<f32>,
}

impl AstralContext {
    /// Create from a birth date.
    pub fn from_date(date: NaiveDate, user_sign: Option<ZodiacSign>) -> Option<Self> {
        let sign = ZodiacSign::from_date(date.month(), date.day())?;
        let archetype_notes = sign.archetype_vulnerability().to_string();
        let compatibility_with_user = user_sign.map(|us| sign.compatibility_score(&us));
        
        Some(Self {
            sign,
            archetype_notes,
            compatibility_with_user,
        })
    }
}

// -----------------------------------------------------------------------------
// Strategic Value Assessment (KB-02 Oikos — Manipulation Detection)
// -----------------------------------------------------------------------------

/// Strategic importance of a subject to the user's domain.
/// Integrates with the Protector trait for resource drain analysis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicImportance {
    /// Critical to domain sovereignty (e.g., spouse, business partner, key client).
    Critical,
    /// High value, regular interaction (e.g., close friend, important colleague).
    High,
    /// Moderate value, occasional interaction (e.g., extended family, casual friend).
    Moderate,
    /// Low value, rare interaction (e.g., acquaintance, distant relative).
    Low,
    /// Potential resource drain — requires monitoring (e.g., manipulative person, energy vampire).
    ResourceDrain,
    /// Confirmed threat to domain — avoid or minimize contact.
    Threat,
}

impl StrategicImportance {
    /// Numeric score for sorting/filtering (higher = more important).
    pub fn score(&self) -> i32 {
        match self {
            Self::Critical => 100,
            Self::High => 75,
            Self::Moderate => 50,
            Self::Low => 25,
            Self::ResourceDrain => -25,
            Self::Threat => -100,
        }
    }

    /// True if this subject should trigger Protector alerts.
    pub fn requires_monitoring(&self) -> bool {
        matches!(self, Self::ResourceDrain | Self::Threat)
    }
}

/// Strategic value assessment for a subject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategicValue {
    /// Overall importance to the user's domain.
    pub importance: StrategicImportance,
    /// Specific domain roles (e.g., "Finance: Tax Accountant", "Ranch: Vet").
    #[serde(default)]
    pub domain_roles: Vec<String>,
    /// Resource drain indicators (e.g., "Frequent last-minute requests", "Emotional dumping").
    #[serde(default)]
    pub drain_indicators: Vec<String>,
    /// Positive contributions (e.g., "Reliable support", "Valuable expertise").
    #[serde(default)]
    pub contributions: Vec<String>,
    /// Last strategic review date (for periodic reassessment).
    #[serde(default)]
    pub last_review: Option<chrono::DateTime<Utc>>,
}

impl Default for StrategicValue {
    fn default() -> Self {
        Self {
            importance: StrategicImportance::Moderate,
            domain_roles: Vec::new(),
            drain_indicators: Vec::new(),
            contributions: Vec::new(),
            last_review: None,
        }
    }
}

impl StrategicValue {
    /// Auto-adjust importance based on drain indicators vs. contributions.
    pub fn auto_adjust_importance(&mut self) {
        let drain_count = self.drain_indicators.len() as i32;
        let contribution_count = self.contributions.len() as i32;
        let net_value = contribution_count - drain_count;

        // If drains significantly outweigh contributions, flag as ResourceDrain
        if net_value < -2 && !matches!(self.importance, StrategicImportance::Critical) {
            self.importance = StrategicImportance::ResourceDrain;
        }
    }
}

// -----------------------------------------------------------------------------
// Subject Profile (KB-07 Kardia — Enhanced Contact Record)
// -----------------------------------------------------------------------------

/// Comprehensive subject profile for the Social Intelligence Layer.
/// Extends the base PersonRecord with birthday tracking, astrological data,
/// strategic importance, and multi-vertical contact management.
///
/// ## Storage
/// - Primary: KB-07 (Kardia) under key `subjects/{name_slug}`
/// - Cross-references: KB-02 (manipulation detection), KB-04 (reminders)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectProfile {
    // --- Core Identity (from PersonRecord) ---
    /// Display name (e.g., "Sarah Johnson", "Dr. Martinez").
    pub name: String,
    /// Relationship role (e.g., "Spouse", "Boss", "Ranch Hand", "Tax Accountant").
    pub relationship: String,
    /// Trust level 0.0–1.0 (inherited from PersonRecord).
    #[serde(default = "default_trust")]
    pub trust_score: f32,
    /// Attachment style (e.g., "Secure", "Anxious", "Avoidant").
    #[serde(default)]
    pub attachment_style: String,
    /// Known triggers (e.g., "criticism", "being ignored").
    #[serde(default)]
    pub triggers: Vec<String>,

    // --- Birthday & Temporal Tracking (KB-04 Integration) ---
    /// Birth date (for birthday reminders and age calculation).
    #[serde(default)]
    pub birthday: Option<NaiveDate>,
    /// Days until next birthday (auto-calculated).
    #[serde(skip)]
    pub days_until_birthday: Option<i64>,
    /// Astrological context (zodiac sign, compatibility).
    #[serde(default)]
    pub astral_context: Option<AstralContext>,

    // --- Children & Dependents ---
    /// Names and ages of children/dependents (e.g., "Emma (8)", "Noah (5)").
    #[serde(default)]
    pub children: Vec<String>,
    /// Important dates for children (e.g., "Emma's birthday: 2018-03-15").
    #[serde(default)]
    pub child_dates: HashMap<String, NaiveDate>,

    // --- Strategic Value (KB-02 Integration) ---
    /// Strategic importance and resource drain analysis.
    #[serde(default)]
    pub strategic_value: StrategicValue,

    // --- Multi-Vertical Context ---
    /// Vertical-specific metadata (e.g., "Finance: CPA License #12345", "Ranch: Preferred Vet").
    #[serde(default)]
    pub vertical_metadata: HashMap<String, String>,

    // --- Interaction History (KB-04 Integration) ---
    /// Last interaction summary (inherited from PersonRecord).
    #[serde(default)]
    pub last_interaction_summary: Option<String>,
    /// Last interaction date.
    #[serde(default)]
    pub last_interaction_date: Option<chrono::DateTime<Utc>>,
    /// Recommended interaction cadence in days (e.g., 7 for weekly check-ins).
    #[serde(default)]
    pub interaction_cadence_days: Option<u32>,

    // --- Metadata ---
    /// When this profile was created.
    #[serde(default = "chrono::Utc::now")]
    pub created_at: chrono::DateTime<Utc>,
    /// When this profile was last updated.
    #[serde(default = "chrono::Utc::now")]
    pub updated_at: chrono::DateTime<Utc>,
}

fn default_trust() -> f32 {
    0.5
}

impl Default for SubjectProfile {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            name: String::new(),
            relationship: String::new(),
            trust_score: 0.5,
            attachment_style: String::new(),
            triggers: Vec::new(),
            birthday: None,
            days_until_birthday: None,
            astral_context: None,
            children: Vec::new(),
            child_dates: HashMap::new(),
            strategic_value: StrategicValue::default(),
            vertical_metadata: HashMap::new(),
            last_interaction_summary: None,
            last_interaction_date: None,
            interaction_cadence_days: None,
            created_at: now,
            updated_at: now,
        }
    }
}

impl SubjectProfile {
    /// Create a new subject profile with minimal required fields.
    pub fn new(name: String, relationship: String) -> Self {
        Self {
            name,
            relationship,
            ..Default::default()
        }
    }

    /// Set birthday and auto-populate astrological context.
    pub fn with_birthday(mut self, birthday: NaiveDate, user_sign: Option<ZodiacSign>) -> Self {
        self.birthday = Some(birthday);
        self.astral_context = AstralContext::from_date(birthday, user_sign);
        self.calculate_days_until_birthday();
        self
    }

    /// Add a child with optional birthday.
    pub fn add_child(&mut self, name: String, birthday: Option<NaiveDate>) {
        let display = if let Some(bd) = birthday {
            let age = Self::calculate_age(bd);
            self.child_dates.insert(name.clone(), bd);
            format!("{} ({})", name, age)
        } else {
            name.clone()
        };
        self.children.push(display);
        self.updated_at = Utc::now();
    }

    /// Calculate age from birth date.
    pub fn calculate_age(birth_date: NaiveDate) -> u32 {
        let today = Utc::now().date_naive();
        let mut age = today.year() - birth_date.year();
        if today.month() < birth_date.month()
            || (today.month() == birth_date.month() && today.day() < birth_date.day())
        {
            age -= 1;
        }
        age as u32
    }

    /// Calculate days until next birthday.
    pub fn calculate_days_until_birthday(&mut self) {
        if let Some(bd) = self.birthday {
            let today = Utc::now().date_naive();
            let this_year_birthday = NaiveDate::from_ymd_opt(today.year(), bd.month(), bd.day());
            
            if let Some(this_year_bd) = this_year_birthday {
                let days = if this_year_bd >= today {
                    (this_year_bd - today).num_days()
                } else {
                    // Birthday already passed this year, calculate for next year
                    if let Some(next_year_bd) = NaiveDate::from_ymd_opt(today.year() + 1, bd.month(), bd.day()) {
                        (next_year_bd - today).num_days()
                    } else {
                        return;
                    }
                };
                self.days_until_birthday = Some(days);
            }
        }
    }

    /// True if birthday is within the next N days (for reminder triggers).
    pub fn birthday_approaching(&self, days_threshold: i64) -> bool {
        self.days_until_birthday
            .map(|d| d <= days_threshold && d >= 0)
            .unwrap_or(false)
    }

    /// True if this subject requires monitoring (resource drain or threat).
    pub fn requires_monitoring(&self) -> bool {
        self.strategic_value.importance.requires_monitoring()
    }

    /// True if interaction cadence is overdue.
    pub fn interaction_overdue(&self) -> bool {
        if let (Some(last), Some(cadence)) = (self.last_interaction_date, self.interaction_cadence_days) {
            let days_since = (Utc::now() - last).num_days();
            days_since > cadence as i64
        } else {
            false
        }
    }

    /// Generate a storage key slug from the name.
    pub fn name_slug(&self) -> String {
        Self::slug_from_name(&self.name)
    }

    /// Generate a slug from any name string.
    pub fn slug_from_name(name: &str) -> String {
        let s: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_lowercase().next().unwrap_or(c)
                } else if c.is_whitespace() {
                    '_'
                } else {
                    '_'
                }
            })
            .collect();
        let mut s = s.replace("__", "_");
        while s.contains("__") {
            s = s.replace("__", "_");
        }
        let s = s.trim_matches('_').to_string();
        if s.is_empty() {
            "unnamed".to_string()
        } else {
            s
        }
    }

    /// Full storage key for KB-07 (Kardia).
    pub fn storage_key(&self) -> String {
        format!("subjects/{}", self.name_slug())
    }

    /// Clamp numeric fields to valid ranges.
    pub fn clamp(&mut self) {
        self.trust_score = self.trust_score.clamp(0.0, 1.0);
    }

    /// Update the last interaction timestamp and summary.
    pub fn record_interaction(&mut self, summary: Option<String>) {
        self.last_interaction_date = Some(Utc::now());
        self.last_interaction_summary = summary;
        self.updated_at = Utc::now();
    }

    /// Auto-adjust strategic importance based on current data.
    pub fn auto_adjust_strategic_value(&mut self) {
        self.strategic_value.auto_adjust_importance();
        self.strategic_value.last_review = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

// -----------------------------------------------------------------------------
// Contact Reminder System (KB-04 Chronos Integration)
// -----------------------------------------------------------------------------

/// Reminder type for contact management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContactReminder {
    /// Birthday reminder (N days before).
    Birthday {
        subject_name: String,
        days_until: i64,
        date: NaiveDate,
    },
    /// Interaction cadence reminder (overdue check-in).
    InteractionOverdue {
        subject_name: String,
        days_since_last: i64,
        recommended_cadence: u32,
    },
    /// Child's birthday reminder.
    ChildBirthday {
        subject_name: String,
        child_name: String,
        days_until: i64,
        date: NaiveDate,
    },
}

impl ContactReminder {
    /// Generate reminders for a subject profile.
    pub fn generate_for_profile(profile: &SubjectProfile, birthday_threshold: i64) -> Vec<Self> {
        let mut reminders = Vec::new();

        // Birthday reminder
        if profile.birthday_approaching(birthday_threshold) {
            if let (Some(bd), Some(days)) = (profile.birthday, profile.days_until_birthday) {
                reminders.push(Self::Birthday {
                    subject_name: profile.name.clone(),
                    days_until: days,
                    date: bd,
                });
            }
        }

        // Interaction overdue reminder
        if profile.interaction_overdue() {
            if let (Some(last), Some(cadence)) = (profile.last_interaction_date, profile.interaction_cadence_days) {
                let days_since = (Utc::now() - last).num_days();
                reminders.push(Self::InteractionOverdue {
                    subject_name: profile.name.clone(),
                    days_since_last: days_since,
                    recommended_cadence: cadence,
                });
            }
        }

        // Child birthday reminders
        for (child_name, child_bd) in &profile.child_dates {
            let today = Utc::now().date_naive();
            let this_year_birthday = NaiveDate::from_ymd_opt(today.year(), child_bd.month(), child_bd.day());
            
            if let Some(this_year_bd) = this_year_birthday {
                let days = if this_year_bd >= today {
                    (this_year_bd - today).num_days()
                } else {
                    if let Some(next_year_bd) = NaiveDate::from_ymd_opt(today.year() + 1, child_bd.month(), child_bd.day()) {
                        (next_year_bd - today).num_days()
                    } else {
                        continue;
                    }
                };

                if days <= birthday_threshold && days >= 0 {
                    reminders.push(Self::ChildBirthday {
                        subject_name: profile.name.clone(),
                        child_name: child_name.clone(),
                        days_until: days,
                        date: *child_bd,
                    });
                }
            }
        }

        reminders
    }

    /// Human-readable reminder message.
    pub fn message(&self) -> String {
        match self {
            Self::Birthday { subject_name, days_until, date } => {
                if *days_until == 0 {
                    format!("🎂 Today is {}'s birthday! ({})", subject_name, date.format("%B %d"))
                } else if *days_until == 1 {
                    format!("🎂 Tomorrow is {}'s birthday! ({})", subject_name, date.format("%B %d"))
                } else {
                    format!("🎂 {}'s birthday is in {} days ({})", subject_name, days_until, date.format("%B %d"))
                }
            }
            Self::InteractionOverdue { subject_name, days_since_last, recommended_cadence } => {
                format!(
                    "⏰ It's been {} days since you last connected with {} (recommended: every {} days)",
                    days_since_last, subject_name, recommended_cadence
                )
            }
            Self::ChildBirthday { subject_name, child_name, days_until, date } => {
                if *days_until == 0 {
                    format!("🎂 Today is {}'s child {}'s birthday! ({})", subject_name, child_name, date.format("%B %d"))
                } else if *days_until == 1 {
                    format!("🎂 Tomorrow is {}'s child {}'s birthday! ({})", subject_name, child_name, date.format("%B %d"))
                } else {
                    format!("🎂 {}'s child {}'s birthday is in {} days ({})", subject_name, child_name, days_until, date.format("%B %d"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zodiac_from_date() {
        assert_eq!(ZodiacSign::from_date(3, 15), Some(ZodiacSign::Pisces));
        assert_eq!(ZodiacSign::from_date(7, 25), Some(ZodiacSign::Leo));
        assert_eq!(ZodiacSign::from_date(12, 25), Some(ZodiacSign::Capricorn));
    }

    #[test]
    fn test_compatibility_score() {
        let pisces = ZodiacSign::Pisces;
        let cancer = ZodiacSign::Cancer;
        let aries = ZodiacSign::Aries;
        
        // Same element (Water) = high compatibility
        assert!(pisces.compatibility_score(&cancer) > 0.8);
        
        // Challenging elements (Water vs Fire) = low compatibility
        assert!(pisces.compatibility_score(&aries) < 0.5);
    }

    #[test]
    fn test_subject_profile_creation() {
        let mut profile = SubjectProfile::new("Sarah Johnson".to_string(), "Spouse".to_string())
            .with_birthday(NaiveDate::from_ymd_opt(1985, 3, 15).unwrap(), Some(ZodiacSign::Pisces));
        
        assert_eq!(profile.name, "Sarah Johnson");
        assert_eq!(profile.relationship, "Spouse");
        assert!(profile.astral_context.is_some());
        
        profile.add_child("Emma".to_string(), Some(NaiveDate::from_ymd_opt(2016, 5, 20).unwrap()));
        assert_eq!(profile.children.len(), 1);
    }

    #[test]
    fn test_strategic_value_auto_adjust() {
        let mut value = StrategicValue {
            importance: StrategicImportance::High,
            drain_indicators: vec!["Late payments".to_string(), "Frequent complaints".to_string(), "Unrealistic demands".to_string()],
            contributions: vec![],
            ..Default::default()
        };
        
        value.auto_adjust_importance();
        assert_eq!(value.importance, StrategicImportance::ResourceDrain);
    }

    #[test]
    fn test_birthday_approaching() {
        let mut profile = SubjectProfile::new("Test".to_string(), "Friend".to_string());
        
        // Set birthday to 5 days from now
        let today = Utc::now().date_naive();
        let future_bd = today + chrono::Duration::days(5);
        profile.birthday = Some(NaiveDate::from_ymd_opt(1990, future_bd.month(), future_bd.day()).unwrap());
        profile.calculate_days_until_birthday();
        
        assert!(profile.birthday_approaching(7));
        assert!(!profile.birthday_approaching(3));
    }
}

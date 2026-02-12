//! Sovereign configuration loaded from `.env`.
//!
//! Toggles for the Phoenix Warden: firewall strictness, astro alerts, KB-05 auto-rank,
//! KB-08 logging level, and human-in-the-loop promotion. Change behavior without code edits.

use serde::{Deserialize, Serialize};

fn default_true() -> bool {
    true
}

fn default_kb08_logging_level() -> String {
    "full".to_string()
}

/// Sovereign configuration loaded from environment.
///
/// | Env | Default | Description |
/// |-----|---------|--------------|
/// | PAGI_FIREWALL_STRICT_MODE | false | If true, only Core (Tier 1) skills may touch any KB layer. |
/// | PAGI_ASTRO_ALERTS_ENABLED | true | Astro-Transit scraper (on-boot + 6h refresh). |
/// | PAGI_SOVEREIGNTY_AUTO_RANK / PAGI_SOVEREIGNTY_AUTO_RANK_ENABLED | true | KB-05 social defense ranking. |
/// | PAGI_KB08_LOGGING_LEVEL | full | "minimal" \| "full" — success metric verbosity in KB-08. |
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SovereignConfig {
    /// PAGI_FIREWALL_STRICT_MODE: When true, blocks ALL non-Core skills from any KB access (dispatch-level).
    #[serde(default)]
    pub firewall_strict_mode: bool,
    /// PAGI_ASTRO_LOGIC_ENABLED: Apply archetype directives (Pisces/Savior, tone overrides) from KB-01.
    #[serde(default = "default_true")]
    pub astro_logic_enabled: bool,
    /// PAGI_ASTRO_ALERTS_ENABLED or PAGI_TRANSIT_ALERTS_ENABLED: Run Astro-Transit scraper (background refresh every 6h).
    #[serde(default = "default_true")]
    pub astro_alerts_enabled: bool,
    /// Legacy alias: same as astro_alerts_enabled (for backward compatibility).
    #[serde(default = "default_true")]
    pub transit_alerts_enabled: bool,
    /// PAGI_SOVEREIGNTY_AUTO_RANK or PAGI_SOVEREIGNTY_AUTO_RANK_ENABLED: KB-05 auto-rank subjects from sovereignty_leak_triggers.
    #[serde(default = "default_true")]
    pub sovereignty_auto_rank_enabled: bool,
    /// PAGI_SKILLS_AUTO_PROMOTE_ALLOWED: Allow promotion without confirmed: true (human-in-the-loop).
    #[serde(default)]
    pub skills_auto_promote_allowed: bool,
    /// PAGI_KB08_SUCCESS_LOGGING: Audit successes (e.g. Failed Leak Attempt) to KB-08.
    #[serde(default = "default_true")]
    pub kb08_success_logging: bool,
    /// PAGI_KB08_LOGGING_LEVEL: "minimal" | "full" — controls success metric verbosity in KB-08 (Soma).
    #[serde(default = "default_kb08_logging_level")]
    pub kb08_logging_level: String,
    /// PAGI_STRICT_TECHNICAL_MODE: Force temperature 0.3 regardless of user/archetype input.
    #[serde(default)]
    pub strict_technical_mode: bool,
    /// PAGI_DAILY_CHECKIN_ENABLED: On first user interaction of the day, prepend a 1–2 sentence morning briefing (transit + energy habits). Default: true.
    #[serde(default = "default_true")]
    pub daily_checkin_enabled: bool,
    /// PAGI_EVENING_AUDIT_ENABLED: After audit start hour, prepend a reflective question once per day; capture Pass/Fail/Lesson to KB-08. Default: true.
    #[serde(default = "default_true")]
    pub evening_audit_enabled: bool,
    /// PAGI_AUDIT_START_HOUR: Hour of day (0–23, UTC) when evening audit prompt may appear. Default: 18 (6 PM).
    #[serde(default = "default_audit_start_hour")]
    pub audit_start_hour: u8,
    /// PAGI_FOCUS_SHIELD_ENABLED: When true, use MS Graph (Viva Insights) for Schedule Outlook and Gatekeeper mode (shorter responses during Focus/Quiet time). Requires MS_GRAPH_* env vars.
    #[serde(default)]
    pub focus_shield_enabled: bool,
    /// MS_GRAPH_HEALTH_ENABLED (Vitality Shield): When true, fetch sleep/activity from MS Graph Beta or KB-08 and reduce emotional load when sleep < 6h (bias Virgo).
    #[serde(default)]
    pub vitality_shield_enabled: bool,
    /// PAGI_HUMANITY_RATIO: 0.0 = Pure Architect (dry, technical, code-only). 1.0 = Pure Archetype (Pisces tone). Default 0.7 (warm but structured).
    #[serde(default = "default_humanity_ratio")]
    pub humanity_ratio: f32,
    /// PAGI_PRIMARY_ARCHETYPE: Tone overlay for the Advisor (pisces, virgo, scorpio, libra, cancer). Default: pisces.
    #[serde(default)]
    pub primary_archetype: Option<String>,
    /// PAGI_SECONDARY_ARCHETYPE: Optional second overlay to blend (e.g. virgo for balanced approach).
    #[serde(default)]
    pub secondary_archetype: Option<String>,
    /// PAGI_ARCHETYPE_OVERRIDE: When set, overrides primary/secondary for this run (e.g. task-based shift).
    #[serde(default)]
    pub archetype_override: Option<String>,
    /// PAGI_ARCHETYPE_AUTO_SWITCH: When true, suggest overlay from query domain (Technical→Virgo, etc.). KB-01 can still disable via tone_preference or always_direct.
    #[serde(default = "default_true")]
    pub archetype_auto_switch_enabled: bool,
    /// PAGI_FORGE_SAFETY_ENABLED: When true (default), Phoenix must request approval before compiling self-generated code. When false, autonomous evolution mode is enabled.
    #[serde(default = "default_true")]
    pub forge_safety_enabled: bool,
}

fn default_humanity_ratio() -> f32 {
    0.7
}

fn default_audit_start_hour() -> u8 {
    18
}

impl SovereignConfig {
    /// Load toggles from environment. Unset or invalid => defaults (see struct field docs).
    pub fn from_env() -> Self {
        let transit = env_bool("PAGI_TRANSIT_ALERTS_ENABLED", true);
        let astro_alerts = env_bool("PAGI_ASTRO_ALERTS_ENABLED", transit);
        let sovereignty = env_bool("PAGI_SOVEREIGNTY_AUTO_RANK_ENABLED", true)
            || env_bool("PAGI_SOVEREIGNTY_AUTO_RANK", true);
        Self {
            firewall_strict_mode: env_bool("PAGI_FIREWALL_STRICT_MODE", false),
            astro_logic_enabled: env_bool("PAGI_ASTRO_LOGIC_ENABLED", true),
            astro_alerts_enabled: astro_alerts,
            transit_alerts_enabled: astro_alerts,
            sovereignty_auto_rank_enabled: sovereignty,
            skills_auto_promote_allowed: env_bool("PAGI_SKILLS_AUTO_PROMOTE_ALLOWED", false),
            kb08_success_logging: env_bool("PAGI_KB08_SUCCESS_LOGGING", true),
            kb08_logging_level: env_kb08_logging_level(),
            strict_technical_mode: env_bool("PAGI_STRICT_TECHNICAL_MODE", false),
            daily_checkin_enabled: env_bool("PAGI_DAILY_CHECKIN_ENABLED", true),
            evening_audit_enabled: env_bool("PAGI_EVENING_AUDIT_ENABLED", true),
            audit_start_hour: env_audit_start_hour(),
            focus_shield_enabled: env_bool("PAGI_FOCUS_SHIELD_ENABLED", false),
            vitality_shield_enabled: env_bool("MS_GRAPH_HEALTH_ENABLED", false),
            humanity_ratio: env_humanity_ratio(),
            primary_archetype: env_opt_string("PAGI_PRIMARY_ARCHETYPE"),
            secondary_archetype: env_opt_string("PAGI_SECONDARY_ARCHETYPE"),
            archetype_override: env_opt_string("PAGI_ARCHETYPE_OVERRIDE"),
            archetype_auto_switch_enabled: env_bool("PAGI_ARCHETYPE_AUTO_SWITCH", true),
            forge_safety_enabled: env_bool("PAGI_FORGE_SAFETY_ENABLED", true),
        }
    }

    /// True when KB-08 should log success metrics with full verbosity.
    pub fn kb08_logging_full(&self) -> bool {
        self.kb08_logging_level.trim().eq_ignore_ascii_case("full")
    }
}

fn env_bool(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(v) => v.trim().eq_ignore_ascii_case("true") || (v.trim().is_empty() && default),
        Err(_) => default,
    }
}

fn env_kb08_logging_level() -> String {
    match std::env::var("PAGI_KB08_LOGGING_LEVEL") {
        Ok(v) => {
            let s = v.trim().to_lowercase();
            if s == "minimal" {
                "minimal".to_string()
            } else {
                "full".to_string()
            }
        }
        Err(_) => "full".to_string(),
    }
}

fn env_audit_start_hour() -> u8 {
    match std::env::var("PAGI_AUDIT_START_HOUR") {
        Ok(v) => v.trim().parse().unwrap_or(18).min(23),
        Err(_) => 18,
    }
}

fn env_humanity_ratio() -> f32 {
    match std::env::var("PAGI_HUMANITY_RATIO") {
        Ok(v) => v.trim().parse::<f32>().unwrap_or(0.7).clamp(0.0, 1.0),
        Err(_) => 0.7,
    }
}

fn env_opt_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

// ══════════════════════════════════════════════════════════════════════════════
// USER CONFIGURATION MANAGER (Beta Distribution)
// ══════════════════════════════════════════════════════════════════════════════
// Manages user-specific configuration (API keys, personal settings) stored locally
// in user_config.toml. This allows beta users to provide their own OpenRouter keys
// without modifying the codebase or environment variables.

use std::fs;
use std::path::{Path, PathBuf};

/// User-specific configuration stored in user_config.toml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    /// User's personal OpenRouter API key (or other LLM provider key)
    #[serde(default)]
    pub api_key: Option<String>,
    
    /// Optional: User's preferred LLM model
    #[serde(default)]
    pub llm_model: Option<String>,
    
    /// Optional: User's preferred LLM API URL
    #[serde(default)]
    pub llm_api_url: Option<String>,
    
    /// Optional: User's name or identifier (for personalization)
    #[serde(default)]
    pub user_name: Option<String>,
    
    /// First run flag - set to false after initial setup
    #[serde(default = "default_true")]
    pub first_run: bool,
    
    /// Beta version the user is running
    #[serde(default)]
    pub version: Option<String>,
}

impl UserConfig {
    /// Default path for user configuration file
    pub fn default_path() -> PathBuf {
        PathBuf::from("user_config.toml")
    }
    
    /// Load user configuration from file, or create default if not exists
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_from_path(&Self::default_path())
    }
    
    /// Load user configuration from specific path
    pub fn load_from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let config: UserConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            // First run - create default config
            let config = UserConfig {
                first_run: true,
                ..Default::default()
            };
            config.save_to_path(path)?;
            Ok(config)
        }
    }
    
    /// Save user configuration to default path
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.save_to_path(&Self::default_path())
    }
    
    /// Save user configuration to specific path
    pub fn save_to_path(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(path, content)?;
        Ok(())
    }
    
    /// Update API key and save
    pub fn set_api_key(&mut self, key: String) -> Result<(), Box<dyn std::error::Error>> {
        self.api_key = Some(key);
        self.first_run = false;
        self.save()
    }
    
    /// Get API key with fallback to environment variables
    pub fn get_api_key(&self) -> Option<String> {
        // Priority: user_config.toml > PAGI_LLM_API_KEY > OPENROUTER_API_KEY
        self.api_key.clone()
            .or_else(|| std::env::var("PAGI_LLM_API_KEY").ok())
            .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
            .filter(|s| !s.trim().is_empty())
    }
    
    /// Get LLM model with fallback to environment
    pub fn get_llm_model(&self) -> Option<String> {
        self.llm_model.clone()
            .or_else(|| std::env::var("PAGI_LLM_MODEL").ok())
            .filter(|s| !s.trim().is_empty())
    }
    
    /// Get LLM API URL with fallback to environment
    pub fn get_llm_api_url(&self) -> Option<String> {
        self.llm_api_url.clone()
            .or_else(|| std::env::var("PAGI_LLM_API_URL").ok())
            .filter(|s| !s.trim().is_empty())
    }
    
    /// Check if this is a first run (no API key configured)
    pub fn is_first_run(&self) -> bool {
        self.first_run || self.api_key.is_none()
    }
    
    /// Mark first run as complete
    pub fn complete_first_run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.first_run = false;
        self.save()
    }
}

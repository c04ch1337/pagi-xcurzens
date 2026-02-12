//! **Microsoft Graph (Focus Shield + Vitality Shield)** — Viva Insights integration.
//!
//! Fetches calendar view and working hours so Phoenix can:
//! - Add a "Schedule Outlook" to the morning briefing (e.g. "I see you have 6 hours of meetings today").
//! - Enter "Gatekeeper" mode when the user is in Focus Time or Quiet Hours (shorter, minimal responses).
//!
//! When `MS_GRAPH_HEALTH_ENABLED` is true (Vitality Shield), attempts to retrieve sleep/activity
//! from MS Graph Beta or from KB-08 (Soma). Sleep/activity can be written to KB-08 key `vitality/last_24h`
//! by an external sync or future Health API. Used to reduce emotional load and bias toward Virgo when sleep < 6h.
//!
//! OAuth tokens are stored in KB-04 (Chronos) and never logged.

use pagi_core::KnowledgeStore;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// KB-04 (Chronos) keys for MS Graph token. Tokens are never logged.
const KB4_SLOT: u8 = 4;
const MS_GRAPH_TOKEN_KEY: &str = "ms_graph/access_token";
const MS_GRAPH_EXPIRES_KEY: &str = "ms_graph/expires_at";

/// KB-08 (Soma) key for last-24h vitality (sleep + activity). Written by Vitality Shield or external sync.
const SOMA_SLOT: u8 = 8;
const VITALITY_LAST_24H_KEY: &str = "vitality/last_24h";

/// Vitality data for the last 24h: sleep and activity level. Used for Vitality Shield (reduce emotional load when sleep < 6h).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserVitality {
    /// Sleep duration in the last 24 hours (or last night). None if unknown.
    pub sleep_hours_last_24: Option<f32>,
    /// Activity level: "low" | "medium" | "high". Derived from MS Graph activityStatistics (focus/meeting) or manual.
    pub activity_level: Option<String>,
    /// When this snapshot was taken (ms since epoch).
    #[serde(default)]
    pub timestamp_ms: u64,
}

/// Result of fetching calendar + working hours. Used for Schedule Outlook and Gatekeeper.
#[derive(Debug, Clone, Default)]
pub struct CalendarHealth {
    /// Total minutes of busy/tentative/oof events today (approximate).
    pub meeting_minutes_today: u32,
    /// Working hours start (e.g. "08:00") if available; empty if not.
    pub working_hours_start: String,
    /// Working hours end (e.g. "17:00") if available.
    pub working_hours_end: String,
    /// True if current time falls inside an event that looks like Focus Time or Quiet Hours (subject contains "focus", "quiet", "do not disturb").
    pub is_in_focus_or_quiet_now: bool,
}

/// One-line schedule outlook for the morning briefing (e.g. "I see you have 6 hours of meetings today.").
pub fn schedule_outlook_sentence(health: Option<&CalendarHealth>) -> String {
    let Some(h) = health else { return String::new() };
    if h.meeting_minutes_today == 0 {
        return String::new();
    }
    let hours = h.meeting_minutes_today / 60;
    let mins = h.meeting_minutes_today % 60;
    if hours > 0 && mins > 0 {
        format!("I see you have {} hours and {} minutes of meetings today. ", hours, mins)
    } else if hours > 0 {
        format!("I see you have {} hour{} of meetings today. ", hours, if hours == 1 { "" } else { "s" })
    } else {
        "I see you have a short meeting block today. ".to_string()
    }
}

/// Returns true when Gatekeeper mode should be used (user in Focus/Quiet time).
pub fn use_gatekeeper_mode(health: Option<&CalendarHealth>) -> bool {
    health.map(|h| h.is_in_focus_or_quiet_now).unwrap_or(false)
}

/// Returns true when the user had low sleep in the last 24h (Vitality Shield: reduce emotional load, bias Virgo).
pub fn is_low_sleep(vitality: Option<&UserVitality>) -> bool {
    vitality
        .and_then(|v| v.sleep_hours_last_24)
        .map(|h| h < 6.0)
        .unwrap_or(false)
}

/// Fetch last-24h vitality (sleep, activity) from KB-08 (Soma). Data may be written by Vitality Shield (MS Graph Beta) or external sync.
/// Returns None if not configured or no recent record. Call after refresh_vitality_from_graph when MS_GRAPH_HEALTH_ENABLED.
pub fn fetch_user_vitality(store: &KnowledgeStore) -> Option<UserVitality> {
    let raw = store.get(SOMA_SLOT, VITALITY_LAST_24H_KEY).ok().flatten()?;
    let val: UserVitality = serde_json::from_slice(&raw).ok()?;
    let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
    if now_ms.saturating_sub(val.timestamp_ms) > 28 * 60 * 60 * 1000 {
        return None;
    }
    Some(val)
}

/// Write vitality to KB-08 so fetch_user_vitality and HealthReport can use it. Also writes to vitality/daily/YYYY-MM-DD for Rest vs. Output.
pub fn write_user_vitality(store: &KnowledgeStore, vitality: &UserVitality) -> Result<(), sled::Error> {
    let ts_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let v = UserVitality {
        timestamp_ms: if vitality.timestamp_ms > 0 { vitality.timestamp_ms } else { ts_ms },
        ..vitality.clone()
    };
    let json = serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string());
    store.insert(SOMA_SLOT, VITALITY_LAST_24H_KEY, json.as_bytes())?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let daily_key = format!("vitality/daily/{}", today);
    store.insert(SOMA_SLOT, &daily_key, json.as_bytes())?;
    Ok(())
}

/// Client for Microsoft Graph (OAuth2 client credentials + calendar/mailbox).
pub struct MicrosoftGraphClient {
    client: reqwest::blocking::Client,
    client_id: String,
    tenant_id: String,
    client_secret: String,
    /// User UPN or id for app-only calendar access (e.g. "user@contoso.com"). If empty, calendar fetch is skipped.
    user_principal_name: Option<String>,
}

impl MicrosoftGraphClient {
    pub fn new(
        client_id: String,
        tenant_id: String,
        client_secret: String,
        user_principal_name: Option<String>,
    ) -> Self {
        Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::blocking::Client::new()),
            client_id,
            tenant_id,
            client_secret,
            user_principal_name,
        }
    }

    /// Build from env: MS_GRAPH_CLIENT_ID, MS_GRAPH_TENANT_ID, MS_GRAPH_CLIENT_SECRET, MS_GRAPH_USER_UPN (optional).
    pub fn from_env() -> Option<Self> {
        let client_id = std::env::var("MS_GRAPH_CLIENT_ID").ok().filter(|s| !s.trim().is_empty())?;
        let tenant_id = std::env::var("MS_GRAPH_TENANT_ID").ok().filter(|s| !s.trim().is_empty())?;
        let client_secret = std::env::var("MS_GRAPH_CLIENT_SECRET").ok().filter(|s| !s.trim().is_empty())?;
        let user_principal_name = std::env::var("MS_GRAPH_USER_UPN").ok().filter(|s| !s.trim().is_empty());
        Some(Self::new(client_id, tenant_id, client_secret, user_principal_name))
    }

    /// Get access token from KB-04 if still valid (with 5 min buffer), else acquire via client credentials and store in KB-04. Never logs token.
    fn get_token(&self, store: &KnowledgeStore) -> Option<String> {
        let now_secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
        let expires_bytes = store.get(KB4_SLOT, MS_GRAPH_EXPIRES_KEY).ok().flatten()?;
        let expires_str = String::from_utf8(expires_bytes).ok()?;
        let expires_secs: u64 = expires_str.trim().parse().ok()?;
        if expires_secs > now_secs + 300 {
            let token_bytes = store.get(KB4_SLOT, MS_GRAPH_TOKEN_KEY).ok().flatten()?;
            String::from_utf8(token_bytes).ok()
        } else {
            self.acquire_and_store_token(store)
        }
    }

    fn acquire_and_store_token(&self, store: &KnowledgeStore) -> Option<String> {
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        );
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("scope", "https://graph.microsoft.com/.default"),
        ];
        let res = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .ok()?;
        if !res.status().is_success() {
            tracing::warn!(target: "pagi::ms_graph", status = %res.status(), "MS Graph token request failed (check client_id/tenant/secret)");
            return None;
        }
        let body: TokenResponse = res.json().ok()?;
        let expires_at = body.expires_in.map(|s| {
            let secs: u64 = s.parse().unwrap_or(3600);
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + secs
        }).unwrap_or_else(|| {
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600
        });
        let token = body.access_token?;
        let _ = store.insert(KB4_SLOT, MS_GRAPH_TOKEN_KEY, token.as_bytes());
        let _ = store.insert(KB4_SLOT, MS_GRAPH_EXPIRES_KEY, expires_at.to_string().as_bytes());
        tracing::info!(target: "pagi::ms_graph", "MS Graph token acquired and stored in KB-04");
        Some(token)
    }

    /// Fetch calendar view (today) and mailbox working hours. Returns None if client not configured or token/user missing.
    pub fn fetch_calendar_health(&self, store: &KnowledgeStore) -> Option<CalendarHealth> {
        let token = self.get_token(store)?;
        let upn = self.user_principal_name.as_deref()?;
        let now = chrono::Utc::now();
        let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
        let end = start + chrono::Duration::days(1);
        let start_param = start.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let end_param = end.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let url = format!(
            "https://graph.microsoft.com/v1.0/users/{}/calendarView?startDateTime={}&endDateTime={}",
            urlencoding::encode(upn),
            urlencoding::encode(&start_param),
            urlencoding::encode(&end_param)
        );
        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .ok()?;
        if !res.status().is_success() {
            tracing::debug!(target: "pagi::ms_graph", status = ?res.status(), "Calendar view request failed");
            return None;
        }
        let body: CalendarViewResponse = match res.json() {
            Ok(b) => b,
            Err(_) => return None,
        };
        let (meeting_minutes, is_in_focus_now) = parse_calendar_events(&body.value, now);
        let (work_start, work_end) = self.fetch_working_hours_inner(&token, upn)?;
        Some(CalendarHealth {
            meeting_minutes_today: meeting_minutes,
            working_hours_start: work_start,
            working_hours_end: work_end,
            is_in_focus_or_quiet_now: is_in_focus_now,
        })
    }

    fn fetch_working_hours_inner(&self, token: &str, user_upn: &str) -> Option<(String, String)> {
        let url = format!(
            "https://graph.microsoft.com/v1.0/users/{}/mailboxSettings",
            urlencoding::encode(user_upn)
        );
        let res = self
            .client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .ok()?;
        if !res.status().is_success() {
            return None;
        }
        let body: MailboxSettingsResponse = res.json().ok()?;
        let wh = body.working_hours?;
        Some((wh.start_time.unwrap_or_default(), wh.end_time.unwrap_or_default()))
    }

    /// Vitality Shield: when MS_GRAPH_HEALTH_ENABLED=true, try to fetch sleep/activity from Graph Beta and write to KB-08.
    /// Sleep: GET /beta/me/wellness/sleep may not be available in all tenants; on 404 we skip. Activity: from activityStatistics (focus/meeting).
    /// Call before fetch_user_vitality(store) so the next read gets fresh data.
    pub fn refresh_vitality_from_graph(&self, store: &KnowledgeStore) {
        if std::env::var("MS_GRAPH_HEALTH_ENABLED").map(|v| v.trim().eq_ignore_ascii_case("true")).unwrap_or(false) {
            let _ = self.try_fetch_wellness_and_store(store);
        }
    }

    fn try_fetch_wellness_and_store(&self, store: &KnowledgeStore) -> Option<()> {
        let token = self.get_token(store)?;
        let upn = self.user_principal_name.as_deref()?;
        let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis() as u64;
        let mut vitality = UserVitality { timestamp_ms: now_ms, ..UserVitality::default() };

        if let Some(sleep_hours) = self.try_fetch_sleep_beta(&token, upn) {
            vitality.sleep_hours_last_24 = Some(sleep_hours);
        }
        if let Some(level) = self.try_fetch_activity_level_beta(&token, upn) {
            vitality.activity_level = Some(level);
        }

        if vitality.sleep_hours_last_24.is_some() || vitality.activity_level.is_some() {
            let _ = write_user_vitality(store, &vitality);
            tracing::debug!(target: "pagi::ms_graph", "Vitality Shield: wrote vitality to KB-08");
        }
        Some(())
    }

    fn try_fetch_sleep_beta(&self, token: &str, upn: &str) -> Option<f32> {
        let url = format!(
            "https://graph.microsoft.com/beta/users/{}/wellness/sleep",
            urlencoding::encode(upn)
        );
        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .ok()?;
        if !res.status().is_success() {
            if res.status().as_u16() == 404 {
                tracing::debug!(target: "pagi::ms_graph", "Vitality: wellness/sleep endpoint not available (404)");
            }
            return None;
        }
        let body: serde_json::Value = res.json().ok()?;
        let duration_mins = body.get("totalSleepDurationMinutes").and_then(|v| v.as_u64())?;
        Some((duration_mins as f32) / 60.0)
    }

    fn try_fetch_activity_level_beta(&self, token: &str, upn: &str) -> Option<String> {
        let url = format!(
            "https://graph.microsoft.com/beta/users/{}/analytics/activityStatistics",
            urlencoding::encode(upn)
        );
        let res = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .ok()?;
        if !res.status().is_success() {
            return None;
        }
        let body: ActivityStatisticsResponse = res.json().ok()?;
        let total_focus_mins: u32 = body
            .value
            .iter()
            .filter(|s| s.activity.as_deref().map(|a| a.eq_ignore_ascii_case("focus")).unwrap_or(false))
            .filter_map(|s| parse_iso_duration_to_mins(s.duration.as_deref()?))
            .sum();
        let level = if total_focus_mins >= 240 { "high" } else if total_focus_mins >= 60 { "medium" } else { "low" };
        Some(level.to_string())
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    expires_in: Option<String>,
}

#[derive(Deserialize)]
struct CalendarViewResponse {
    value: Vec<GraphEvent>,
}

#[derive(Deserialize)]
struct GraphEvent {
    subject: Option<String>,
    start: Option<DateTimeTimeZone>,
    end: Option<DateTimeTimeZone>,
    #[serde(rename = "showAs")]
    show_as: Option<String>,
}

#[derive(Deserialize)]
struct DateTimeTimeZone {
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
    #[serde(rename = "timeZone")]
    time_zone: Option<String>,
}

#[derive(Deserialize)]
struct MailboxSettingsResponse {
    working_hours: Option<WorkingHours>,
}

#[derive(Deserialize)]
struct ActivityStatisticsResponse {
    value: Vec<ActivityStatisticsItem>,
}

#[derive(Deserialize)]
struct ActivityStatisticsItem {
    activity: Option<String>,
    duration: Option<String>,
}

fn parse_iso_duration_to_mins(duration: &str) -> Option<u32> {
    let s = duration.trim().strip_prefix("PT")?;
    let mut hours = 0u32;
    let mut mins = 0u32;
    let mut secs = 0u32;
    let mut num = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            num.push(c);
        } else if c == 'H' {
            hours = num.parse().unwrap_or(0);
            num.clear();
        } else if c == 'M' {
            mins = num.parse().unwrap_or(0);
            num.clear();
        } else if c == 'S' {
            secs = num.parse().unwrap_or(0);
            num.clear();
        }
    }
    Some(hours * 60 + mins + secs / 60)
}

#[derive(Deserialize)]
struct WorkingHours {
    #[serde(rename = "startTime")]
    start_time: Option<String>,
    #[serde(rename = "endTime")]
    end_time: Option<String>,
}

fn parse_calendar_events(events: &[GraphEvent], now: chrono::DateTime<chrono::Utc>) -> (u32, bool) {
    let mut total_minutes = 0u32;
    let mut is_in_focus_now = false;
    let focus_keywords = ["focus", "quiet", "do not disturb", "dnd", "concentration"];
    for ev in events {
        let show_as = ev.show_as.as_deref().unwrap_or("").to_lowercase();
        if !matches!(show_as.as_str(), "busy" | "tentative" | "oof") {
            continue;
        }
        let (start_utc, end_utc) = match (parse_iso_opt(&ev.start), parse_iso_opt(&ev.end)) {
            (Some(s), Some(e)) => (s, e),
            _ => continue,
        };
        let duration_mins = (end_utc - start_utc).num_minutes().max(0) as u32;
        total_minutes = total_minutes.saturating_add(duration_mins);
        let subject = ev.subject.as_deref().unwrap_or("").to_lowercase();
        if focus_keywords.iter().any(|kw| subject.contains(kw)) {
            if now >= start_utc && now <= end_utc {
                is_in_focus_now = true;
            }
        }
    }
    (total_minutes, is_in_focus_now)
}

fn parse_iso_opt(dtz: &Option<DateTimeTimeZone>) -> Option<chrono::DateTime<chrono::Utc>> {
    let dt = dtz.as_ref()?.date_time.as_ref()?;
    chrono::DateTime::parse_from_rfc3339(dt).ok().map(|t| t.with_timezone(&chrono::Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_outlook_empty_when_no_health() {
        assert!(schedule_outlook_sentence(None).is_empty());
    }

    #[test]
    fn schedule_outlook_zero_meetings() {
        let h = CalendarHealth { meeting_minutes_today: 0, ..Default::default() };
        assert!(schedule_outlook_sentence(Some(&h)).is_empty());
    }

    #[test]
    fn schedule_outlook_formats_hours() {
        let h = CalendarHealth { meeting_minutes_today: 360, ..Default::default() };
        let s = schedule_outlook_sentence(Some(&h));
        assert!(s.contains("6 hour"));
        assert!(s.contains("meetings today"));
    }

    #[test]
    fn use_gatekeeper_false_when_no_health() {
        assert!(!use_gatekeeper_mode(None));
    }

    #[test]
    fn use_gatekeeper_follows_flag() {
        let h = CalendarHealth { is_in_focus_or_quiet_now: true, ..Default::default() };
        assert!(use_gatekeeper_mode(Some(&h)));
    }
}

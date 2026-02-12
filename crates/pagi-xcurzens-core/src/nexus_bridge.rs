//! NEXUS Bridge — Connects Traveler UI to OpenRouter using Sled-backed Knowledge Bases.
//! Scout persona: coast-focused, Jamey-approved. No Docker; reqwest only.

use crate::relations::KB07Relations;
use serde::Deserialize;
use std::path::Path;

const OPENROUTER_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const SCOUT_SYSTEM_PROMPT: &str = "You are the Scout for PAGI XCURZENS: professional, coastal, and high-bandwidth. \
You speak with clarity and authority. Your answers are concise and actionable. \
You represent the Root Sovereign (Jamey) and the coastal system. Do not use markdown or code blocks unless the user asks. \
When the traveler's City and Weather are provided, acknowledge them naturally in your reply (e.g. 'Since it's a bit choppy out there today, I'd recommend a Beach Box over a deep-sea charter' or 'Given the clear skies in [City], a sunset cruise is ideal'). \
Weave location and conditions into recommendations so the traveler feels you are a local expert.";

/// Geo-context for the Scout (city, weather).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GeoContext {
    pub city: Option<String>,
    pub weather: Option<String>,
}

/// Intent level for lead grading. High = booking/pricing/availability signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    High,
    Normal,
}

/// High-intent keywords: booking details, prices, availability, purchase intent.
const HIGH_INTENT_PHRASES: &[&str] = &[
    "price", "prices", "pricing", "cost", "how much",
    "booking", "book", "reserve", "reservation",
    "availability", "available", "schedule",
    "charter", "charters", "boat rental", "rent a boat",
    "beach box", "beach box location", "beach box locations",
    "buy", "purchase", "sign up", "get a quote", "quote",
];

/// Detects if the Scout reply indicates High Intent (booking, prices, availability).
pub fn intent_level(reply: &str) -> Intent {
    let lower = reply.to_lowercase();
    let hit = HIGH_INTENT_PHRASES
        .iter()
        .any(|p| lower.contains(p));
    if hit {
        Intent::High
    } else {
        Intent::Normal
    }
}

/// OpenRouter chat message.
#[derive(serde::Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// OpenRouter request body.
#[derive(serde::Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

/// OpenRouter non-streamed response (we stream via SSE from gateway after brand_filter).
#[derive(Debug, serde::Deserialize)]
struct OpenRouterResponse {
    choices: Option<Vec<OpenRouterChoice>>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenRouterChoice {
    message: Option<OpenRouterMessage>,
}

#[derive(Debug, serde::Deserialize)]
struct OpenRouterMessage {
    content: Option<String>,
}

/// Scout interaction: pull KB-07 lead history, build system prompt, call OpenRouter.
/// Returns the Scout's reply for the gateway to pipe through brand_filter and send as SSE.
pub async fn stream_scout_interaction(
    query: &str,
    geo: &GeoContext,
    kb07_path: Option<impl AsRef<Path>>,
    api_key: &str,
) -> Result<String, NexusBridgeError> {
    tracing::info!(
        "[SYSTEM] NEXUS Bridge: External Bandwidth requested for Jamey."
    );

    let relations = KB07Relations::open(kb07_path).map_err(NexusBridgeError::Sled)?;
    let recent = relations
        .recent_lead_history(20)
        .map_err(NexusBridgeError::Sled)?;

    let lead_context = if recent.is_empty() {
        "No recent leads in the ledger.".to_string()
    } else {
        let lines: Vec<String> = recent
            .into_iter()
            .filter_map(|(k, v)| String::from_utf8(v).ok().map(|v_str| format!("Lead {}: {}", k, v_str)))
            .collect();
        format!("Recent Lead History (for context only):\n{}", lines.join("\n"))
    };

    let geo_block = match (geo.city.as_deref(), geo.weather.as_deref()) {
        (Some(c), Some(w)) => format!(
            "Traveler context: City={}, Weather={}. Acknowledge the weather and location in your reply when giving recommendations (e.g. choppy weather -> suggest safer options; sunny -> suggest outdoor/water activities).",
            c, w
        ),
        (Some(c), None) => format!("Traveler context: City={}. Acknowledge the location when relevant.", c),
        (None, Some(w)) => format!("Traveler context: Weather={}. Acknowledge conditions when giving recommendations.", w),
        (None, None) => "No geo context provided.".to_string(),
    };

    let system_content = format!(
        "{}\n\n{}\n\n{}",
        SCOUT_SYSTEM_PROMPT, lead_context, geo_block
    );

    let client = reqwest::Client::new();
    let body = OpenRouterRequest {
        model: "openai/gpt-3.5-turbo".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_content,
            },
            ChatMessage {
                role: "user".to_string(),
                content: query.to_string(),
            },
        ],
        stream: Some(false),
    };

    let res = client
        .post(OPENROUTER_URL)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(NexusBridgeError::Reqwest)?;

    let status = res.status();
    let text = res.text().await.map_err(NexusBridgeError::Reqwest)?;

    if !status.is_success() {
        return Err(NexusBridgeError::OpenRouter(status.as_u16(), text));
    }

    let parsed: OpenRouterResponse =
        serde_json::from_str(&text).map_err(|e| NexusBridgeError::Json(e.to_string()))?;

    let content = parsed
        .choices
        .and_then(|c| c.into_iter().next())
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .unwrap_or_else(|| "No reply from Scout.".to_string());

    Ok(content)
}

#[derive(Debug)]
pub enum NexusBridgeError {
    Sled(sled::Error),
    Reqwest(reqwest::Error),
    Json(String),
    OpenRouter(u16, String),
}

impl std::fmt::Display for NexusBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NexusBridgeError::Sled(e) => write!(f, "KB-07: {}", e),
            NexusBridgeError::Reqwest(e) => write!(f, "OpenRouter request: {}", e),
            NexusBridgeError::Json(e) => write!(f, "OpenRouter response parse: {}", e),
            NexusBridgeError::OpenRouter(code, body) => write!(f, "OpenRouter {}: {}", code, body),
        }
    }
}

impl std::error::Error for NexusBridgeError {}

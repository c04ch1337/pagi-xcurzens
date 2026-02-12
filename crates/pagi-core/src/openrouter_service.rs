//! OpenRouter Sovereign Bridge: high-level reasoning via OpenRouter only.
//!
//! **Grounding:** The Bridge does not send raw user questions to OpenRouter. Callers must:
//! 1. Query the 8 local Knowledge Bases (LanceDB/Sled) first for data retrieval.
//! 2. Attach that local context to the prompt.
//! 3. Use this service for *thinking* and *planning*; all execution stays in Rust.
//!
//! API key: `OPENROUTER_API_KEY` in `.env`. Default model: `meta-llama/llama-3.3-70b-instruct`.

use serde::{Deserialize, Serialize};
use std::time::Duration;

const OPENROUTER_API_BASE: &str = "https://openrouter.ai/api/v1";
const DEFAULT_MODEL: &str = "meta-llama/llama-3.3-70b-instruct";

/// High-level plan returned by the Bridge (summary + steps for the orchestrator to map to local skills).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgePlan {
    /// Short summary of the intent.
    pub summary: String,
    /// Ordered steps (skill names or natural-language steps for the orchestrator to map).
    pub steps: Vec<String>,
}

// OpenAI-compatible request/response for OpenRouter
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: String,
}

/// OpenRouter Sovereign Bridge: high-level planning only. Actions and memory stay local.
pub struct OpenRouterBridge {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenRouterBridge {
    /// Create a bridge using API key from UserConfig, falling back to environment.
    /// Priority: user_config.toml > OPENROUTER_API_KEY env var
    /// Returns `None` if no key is found.
    pub fn from_env() -> Option<Self> {
        use crate::config::UserConfig;
        
        // Try to load from user config first (beta distribution)
        let api_key = if let Ok(user_config) = UserConfig::load() {
            user_config.get_api_key()
        } else {
            None
        };
        
        // Fallback to environment variable
        let api_key = api_key.or_else(|| std::env::var("OPENROUTER_API_KEY").ok());
        
        let key = api_key?.trim().to_string();
        if key.is_empty() {
            return None;
        }
        Some(Self::new(key))
    }

    /// Create a bridge with an explicit API key.
    pub fn new(api_key: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            api_key: api_key.trim().to_string(),
            model: DEFAULT_MODEL.to_string(),
            client,
        }
    }

    /// Set the model (e.g. `meta-llama/llama-3.3-70b-instruct`, `anthropic/claude-3.5-sonnet`).
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Request a high-level plan for the given intent. **Callers should query local KBs first**
    /// and pass the result as `context` so the Bridge reasons over grounded data and saves tokens.
    pub async fn plan(
        &self,
        intent: &str,
        context: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let system = "You are a high-level planner for an AGI system. \
            Output only a concise plan: a short summary and an ordered list of steps (one per line or as a JSON array). \
            Do not execute any actions; all actions and memory retrieval are performed by the local system. \
            Be brief and structured.";

        let mut user_text = format!("Intent: {}", intent);
        if let Some(ctx) = context {
            user_text.push_str("\n\nLocal context (from Knowledge Bases):\n");
            user_text.push_str(ctx);
        }

        let url = format!("{}/chat/completions", OPENROUTER_API_BASE);
        let body = ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_text,
                },
            ],
            temperature: Some(0.3),
            max_tokens: Some(1024),
        };

        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://pagi-sovereign.local")
            .header("X-Title", "PAGI-OpenRouter-Bridge")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("OpenRouter Bridge request failed: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(format!("OpenRouter API error {}: {}", status, body).into());
        }

        let parsed: ChatResponse = res
            .json()
            .await
            .map_err(|e| format!("OpenRouter response parse failed: {}", e))?;

        let text = parsed
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_else(|| "[No response from OpenRouter]".to_string());

        Ok(text)
    }

    /// Convenience: request a plan and try to parse as BridgePlan.
    pub async fn plan_structured(
        &self,
        intent: &str,
        context: Option<&str>,
    ) -> Result<BridgePlan, Box<dyn std::error::Error + Send + Sync>> {
        let raw = self.plan(intent, context).await?;
        match serde_json::from_str::<BridgePlan>(&raw) {
            Ok(p) => Ok(p),
            Err(_) => Ok(BridgePlan {
                summary: raw,
                steps: Vec::new(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Rig OpenRouter provider (optional; enable with --features rig)
// ---------------------------------------------------------------------------

#[cfg(feature = "rig")]
/// OpenRouter completion when `pagi-core` is built with `--features rig`.
/// Delegates to OpenRouterBridge; replace with `rig::providers::openrouter::Client` for full rig integration.
pub struct RigOpenRouter {
    api_key: String,
    model: String,
}

#[cfg(feature = "rig")]
impl RigOpenRouter {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY").ok()?;
        let key = api_key.trim().to_string();
        if key.is_empty() {
            return None;
        }
        Some(Self {
            api_key: key,
            model: DEFAULT_MODEL.to_string(),
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            api_key: api_key.trim().to_string(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    /// Completion via OpenRouter (OpenRouterBridge under the hood).
    pub async fn complete(
        &self,
        system: &str,
        user: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        OpenRouterBridge::new(self.api_key.clone())
            .with_model(self.model.as_str())
            .plan(user, Some(system))
            .await
    }
}

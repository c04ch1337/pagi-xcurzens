//! **Slot 4 — The External Gateway:** Web search and URL fetch for the Orchestrator.
//!
//! Gives PAGI "Internet Sight": when the user asks for current events or information
//! outside local Knowledge Bases, the Orchestrator can invoke `web_search` to fetch
//! search results or scrape a specific URL.
//!
//! - **Tavily** (preferred): set `TAVILY_API_KEY` in `.env` for AI-optimized search.
//! - **SerpAPI**: set `SERPAPI_KEY` in `.env` for Google SERP results.
//! - **Fallback**: no key → fetch a single `url` from payload (basic HTML fetcher).

use pagi_core::{AgentSkill, EventRecord, KnowledgeStore, TenantContext};
use serde::Deserialize;
use std::sync::Arc;

const SKILL_NAME: &str = "web_search";

/// Max results to return when using search APIs (capped for token safety).
const DEFAULT_MAX_RESULTS: u32 = 5;
const MAX_RESULTS_CAP: u32 = 20;

/// Payload schema for the Orchestrator.
/// - `query`: search term (used when TAVILY_API_KEY or SERPAPI_KEY is set).
/// - `max_results`: number of links to retrieve (default 5, max 20).
/// - `url`: optional single URL to fetch (works without any API key).
#[derive(Debug, Clone, Default, Deserialize)]
struct WebSearchArgs {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    max_results: Option<u32>,
    #[serde(default)]
    url: Option<String>,
}

/// Web search / URL fetch skill. Uses Tavily or SerpAPI when keys are set; otherwise
/// fetches the given `url` with reqwest (basic HTML fetcher).
pub struct WebSearch {
    store: Option<Arc<KnowledgeStore>>,
}

impl WebSearch {
    pub fn new() -> Self {
        Self { store: None }
    }

    /// With store: search events are logged to Chronos so the agent can discuss results.
    pub fn new_with_store(store: Arc<KnowledgeStore>) -> Self {
        Self {
            store: Some(store),
        }
    }
}

impl Default for WebSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AgentSkill for WebSearch {
    fn name(&self) -> &str {
        SKILL_NAME
    }

    async fn execute(
        &self,
        ctx: &TenantContext,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let args: WebSearchArgs = payload
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        // Single-URL fetch (no API key required)
        if let Some(ref url) = args.url {
            let url = url.trim();
            if !url.is_empty() {
                return fetch_url(url).await.map(|out| {
                    serde_json::json!({
                        "status": "ok",
                        "skill": SKILL_NAME,
                        "mode": "fetch_url",
                        "url": url,
                        "content_preview": out.content_preview,
                        "content_length": out.content_length,
                        "title": out.title,
                    })
                });
            }
        }

        // Search path: need query and at least one API key
        let query = args
            .query
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or("web_search requires either 'url' (to fetch one page) or 'query' (requires TAVILY_API_KEY or SERPAPI_KEY)")?;

        let max_results = args
            .max_results
            .unwrap_or(DEFAULT_MAX_RESULTS)
            .min(MAX_RESULTS_CAP);

        let result = if let Ok(key) = std::env::var("TAVILY_API_KEY") {
            let key = key.trim();
            if key.is_empty() {
                search_fallback(query, max_results).await
            } else {
                search_tavily(key, query, max_results).await
            }
        } else if let Ok(key) = std::env::var("SERPAPI_KEY") {
            let key = key.trim();
            if key.is_empty() {
                search_fallback(query, max_results).await
            } else {
                search_serpapi(key, query, max_results).await
            }
        } else {
            search_fallback(query, max_results).await
        };

        let (status, results_json, summary) = match result {
            Ok((results, summary)) => (
                "ok",
                serde_json::to_value(&results).unwrap_or(serde_json::json!([])),
                summary,
            ),
            Err(e) => {
                let err_msg = e.to_string();
                tracing::warn!(target: "pagi::web_search", error = %err_msg, "Web search failed");
                return Ok(serde_json::json!({
                    "status": "error",
                    "skill": SKILL_NAME,
                    "error": err_msg,
                    "query": query,
                    "hint": "Set TAVILY_API_KEY or SERPAPI_KEY in .env for search; or use payload.url to fetch a single page."
                }));
            }
        };

        // Chronos: log so the agent can discuss search results
        if let Some(ref store) = self.store {
            let agent_id = ctx.agent_id.as_deref().unwrap_or("default");
            let event = EventRecord::now(
                "Chronos",
                format!("Web search: \"{}\". {}", query, summary),
            )
            .with_skill(SKILL_NAME)
            .with_outcome(status);
            let _ = store.append_chronos_event(agent_id, &event);
        }

        Ok(serde_json::json!({
            "status": status,
            "skill": SKILL_NAME,
            "query": query,
            "max_results": max_results,
            "summary": summary,
            "results": results_json,
        }))
    }
}

/// One search result for the LLM (title, url, snippet).
#[derive(Debug, Clone, serde::Serialize)]
struct SearchResult {
    title: String,
    url: String,
    content: String,
}

/// Result of fetching a single URL (plain-text preview).
struct FetchedPage {
    title: Option<String>,
    content_preview: String,
    content_length: usize,
}

async fn fetch_url(url: &str) -> Result<FetchedPage, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("PAGI-Gateway/1.0 (Bare-Metal Rust)")
        .build()?;
    let body = client.get(url).send().await?.bytes().await?;
    let html = String::from_utf8_lossy(&body);
    let (title, text) = extract_title_and_text(&html);
    let preview_len = 12_000.min(text.len());
    let mut content_preview: String = text.chars().take(preview_len).collect();
    if preview_len < text.len() {
        content_preview.push_str("…");
    }
    Ok(FetchedPage {
        title,
        content_preview,
        content_length: text.len(),
    })
}

fn extract_title_and_text(html: &str) -> (Option<String>, String) {
    let doc = scraper::Html::parse_document(html);
    let title_sel = scraper::Selector::parse("title").unwrap_or_else(|_| unreachable!());
    let title = doc
        .select(&title_sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty());
    let body_sel = scraper::Selector::parse("body").unwrap_or_else(|_| unreachable!());
    let body = doc.select(&body_sel).next();
    let text = body
        .map(|el| el.text().collect::<String>())
        .unwrap_or_else(|| html.to_string());
    let text = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (title, text)
}

async fn search_tavily(
    api_key: &str,
    query: &str,
    max_results: u32,
) -> Result<(Vec<SearchResult>, String), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let body = serde_json::json!({
        "query": query,
        "max_results": max_results,
        "search_depth": "basic",
        "include_answer": false,
    });
    let res = client
        .post("https://api.tavily.com/search")
        .header("Content-Type", "application/json")
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()
        .await?;
    let status = res.status();
    let json: serde_json::Value = res.json().await?;
    if !status.is_success() {
        let err = json
            .get("detail")
            .or_else(|| json.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("Tavily API error");
        return Err(err.to_string().into());
    }
    let empty: Vec<serde_json::Value> = vec![];
    let arr = json.get("results").and_then(|r| r.as_array()).unwrap_or(&empty);
    let results: Vec<SearchResult> = arr
        .iter()
        .filter_map(|r| {
            Some(SearchResult {
                title: r.get("title")?.as_str()?.to_string(),
                url: r.get("url")?.as_str()?.to_string(),
                content: r.get("content")?.as_str().unwrap_or("").to_string(),
            })
        })
        .collect();
    let answer = json.get("answer").and_then(|v| v.as_str()).unwrap_or("");
    let summary = if answer.is_empty() {
        format!("{} result(s).", results.len())
    } else {
        format!("{} result(s). Answer preview: {}…", results.len(), answer.chars().take(120).collect::<String>())
    };
    Ok((results, summary))
}

async fn search_serpapi(
    api_key: &str,
    query: &str,
    max_results: u32,
) -> Result<(Vec<SearchResult>, String), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    let url = reqwest::Url::parse_with_params(
        "https://serpapi.com/search",
        &[
            ("engine", "google"),
            ("q", query),
            ("api_key", api_key.trim()),
            ("num", max_results.to_string().as_str()),
        ],
    )?;
    let res = client.get(url).send().await?;
    let json: serde_json::Value = res.json().await?;
    let empty: Vec<serde_json::Value> = vec![];
    let arr = json
        .get("organic_results")
        .and_then(|r| r.as_array())
        .unwrap_or(&empty);
    let results: Vec<SearchResult> = arr
        .iter()
        .take(max_results as usize)
        .filter_map(|r| {
            Some(SearchResult {
                title: r.get("title")?.as_str()?.to_string(),
                url: r.get("link")?.as_str()?.to_string(),
                content: r.get("snippet")?.as_str().unwrap_or("").to_string(),
            })
        })
        .collect();
    let summary = format!("{} result(s).", results.len());
    Ok((results, summary))
}

/// Fallback when no API key: we cannot run a real search; return a clear error
/// so the orchestrator can tell the user to set TAVILY_API_KEY or SERPAPI_KEY,
/// or to use the `url` field to fetch a single page.
async fn search_fallback(
    _query: &str,
    _max_results: u32,
) -> Result<(Vec<SearchResult>, String), Box<dyn std::error::Error + Send + Sync>> {
    Err("No search API key. Set TAVILY_API_KEY or SERPAPI_KEY in .env for web search, or use payload.url to fetch a single URL.".into())
}

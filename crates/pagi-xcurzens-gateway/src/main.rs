//! PAGI XCURZENS Gateway — Single source of truth at 127.0.0.1:8000
//! Bare Metal, no Docker. NEXUS Bridge (Scout + OpenRouter), KB-07 leads, SSE.

use axum::{
    body::Body,
    extract::{ConnectInfo, Form, State},
    http::Request,
    middleware::Next,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::iter;
use pagi_xcurzens_core::{
    brand_filter,
    intent_level,
    nexus_bridge::{stream_scout_interaction, GeoContext},
    Intent, KB07Relations, LeadDispatcher, ROOT_SOVEREIGN,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
struct AppState {
    kb07_path: String,
    openrouter_api_key: String,
}

#[derive(Deserialize)]
struct ScoutRequest {
    query: String,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    weather: Option<String>,
}

#[derive(Deserialize)]
struct PartnerRegisterForm {
    business_name: String,
    primary_city: String,
    service_type: String,
    webhook_url: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let kb07_path = std::env::var("KB07_PATH").unwrap_or_else(|_| "./data/kbs/kb07_relations".into());
    let openrouter_api_key =
        std::env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| String::new());

    let state = Arc::new(AppState {
        kb07_path: kb07_path.clone(),
        openrouter_api_key: openrouter_api_key.clone(),
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/", get(serve_traveler_ui))
        .route("/api/v1/scout", post(scout_handler))
        .route("/infrastructure/leads", get(leads_handler))
        .route("/nexus/onboard", get(serve_onboard_ui))
        .route("/nexus/register", post(register_partner_handler))
        .route("/command", get(serve_command_dashboard))
        .route("/command/feed", get(command_feed_handler))
        .with_state(state)
        .layer(axum::middleware::from_fn(log_traveler_traffic));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await.unwrap();

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn log_traveler_traffic(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    tracing::info!("[XCURZENS SYSTEM] Bandwidth allocated for: {}", addr);
    tracing::info!(
        "[XCURZENS AUTH] Root Sovereign: {} — traveler request authorized",
        ROOT_SOVEREIGN
    );
    next.run(request).await
}

async fn health() -> &'static str {
    "OK"
}

/// Traveler UI: embedded Command Bar + Scout Console (Navy / Orange).
async fn serve_traveler_ui() -> Html<&'static str> {
    const INDEX: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/index.html"));
    Html(INDEX)
}

/// Partner Onboarding Terminal: Navy background, Orange accents.
async fn serve_onboard_ui() -> Html<&'static str> {
    const ONBOARD: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/onboard.html"));
    Html(ONBOARD)
}

/// POST /nexus/register: validate, write to KB-07, connection test, return success/error HTML.
async fn register_partner_handler(
    State(state): State<Arc<AppState>>,
    Form(form): Form<PartnerRegisterForm>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let relations = KB07Relations::open(Some(state.kb07_path.as_str()))
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let partner_id = relations
        .register_partner(
            form.business_name.trim(),
            form.primary_city.trim(),
            form.service_type.trim(),
            form.webhook_url.trim(),
        )
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(
        "[SYSTEM] New Partner Infrastructure Registered: {} in {}.",
        form.business_name.trim(),
        form.primary_city.trim()
    );

    let test_ok = reqwest::Client::new()
        .post(form.webhook_url.trim())
        .json(&serde_json::json!({
            "test": true,
            "message": "XCURZENS connection test",
            "partner_id": partner_id,
        }))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    let (class, msg) = if test_ok {
        (
            "p-4 rounded text-navy font-medium",
            "Infrastructure Synchronized: You are now active in the XCURZENS network. Your bandwidth is live.",
        )
    } else {
        (
            "p-4 rounded text-amber-200 font-medium",
            "Bandwidth Error: Please verify your Webhook URL. Your registration was saved; fix the endpoint to receive leads.",
        )
    };

    let html = format!(
        r#"<div class="{}" style="background-color: #FA921C;">{}</div>"#,
        class, msg
    );
    Ok(Html(html))
}

/// Scout: NEXUS Bridge + brand_filter, returned as SSE. High Intent -> dispatch lead + partner notification.
async fn scout_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ScoutRequest>,
) -> Result<Response, (axum::http::StatusCode, String)> {
    let geo = GeoContext {
        city: body.city.clone(),
        weather: body.weather.clone(),
    };

    let raw = stream_scout_interaction(
        &body.query,
        &geo,
        Some(state.kb07_path.as_str()),
        &state.openrouter_api_key,
    )
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if intent_level(&raw) == Intent::High {
        let lead_id = format!(
            "lead_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        let snippet = raw.chars().take(500).collect::<String>();
        let lead_payload = serde_json::json!({
            "query": body.query,
            "reply_snippet": snippet,
            "city": body.city,
            "weather": body.weather,
            "intent": "high",
        });
        if let Ok(dispatcher) = LeadDispatcher::new(Some(state.kb07_path.as_str())) {
            let _ = dispatcher.dispatch_lead_str(&lead_id, &lead_payload.to_string());
            let webhook = std::env::var("PARTNER_WEBHOOK_URL").ok();
            let _ = dispatcher
                .trigger_partner_notification(
                    &lead_id,
                    &snippet,
                    body.city.as_deref(),
                    body.weather.as_deref(),
                    webhook.as_deref(),
                )
                .await;
        }
    }

    let filtered = brand_filter(&raw);

    let event = axum::response::sse::Event::default().data(filtered);
    let stream = iter(vec![Ok::<_, std::convert::Infallible>(event)]);

    Ok(axum::response::sse::Sse::new(stream).into_response())
}

/// Command Center: raw JSON feed of KB-07 leads for Jamey. High-intent leads include highlight #FA921C (Orange).
async fn leads_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let relations = KB07Relations::open(Some(state.kb07_path.as_str()))
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let recent = relations
        .recent_lead_history(100)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let payload: Vec<serde_json::Value> = recent
        .into_iter()
        .map(|(k, v)| {
            let value_str = String::from_utf8(v).unwrap_or_else(|_| "<binary>".to_string());
            let high_intent = serde_json::from_str::<serde_json::Value>(&value_str)
                .ok()
                .and_then(|j| j.get("intent").and_then(|v| v.as_str()).map(|s| s == "high"))
                .unwrap_or(false);
            serde_json::json!({
                "id": k,
                "payload": value_str,
                "high_intent": high_intent,
                "highlight": if high_intent { serde_json::Value::String("#FA921C".into()) } else { serde_json::Value::Null }
            })
        })
        .collect();

    let high_intent_count = payload
        .iter()
        .filter(|p| p.get("high_intent").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    let active_partner_count = relations
        .get_partners()
        .map(|p| p.len())
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "sovereign": ROOT_SOVEREIGN,
        "source": "KB-07 Relations",
        "system_summary": {
            "total_leads": payload.len(),
            "high_intent_count": high_intent_count,
            "active_partner_count": active_partner_count,
        },
        "leads": payload
    })))
}

/// Lead Ledger Dashboard: God-View for Jamey (Navy/Orange).
async fn serve_command_dashboard() -> Html<&'static str> {
    const DASH: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/command.html"));
    Html(DASH)
}

/// HTMX feed: summary + leads table body. Polled every 60s by the dashboard.
async fn command_feed_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let relations = KB07Relations::open(Some(state.kb07_path.as_str()))
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let recent = relations
        .recent_lead_history(100)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut rows: Vec<(String, String, bool)> = Vec::new();
    for (k, v) in recent {
        let value_str = String::from_utf8(v).unwrap_or_else(|_| "<binary>".to_string());
        let high_intent = serde_json::from_str::<serde_json::Value>(&value_str)
            .ok()
            .and_then(|j| j.get("intent").and_then(|v| v.as_str()).map(|s| s == "high"))
            .unwrap_or(false);
        let snippet = value_str.chars().take(120).collect::<String>();
        rows.push((k, snippet, high_intent));
    }

    let high_intent_count = rows.iter().filter(|(_, _, hi)| *hi).count();
    let active_partner_count = relations.get_partners().map(|p| p.len()).unwrap_or(0);
    let total_leads = rows.len();

    let summary = format!(
        r#"<div class="grid grid-cols-3 gap-4 mb-6">
  <div class="p-4 rounded text-center" style="background-color: #051C55; color: #e2e8f0;"><span class="block text-2xl font-bold" style="color: #FA921C;">{}</span>Total Leads</div>
  <div class="p-4 rounded text-center" style="background-color: #051C55; color: #e2e8f0;"><span class="block text-2xl font-bold" style="color: #FA921C;">{}</span>High-Intent</div>
  <div class="p-4 rounded text-center" style="background-color: #051C55; color: #e2e8f0;"><span class="block text-2xl font-bold" style="color: #FA921C;">{}</span>Active Partners</div>
</div>"#,
        total_leads, high_intent_count, active_partner_count
    );

    let mut table_rows = String::new();
    for (id, snippet, hi) in rows {
        let tr_style = if hi {
            r#" style="background-color: rgba(250,146,28,0.2);""#
        } else {
            ""
        };
        let badge = if hi {
            r#"<span class="text-xs font-semibold px-2 py-0.5 rounded" style="background-color: #FA921C; color: #051C55;">High-Intent</span>"#
        } else {
            r#"<span class="text-slate-500">—</span>"#
        };
        let snippet_esc = html_escape(&snippet);
        table_rows.push_str(&format!(
            r#"<tr{}><td class="px-3 py-2 font-mono text-sm">{}</td><td class="px-3 py-2 text-sm">{}</td><td class="px-3 py-2">{}</td></tr>"#,
            tr_style, html_escape(&id), snippet_esc, badge
        ));
    }

    let html = format!(
        r#"{}<table class="w-full border-collapse rounded-lg overflow-hidden" style="background-color: #051C55; color: #e2e8f0;"><thead><tr style="border-bottom: 1px solid #475569;"><th class="px-3 py-2 text-left" style="color: #FA921C;">ID</th><th class="px-3 py-2 text-left" style="color: #FA921C;">Payload</th><th class="px-3 py-2 text-left" style="color: #FA921C;">Intent</th></tr></thead><tbody>{}</tbody></table>"#,
        summary, table_rows
    );
    Ok(Html(html))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

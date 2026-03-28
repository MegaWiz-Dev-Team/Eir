//! Chat module — Embedded chat interface for commanding Bifrost/Fenrir agents.
//!
//! Provides:
//! - GET /chat — Serves the chat HTML page
//! - POST /v1/chat — Proxies messages to Bifrost agent runtime (SSE streaming)

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::config::Config;

/// Build the chat router.
pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/chat", get(chat_page))
        .route("/eir-chat-widget.js", get(chat_widget_js))
        .route("/v1/chat", post(chat_proxy))
        .route("/v1/chat/status", get(chat_status))
}

// === DTOs ===

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// User message
    pub message: String,
    /// Optional agent ID (default: "default")
    pub agent: Option<String>,
    /// Optional session/conversation ID for context
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatStatus {
    pub eir_gateway: String,
    pub bifrost_url: String,
    pub bifrost_reachable: bool,
}

// === Handlers ===

/// GET /chat — Serve the embedded chat page.
async fn chat_page() -> Html<&'static str> {
    Html(include_str!("../static/chat.html"))
}

/// GET /eir-chat-widget.js — Serve the chat widget script.
async fn chat_widget_js() -> Response {
    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "application/javascript")
        .header(header::CACHE_CONTROL, "public, max-age=3600")
        .body(Body::from(include_str!("../static/chat-widget.js")))
        .unwrap()
}

/// Script tag to inject into proxied HTML pages.
const WIDGET_SCRIPT: &str = r#"<script src="/eir-chat-widget.js" defer></script>"#;

/// Inject the chat widget script tag before </body> in an HTML response body.
pub fn inject_widget(html: &str) -> String {
    if html.contains("eir-chat-widget") {
        return html.to_string();
    }
    if let Some(pos) = html.rfind("</body>") {
        let mut result = String::with_capacity(html.len() + WIDGET_SCRIPT.len() + 1);
        result.push_str(&html[..pos]);
        result.push_str(WIDGET_SCRIPT);
        result.push('\n');
        result.push_str(&html[pos..]);
        result
    } else {
        format!("{}{}", html, WIDGET_SCRIPT)
    }
}

/// GET /v1/chat/status — Check Bifrost connectivity.
async fn chat_status(
    State(config): State<Arc<Config>>,
) -> Json<ChatStatus> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

    let bifrost_ok = client
        .get(format!("{}/healthz", config.bifrost_url))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    Json(ChatStatus {
        eir_gateway: "ok".into(),
        bifrost_url: config.bifrost_url.clone(),
        bifrost_reachable: bifrost_ok,
    })
}

/// POST /v1/chat — Proxy user message to Bifrost agent streaming endpoint.
///
/// Forwards the message to Bifrost's `/agents/{agent_id}/stream` endpoint
/// and streams the SSE response back to the client.
async fn chat_proxy(
    State(config): State<Arc<Config>>,
    Json(payload): Json<ChatRequest>,
) -> Response {
    let agent_id = payload.agent.as_deref().unwrap_or("default");

    let bifrost_url = format!(
        "{}/v1/agents/{}/stream",
        config.bifrost_url, agent_id
    );

    tracing::info!(
        bifrost_url = %bifrost_url,
        message = %payload.message,
        agent = %agent_id,
        "Chat proxy → Bifrost"
    );

    // Build Bifrost request
    let body = json!({
        "message": payload.message,
        "session_id": payload.session_id,
        "stream": true,
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();

    let response = match client
        .post(&bifrost_url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .header("X-Gateway", "eir-gateway")
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!(error = %e, "Failed to reach Bifrost");
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "error": format!("Cannot reach Bifrost at {}: {}", config.bifrost_url, e),
                    "hint": "Ensure Bifrost is running: docker compose up -d bifrost"
                })),
            )
                .into_response();
        }
    };

    let status = response.status();

    // If Bifrost returns SSE, stream it through
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/event-stream"))
        .unwrap_or(false)
    {
        let stream = response.bytes_stream();
        return Response::builder()
            .status(status.as_u16())
            .header(header::CONTENT_TYPE, "text/event-stream")
            .header(header::CACHE_CONTROL, "no-cache")
            .header("X-Accel-Buffering", "no")
            .body(Body::from_stream(stream))
            .unwrap();
    }

    // Non-streaming response: forward JSON body
    let body_bytes = response.bytes().await.unwrap_or_default();
    let json_body: Value =
        serde_json::from_slice(&body_bytes).unwrap_or(json!({"raw": String::from_utf8_lossy(&body_bytes)}));

    (
        StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(json_body),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_router_creation() {
        let config = Arc::new(Config::from_env());
        let _app: axum::Router = router().with_state(config);
    }

    #[test]
    fn test_chat_request_serde() {
        let json_str = r#"{"message": "hello", "agent": "default"}"#;
        let req: ChatRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.message, "hello");
        assert_eq!(req.agent.unwrap(), "default");
    }
}

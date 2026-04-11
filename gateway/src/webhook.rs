//! OpenEMR Webhook Router Context Router
//!
//! Listens for events from OpenEMR (`patient.opened`, `medication.prescribed`, `encounter.closed`)
//! and translates them into a standard "Context Object" JSON.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, RwLock};

/// Standard Context Object definition
#[derive(Debug, Clone, Serialize)]
pub struct ContextObject {
    pub context_id: String,
    pub event_type: String,
    pub timestamp: String,
    pub payload: Value,
}

/// In-memory store to keep recent contexts for debugging/validation
#[derive(Debug, Clone)]
pub struct WebhookStore {
    inner: Arc<RwLock<Vec<ContextObject>>>,
}

impl Default for WebhookStore {
    fn default() -> Self {
        Self::new()
    }
}

impl WebhookStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn record_context(&self, context: ContextObject) {
        let mut store = self.inner.write().unwrap();
        store.push(context.clone());
        if store.len() > 100 {
            store.remove(0);
        }
    }
}

pub fn router() -> Router<Arc<WebhookStore>> {
    Router::new()
        .route("/v1/webhooks/openemr", post(openemr_webhook))
}

#[derive(Debug, Deserialize)]
pub struct OpenEmrEvent {
    pub event: String,
    pub data: Value,
}

async fn openemr_webhook(
    State(store): State<Arc<WebhookStore>>,
    Json(payload): Json<OpenEmrEvent>,
) -> (StatusCode, Json<Value>) {
    tracing::info!(event = %payload.event, "Received OpenEMR webhook");

    let is_valid = matches!(
        payload.event.as_str(),
        "patient.opened" | "medication.prescribed" | "encounter.closed"
    );

    if !is_valid {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"status": "ignored", "reason": "unsupported event"})),
        );
    }

    let context = ContextObject {
        context_id: uuid::Uuid::new_v4().to_string(),
        event_type: payload.event.clone(),
        timestamp: Utc::now().to_rfc3339(),
        payload: payload.data,
    };

    tracing::debug!("Generated context object: {:?}", context);
    store.record_context(context);

    (
        StatusCode::OK,
        Json(json!({"status": "accepted", "event": payload.event})),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_valid_webhook_event() {
        let store = Arc::new(WebhookStore::new());
        let app = router().with_state(store.clone());

        let payload = json!({
            "event": "patient.opened",
            "data": { "patient_id": "123" }
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/webhooks/openemr")
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let s = store.inner.read().unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].event_type, "patient.opened");
        assert_eq!(s[0].payload["patient_id"], "123");
    }

    #[tokio::test]
    async fn test_invalid_webhook_event() {
        let store = Arc::new(WebhookStore::new());
        let app = router().with_state(store.clone());

        let payload = json!({
            "event": "unknown.event",
            "data": {}
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/webhooks/openemr")
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let s = store.inner.read().unwrap();
        assert_eq!(s.len(), 0);
    }
}

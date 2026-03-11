//! Knowledge Sync — Webhook and status endpoints for Mimir integration.
//!
//! Receives knowledge base update notifications from Mimir and tracks
//! the sync status in an in-memory store.

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// In-memory knowledge sync state.
#[derive(Debug, Clone)]
pub struct KnowledgeStore {
    inner: Arc<RwLock<KnowledgeState>>,
}

#[derive(Debug, Clone, Default)]
struct KnowledgeState {
    /// Map of source_id → latest sync entry.
    sources: HashMap<String, KnowledgeSyncEntry>,
    /// Total webhook events received.
    total_events: u64,
}

/// A single knowledge sync entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSyncEntry {
    pub source_id: String,
    pub event: String,
    pub updated_at: String,
    pub received_at: String,
    pub metadata: Value,
}

/// Incoming webhook payload from Mimir.
#[derive(Debug, Deserialize, Serialize)]
pub struct MimirWebhookPayload {
    /// Event type (e.g. "knowledge.updated", "knowledge.created").
    pub event: String,
    /// Source/knowledge base identifier.
    pub source_id: String,
    /// Timestamp from Mimir.
    pub updated_at: Option<String>,
    /// Additional metadata.
    pub metadata: Option<Value>,
}

/// Knowledge sync status response.
#[derive(Debug, Serialize)]
pub struct KnowledgeStatusResponse {
    pub status: String,
    pub total_events_received: u64,
    pub sources_tracked: usize,
    pub sources: Vec<KnowledgeSyncEntry>,
    pub last_sync: Option<String>,
}

impl KnowledgeStore {
    /// Create a new empty knowledge store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(KnowledgeState::default())),
        }
    }

    /// Record a webhook event.
    pub fn record_event(&self, payload: &MimirWebhookPayload) {
        let mut state = self.inner.write().unwrap();
        let now = Utc::now().to_rfc3339();
        state.total_events += 1;

        let entry = KnowledgeSyncEntry {
            source_id: payload.source_id.clone(),
            event: payload.event.clone(),
            updated_at: payload.updated_at.clone().unwrap_or_else(|| now.clone()),
            received_at: now,
            metadata: payload.metadata.clone().unwrap_or(json!({})),
        };
        state.sources.insert(payload.source_id.clone(), entry);
    }

    /// Get current sync status.
    pub fn status(&self) -> KnowledgeStatusResponse {
        let state = self.inner.read().unwrap();
        let mut sources: Vec<KnowledgeSyncEntry> = state.sources.values().cloned().collect();
        sources.sort_by(|a, b| b.received_at.cmp(&a.received_at));

        let last_sync = sources.first().map(|s| s.received_at.clone());

        KnowledgeStatusResponse {
            status: if sources.is_empty() {
                "no_data".into()
            } else {
                "synced".into()
            },
            total_events_received: state.total_events,
            sources_tracked: sources.len(),
            sources,
            last_sync,
        }
    }
}

/// Build the knowledge sync router.
pub fn router() -> Router<Arc<KnowledgeStore>> {
    Router::new()
        .route("/v1/webhooks/mimir", post(mimir_webhook))
        .route("/v1/knowledge/status", get(knowledge_status))
}

/// POST /v1/webhooks/mimir — Receive knowledge update from Mimir.
async fn mimir_webhook(
    State(store): State<Arc<KnowledgeStore>>,
    Json(payload): Json<MimirWebhookPayload>,
) -> (StatusCode, Json<Value>) {
    tracing::info!(
        event = %payload.event,
        source_id = %payload.source_id,
        "Mimir webhook received"
    );

    store.record_event(&payload);

    (
        StatusCode::OK,
        Json(json!({
            "status": "accepted",
            "event": payload.event,
            "source_id": payload.source_id,
        })),
    )
}

/// GET /v1/knowledge/status — View knowledge sync status.
async fn knowledge_status(
    State(store): State<Arc<KnowledgeStore>>,
) -> Json<KnowledgeStatusResponse> {
    Json(store.status())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_store_creation() {
        let store = KnowledgeStore::new();
        let status = store.status();
        assert_eq!(status.status, "no_data");
        assert_eq!(status.total_events_received, 0);
        assert_eq!(status.sources_tracked, 0);
    }

    #[test]
    fn test_webhook_record_event() {
        let store = KnowledgeStore::new();
        let payload = MimirWebhookPayload {
            event: "knowledge.updated".into(),
            source_id: "source-1".into(),
            updated_at: Some("2026-03-12T00:00:00Z".into()),
            metadata: Some(json!({"chunks": 42})),
        };
        store.record_event(&payload);

        let status = store.status();
        assert_eq!(status.status, "synced");
        assert_eq!(status.total_events_received, 1);
        assert_eq!(status.sources_tracked, 1);
        assert!(status.last_sync.is_some());
    }

    #[test]
    fn test_webhook_duplicate_source_updates() {
        let store = KnowledgeStore::new();

        // First event
        store.record_event(&MimirWebhookPayload {
            event: "knowledge.created".into(),
            source_id: "source-1".into(),
            updated_at: None,
            metadata: None,
        });

        // Second event for same source (should update, not duplicate)
        store.record_event(&MimirWebhookPayload {
            event: "knowledge.updated".into(),
            source_id: "source-1".into(),
            updated_at: None,
            metadata: Some(json!({"version": 2})),
        });

        let status = store.status();
        assert_eq!(status.total_events_received, 2);
        assert_eq!(status.sources_tracked, 1); // Only 1 source
        assert_eq!(status.sources[0].event, "knowledge.updated"); // Latest event
    }

    #[test]
    fn test_webhook_multiple_sources() {
        let store = KnowledgeStore::new();

        store.record_event(&MimirWebhookPayload {
            event: "knowledge.created".into(),
            source_id: "source-1".into(),
            updated_at: None,
            metadata: None,
        });
        store.record_event(&MimirWebhookPayload {
            event: "knowledge.created".into(),
            source_id: "source-2".into(),
            updated_at: None,
            metadata: None,
        });

        let status = store.status();
        assert_eq!(status.sources_tracked, 2);
    }

    #[test]
    fn test_webhook_payload_serde() {
        let payload = MimirWebhookPayload {
            event: "knowledge.updated".into(),
            source_id: "src-42".into(),
            updated_at: Some("2026-03-12T00:00:00Z".into()),
            metadata: Some(json!({"chunks": 100})),
        };
        let json_str = serde_json::to_string(&payload).unwrap();
        let parsed: MimirWebhookPayload = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.event, "knowledge.updated");
        assert_eq!(parsed.source_id, "src-42");
    }

    #[test]
    fn test_knowledge_router_creation() {
        let store = Arc::new(KnowledgeStore::new());
        let _app: axum::Router = router().with_state(store);
    }
}

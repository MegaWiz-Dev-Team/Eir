//! MCP Audit Trail — logs all MCP tool invocations for compliance.
//!
//! Sprint 6: Provides in-memory audit logging for MCP-related REST calls,
//! with a query endpoint for monitoring and compliance review.

use axum::{
    extract::{Query, Request, State},
    middleware::Next,
    response::{Json, Response},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum number of audit entries to keep in memory (ring buffer).
const MAX_ENTRIES: usize = 10_000;

/// A single audit log entry for an MCP tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: u64,
    pub timestamp: DateTime<Utc>,
    pub user: String,
    pub role: String,
    pub tool_name: String,
    pub patient_id: Option<String>,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u64,
}

/// Thread-safe audit storage with ring buffer semantics.
#[derive(Debug)]
pub struct AuditStore {
    entries: RwLock<Vec<AuditEntry>>,
    counter: RwLock<u64>,
}

impl AuditStore {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::with_capacity(MAX_ENTRIES)),
            counter: RwLock::new(0),
        }
    }

    /// Insert a new audit entry. If at capacity, remove the oldest.
    pub async fn insert(&self, mut entry: AuditEntry) {
        let mut counter = self.counter.write().await;
        *counter += 1;
        entry.id = *counter;

        let mut entries = self.entries.write().await;
        if entries.len() >= MAX_ENTRIES {
            entries.remove(0);
        }
        entries.push(entry);
    }

    /// Query recent audit entries with optional filters.
    pub async fn query(&self, limit: usize, user_filter: Option<&str>) -> Vec<AuditEntry> {
        let entries = self.entries.read().await;
        entries
            .iter()
            .rev()
            .filter(|e| {
                if let Some(user) = user_filter {
                    e.user == user
                } else {
                    true
                }
            })
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get total count of entries.
    pub async fn count(&self) -> usize {
        self.entries.read().await.len()
    }
}

/// Build the audit query router.
pub fn router() -> Router<Arc<AuditStore>> {
    Router::new().route("/v1/audit/mcp", get(query_audit))
}

/// Query parameters for audit endpoint.
#[derive(Debug, Deserialize)]
pub struct AuditQueryParams {
    /// Maximum entries to return (default: 50).
    pub limit: Option<usize>,
    /// Filter by user ID.
    pub user: Option<String>,
}

/// GET /v1/audit/mcp — Query MCP audit trail.
async fn query_audit(
    State(store): State<Arc<AuditStore>>,
    Query(params): Query<AuditQueryParams>,
) -> Json<serde_json::Value> {
    let limit = params.limit.unwrap_or(50).min(500);
    let entries = store.query(limit, params.user.as_deref()).await;
    let total = store.count().await;

    Json(json!({
        "status": "success",
        "total_entries": total,
        "returned": entries.len(),
        "entries": entries,
    }))
}

/// Detect the MCP tool name from a request path.
pub fn detect_tool_name(path: &str, method: &str) -> Option<String> {
    if !path.starts_with("/api/patients") {
        return None;
    }

    if path == "/api/patients" && method == "GET" {
        return Some("search_patients".into());
    }
    if path.ends_with("/summary") && method == "GET" {
        return Some("get_patient_summary".into());
    }
    if path.ends_with("/encounters") && method == "POST" {
        return Some("create_encounter".into());
    }
    if path.ends_with("/sleep-reports") || path.contains("/sleep-reports?") {
        return Some("get_sleep_reports".into());
    }

    None
}

/// Extract patient ID from path like /api/patients/{id}/...
pub fn extract_patient_id(path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('/').collect();
    // /api/patients/{id}/...
    if parts.len() >= 4 && parts[1] == "api" && parts[2] == "patients" {
        let id = parts[3];
        if !id.is_empty() && id != "patients" {
            return Some(id.to_string());
        }
    }
    None
}

/// MCP audit middleware — captures /api/patients/* calls.
pub async fn mcp_audit_middleware(
    State(store): State<Arc<AuditStore>>,
    request: Request,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().as_str().to_string();

    // Only audit MCP-related paths
    let tool_name = match detect_tool_name(&path, &method) {
        Some(name) => name,
        None => return next.run(request).await,
    };

    let patient_id = extract_patient_id(&path);
    let user = request
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("anonymous")
        .to_string();
    let role = request
        .headers()
        .get("x-user-role")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let start = std::time::Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();

    let entry = AuditEntry {
        id: 0, // Will be set by store
        timestamp: Utc::now(),
        user,
        role,
        tool_name: tool_name.clone(),
        patient_id,
        method: method.clone(),
        path: path.clone(),
        status_code: response.status().as_u16(),
        duration_ms: duration.as_millis() as u64,
    };

    tracing::info!(
        tool = %tool_name,
        path = %path,
        status = response.status().as_u16(),
        duration_ms = duration.as_millis() as u64,
        "MCP audit entry"
    );

    store.insert(entry).await;
    response
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_store_insert_and_query() {
        let store = AuditStore::new();
        let entry = AuditEntry {
            id: 0,
            timestamp: Utc::now(),
            user: "dr-smith".into(),
            role: "doctor".into(),
            tool_name: "search_patients".into(),
            patient_id: Some("PT-001".into()),
            method: "GET".into(),
            path: "/api/patients".into(),
            status_code: 200,
            duration_ms: 42,
        };

        store.insert(entry).await;
        let results = store.query(10, None).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].user, "dr-smith");
        assert_eq!(results[0].id, 1);
    }

    #[tokio::test]
    async fn test_audit_store_ring_buffer() {
        let store = AuditStore::new();
        // Insert MAX_ENTRIES + 1 to test overflow
        for i in 0..MAX_ENTRIES + 1 {
            let entry = AuditEntry {
                id: 0,
                timestamp: Utc::now(),
                user: format!("user-{}", i),
                role: "doctor".into(),
                tool_name: "search_patients".into(),
                patient_id: None,
                method: "GET".into(),
                path: "/api/patients".into(),
                status_code: 200,
                duration_ms: 1,
            };
            store.insert(entry).await;
        }

        assert_eq!(store.count().await, MAX_ENTRIES);
    }

    #[tokio::test]
    async fn test_audit_store_query_by_user() {
        let store = AuditStore::new();
        for user in ["alice", "bob", "alice"] {
            let entry = AuditEntry {
                id: 0,
                timestamp: Utc::now(),
                user: user.into(),
                role: "doctor".into(),
                tool_name: "search_patients".into(),
                patient_id: None,
                method: "GET".into(),
                path: "/api/patients".into(),
                status_code: 200,
                duration_ms: 1,
            };
            store.insert(entry).await;
        }

        let alice = store.query(10, Some("alice")).await;
        assert_eq!(alice.len(), 2);
        let bob = store.query(10, Some("bob")).await;
        assert_eq!(bob.len(), 1);
    }

    #[test]
    fn test_detect_tool_name() {
        assert_eq!(
            detect_tool_name("/api/patients", "GET"),
            Some("search_patients".into())
        );
        assert_eq!(
            detect_tool_name("/api/patients/1/summary", "GET"),
            Some("get_patient_summary".into())
        );
        assert_eq!(
            detect_tool_name("/api/patients/1/encounters", "POST"),
            Some("create_encounter".into())
        );
        assert_eq!(
            detect_tool_name("/api/patients/1/sleep-reports", "GET"),
            Some("get_sleep_reports".into())
        );
        assert_eq!(detect_tool_name("/healthz", "GET"), None);
    }

    #[test]
    fn test_extract_patient_id() {
        assert_eq!(
            extract_patient_id("/api/patients/123/summary"),
            Some("123".into())
        );
        assert_eq!(
            extract_patient_id("/api/patients/SANDBOX-PT-001/sleep-reports"),
            Some("SANDBOX-PT-001".into())
        );
        assert_eq!(extract_patient_id("/api/patients"), None);
    }

    #[test]
    fn test_audit_entry_serialization() {
        let entry = AuditEntry {
            id: 1,
            timestamp: Utc::now(),
            user: "dr-smith".into(),
            role: "doctor".into(),
            tool_name: "search_patients".into(),
            patient_id: Some("PT-001".into()),
            method: "GET".into(),
            path: "/api/patients".into(),
            status_code: 200,
            duration_ms: 42,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"tool_name\":\"search_patients\""));
        assert!(json.contains("\"user\":\"dr-smith\""));
    }
}

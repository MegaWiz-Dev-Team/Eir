//! A2A Protocol — Agent-to-Agent communication endpoints.
//!
//! Implements Google's A2A protocol for inter-agent communication,
//! matching the patterns used by Bifrost (bifrost/api/a2a.py).
//! - Agent Card (/.well-known/agent.json)
//! - Task lifecycle (send, get status)

use axum::{
    extract::{Path, State},
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

// === A2A Models ===

/// A2A Task lifecycle states (matching Bifrost's TaskState enum).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Failed,
    Canceled,
}

/// A message in A2A format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    /// "user" or "agent"
    pub role: String,
    /// Message parts: [{"type": "text", "text": "..."}]
    pub parts: Vec<Value>,
}

/// An A2A task with full lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    pub id: String,
    pub state: TaskState,
    pub messages: Vec<A2AMessage>,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

// === Task Store ===

/// In-memory A2A task store.
#[derive(Debug, Clone)]
pub struct A2ATaskStore {
    inner: Arc<RwLock<HashMap<String, A2ATask>>>,
}

impl A2ATaskStore {
    /// Create a new empty task store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Insert a task.
    pub fn insert(&self, task: A2ATask) {
        let mut store = self.inner.write().unwrap();
        store.insert(task.id.clone(), task);
    }

    /// Get a task by ID.
    pub fn get(&self, id: &str) -> Option<A2ATask> {
        let store = self.inner.read().unwrap();
        store.get(id).cloned()
    }

    /// Update a task's state.
    #[allow(dead_code)]
    pub fn update_state(&self, id: &str, state: TaskState) {
        let mut store = self.inner.write().unwrap();
        if let Some(task) = store.get_mut(id) {
            task.state = state;
            task.updated_at = Utc::now().to_rfc3339();
        }
    }

    /// List all tasks (most recent first), limited.
    pub fn list(&self, limit: usize) -> Vec<A2ATask> {
        let store = self.inner.read().unwrap();
        let mut tasks: Vec<A2ATask> = store.values().cloned().collect();
        tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        tasks.truncate(limit);
        tasks
    }

    /// Total task count.
    pub fn count(&self) -> usize {
        let store = self.inner.read().unwrap();
        store.len()
    }
}

// === Requests ===

/// Request to send a task.
#[derive(Debug, Deserialize, Serialize)]
pub struct SendTaskRequest {
    pub message: A2AMessage,
    #[serde(default = "default_skill")]
    pub skill: String,
    #[serde(default)]
    pub metadata: Value,
}

fn default_skill() -> String {
    "fhir-query".to_string()
}

// === Router ===

/// Build the A2A protocol router.
pub fn router() -> Router<Arc<A2ATaskStore>> {
    Router::new()
        .route("/.well-known/agent.json", get(agent_card))
        .route("/a2a/tasks/send", post(send_task))
        .route("/a2a/tasks/{id}", get(get_task))
        .route("/a2a/tasks", get(list_tasks))
}

// === Handlers ===

/// GET /.well-known/agent.json — A2A Agent Card.
async fn agent_card() -> Json<Value> {
    Json(build_agent_card())
}

/// Build the Agent Card for Eir Gateway.
fn build_agent_card() -> Value {
    json!({
        "name": "Eir Gateway",
        "description": "Rust API Gateway for OpenEMR — part of the Asgard AI Platform. Provides FHIR R4 proxy, patient search, clinical summary, and knowledge sync capabilities.",
        "url": "http://localhost:9090",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "a2a",
        "capabilities": {
            "streaming": false,
            "pushNotifications": false,
            "stateTransitionHistory": true
        },
        "skills": [
            {
                "id": "fhir-query",
                "name": "FHIR Query",
                "description": "Query FHIR R4 resources from OpenEMR using natural language or structured parameters.",
                "tags": ["fhir", "query", "openemr"]
            },
            {
                "id": "patient-search",
                "name": "Patient Search",
                "description": "Search for patients by name, birthdate, or identifier.",
                "tags": ["patient", "search", "fhir"]
            },
            {
                "id": "clinical-summary",
                "name": "Clinical Summary",
                "description": "Aggregate clinical data (conditions, medications, allergies) for a patient.",
                "tags": ["clinical", "summary", "patient"]
            },
            {
                "id": "knowledge-sync",
                "name": "Knowledge Sync",
                "description": "Receive and track knowledge base updates from Mimir RAG platform.",
                "tags": ["knowledge", "mimir", "sync"]
            }
        ],
        "authentication": {
            "schemes": ["bearer"]
        }
    })
}

/// POST /a2a/tasks/send — Receive a task from Bifrost or other agents.
async fn send_task(
    State(store): State<Arc<A2ATaskStore>>,
    Json(request): Json<SendTaskRequest>,
) -> (StatusCode, Json<Value>) {
    let now = Utc::now().to_rfc3339();
    let task_id = uuid::Uuid::new_v4().to_string();

    // Extract text from message parts
    let user_text: String = request
        .message
        .parts
        .iter()
        .filter_map(|p| {
            if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                p.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    if user_text.is_empty() {
        let task = A2ATask {
            id: task_id,
            state: TaskState::Failed,
            messages: vec![
                request.message,
                A2AMessage {
                    role: "agent".into(),
                    parts: vec![json!({"type": "text", "text": "No text content in message"})],
                },
            ],
            metadata: request.metadata,
            created_at: now.clone(),
            updated_at: now,
        };
        store.insert(task.clone());
        return (StatusCode::BAD_REQUEST, Json(json!({"task": task})));
    }

    // Create task in working state
    let response_text = format!(
        "Task received by Eir Gateway. Skill: '{}'. Query: '{}'. \
         Route to the appropriate /v1/ endpoint for execution.",
        request.skill, user_text
    );

    let task = A2ATask {
        id: task_id,
        state: TaskState::Completed,
        messages: vec![
            request.message,
            A2AMessage {
                role: "agent".into(),
                parts: vec![json!({"type": "text", "text": response_text})],
            },
        ],
        metadata: json!({
            "skill": request.skill,
            "original_metadata": request.metadata,
        }),
        created_at: now.clone(),
        updated_at: now,
    };

    tracing::info!(
        task_id = %task.id,
        skill = %request.skill,
        "A2A task completed"
    );

    store.insert(task.clone());

    (StatusCode::OK, Json(json!({"task": task})))
}

/// GET /a2a/tasks/{id} — Get task status and messages.
async fn get_task(
    State(store): State<Arc<A2ATaskStore>>,
    Path(task_id): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    match store.get(&task_id) {
        Some(task) => Ok(Json(json!({"task": task}))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({"error": format!("Task '{}' not found", task_id)})),
        )),
    }
}

/// GET /a2a/tasks — List recent tasks.
async fn list_tasks(
    State(store): State<Arc<A2ATaskStore>>,
) -> Json<Value> {
    let tasks = store.list(20);
    Json(json!({
        "tasks": tasks,
        "total": store.count(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_card_content() {
        let card = build_agent_card();
        assert_eq!(card["name"], "Eir Gateway");
        assert_eq!(card["protocol"], "a2a");
        assert!(card["skills"].is_array());
        assert_eq!(card["skills"].as_array().unwrap().len(), 4);
        assert_eq!(card["version"], env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_agent_card_skills() {
        let card = build_agent_card();
        let skills = card["skills"].as_array().unwrap();
        let skill_ids: Vec<&str> = skills
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect();
        assert!(skill_ids.contains(&"fhir-query"));
        assert!(skill_ids.contains(&"patient-search"));
        assert!(skill_ids.contains(&"clinical-summary"));
        assert!(skill_ids.contains(&"knowledge-sync"));
    }

    #[test]
    fn test_task_store_creation() {
        let store = A2ATaskStore::new();
        assert_eq!(store.count(), 0);
        assert!(store.list(10).is_empty());
    }

    #[test]
    fn test_task_store_insert_and_get() {
        let store = A2ATaskStore::new();
        let task = A2ATask {
            id: "test-task-1".into(),
            state: TaskState::Submitted,
            messages: vec![],
            metadata: json!({}),
            created_at: "2026-03-12T00:00:00Z".into(),
            updated_at: "2026-03-12T00:00:00Z".into(),
        };
        store.insert(task);
        assert_eq!(store.count(), 1);

        let retrieved = store.get("test-task-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().state, TaskState::Submitted);
    }

    #[test]
    fn test_task_not_found() {
        let store = A2ATaskStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_task_state_update() {
        let store = A2ATaskStore::new();
        store.insert(A2ATask {
            id: "task-2".into(),
            state: TaskState::Submitted,
            messages: vec![],
            metadata: json!({}),
            created_at: "2026-03-12T00:00:00Z".into(),
            updated_at: "2026-03-12T00:00:00Z".into(),
        });

        store.update_state("task-2", TaskState::Completed);
        let task = store.get("task-2").unwrap();
        assert_eq!(task.state, TaskState::Completed);
    }

    #[test]
    fn test_task_state_serde() {
        let state = TaskState::InputRequired;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"input-required\"");

        let parsed: TaskState = serde_json::from_str("\"completed\"").unwrap();
        assert_eq!(parsed, TaskState::Completed);
    }

    #[test]
    fn test_a2a_message_serde() {
        let msg = A2AMessage {
            role: "user".into(),
            parts: vec![json!({"type": "text", "text": "Hello Eir"})],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: A2AMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.parts.len(), 1);
    }

    #[test]
    fn test_a2a_router_creation() {
        let store = Arc::new(A2ATaskStore::new());
        let _app: axum::Router = router().with_state(store);
    }
}

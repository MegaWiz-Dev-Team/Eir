//! OpenAPI documentation — auto-generated spec using utoipa with Scalar UI.

use axum::{routing::get, Json, Router};
use std::sync::Arc;
use utoipa::OpenApi;

use crate::config::Config;

/// OpenAPI 3.1 specification for Eir Gateway.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Eir Gateway API",
        version = "0.5.0",
        description = "Rust API Gateway for OpenEMR — part of the Asgard AI Platform.\n\nProvides FHIR R4 proxy, response caching, rate limiting, request transformation, Bifrost agent tools, Mimir knowledge sync, A2A protocol, Hermóðr MCP patient endpoints, role-based access control, and MCP audit trail.",
        contact(name = "MegaWiz", email = "paripol@megawiz.co"),
        license(name = "AGPL-3.0", url = "https://www.gnu.org/licenses/agpl-3.0.en.html"),
    ),
    paths(
        openapi_spec,
        health_check,
        readiness_check,
        fhir_proxy,
        fhir_query,
        patients_search,
        clinical_summary,
        mimir_webhook,
        knowledge_status,
        agent_card,
        a2a_send_task,
        a2a_get_task,
        patients_search_api,
        patient_summary,
        patient_create_encounter,
        patient_sleep_reports,
        mcp_audit_query,
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "fhir", description = "FHIR R4 proxy endpoints"),
        (name = "agent-tools", description = "Bifrost agent tool endpoints"),
        (name = "knowledge", description = "Mimir knowledge sync endpoints"),
        (name = "a2a", description = "A2A Agent-to-Agent protocol endpoints"),
        (name = "patients", description = "Hermóðr MCP patient endpoints (Sprint 6)"),
        (name = "audit", description = "MCP audit trail endpoints"),
        (name = "docs", description = "API documentation"),
    ),
)]
struct ApiDoc;

/// Build the OpenAPI docs router.
pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/api-docs/openapi.json", get(openapi_spec))
        .route("/api-docs", get(scalar_ui))
}

/// Returns the OpenAPI 3.1 JSON specification.
#[utoipa::path(
    get,
    path = "/api-docs/openapi.json",
    tag = "docs",
    responses(
        (status = 200, description = "OpenAPI JSON specification"),
    ),
)]
async fn openapi_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// Liveness probe — returns OK if gateway is running.
#[utoipa::path(
    get,
    path = "/healthz",
    tag = "health",
    responses(
        (status = 200, description = "Gateway is alive", body = inline(serde_json::Value)),
    ),
)]
#[allow(dead_code)]
async fn health_check() {}

/// Readiness probe — checks if upstream OpenEMR is reachable.
#[utoipa::path(
    get,
    path = "/readyz",
    tag = "health",
    responses(
        (status = 200, description = "Gateway and upstream ready"),
        (status = 503, description = "Upstream unavailable"),
    ),
)]
#[allow(dead_code)]
async fn readiness_check() {}

/// FHIR R4 proxy — forwards requests to OpenEMR FHIR endpoint.
#[utoipa::path(
    get,
    path = "/fhir/r4/{resource}",
    tag = "fhir",
    params(
        ("resource" = String, Path, description = "FHIR resource path (e.g. Patient/123)"),
    ),
    responses(
        (status = 200, description = "FHIR resource response"),
        (status = 502, description = "Upstream error"),
    ),
)]
#[allow(dead_code)]
async fn fhir_proxy() {}

/// Query FHIR resources via natural language or structured parameters.
#[utoipa::path(
    post,
    path = "/v1/fhir/query",
    tag = "agent-tools",
    responses(
        (status = 200, description = "FHIR query results"),
        (status = 502, description = "Upstream FHIR error"),
    ),
)]
#[allow(dead_code)]
async fn fhir_query() {}

/// Search patients for agent workflows.
#[utoipa::path(
    get,
    path = "/v1/patients/search",
    tag = "agent-tools",
    params(
        ("name" = Option<String>, Query, description = "Patient name"),
        ("birthdate" = Option<String>, Query, description = "Date of birth (YYYY-MM-DD)"),
        ("identifier" = Option<String>, Query, description = "Patient identifier"),
    ),
    responses(
        (status = 200, description = "Patient search results"),
    ),
)]
#[allow(dead_code)]
async fn patients_search() {}

/// Aggregate clinical data for a patient.
#[utoipa::path(
    post,
    path = "/v1/clinical/summary",
    tag = "agent-tools",
    responses(
        (status = 200, description = "Clinical summary"),
    ),
)]
#[allow(dead_code)]
async fn clinical_summary() {}

/// Receive knowledge update webhook from Mimir.
#[utoipa::path(
    post,
    path = "/v1/webhooks/mimir",
    tag = "knowledge",
    responses(
        (status = 200, description = "Webhook accepted"),
    ),
)]
#[allow(dead_code)]
async fn mimir_webhook() {}

/// View knowledge sync status.
#[utoipa::path(
    get,
    path = "/v1/knowledge/status",
    tag = "knowledge",
    responses(
        (status = 200, description = "Knowledge sync status"),
    ),
)]
#[allow(dead_code)]
async fn knowledge_status() {}

/// A2A Agent Card — describes Eir's capabilities.
#[utoipa::path(
    get,
    path = "/.well-known/agent.json",
    tag = "a2a",
    responses(
        (status = 200, description = "A2A Agent Card JSON"),
    ),
)]
#[allow(dead_code)]
async fn agent_card() {}

/// Send a task to Eir via A2A protocol.
#[utoipa::path(
    post,
    path = "/a2a/tasks/send",
    tag = "a2a",
    responses(
        (status = 200, description = "Task created and processed"),
        (status = 400, description = "Invalid task message"),
    ),
)]
#[allow(dead_code)]
async fn a2a_send_task() {}

/// Get task status and messages.
#[utoipa::path(
    get,
    path = "/a2a/tasks/{id}",
    tag = "a2a",
    params(
        ("id" = String, Path, description = "Task ID"),
    ),
    responses(
        (status = 200, description = "Task details"),
        (status = 404, description = "Task not found"),
    ),
)]
#[allow(dead_code)]
async fn a2a_get_task() {}

/// Search patients by name, DOB, or identifier (Hermóðr search_patients tool).
#[utoipa::path(
    get,
    path = "/api/patients",
    tag = "patients",
    params(
        ("query" = Option<String>, Query, description = "Search query: name, DOB, or PID"),
    ),
    responses(
        (status = 200, description = "Patient search results"),
        (status = 403, description = "Insufficient role permissions"),
    ),
)]
#[allow(dead_code)]
async fn patients_search_api() {}

/// Get comprehensive patient summary with demographics, CPAP, and sleep data.
#[utoipa::path(
    get,
    path = "/api/patients/{id}/summary",
    tag = "patients",
    params(
        ("id" = String, Path, description = "Patient PID"),
    ),
    responses(
        (status = 200, description = "Patient summary"),
        (status = 403, description = "Insufficient role permissions"),
    ),
)]
#[allow(dead_code)]
async fn patient_summary() {}

/// Create a new clinical encounter for a patient.
#[utoipa::path(
    post,
    path = "/api/patients/{id}/encounters",
    tag = "patients",
    params(
        ("id" = String, Path, description = "Patient PID"),
    ),
    responses(
        (status = 200, description = "Encounter created"),
        (status = 403, description = "Insufficient role permissions (nurse/admin blocked)"),
    ),
)]
#[allow(dead_code)]
async fn patient_create_encounter() {}

/// Get sleep therapy reports for a patient.
#[utoipa::path(
    get,
    path = "/api/patients/{id}/sleep-reports",
    tag = "patients",
    params(
        ("id" = String, Path, description = "Patient PID"),
        ("days" = Option<i32>, Query, description = "Days to look back (default: 30)"),
    ),
    responses(
        (status = 200, description = "Sleep report data"),
        (status = 403, description = "Insufficient role permissions"),
    ),
)]
#[allow(dead_code)]
async fn patient_sleep_reports() {}

/// Query MCP audit trail entries.
#[utoipa::path(
    get,
    path = "/v1/audit/mcp",
    tag = "audit",
    params(
        ("limit" = Option<usize>, Query, description = "Max entries to return (default: 50)"),
        ("user" = Option<String>, Query, description = "Filter by user ID"),
    ),
    responses(
        (status = 200, description = "Audit trail entries"),
    ),
)]
#[allow(dead_code)]
async fn mcp_audit_query() {}

/// Renders the Scalar API documentation UI.
async fn scalar_ui() -> axum::response::Html<String> {
    let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <title>Eir Gateway — API Docs</title>
    <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
    <script id="api-reference" data-url="/api-docs/openapi.json"></script>
    <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#.to_string();
    axum::response::Html(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let spec = ApiDoc::openapi();
        assert_eq!(spec.info.title, "Eir Gateway API");
        assert_eq!(spec.info.version, "0.5.0");
        assert!(!spec.paths.paths.is_empty());
    }

    #[test]
    fn test_openapi_has_fhir_path() {
        let spec = ApiDoc::openapi();
        assert!(
            spec.paths.paths.contains_key("/fhir/r4/{resource}"),
            "Should contain FHIR proxy path"
        );
    }

    #[test]
    fn test_openapi_has_health_paths() {
        let spec = ApiDoc::openapi();
        assert!(spec.paths.paths.contains_key("/healthz"));
        assert!(spec.paths.paths.contains_key("/readyz"));
    }

    #[test]
    fn test_openapi_has_agent_tools_paths() {
        let spec = ApiDoc::openapi();
        assert!(spec.paths.paths.contains_key("/v1/fhir/query"));
        assert!(spec.paths.paths.contains_key("/v1/patients/search"));
        assert!(spec.paths.paths.contains_key("/v1/clinical/summary"));
    }

    #[test]
    fn test_openapi_has_knowledge_paths() {
        let spec = ApiDoc::openapi();
        assert!(spec.paths.paths.contains_key("/v1/webhooks/mimir"));
        assert!(spec.paths.paths.contains_key("/v1/knowledge/status"));
    }

    #[test]
    fn test_openapi_has_a2a_paths() {
        let spec = ApiDoc::openapi();
        assert!(spec.paths.paths.contains_key("/.well-known/agent.json"));
        assert!(spec.paths.paths.contains_key("/a2a/tasks/send"));
        assert!(spec.paths.paths.contains_key("/a2a/tasks/{id}"));
    }

    #[test]
    fn test_openapi_has_patient_tools_paths() {
        let spec = ApiDoc::openapi();
        assert!(spec.paths.paths.contains_key("/api/patients"));
        assert!(spec.paths.paths.contains_key("/api/patients/{id}/summary"));
        assert!(spec.paths.paths.contains_key("/api/patients/{id}/encounters"));
        assert!(spec.paths.paths.contains_key("/api/patients/{id}/sleep-reports"));
    }

    #[test]
    fn test_openapi_has_audit_path() {
        let spec = ApiDoc::openapi();
        assert!(spec.paths.paths.contains_key("/v1/audit/mcp"));
    }

    #[test]
    fn test_router_creation() {
        let config = Arc::new(Config::from_env());
        let _app: axum::Router = router().with_state(config);
    }
}

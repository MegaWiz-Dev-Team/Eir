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
        version = "0.2.0",
        description = "Rust API Gateway for OpenEMR — part of the Asgard AI Platform.\n\nProvides FHIR R4 proxy, response caching, rate limiting, and request transformation.",
        contact(name = "MegaWiz", email = "paripol@megawiz.co"),
        license(name = "AGPL-3.0", url = "https://www.gnu.org/licenses/agpl-3.0.en.html"),
    ),
    paths(
        openapi_spec,
        health_check,
        readiness_check,
        fhir_proxy,
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "fhir", description = "FHIR R4 proxy endpoints"),
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
        assert_eq!(spec.info.version, "0.2.0");
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
    fn test_router_creation() {
        let config = Arc::new(Config::from_env());
        let _app: axum::Router = router().with_state(config);
    }
}

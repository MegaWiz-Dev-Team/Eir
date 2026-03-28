//! FHIR-aware proxy — routes FHIR R4 requests with FHIR-specific headers.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use std::sync::Arc;

use crate::config::Config;

/// Build the FHIR R4 proxy router.
pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/fhir/r4/{*path}", any(fhir_proxy_handler))
        .route("/fhir/r4", any(fhir_proxy_handler))
}

/// Forward FHIR requests to upstream with FHIR-specific headers.
async fn fhir_proxy_handler(
    State(config): State<Arc<Config>>,
    request: Request,
) -> Result<Response, (StatusCode, String)> {
    let path = request.uri().path();
    let query = request
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    // Map /fhir/r4/* → upstream /apis/default/fhir/*
    let fhir_path = path.strip_prefix("/fhir/r4").unwrap_or(path);
    let upstream_url = format!(
        "{}/apis/default/fhir{}{}",
        config.openemr_url, fhir_path, query
    );

    tracing::info!(
        upstream_url = %upstream_url,
        method = %request.method(),
        "FHIR R4 proxy request"
    );

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let method = request.method().clone();

    let mut builder = client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap(),
        &upstream_url,
    );

    // Forward headers (except host), add FHIR-specific headers
    for (name, value) in request.headers().iter() {
        if name != "host" {
            if let Ok(v) = value.to_str() {
                builder = builder.header(name.as_str(), v);
            }
        }
    }

    // FHIR-specific headers
    builder = builder.header("Accept", "application/fhir+json");
    builder = builder.header("X-FHIR-Version", "R4");
    builder = builder.header("X-Gateway", "eir-gateway");
    builder = builder.header("X-Gateway-Route", "fhir-r4");

    // Forward body
    let body_bytes = axum::body::to_bytes(request.into_body(), 10 * 1024 * 1024)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Body read error: {e}")))?;

    if !body_bytes.is_empty() {
        builder = builder.body(body_bytes.to_vec());
    }

    // Send upstream request
    let upstream_response = builder
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("FHIR upstream error: {e}")))?;

    // Build response
    let status = StatusCode::from_u16(upstream_response.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let mut response_headers = HeaderMap::new();
    for (name, value) in upstream_response.headers().iter() {
        if let Ok(v) = HeaderValue::from_str(value.to_str().unwrap_or("")) {
            response_headers.insert(name.clone(), v);
        }
    }

    let response_body = upstream_response
        .bytes()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("FHIR response body error: {e}")))?;

    let mut response = Response::builder()
        .status(status)
        .body(Body::from(response_body))
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Response build error: {e}"),
            )
        })?;

    *response.headers_mut() = response_headers;

    // Add FHIR content-type indicator
    response.headers_mut().insert(
        "X-FHIR-Content",
        HeaderValue::from_static("application/fhir+json"),
    );

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fhir_router_creation() {
        let config = Arc::new(Config::from_env());
        let _app: axum::Router = router().with_state(config);
        // Router construction should not panic
    }

    #[test]
    fn test_fhir_path_mapping() {
        // Verify the path rewriting logic
        let path = "/fhir/r4/Patient/123";
        let fhir_path = path.strip_prefix("/fhir/r4").unwrap_or(path);
        assert_eq!(fhir_path, "/Patient/123");

        let metadata_path = "/fhir/r4/metadata";
        let fhir_metadata = metadata_path.strip_prefix("/fhir/r4").unwrap_or(metadata_path);
        assert_eq!(fhir_metadata, "/metadata");
    }

    #[test]
    fn test_fhir_base_path() {
        let path = "/fhir/r4";
        let fhir_path = path.strip_prefix("/fhir/r4").unwrap_or(path);
        assert_eq!(fhir_path, "");
    }
}

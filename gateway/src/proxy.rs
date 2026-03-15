//! Reverse proxy — forwards all requests to the OpenEMR PHP backend.

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

/// Build the catch-all proxy router.
pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/{*path}", any(proxy_handler))
        .route("/", any(proxy_handler))
}

/// Forward any request to the upstream OpenEMR server.
async fn proxy_handler(
    State(config): State<Arc<Config>>,
    request: Request,
) -> Result<Response, (StatusCode, String)> {
    let path = request.uri().path();
    let query = request
        .uri()
        .query()
        .map(|q| format!("?{q}"))
        .unwrap_or_default();

    let upstream_url = format!("{}{}{}", config.openemr_url, path, query);

    // Build upstream request
    let client = reqwest::Client::new();
    let method = request.method().clone();

    let mut builder = client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap(),
        &upstream_url,
    );

    // Forward headers (except host)
    for (name, value) in request.headers().iter() {
        if name != "host" {
            if let Ok(v) = value.to_str() {
                builder = builder.header(name.as_str(), v);
            }
        }
    }

    // Add proxy headers
    builder = builder.header("X-Forwarded-Proto", "http");
    builder = builder.header("X-Gateway", "eir-gateway");

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
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Upstream error: {e}")))?;

    // Build response
    let status = StatusCode::from_u16(upstream_response.status().as_u16())
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let is_html = upstream_response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.contains("text/html"))
        .unwrap_or(false);

    let mut response_headers = HeaderMap::new();
    for (name, value) in upstream_response.headers().iter() {
        // Skip content-length since we may modify the body
        if is_html && name == "content-length" {
            continue;
        }
        if let Ok(v) = HeaderValue::from_str(value.to_str().unwrap_or("")) {
            response_headers.insert(name.clone(), v);
        }
    }

    let response_body = upstream_response
        .bytes()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("Response body error: {e}")))?;

    // Inject chat widget into HTML responses
    let final_body = if is_html {
        let html = String::from_utf8_lossy(&response_body);
        if html.contains("</body>") {
            let injected = crate::chat::inject_widget(&html);
            Body::from(injected)
        } else {
            Body::from(response_body)
        }
    } else {
        Body::from(response_body)
    };

    let mut response = Response::builder()
        .status(status)
        .body(final_body)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Response build error: {e}")))?;

    *response.headers_mut() = response_headers;

    Ok(response)
}

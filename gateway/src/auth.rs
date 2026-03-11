//! Auth middleware — Bearer token validation.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{Json, Response},
};
use serde_json::json;
use std::sync::Arc;

use crate::config::Config;

/// Paths that skip authentication.
const PUBLIC_PATHS: [&str; 5] = [
    "/healthz",
    "/readyz",
    "/fhir/r4/metadata",
    "/api-docs",
    "/api-docs/openapi.json",
];

/// Auth middleware — validates Bearer token in Authorization header.
pub async fn auth_middleware(
    State(config): State<Arc<Config>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    // Skip auth if disabled
    if !config.auth_enabled {
        return Ok(next.run(request).await);
    }

    // Skip auth for public paths
    let path = request.uri().path().to_string();
    if PUBLIC_PATHS.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(request).await);
    }

    // Extract Bearer token
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            if token == config.auth_secret {
                Ok(next.run(request).await)
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Invalid token"})),
                ))
            }
        }
        _ => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Missing or invalid Authorization header"})),
        )),
    }
}

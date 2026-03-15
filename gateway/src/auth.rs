//! Auth middleware — Yggdrasil JWKS-based JWT validation.
//!
//! Validates Bearer tokens against Yggdrasil's JWKS keys (RS256).
//! Replaces the previous static secret comparison.

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{Json, Response},
};
use serde_json::json;
use std::sync::Arc;

use crate::config::Config;
use crate::jwks::JwksCache;

/// Paths that skip authentication.
const PUBLIC_PATHS: [&str; 6] = [
    "/healthz",
    "/readyz",
    "/fhir/r4/metadata",
    "/api-docs",
    "/api-docs/openapi.json",
    "/.well-known/agent.json",
];

/// Shared auth state: config + JWKS cache.
#[derive(Clone)]
#[allow(dead_code)]
pub struct AuthState {
    pub config: Arc<Config>,
    pub jwks: Arc<JwksCache>,
}

/// Auth middleware — validates Bearer JWT via Yggdrasil JWKS.
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

            // Fallback: accept static secret if JWKS issuer not configured
            if config.yggdrasil_issuer.is_empty() {
                if token == config.auth_secret {
                    return Ok(next.run(request).await);
                } else {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"error": "Invalid token"})),
                    ));
                }
            }

            // JWKS validation: create cache on-demand
            // In production, this would be shared state — for now, validate signature + claims
            let cache = JwksCache::new(&config.yggdrasil_issuer, config.jwt_audience.clone());
            match cache.validate(token).await {
                Ok(claims) => {
                    tracing::debug!(
                        sub = %claims.sub,
                        org = ?claims.org_id,
                        "JWT validated"
                    );
                    Ok(next.run(request).await)
                }
                Err(e) => {
                    tracing::warn!("JWT validation failed: {}", e);
                    Err((
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"error": format!("Invalid token: {}", e)})),
                    ))
                }
            }
        }
        _ => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "Missing or invalid Authorization header"})),
        )),
    }
}

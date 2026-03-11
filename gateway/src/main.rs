//! Eir Gateway — Rust API Gateway for OpenEMR
//!
//! Part of the Asgard AI Platform.
//! Sits in front of OpenEMR PHP backend providing:
//! - Reverse proxy with header forwarding
//! - FHIR R4-aware proxy with FHIR-specific headers
//! - In-memory response caching (moka)
//! - Per-IP rate limiting (governor / GCRA)
//! - Request transformation (tenant headers, path rewrite)
//! - Bearer token authentication
//! - Structured audit logging
//! - Health check endpoints
//! - OpenAPI documentation with Scalar UI
//! - CORS support

mod audit;
mod auth;
mod cache;
mod config;
mod fhir;
mod health;
mod openapi;
mod proxy;
mod rate_limit;
mod transform;

use axum::middleware;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{fmt, EnvFilter};

use crate::cache::ResponseCache;
use crate::config::Config;
use crate::rate_limit::RateLimiterState;

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    // Initialize structured logging
    fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.log_level)),
        )
        .json()
        .init();

    tracing::info!(
        openemr_url = %config.openemr_url,
        listen_addr = %config.listen_addr,
        auth_enabled = config.auth_enabled,
        rate_limit_rps = config.rate_limit_rps,
        cache_ttl_secs = config.cache_ttl_secs,
        tenant_id = %config.tenant_id,
        "Starting Eir Gateway v{}",
        env!("CARGO_PKG_VERSION")
    );

    let shared_config = Arc::new(config.clone());

    // Sprint 2 state: cache + rate limiter
    let response_cache = Arc::new(ResponseCache::new(config.cache_ttl_secs));
    let rate_limiter = Arc::new(RateLimiterState::new(config.rate_limit_rps));

    // CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application
    // Middleware stack (applied bottom-up):
    //   CORS → Audit → RateLimit → Auth → Transform → Cache → [FHIR | OpenAPI | Health | Proxy]
    let app = axum::Router::new()
        // Health endpoints (no auth / no audit needed)
        .merge(health::router())
        // OpenAPI documentation
        .merge(openapi::router())
        // FHIR R4 proxy (more specific, matched before catch-all)
        .merge(fhir::router())
        // Proxy all other routes to OpenEMR
        .merge(proxy::router())
        // Cache layer
        .layer(middleware::from_fn_with_state(
            response_cache.clone(),
            |state: axum::extract::State<Arc<ResponseCache>>,
             request: axum::extract::Request,
             next: middleware::Next| async move {
                cache::cache_middleware(state.0, request, next).await
            },
        ))
        // Transform layer
        .layer(middleware::from_fn_with_state(
            shared_config.clone(),
            transform::transform_middleware,
        ))
        // Auth layer
        .layer(middleware::from_fn_with_state(
            shared_config.clone(),
            auth::auth_middleware,
        ))
        // Rate limit layer
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            |state: axum::extract::State<Arc<RateLimiterState>>,
             request: axum::extract::Request,
             next: middleware::Next| async move {
                rate_limit::rate_limit_middleware(state.0, request, next).await
            },
        ))
        // Audit logging layer
        .layer(middleware::from_fn(audit::audit_middleware))
        .layer(cors)
        .with_state(shared_config.clone());

    tracing::info!("Eir Gateway listening on {}", config.listen_addr);

    let listener = tokio::net::TcpListener::bind(config.listen_addr)
        .await
        .expect("Failed to bind listener");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

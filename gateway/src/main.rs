//! Eir Gateway — Rust API Gateway for OpenEMR
//!
//! Part of the Asgard AI Platform.
//! Sits in front of OpenEMR PHP backend providing:
//! - Reverse proxy with header forwarding
//! - Bearer token authentication  
//! - Structured audit logging
//! - Health check endpoints
//! - CORS support

mod audit;
mod auth;
mod config;
mod health;
mod proxy;

use axum::middleware;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{fmt, EnvFilter};

use crate::config::Config;

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
        "Starting Eir Gateway"
    );

    let shared_config = Arc::new(config.clone());

    // CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application
    let app = axum::Router::new()
        // Health endpoints (no auth / no audit needed)
        .merge(health::router())
        // Proxy all other routes to OpenEMR
        .merge(proxy::router())
        // Middleware stack (applied bottom-up: audit first, then auth)
        .layer(middleware::from_fn_with_state(
            shared_config.clone(),
            auth::auth_middleware,
        ))
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

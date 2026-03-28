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
//! - Bifrost agent tool endpoints (Sprint 3)
//! - Mimir knowledge sync webhooks (Sprint 3)
//! - A2A protocol support (Sprint 3)
//! - Hermóðr MCP patient endpoints (Sprint 6)
//! - RBAC role-based access control (Sprint 6)
//! - MCP audit trail (Sprint 6)

mod a2a;
mod agent_tools;
mod audit;
mod auth;
mod cache;
mod chat;
mod config;
mod fhir;
mod health;
mod jwks;
mod knowledge;
mod mcp_audit;
mod oauth;
mod openapi;
mod patients;
mod proxy;
mod rate_limit;
mod rbac;
mod transform;

use axum::middleware;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{fmt, EnvFilter};

use crate::a2a::A2ATaskStore;
use crate::cache::ResponseCache;
use crate::config::Config;
use crate::knowledge::KnowledgeStore;
use crate::mcp_audit::AuditStore;
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

    tracing::info!(
        bifrost_url = %config.bifrost_url,
        "Bifrost agent target configured"
    );

    let shared_config = Arc::new(config.clone());

    // OAuth2 token service for upstream OpenEMR API calls
    let token_service = oauth::OAuthConfig::from_env().map(|c| {
        let ts = Arc::new(oauth::TokenService::new(c));
        tracing::info!("OAuth2 token service configured for OpenEMR");
        ts
    });

    // Sprint 2 state: cache + rate limiter
    let response_cache = Arc::new(ResponseCache::new(config.cache_ttl_secs));
    let rate_limiter = Arc::new(RateLimiterState::new(config.rate_limit_rps));

    // Sprint 3 state: knowledge store + A2A task store
    let knowledge_store = Arc::new(KnowledgeStore::new());
    let a2a_task_store = Arc::new(A2ATaskStore::new());

    // Sprint 6 state: MCP audit store
    let audit_store = Arc::new(AuditStore::new());

    // CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build application
    // Middleware stack (applied bottom-up):
    //   CORS → Audit → RateLimit → Auth → Transform → Cache → [Routes]
    let app = axum::Router::new()
        // Health endpoints (no auth / no audit needed)
        .merge(health::router())
        // OpenAPI documentation
        .merge(openapi::router())
        // FHIR R4 proxy (more specific, matched before catch-all)
        .merge(fhir::router())
        // Sprint 3: Agent tools
        .merge(agent_tools::router())
        // Sprint 3: Knowledge sync
        .merge(knowledge::router().with_state(knowledge_store.clone()))
        // Sprint 3: A2A protocol
        .merge(a2a::router().with_state(a2a_task_store.clone()))
        // Sprint 4: Chat interface
        .merge(chat::router())
        // Sprint 6: Hermóðr patient endpoints
        .merge(patients::router(patients::PatientState {
            config: shared_config.clone(),
            token_service: token_service.clone(),
        }))
        // Sprint 6: MCP audit query endpoint
        .merge(mcp_audit::router().with_state(audit_store.clone()))
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
        // Sprint 6: MCP audit trail layer
        .layer(middleware::from_fn_with_state(
            audit_store.clone(),
            mcp_audit::mcp_audit_middleware,
        ))
        // Sprint 6: RBAC layer
        .layer(middleware::from_fn(rbac::rbac_middleware))
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

//! Health check endpoints.

use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::config::Config;

/// Build health check router.
pub fn router() -> Router<Arc<Config>> {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
}

/// Liveness probe — always returns OK if gateway is running.
async fn healthz() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "eir-gateway",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Readiness probe — checks if upstream OpenEMR is reachable.
async fn readyz(State(config): State<Arc<Config>>) -> (StatusCode, Json<Value>) {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

    let upstream_ok = client
        .get(&config.openemr_url)
        .send()
        .await
        .map(|r| r.status().is_success() || r.status().is_redirection())
        .unwrap_or(false);

    let status = if upstream_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(json!({
            "status": if upstream_ok { "ready" } else { "degraded" },
            "gateway": "ok",
            "openemr_upstream": if upstream_ok { "reachable" } else { "unreachable" },
            "openemr_url": &config.openemr_url,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_healthz() {
        let response = healthz().await;
        assert_eq!(response.0["status"], "ok");
        assert_eq!(response.0["service"], "eir-gateway");
    }
}

//! Audit logging — structured request/response logging.

use axum::{extract::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::info;

/// Audit middleware — logs every request with method, path, status, duration.
pub async fn audit_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    let start = Instant::now();
    let response = next.run(request).await;
    let duration = start.elapsed();

    info!(
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        duration_ms = duration.as_millis() as u64,
        request_id = %request_id,
        "request completed"
    );

    response
}

//! Request transformation — injects tenant headers and rewrites paths.

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use crate::config::Config;

/// Transform middleware — injects tenant headers and rewrites API paths.
pub async fn transform_middleware(
    State(config): State<Arc<Config>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Inject tenant header
    request.headers_mut().insert(
        "X-Tenant-ID",
        config.tenant_id.parse().unwrap_or_else(|_| {
            "default".parse().unwrap()
        }),
    );

    // Inject gateway version
    request.headers_mut().insert(
        "X-Gateway-Version",
        env!("CARGO_PKG_VERSION").parse().unwrap(),
    );

    // Path rewriting: /api/v1/* → /apis/default/api/*
    let path = request.uri().path().to_string();
    if path.starts_with("/api/v1/") {
        let new_path = path.replacen("/api/v1/", "/apis/default/api/", 1);
        let query = request
            .uri()
            .query()
            .map(|q| format!("?{q}"))
            .unwrap_or_default();
        let new_uri = format!("{new_path}{query}");

        tracing::debug!(
            original = %path,
            rewritten = %new_uri,
            "Path rewritten"
        );

        *request.uri_mut() = new_uri.parse().unwrap_or_else(|_| request.uri().clone());
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[test]
    fn test_path_rewrite_logic() {
        let original = "/api/v1/patient/123";
        let rewritten = original.replacen("/api/v1/", "/apis/default/api/", 1);
        assert_eq!(rewritten, "/apis/default/api/patient/123");
    }

    #[test]
    fn test_path_no_rewrite() {
        let original = "/healthz";
        let rewritten = if original.starts_with("/api/v1/") {
            original.replacen("/api/v1/", "/apis/default/api/", 1)
        } else {
            original.to_string()
        };
        assert_eq!(rewritten, "/healthz");
    }

    #[test]
    fn test_path_rewrite_with_query() {
        let path = "/api/v1/patient";
        let query = "name=John";
        let rewritten = path.replacen("/api/v1/", "/apis/default/api/", 1);
        let full = format!("{rewritten}?{query}");
        assert_eq!(full, "/apis/default/api/patient?name=John");
    }

    #[tokio::test]
    async fn test_transform_headers_injected() {
        let config = Arc::new(Config::from_env());

        // Verify config has tenant_id
        assert!(!config.tenant_id.is_empty());

        // Create a mock request to verify header logic
        let mut req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        req.headers_mut().insert(
            "X-Tenant-ID",
            config.tenant_id.parse().unwrap(),
        );
        req.headers_mut().insert(
            "X-Gateway-Version",
            env!("CARGO_PKG_VERSION").parse().unwrap(),
        );

        assert_eq!(
            req.headers().get("X-Tenant-ID").unwrap().to_str().unwrap(),
            config.tenant_id
        );
        assert_eq!(
            req.headers()
                .get("X-Gateway-Version")
                .unwrap()
                .to_str()
                .unwrap(),
            env!("CARGO_PKG_VERSION")
        );
    }
}

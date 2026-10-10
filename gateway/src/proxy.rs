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
use crate::upstream;

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
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let method = request.method().clone();

    let mut builder = client.request(
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap(),
        &upstream_url,
    );

    // Forward headers (except host and any client-supplied gateway trust header)
    for (name, value) in request.headers().iter() {
        if name != "host" && !upstream::is_trust_header(name.as_str()) {
            if let Ok(v) = value.to_str() {
                builder = builder.header(name.as_str(), v);
            }
        }
    }

    // Add proxy headers
    builder = builder.header("X-Forwarded-Proto", "http");
    builder = upstream::with_gateway_identity(builder, &config);

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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::upstream::tests::{test_config, TEST_SECRET};
    use crate::upstream::{GATEWAY_MARKER_HEADER, GATEWAY_SECRET_HEADER};
    use axum::{body::to_bytes, http::Request as HttpRequest, Json};
    use tower::ServiceExt;

    /// Local stand-in for OpenEMR that echoes the trust headers it received.
    pub(crate) async fn spawn_echo_upstream() -> String {
        async fn echo(headers: HeaderMap) -> Json<serde_json::Value> {
            let values = |name: &str| -> Vec<String> {
                headers
                    .get_all(name)
                    .iter()
                    .map(|v| v.to_str().unwrap_or("").to_string())
                    .collect()
            };
            Json(serde_json::json!({
                "secret": values(GATEWAY_SECRET_HEADER),
                "marker": values(GATEWAY_MARKER_HEADER),
                "authorization": values("authorization"),
            }))
        }
        let app = Router::new().fallback(echo);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{addr}")
    }

    pub(crate) async fn send_forged(app: Router, path: &str) -> serde_json::Value {
        let request = HttpRequest::builder()
            .uri(path)
            .header("X-Gateway", "eir-gateway")
            .header("X-Eir-Gateway-Secret", "client-forged-secret")
            .header("Authorization", "Bearer caller-token")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[tokio::test]
    async fn test_proxy_replaces_client_trust_headers_with_gateway_secret() {
        let upstream_url = spawn_echo_upstream().await;
        let app = router().with_state(Arc::new(test_config(&upstream_url, Some(TEST_SECRET))));
        let seen = send_forged(app, "/apis/default/api/patient").await;
        assert_eq!(seen["secret"], serde_json::json!([TEST_SECRET]));
        assert_eq!(seen["marker"], serde_json::json!(["eir-gateway"]));
        // caller credentials still pass through
        assert_eq!(
            seen["authorization"],
            serde_json::json!(["Bearer caller-token"])
        );
    }

    #[tokio::test]
    async fn test_proxy_without_secret_forwards_no_secret() {
        let upstream_url = spawn_echo_upstream().await;
        let app = router().with_state(Arc::new(test_config(&upstream_url, None)));
        let seen = send_forged(app, "/apis/default/api/patient").await;
        assert_eq!(seen["secret"], serde_json::json!([]));
    }
}

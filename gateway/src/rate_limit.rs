//! Rate limiting — per-IP rate limiter using governor (GCRA algorithm).

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{Json, Response},
};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use serde_json::json;
use std::num::NonZeroU32;
use std::sync::Arc;

/// Per-IP rate limiter state.
#[derive(Clone)]
pub struct RateLimiterState {
    limiter: Arc<DefaultKeyedRateLimiter<String>>,
}

impl RateLimiterState {
    /// Create a new rate limiter with the given requests-per-second limit.
    pub fn new(rps: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(rps.max(1)).unwrap());
        let limiter = Arc::new(RateLimiter::keyed(quota));
        Self { limiter }
    }

    /// Extract client IP from request headers or connection info.
    fn extract_ip(request: &Request) -> String {
        // Try X-Forwarded-For first (for proxied connections)
        if let Some(forwarded) = request.headers().get("x-forwarded-for") {
            if let Ok(val) = forwarded.to_str() {
                if let Some(first_ip) = val.split(',').next() {
                    return first_ip.trim().to_string();
                }
            }
        }

        // Try X-Real-IP
        if let Some(real_ip) = request.headers().get("x-real-ip") {
            if let Ok(val) = real_ip.to_str() {
                return val.trim().to_string();
            }
        }

        // Fallback to unknown
        "unknown".to_string()
    }
}

/// Rate limit middleware — returns 429 when limit exceeded.
pub async fn rate_limit_middleware(
    state: Arc<RateLimiterState>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let client_ip = RateLimiterState::extract_ip(&request);

    match state.limiter.check_key(&client_ip) {
        Ok(_) => {
            let mut response = next.run(request).await;
            // Add rate limit headers
            response.headers_mut().insert(
                "X-RateLimit-Policy",
                HeaderValue::from_static("per-ip"),
            );
            Ok(response)
        }
        Err(_not_until) => {
            tracing::warn!(
                client_ip = %client_ip,
                "Rate limit exceeded"
            );
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({
                    "error": "Rate limit exceeded",
                    "retry_after_secs": 1,
                    "client_ip": client_ip,
                })),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;

    #[test]
    fn test_rate_limiter_creation() {
        let state = RateLimiterState::new(100);
        // Should not panic
        assert!(Arc::strong_count(&state.limiter) >= 1);
    }

    #[test]
    fn test_extract_ip_forwarded_for() {
        let req = Request::builder()
            .header("x-forwarded-for", "192.168.1.1, 10.0.0.1")
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        assert_eq!(RateLimiterState::extract_ip(&req), "192.168.1.1");
    }

    #[test]
    fn test_extract_ip_real_ip() {
        let req = Request::builder()
            .header("x-real-ip", "172.16.0.1")
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        assert_eq!(RateLimiterState::extract_ip(&req), "172.16.0.1");
    }

    #[test]
    fn test_extract_ip_fallback() {
        let req = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        assert_eq!(RateLimiterState::extract_ip(&req), "unknown");
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let state = RateLimiterState::new(10);
        // First request should be allowed
        let result = state.limiter.check_key(&"test-ip".to_string());
        assert!(result.is_ok());
    }
}

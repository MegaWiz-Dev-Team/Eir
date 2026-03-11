//! Response caching — in-memory cache for GET requests using moka.

use axum::{
    body::Body,
    extract::Request,
    http::{HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use bytes::Bytes;
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// Cached response data.
#[derive(Clone, Debug)]
struct CachedResponse {
    status: u16,
    body: Bytes,
    content_type: Option<String>,
}

/// In-memory response cache wrapping moka.
#[derive(Clone)]
pub struct ResponseCache {
    inner: Cache<String, CachedResponse>,
}

impl ResponseCache {
    /// Create a new cache with the given TTL in seconds.
    pub fn new(ttl_secs: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(ttl_secs))
            .build();

        Self { inner: cache }
    }

    /// Build a cache key from method + path + query.
    fn cache_key(request: &Request) -> Option<String> {
        if request.method() != Method::GET {
            return None;
        }
        let path = request.uri().path();
        let query = request.uri().query().unwrap_or("");
        Some(format!("GET:{}?{}", path, query))
    }
}

/// Cache middleware — caches GET responses, adds X-Cache header.
pub async fn cache_middleware(
    cache: Arc<ResponseCache>,
    request: Request,
    next: Next,
) -> Response {
    // Only cache GET requests
    let cache_key = match ResponseCache::cache_key(&request) {
        Some(key) => key,
        None => {
            // Non-GET request, pass through
            return next.run(request).await;
        }
    };

    // Check cache
    if let Some(cached) = cache.inner.get(&cache_key).await {
        tracing::debug!(key = %cache_key, "Cache HIT");

        let mut response = Response::builder()
            .status(StatusCode::from_u16(cached.status).unwrap_or(StatusCode::OK))
            .body(Body::from(cached.body.clone()))
            .unwrap();

        if let Some(ct) = &cached.content_type {
            response
                .headers_mut()
                .insert("content-type", HeaderValue::from_str(ct).unwrap());
        }
        response
            .headers_mut()
            .insert("X-Cache", HeaderValue::from_static("HIT"));

        return response;
    }

    tracing::debug!(key = %cache_key, "Cache MISS");

    // Forward request
    let response = next.run(request).await;

    // Only cache successful responses
    if response.status().is_success() {
        let (parts, body) = response.into_parts();
        let body_bytes = axum::body::to_bytes(body, 5 * 1024 * 1024)
            .await
            .unwrap_or_default();

        let content_type = parts
            .headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let cached = CachedResponse {
            status: parts.status.as_u16(),
            body: body_bytes.clone(),
            content_type: content_type.clone(),
        };

        cache.inner.insert(cache_key, cached).await;

        let mut response = Response::from_parts(parts, Body::from(body_bytes));
        response
            .headers_mut()
            .insert("X-Cache", HeaderValue::from_static("MISS"));
        response
    } else {
        let mut response = response;
        response
            .headers_mut()
            .insert("X-Cache", HeaderValue::from_static("SKIP"));
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_creation() {
        let cache = ResponseCache::new(60);
        assert_eq!(cache.inner.entry_count(), 0);
    }

    #[test]
    fn test_cache_key_get_only() {
        // GET request should produce a key
        let get_req = Request::builder()
            .method(Method::GET)
            .uri("/test?q=1")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            ResponseCache::cache_key(&get_req),
            Some("GET:/test?q=1".to_string())
        );

        // POST request should return None
        let post_req = Request::builder()
            .method(Method::POST)
            .uri("/test")
            .body(Body::empty())
            .unwrap();
        assert!(ResponseCache::cache_key(&post_req).is_none());
    }

    #[test]
    fn test_cache_key_no_query() {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/patients")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            ResponseCache::cache_key(&req),
            Some("GET:/patients?".to_string())
        );
    }

    #[tokio::test]
    async fn test_cache_insert_and_get() {
        let cache = ResponseCache::new(60);
        let entry = CachedResponse {
            status: 200,
            body: Bytes::from("test body"),
            content_type: Some("application/json".to_string()),
        };
        cache.inner.insert("test-key".to_string(), entry).await;
        let retrieved = cache.inner.get(&"test-key".to_string()).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().status, 200);
    }
}

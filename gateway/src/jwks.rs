//! JWKS-based JWT validation for Zitadel tokens.
//!
//! Fetches JSON Web Key Set from Zitadel's OIDC discovery endpoint,
//! caches keys, and validates RS256 JWT signatures.

use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Claims extracted from a validated Zitadel JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZitadelClaims {
    pub sub: String,
    pub iss: String,
    pub aud: Option<serde_json::Value>,
    pub exp: u64,
    pub iat: Option<u64>,
    /// Zitadel organization ID (tenant)
    #[serde(rename = "urn:zitadel:iam:org:id")]
    pub org_id: Option<String>,
    /// Zitadel project roles
    #[serde(rename = "urn:zitadel:iam:org:project:roles")]
    pub roles: Option<serde_json::Value>,
}

/// A single JWK from the JWKS endpoint.
#[derive(Debug, Clone, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
    #[serde(default)]
    #[allow(dead_code)]
    alg: Option<String>,
}

/// JWKS response from Zitadel.
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

/// Cached JWKS keys with expiry.
pub struct JwksCache {
    issuer: String,
    audience: Option<String>,
    keys: Arc<RwLock<HashMap<String, DecodingKey>>>,
    last_refresh: Arc<RwLock<Option<Instant>>>,
    refresh_interval: Duration,
    client: Client,
}

impl JwksCache {
    /// Create a new JWKS cache for the given Zitadel issuer.
    pub fn new(issuer: &str, audience: Option<String>) -> Self {
        Self {
            issuer: issuer.trim_end_matches('/').to_string(),
            audience,
            keys: Arc::new(RwLock::new(HashMap::new())),
            last_refresh: Arc::new(RwLock::new(None)),
            refresh_interval: Duration::from_secs(3600), // 1 hour
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Fetch JWKS from Zitadel's discovery endpoint.
    async fn refresh_keys(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let jwks_url = format!("{}/.well-known/keys", self.issuer);
        tracing::debug!("Fetching JWKS from {}", jwks_url);

        let resp = self.client.get(&jwks_url).send().await?;
        let jwks: JwksResponse = resp.json().await?;

        let mut keys = self.keys.write().await;
        keys.clear();

        for jwk in jwks.keys {
            if jwk.kty == "RSA" {
                match DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
                    Ok(key) => {
                        keys.insert(jwk.kid.clone(), key);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse JWK {}: {}", jwk.kid, e);
                    }
                }
            }
        }

        let mut last = self.last_refresh.write().await;
        *last = Some(Instant::now());

        tracing::info!("JWKS refreshed: {} keys cached", keys.len());
        Ok(())
    }

    /// Check if cache needs refresh.
    async fn needs_refresh(&self) -> bool {
        let last = self.last_refresh.read().await;
        match *last {
            None => true,
            Some(t) => t.elapsed() > self.refresh_interval,
        }
    }

    /// Validate a JWT token against cached JWKS keys.
    pub async fn validate(
        &self,
        token: &str,
    ) -> Result<ZitadelClaims, Box<dyn std::error::Error + Send + Sync>> {
        // Refresh if needed
        if self.needs_refresh().await {
            self.refresh_keys().await?;
        }

        // Decode header to get kid
        let header = decode_header(token)?;
        let kid = header
            .kid
            .ok_or("JWT missing kid header")?;

        // Find matching key
        let keys = self.keys.read().await;
        let key = keys
            .get(&kid)
            .ok_or_else(|| format!("Unknown kid: {}", kid))?;

        // Build validation
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&self.issuer]);

        if let Some(ref aud) = self.audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }

        let token_data = decode::<ZitadelClaims>(token, key, &validation)?;
        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_deserialize() {
        let json_str = r#"{
            "sub": "user-123",
            "iss": "http://localhost:8085",
            "aud": "app-id",
            "exp": 9999999999,
            "iat": 1000000000,
            "urn:zitadel:iam:org:id": "org-abc",
            "urn:zitadel:iam:org:project:roles": {"admin": {"org-abc": "org-abc"}}
        }"#;
        let claims: ZitadelClaims = serde_json::from_str(json_str).unwrap();
        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.iss, "http://localhost:8085");
        assert_eq!(claims.org_id, Some("org-abc".to_string()));
        assert!(claims.roles.is_some());
    }

    #[test]
    fn test_claims_minimal() {
        let json_str = r#"{
            "sub": "svc-account",
            "iss": "http://localhost:8085",
            "exp": 9999999999
        }"#;
        let claims: ZitadelClaims = serde_json::from_str(json_str).unwrap();
        assert_eq!(claims.sub, "svc-account");
        assert!(claims.org_id.is_none());
        assert!(claims.roles.is_none());
        assert!(claims.aud.is_none());
    }

    #[test]
    fn test_jwks_cache_creation() {
        let cache = JwksCache::new("http://localhost:8085", Some("eir-app".to_string()));
        assert_eq!(cache.issuer, "http://localhost:8085");
        assert_eq!(cache.audience, Some("eir-app".to_string()));
    }

    #[test]
    fn test_jwks_cache_strips_trailing_slash() {
        let cache = JwksCache::new("http://localhost:8085/", None);
        assert_eq!(cache.issuer, "http://localhost:8085");
    }

    #[test]
    fn test_jwks_cache_no_audience() {
        let cache = JwksCache::new("http://localhost:8085", None);
        assert!(cache.audience.is_none());
    }

    #[tokio::test]
    async fn test_needs_refresh_initially_true() {
        let cache = JwksCache::new("http://localhost:8085", None);
        assert!(cache.needs_refresh().await);
    }

    #[test]
    fn test_malformed_token_header() {
        let result = decode_header("not-a-jwt");
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_rs256_config() {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&["http://localhost:8085"]);
        validation.set_audience(&["eir-gateway"]);
        assert_eq!(validation.algorithms, vec![Algorithm::RS256]);
    }
}

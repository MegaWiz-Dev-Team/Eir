//! OAuth2 Token Service — Auto-acquires and caches OpenEMR access tokens.
//!
//! Uses the OAuth2 password grant to obtain tokens for internal
//! service-to-service communication with OpenEMR's FHIR/REST APIs.
//! Tokens are cached and auto-refreshed before expiry.

use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cached OAuth2 token with expiry tracking.
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    /// Epoch seconds when this token expires.
    expires_at: u64,
}

/// OAuth2 configuration for OpenEMR password grant.
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub token_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub username: String,
    pub password: String,
    pub scope: String,
}

impl OAuthConfig {
    /// Load from environment variables.
    pub fn from_env() -> Option<Self> {
        let client_id = std::env::var("OPENEMR_CLIENT_ID").ok()?;
        let client_secret = std::env::var("OPENEMR_CLIENT_SECRET").ok()?;
        let openemr_url = std::env::var("OPENEMR_URL")
            .unwrap_or_else(|_| "http://localhost:80".to_string());
        // Use HTTPS for OAuth2 token endpoint (OpenEMR requires it)
        let token_url = std::env::var("OPENEMR_TOKEN_URL")
            .unwrap_or_else(|_| format!("{}/oauth2/default/token", openemr_url.replace("http://", "https://")));

        Some(Self {
            token_url,
            client_id,
            client_secret,
            username: std::env::var("OPENEMR_USERNAME").unwrap_or_else(|_| "admin".to_string()),
            password: std::env::var("OPENEMR_PASSWORD").unwrap_or_else(|_| "pass".to_string()),
            scope: std::env::var("OPENEMR_SCOPE")
                .unwrap_or_else(|_| "openid api:fhir api:oemr".to_string()),
        })
    }
}

/// Token response from OpenEMR OAuth2 endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
    #[allow(dead_code)]
    token_type: String,
}

/// Thread-safe OAuth2 token service with auto-refresh.
#[derive(Clone)]
pub struct TokenService {
    config: OAuthConfig,
    cached: Arc<RwLock<Option<CachedToken>>>,
    http: reqwest::Client,
}

impl TokenService {
    pub fn new(config: OAuthConfig) -> Self {
        // Build an HTTP client that accepts self-signed certs (internal K8s)
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            cached: Arc::new(RwLock::new(None)),
            http,
        }
    }

    /// Get a valid access token, refreshing if needed.
    pub async fn get_token(&self) -> Result<String, String> {
        // Check cache first
        {
            let cache = self.cached.read().await;
            if let Some(ref token) = *cache {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                // Refresh 60s before expiry
                if now < token.expires_at.saturating_sub(60) {
                    return Ok(token.access_token.clone());
                }
            }
        }

        // Acquire new token
        self.refresh_token().await
    }

    async fn refresh_token(&self) -> Result<String, String> {
        let mut cache = self.cached.write().await;

        // Double-check after acquiring write lock
        if let Some(ref token) = *cache {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if now < token.expires_at.saturating_sub(60) {
                return Ok(token.access_token.clone());
            }
        }

        tracing::info!(
            token_url = %self.config.token_url,
            "Acquiring new OpenEMR OAuth2 token"
        );

        let params = [
            ("grant_type", "password"),
            ("client_id", &self.config.client_id),
            ("client_secret", &self.config.client_secret),
            ("user_role", "users"),
            ("username", &self.config.username),
            ("password", &self.config.password),
            ("scope", &self.config.scope),
        ];

        let response = self
            .http
            .post(&self.config.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Token request failed: {e}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %body, "OAuth2 token request rejected");
            return Err(format!("OAuth2 token request failed ({}): {}", status, body));
        }

        let token_resp: TokenResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse token response: {e}"))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let cached_token = CachedToken {
            access_token: token_resp.access_token.clone(),
            expires_at: now + token_resp.expires_in,
        };

        tracing::info!(
            expires_in = token_resp.expires_in,
            "OpenEMR OAuth2 token acquired"
        );

        *cache = Some(cached_token);
        Ok(token_resp.access_token)
    }
}

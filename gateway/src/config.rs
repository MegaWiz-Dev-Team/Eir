//! Configuration module — loads settings from environment variables.

use std::env;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    /// Upstream OpenEMR URL (e.g. "http://localhost:80")
    pub openemr_url: String,
    /// Gateway listen address
    pub listen_addr: SocketAddr,
    /// Auth secret for Bearer token validation (legacy fallback)
    pub auth_secret: String,
    /// Log level (e.g. "info", "debug", "warn")
    pub log_level: String,
    /// Whether auth is enabled
    pub auth_enabled: bool,
    /// Zitadel issuer URL for JWKS validation (empty = use static auth_secret)
    pub zitadel_issuer: String,
    /// Expected JWT audience (optional)
    pub jwt_audience: Option<String>,
    /// Rate limit: max requests per second per IP
    pub rate_limit_rps: u32,
    /// Cache TTL in seconds for GET responses
    pub cache_ttl_secs: u64,
    /// Tenant identifier for header injection
    pub tenant_id: String,
    /// Bifrost agent runtime URL
    pub bifrost_url: String,
}

impl Config {
    /// Load config from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        let host = env::var("GATEWAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("GATEWAY_PORT")
            .unwrap_or_else(|_| "9090".to_string())
            .parse::<u16>()
            .expect("GATEWAY_PORT must be a valid port number");

        Self {
            openemr_url: env::var("OPENEMR_URL")
                .unwrap_or_else(|_| "http://localhost:80".to_string()),
            listen_addr: format!("{host}:{port}")
                .parse()
                .expect("Invalid listen address"),
            auth_secret: env::var("AUTH_SECRET").unwrap_or_else(|_| "dev-secret".to_string()),
            log_level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            auth_enabled: env::var("AUTH_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            zitadel_issuer: env::var("ZITADEL_ISSUER").unwrap_or_default(),
            jwt_audience: env::var("JWT_AUDIENCE").ok().filter(|s| !s.is_empty()),
            rate_limit_rps: env::var("RATE_LIMIT_RPS")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            cache_ttl_secs: env::var("CACHE_TTL_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            tenant_id: env::var("TENANT_ID").unwrap_or_else(|_| "default".to_string()),
            bifrost_url: env::var("BIFROST_URL")
                .unwrap_or_else(|_| "http://bifrost:8100".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        // Clear env vars to test defaults
        env::remove_var("OPENEMR_URL");
        env::remove_var("GATEWAY_PORT");
        env::remove_var("AUTH_SECRET");
        env::remove_var("RATE_LIMIT_RPS");
        env::remove_var("CACHE_TTL_SECS");
        env::remove_var("TENANT_ID");
        let config = Config::from_env();
        assert_eq!(config.openemr_url, "http://localhost:80");
        assert_eq!(config.listen_addr.port(), 9090);
        assert_eq!(config.log_level, "info");
        assert_eq!(config.rate_limit_rps, 100);
        assert_eq!(config.cache_ttl_secs, 60);
        assert_eq!(config.tenant_id, "default");
    }
}


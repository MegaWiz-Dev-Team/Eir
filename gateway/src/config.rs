//! Configuration module — loads settings from environment variables.

use std::env;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    /// Upstream OpenEMR URL (e.g. "http://localhost:80")
    pub openemr_url: String,
    /// Gateway listen address
    pub listen_addr: SocketAddr,
    /// Auth secret for Bearer token validation
    pub auth_secret: String,
    /// Log level (e.g. "info", "debug", "warn")
    pub log_level: String,
    /// Whether auth is enabled
    pub auth_enabled: bool,
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
        let config = Config::from_env();
        assert_eq!(config.openemr_url, "http://localhost:80");
        assert_eq!(config.listen_addr.port(), 9090);
        assert_eq!(config.log_level, "info");
    }
}

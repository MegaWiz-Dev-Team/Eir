//! Gateway identity on upstream OpenEMR requests.
//!
//! OpenEMR (`SkipAuthorizationStrategy.php`) trusts a request as coming from
//! this gateway only when it carries the secret shared through
//! `EIR_GATEWAY_SECRET`. The bare `X-Gateway: eir-gateway` header is
//! informational and trusted by nobody.
//!
//! Two rules keep that trust unforgeable:
//! 1. A client's own copy of a trust header is never forwarded upstream
//!    (`is_trust_header`), on every path that copies client headers.
//! 2. The secret is attached only by the gateway, only to OpenEMR
//!    (`with_gateway_identity`), never to other services such as Bifrost.

use reqwest::header::{HeaderMap, HeaderValue};

use crate::config::Config;

/// Header carrying the shared secret to OpenEMR (lower-case for `http`).
pub const GATEWAY_SECRET_HEADER: &str = "x-eir-gateway-secret";

/// Informational marker header; not a credential.
pub const GATEWAY_MARKER_HEADER: &str = "x-gateway";

/// Headers a client must never be able to set on a request to OpenEMR.
pub fn is_trust_header(name: &str) -> bool {
    name.eq_ignore_ascii_case(GATEWAY_SECRET_HEADER)
        || name.eq_ignore_ascii_case(GATEWAY_MARKER_HEADER)
}

/// Headers that mark an upstream OpenEMR request as sent by this gateway:
/// the informational marker and, when configured, the shared secret.
/// The secret value is flagged sensitive so `Debug` output never shows it.
pub fn gateway_identity_headers(config: &Config) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        GATEWAY_MARKER_HEADER,
        HeaderValue::from_static("eir-gateway"),
    );
    if let Some(secret) = &config.eir_gateway_secret {
        if let Ok(mut value) = HeaderValue::from_str(secret.expose()) {
            value.set_sensitive(true);
            headers.insert(GATEWAY_SECRET_HEADER, value);
        }
    }
    headers
}

/// Mark an upstream OpenEMR request as sent by this gateway.
///
/// `RequestBuilder::headers` replaces any value already set under the same
/// name, so a client copy that slipped through is overwritten, not appended.
/// With no secret configured OpenEMR fails closed and the request has to
/// stand on its own credentials (e.g. a Bearer token).
pub fn with_gateway_identity(
    builder: reqwest::RequestBuilder,
    config: &Config,
) -> reqwest::RequestBuilder {
    builder.headers(gateway_identity_headers(config))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::config::GatewaySecret;

    pub(crate) const TEST_SECRET: &str = "test-only-gateway-secret-xxxxxxxxxxxxxxxx";

    pub(crate) fn test_config(openemr_url: &str, secret: Option<&str>) -> Config {
        let mut config = Config::from_env();
        config.openemr_url = openemr_url.to_string();
        config.eir_gateway_secret = secret.and_then(GatewaySecret::new);
        config
    }

    fn header_values(request: &reqwest::Request, name: &str) -> Vec<String> {
        request
            .headers()
            .get_all(name)
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn test_trust_headers_case_insensitive() {
        assert!(is_trust_header("X-Eir-Gateway-Secret"));
        assert!(is_trust_header("x-eir-gateway-secret"));
        assert!(is_trust_header("X-Gateway"));
        assert!(!is_trust_header("X-Gateway-Route"));
        assert!(!is_trust_header("Authorization"));
    }

    #[test]
    fn test_identity_with_secret() {
        let config = test_config("http://openemr.test", Some(TEST_SECRET));
        let request = with_gateway_identity(
            reqwest::Client::new().get("http://openemr.test/apis/default/fhir/Patient"),
            &config,
        )
        .build()
        .unwrap();
        assert_eq!(
            header_values(&request, GATEWAY_SECRET_HEADER),
            vec![TEST_SECRET]
        );
        assert_eq!(
            header_values(&request, GATEWAY_MARKER_HEADER),
            vec!["eir-gateway"]
        );
    }

    #[test]
    fn test_identity_replaces_existing_copies() {
        let config = test_config("http://openemr.test", Some(TEST_SECRET));
        let builder = reqwest::Client::new()
            .get("http://openemr.test/apis/default/fhir/Patient")
            .header(GATEWAY_SECRET_HEADER, "client-forged")
            .header("X-Gateway", "eir-gateway");
        let request = with_gateway_identity(builder, &config).build().unwrap();
        assert_eq!(
            header_values(&request, GATEWAY_SECRET_HEADER),
            vec![TEST_SECRET]
        );
        assert_eq!(
            header_values(&request, GATEWAY_MARKER_HEADER),
            vec!["eir-gateway"]
        );
    }

    #[test]
    fn test_secret_header_is_sensitive() {
        let config = test_config("http://openemr.test", Some(TEST_SECRET));
        let headers = gateway_identity_headers(&config);
        assert!(headers.get(GATEWAY_SECRET_HEADER).unwrap().is_sensitive());
        assert!(!format!("{headers:?}").contains(TEST_SECRET));
    }

    #[test]
    fn test_identity_without_secret_sends_no_secret_header() {
        let config = test_config("http://openemr.test", None);
        let request = with_gateway_identity(
            reqwest::Client::new().get("http://openemr.test/apis/default/fhir/Patient"),
            &config,
        )
        .build()
        .unwrap();
        assert!(request.headers().get(GATEWAY_SECRET_HEADER).is_none());
    }
}

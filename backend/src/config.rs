//! Loads and validates the server and content-delivery configuration.

use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use axum::http::HeaderValue;
use serde::Deserialize;
use url::Url;

/// A failure to load or deserialize application settings.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub(crate) struct ConfigLoadError(#[from] ::config::ConfigError);

/// A failure to validate the configured browser origin.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CorsOriginError {
    /// The configured value cannot be interpreted as a URL.
    #[error("invalid CORS origin URL: {0}")]
    UrlParse(#[from] url::ParseError),
    /// The URL cannot represent one HTTP(S) browser origin.
    #[error("CORS origin must be a single HTTP(S) origin")]
    InvalidOrigin,
    /// The origin cannot be represented in an HTTP header.
    #[error("invalid CORS origin header: {0}")]
    HeaderValue(#[from] axum::http::header::InvalidHeaderValue),
}

/// Server settings loaded from the TONHALLE environment namespace.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TonhalleConfig {
    #[serde(default = "default_address")]
    listen_address: IpAddr,
    #[serde(default = "default_port")]
    listen_port: u16,
    cors_origin: Option<String>,
    content_storage: ContentStorageConfig,
}

/// Settings for reading audio files and linking to their delivery endpoint.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentStorageConfig {
    root_path: PathBuf,
    playback_url_base: Url,
}

/// Binds all IPv4 interfaces when no listener address is configured.
fn default_address() -> IpAddr {
    IpAddr::from([0, 0, 0, 0])
}

/// Uses the development API port when no listener port is configured.
fn default_port() -> u16 {
    3000
}

impl TonhalleConfig {
    /// Exposes the settings used to construct music storage.
    pub(crate) fn content_storage(&self) -> &ContentStorageConfig {
        &self.content_storage
    }

    /// Reads TONHALLE environment variables using double underscores for nested settings.
    pub(crate) fn load() -> Result<Self, ConfigLoadError> {
        Ok(::config::Config::builder()
            .add_source(
                ::config::Environment::with_prefix("TONHALLE")
                    .separator("__")
                    .prefix_separator("_"),
            )
            .build()?
            .try_deserialize()?)
    }

    /// Combines the configured IP and port for listener binding.
    pub(crate) fn listen_address(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.listen_port)
    }

    /// Validates an optional HTTP(S) origin for cross-origin API requests.
    pub(crate) fn cors_origin(&self) -> Result<Option<HeaderValue>, CorsOriginError> {
        self.cors_origin
            .as_ref()
            .map(|value| {
                let url = Url::parse(value)?;

                if !matches!(url.scheme(), "http" | "https")
                    || url.host_str().is_none()
                    || url.origin().ascii_serialization() != *value
                {
                    return Err(CorsOriginError::InvalidOrigin);
                }

                Ok(value.parse()?)
            })
            .transpose()
    }
}

impl ContentStorageConfig {
    /// Locates the directory scanned during startup indexing.
    pub(crate) fn root_path(&self) -> &std::path::Path {
        &self.root_path
    }

    /// Provides the delivery endpoint used to construct playback links.
    pub(crate) fn playback_url_base(&self) -> &Url {
        &self.playback_url_base
    }
}

/// Verifies configuration defaults and input validation.
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tonhalle_config_defaults_the_listener_and_rejects_invalid_inputs() {
        let mut value = json!({"content_storage": {"root_path": "../sample", "playback_url_base": "http://localhost:8000/"}});
        let config: TonhalleConfig = serde_json::from_value(value.clone()).unwrap();

        assert_eq!(config.listen_address().to_string(), "0.0.0.0:3000");
        assert_eq!(config.cors_origin().unwrap(), None);

        value["listen_address"] = json!("::1");
        value["cors_origin"] = json!("http://localhost:5173");
        let config: TonhalleConfig = serde_json::from_value(value.clone()).unwrap();

        assert_eq!(config.listen_address().to_string(), "[::1]:3000");
        assert!(config.cors_origin().unwrap().is_some());

        for origin in [
            "*",
            "https://example.com/path",
            "https://example.com/",
            "null",
        ] {
            value["cors_origin"] = json!(origin);

            assert!(
                serde_json::from_value::<TonhalleConfig>(value.clone())
                    .unwrap()
                    .cors_origin()
                    .is_err()
            );
        }

        value["listen_address"] = json!("invalid");

        assert!(serde_json::from_value::<TonhalleConfig>(value).is_err());
    }
}

//! Loads and validates the server and content-delivery configuration.

use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
};

use axum::http::HeaderValue;
use clap::Args;
use url::Url;

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

/// Server settings parsed from CLI arguments or the TONHALLE environment namespace.
#[derive(Debug, Args)]
pub(crate) struct ServerConfig {
    /// IP interface used by the HTTP listener.
    #[arg(
        long = "listen-address",
        env = "TONHALLE_LISTEN_ADDRESS",
        default_value = "0.0.0.0"
    )]
    listen_address: IpAddr,
    /// TCP port used by the HTTP listener.
    #[arg(
        long = "listen-port",
        env = "TONHALLE_LISTEN_PORT",
        default_value_t = 3000
    )]
    listen_port: u16,
    /// Optional browser origin permitted to read API responses.
    #[arg(long = "cors-origin", env = "TONHALLE_CORS_ORIGIN")]
    cors_origin: Option<String>,
    /// Audio storage and playback delivery settings.
    #[command(flatten)]
    content_storage: ContentStorageConfig,
}

/// Settings for reading audio files and linking to their delivery endpoint.
#[derive(Debug, Args)]
pub(crate) struct ContentStorageConfig {
    /// Directory scanned for stored audio releases.
    #[arg(
        id = "content_storage_root_path",
        long = "content-storage--root-path",
        env = "TONHALLE_CONTENT_STORAGE__ROOT_PATH"
    )]
    root_path: PathBuf,
    /// Delivery endpoint used to build playback links.
    #[arg(
        id = "content_storage_playback_url_base",
        long = "content-storage--playback-url-base",
        env = "TONHALLE_CONTENT_STORAGE__PLAYBACK_URL_BASE"
    )]
    playback_url_base: Url,
}

impl ServerConfig {
    /// Exposes the settings used to construct music storage.
    pub(crate) fn content_storage(&self) -> &ContentStorageConfig {
        &self.content_storage
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

/// Verifies the application-specific browser-origin constraints.
#[cfg(test)]
mod tests {
    use super::*;

    /// Constructs settings without invoking command-line or environment parsing.
    fn config(origin: Option<&str>) -> ServerConfig {
        ServerConfig {
            listen_address: IpAddr::from([127, 0, 0, 1]),
            listen_port: 3000,
            cors_origin: origin.map(str::to_owned),
            content_storage: ContentStorageConfig {
                root_path: PathBuf::from("../sample"),
                playback_url_base: Url::parse("http://localhost:8000/").unwrap(),
            },
        }
    }

    #[test]
    fn cors_origin_accepts_absent_or_single_http_origins() {
        assert_eq!(config(None).cors_origin().unwrap(), None);
        for origin in ["http://localhost:5173", "https://example.com"] {
            assert_eq!(
                config(Some(origin)).cors_origin().unwrap(),
                Some(HeaderValue::from_str(origin).unwrap())
            );
        }
    }

    #[test]
    fn cors_origin_rejects_values_that_are_not_single_http_origins() {
        for origin in [
            "*",
            "https://example.com/path",
            "https://example.com/",
            "https://example.com?query=value",
            "https://example.com#fragment",
            "https://user@example.com",
            "ftp://example.com",
            "null",
        ] {
            assert!(config(Some(origin)).cors_origin().is_err(), "{origin}");
        }
    }
}

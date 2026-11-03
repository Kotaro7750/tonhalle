//! Defines HTTP server settings and browser-origin validation.

use axum::http::{HeaderValue, header::InvalidHeaderValue};
use clap::Args;
use std::net::{IpAddr, SocketAddr};
use url::{ParseError as UrlParseError, Url};

use crate::config::music_storage::{MusicStorageConfig, MusicStorageConfigError};

/// A failure to validate the configured browser origin.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CorsOriginError {
    /// The configured value cannot be interpreted as a URL.
    #[error("invalid CORS origin URL: {0}")]
    UrlParse(#[from] UrlParseError),
    /// The URL cannot represent one HTTP(S) browser origin.
    #[error("CORS origin must be a single HTTP(S) origin")]
    InvalidOrigin,
    /// The origin cannot be represented in an HTTP header.
    #[error("invalid CORS origin header: {0}")]
    HeaderValue(#[from] InvalidHeaderValue),
}

/// Server settings parsed from CLI arguments or the TONHALLE environment namespace.
#[derive(getset::Getters, Debug, Args)]
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
    #[getset(get = "pub(crate)")]
    music_storage: MusicStorageConfig,
}

/// A failure to validate settings needed by the HTTP server.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerConfigError {
    /// Browser-origin configuration is invalid.
    #[error(transparent)]
    Cors(#[from] CorsOriginError),
    /// The storage delivery configuration is missing or invalid.
    #[error(transparent)]
    Storage(#[from] MusicStorageConfigError),
    /// The selected storage cannot provide server playback.
    #[error(transparent)]
    Compatibility(#[from] ServerCompatibilityError),
}

impl ServerConfig {
    /// Validates command-specific requirements before initializing server resources.
    pub(crate) fn validate(&self) -> Result<(), ServerConfigError> {
        self.cors_origin()?;
        self.music_storage.validate()?;
        self.music_storage
            .playback_url_base()
            .as_ref()
            .ok_or(ServerCompatibilityError::MissingPlaybackUrlBase)?;
        Ok(())
    }

    /// Combines the configured IP and port for listener binding.
    pub(crate) fn listen_address(&self) -> SocketAddr {
        SocketAddr::new(self.listen_address, self.listen_port)
    }

    /// Validates an optional HTTP(S) origin for cross-origin API requests.
    pub(crate) fn cors_origin(&self) -> Result<Option<HeaderValue>, CorsOriginError> {
        self.cors_origin
            .as_deref()
            .map(Self::parse_cors_origin)
            .transpose()
    }

    /// Converts a single HTTP(S) browser origin into a response header value.
    fn parse_cors_origin(value: &str) -> Result<HeaderValue, CorsOriginError> {
        let url = Url::parse(value)?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || url.origin().ascii_serialization() != value
        {
            return Err(CorsOriginError::InvalidOrigin);
        }
        Ok(value.parse()?)
    }
}

/// A failure to satisfy shared storage requirements for server playback.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerCompatibilityError {
    /// Server playback requires a delivery endpoint.
    #[error("MusicStorageConfig.playback_url_base is required for server playback")]
    MissingPlaybackUrlBase,
}

/// Verifies the application-specific browser-origin constraints.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::music_storage::{MusicStorageMode, sample_config};

    #[test]
    fn parse_cors_origin_accepts_single_http_origins() {
        for origin in ["http://localhost:5173", "https://example.com"] {
            assert_eq!(
                ServerConfig::parse_cors_origin(origin).unwrap(),
                HeaderValue::from_str(origin).unwrap()
            );
        }
    }

    #[test]
    fn parse_cors_origin_rejects_values_that_are_not_single_http_origins() {
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
            assert!(ServerConfig::parse_cors_origin(origin).is_err(), "{origin}");
        }
    }
    #[test]
    fn server_config_validate_requires_playback_url_for_all_storage_modes() {
        for mode in [MusicStorageMode::Filesystem, MusicStorageMode::S3] {
            for url in [None, Some("https://example.com/audio")] {
                let config = ServerConfig {
                    listen_address: "127.0.0.1".parse().unwrap(),
                    listen_port: 3000,
                    cors_origin: None,
                    music_storage: sample_config(mode, url.map(|url| url.parse().unwrap())),
                };
                let result = config.validate();
                if url.is_none() {
                    assert!(
                        matches!(
                            result,
                            Err(ServerConfigError::Compatibility(
                                ServerCompatibilityError::MissingPlaybackUrlBase
                            ))
                        ),
                        "{mode:?}"
                    );
                } else {
                    assert!(result.is_ok(), "{mode:?}");
                }
            }
        }
    }
}

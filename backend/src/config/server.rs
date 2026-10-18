//! Defines HTTP server settings and browser-origin validation.

use std::net::{IpAddr, SocketAddr};

use axum::http::HeaderValue;
use clap::Args;
use url::Url;

use super::content_storage::ContentStorageConfig;

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
    content_storage: ContentStorageConfig,
}

/// A failure to validate settings needed by the HTTP server.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerConfigError {
    /// Browser-origin configuration is invalid.
    #[error(transparent)]
    Cors(#[from] CorsOriginError),
    /// The storage delivery configuration is missing or invalid.
    #[error(transparent)]
    Storage(
        #[from]
        crate::infrastructure::music_storage::filesystem::FileSystemStorageConstructionError,
    ),
}

impl ServerConfig {
    /// Validates command-specific requirements before initializing server resources.
    pub(crate) fn validate(&self) -> Result<(), ServerConfigError> {
        self.cors_origin()?;
        self.content_storage.validate_playback_url()?;
        Ok(())
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

/// Verifies the application-specific browser-origin constraints.
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Constructs server settings with a valid playback endpoint.
    fn config(origin: Option<&str>) -> ServerConfig {
        config_with_playback_url(origin, Some("http://localhost:8000/"))
    }

    /// Parses adapter settings without accessing their private fields.
    fn config_with_playback_url(origin: Option<&str>, url: Option<&str>) -> ServerConfig {
        ServerConfig {
            listen_address: IpAddr::from([127, 0, 0, 1]),
            listen_port: 3000,
            cors_origin: origin.map(str::to_owned),
            content_storage: {
                /// Parses shared storage arguments for configuration tests.
                #[derive(Parser)]
                struct Settings {
                    /// Holds the adapter settings under validation.
                    #[command(flatten)]
                    storage: ContentStorageConfig,
                }
                let mut args = vec!["test", "--content-storage--root-path", "../sample"];
                if let Some(url) = url {
                    args.extend(["--content-storage--playback-url-base", url]);
                }
                Settings::try_parse_from(args).unwrap().storage
            },
        }
    }

    #[test]
    fn server_config_validate_requires_a_valid_storage_playback_url() {
        let settings = config(None);
        assert!(settings.validate().is_ok());

        let settings = config_with_playback_url(None, None);
        assert!(matches!(settings.validate(), Err(ServerConfigError::Storage(
            crate::infrastructure::music_storage::filesystem::FileSystemStorageConstructionError::MissingPlaybackUrlBase
        ))));

        for url in [
            "ftp://example.com/",
            "https://example.com/?token=value",
            "https://example.com/#fragment",
        ] {
            let settings = config_with_playback_url(None, Some(url));
            assert!(matches!(settings.validate(), Err(ServerConfigError::Storage(
                crate::infrastructure::music_storage::filesystem::FileSystemStorageConstructionError::InvalidPlaybackUrlBase
            ))));
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

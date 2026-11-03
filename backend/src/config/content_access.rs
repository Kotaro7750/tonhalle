//! Selects public delivery or CloudFront signed-cookie access.

use clap::{Args, ValueEnum};
use rsa::{RsaPrivateKey, pkcs8::DecodePrivateKey};
use std::sync::Arc;
use url::Url;

use crate::{
    infrastructure::content_access::{
        CloudFrontContentAccessConfigurator, NopContentAccessConfigurator,
    },
    port::content_access::ContentAccessConfigurator,
};

/// Invalid delivery-access configuration.
#[derive(Debug, thiserror::Error)]
#[error("invalid content access configuration: {0}")]
pub(crate) struct ContentAccessConfigError(String);

/// Delivery-access provider selected independently of music storage.
#[derive(Debug, Default, Clone, Copy, ValueEnum)]
enum ContentAccessMode {
    /// Leaves public delivery unchanged.
    #[default]
    Nop,
    /// Issues CloudFront signed cookies.
    CloudFront,
}

/// Optional delivery-access settings for the API server.
#[derive(Debug, Default, Args)]
pub(crate) struct ContentAccessConfig {
    /// Delivery-access implementation.
    #[arg(
        long = "content-access--mode",
        env = "TONHALLE_CONTENT_ACCESS__MODE",
        value_enum,
        default_value = "nop"
    )]
    mode: ContentAccessMode,
    /// CloudFront settings required only when that provider is selected.
    #[command(flatten)]
    cloud_front: Box<CloudFrontConfig>,
}

/// CloudFront policy, signing key, and cookie scope.
#[derive(Default, Args)]
struct CloudFrontConfig {
    /// CloudFront public-key identifier.
    #[arg(
        long = "content-access--cloud-front--key-pair-id",
        env = "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__KEY_PAIR_ID"
    )]
    key_pair_id: Option<String>,
    /// PKCS#8 PEM private key, excluded from debug output.
    #[arg(
        long = "content-access--cloud-front--private-key",
        env = "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__PRIVATE_KEY",
        hide_env_values = true
    )]
    private_key: Option<String>,
    /// Resource pattern covered by the signed policy.
    #[arg(
        long = "content-access--cloud-front--resource-url",
        env = "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__RESOURCE_URL"
    )]
    resource_url: Option<String>,
    /// Lifetime of each signed policy in seconds.
    #[arg(
        long = "content-access--cloud-front--validity-seconds",
        env = "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__VALIDITY_SECONDS"
    )]
    validity_seconds: Option<u64>,
    /// Optional shared parent domain for API and delivery hosts.
    #[arg(
        long = "content-access--cloud-front--cookie-domain",
        env = "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__COOKIE_DOMAIN"
    )]
    cookie_domain: Option<String>,
    /// Cookie scope on the delivery host.
    #[arg(
        long = "content-access--cloud-front--cookie-path",
        env = "TONHALLE_CONTENT_ACCESS__CLOUD_FRONT__COOKIE_PATH",
        default_value = "/"
    )]
    cookie_path: String,
}

impl std::fmt::Debug for CloudFrontConfig {
    /// Redacts credentials from diagnostic formatting.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CloudFrontConfig")
            .finish_non_exhaustive()
    }
}

impl ContentAccessConfig {
    /// Validates only the selected provider, including its signing key.
    pub(crate) fn validate(&self) -> Result<(), ContentAccessConfigError> {
        self.construct().map(|_| ())
    }

    /// Constructs delivery access without coupling it to the music-storage provider.
    pub(crate) fn construct(
        &self,
    ) -> Result<Arc<dyn ContentAccessConfigurator>, ContentAccessConfigError> {
        match self.mode {
            ContentAccessMode::Nop => Ok(Arc::new(NopContentAccessConfigurator)),
            ContentAccessMode::CloudFront => self.cloud_front.construct(),
        }
    }
}

impl CloudFrontConfig {
    /// Requires a nonempty setting when CloudFront access is enabled.
    fn required<'a>(
        value: &'a Option<String>,
        name: &str,
    ) -> Result<&'a str, ContentAccessConfigError> {
        value
            .as_deref()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ContentAccessConfigError(format!("{name} is required for CloudFront mode"))
            })
    }

    /// Checks application configuration constraints without parsing the signing key.
    fn validate_settings(&self) -> Result<(), ContentAccessConfigError> {
        let key_pair_id = Self::required(&self.key_pair_id, "key_pair_id")?;
        Self::required(&self.private_key, "private_key")?;
        let resource_url = Self::required(&self.resource_url, "resource_url")?;
        self.validity_seconds
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                ContentAccessConfigError("validity_seconds must be greater than zero".into())
            })?;
        Url::parse(resource_url)
            .map_err(|_| ContentAccessConfigError("invalid resource_url".into()))?;
        if !self.cookie_path.starts_with('/')
            || self.cookie_path.chars().any(|c| c.is_control() || c == ';')
        {
            return Err(ContentAccessConfigError(
                "cookie_path must start with '/' and contain no control characters or semicolons"
                    .into(),
            ));
        }
        if self.cookie_domain.as_ref().is_some_and(|domain| {
            domain.is_empty()
                || domain
                    .chars()
                    .any(|c| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
        }) {
            return Err(ContentAccessConfigError("invalid cookie_domain".into()));
        }
        if key_pair_id
            .chars()
            .any(|c| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        {
            return Err(ContentAccessConfigError("invalid key_pair_id".into()));
        }
        Ok(())
    }

    /// Constructs the provider using validated settings and the external key parser.
    fn construct(&self) -> Result<Arc<dyn ContentAccessConfigurator>, ContentAccessConfigError> {
        self.validate_settings()?;
        let key_pair_id = Self::required(&self.key_pair_id, "key_pair_id")?;
        let private_key = Self::required(&self.private_key, "private_key")?;
        let resource_url = Self::required(&self.resource_url, "resource_url")?;
        let validity_seconds = self.validity_seconds.expect("validated policy lifetime");
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key)
            .map_err(|_| ContentAccessConfigError("invalid PKCS#8 private key".into()))?;
        Ok(Arc::new(CloudFrontContentAccessConfigurator::new(
            key_pair_id.into(),
            private_key,
            resource_url.into(),
            validity_seconds,
            self.cookie_domain.clone(),
            self.cookie_path.clone(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_access_validates_only_selected_provider() {
        assert!(ContentAccessConfig::default().validate().is_ok());
        let mut config = ContentAccessConfig {
            mode: ContentAccessMode::CloudFront,
            ..Default::default()
        };
        assert!(config.validate().is_err());
        *config.cloud_front = CloudFrontConfig {
            key_pair_id: Some("test-key".into()),
            private_key: Some("opaque-private-key".into()),
            resource_url: Some("https://cdn.example.com/*".into()),
            validity_seconds: Some(300),
            cookie_domain: Some("example.com".into()),
            cookie_path: "/".into(),
        };
        assert!(config.cloud_front.validate_settings().is_ok());
        config.cloud_front.validity_seconds = Some(0);
        assert!(config.cloud_front.validate_settings().is_err());
        config.cloud_front.validity_seconds = Some(300);
        config.cloud_front.private_key = None;
        assert!(config.cloud_front.validate_settings().is_err());
        config.cloud_front.private_key = Some("opaque-private-key".into());
        assert!(!format!("{config:?}").contains("opaque-private-key"));
    }
}

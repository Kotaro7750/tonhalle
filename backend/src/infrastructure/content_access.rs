//! Issues CloudFront signed cookies or leaves public delivery unchanged.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::STANDARD};
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use sha1::{Digest, Sha1};

use crate::{
    domain::content_access::{ContentAccessConfiguration, ContentAccessCookie},
    port::content_access::{ContentAccessConfigurator, ContentAccessError},
};

/// CloudFront custom-policy cookie.
const POLICY_COOKIE_NAME: &str = "CloudFront-Policy";
/// CloudFront policy signature cookie.
const SIGNATURE_COOKIE_NAME: &str = "CloudFront-Signature";
/// CloudFront signing-key identifier cookie.
const KEY_PAIR_ID_COOKIE_NAME: &str = "CloudFront-Key-Pair-Id";
/// Credential names removed when clearing access.
const COOKIE_NAMES: [&str; 3] = [
    POLICY_COOKIE_NAME,
    SIGNATURE_COOKIE_NAME,
    KEY_PAIR_ID_COOKIE_NAME,
];

/// A no-op implementation of `ContentAccessConfigurator` that does not set any cookies.
pub(crate) struct NopContentAccessConfigurator;

impl ContentAccessConfigurator for NopContentAccessConfigurator {
    fn configure(&self) -> Result<ContentAccessConfiguration, ContentAccessError> {
        Ok(ContentAccessConfiguration::new(Vec::new(), None))
    }

    fn clear(&self) -> Result<ContentAccessConfiguration, ContentAccessError> {
        Ok(ContentAccessConfiguration::new(Vec::new(), None))
    }
}

/// A cloudfront implementation of `ContentAccessConfigurator` that generates signed cookies for accessing content.
pub(crate) struct CloudFrontContentAccessConfigurator {
    /// Identifier of the CloudFront public key.
    key_pair_id: String,
    /// Private key used to sign policies.
    private_key: RsaPrivateKey,
    /// Resource pattern covered by the policy.
    resource_url: String,
    /// Policy lifetime in seconds.
    validity_seconds: u64,
    /// Optional domain shared with the delivery endpoint.
    cookie_domain: Option<String>,
    /// Delivery path covered by the cookies.
    cookie_path: String,
}

impl CloudFrontContentAccessConfigurator {
    /// Constructs a signer for the configured resource and cookie scope.
    pub(crate) fn new(
        key_pair_id: String,
        private_key: RsaPrivateKey,
        resource_url: String,
        validity_seconds: u64,
        cookie_domain: Option<String>,
        cookie_path: String,
    ) -> Self {
        Self {
            key_pair_id,
            private_key,
            resource_url,
            validity_seconds,
            cookie_domain,
            cookie_path,
        }
    }
}

impl ContentAccessConfigurator for CloudFrontContentAccessConfigurator {
    fn configure(&self) -> Result<ContentAccessConfiguration, ContentAccessError> {
        let (policy, invalid_after) =
            build_policy(&self.resource_url, self.validity_seconds, SystemTime::now())?;
        let signature = self
            .private_key
            .sign(
                Pkcs1v15Sign::new::<Sha1>(),
                &Sha1::digest(policy.as_bytes()),
            )
            .map_err(|e| {
                ContentAccessError::new(format!("failed to sign content access policy: {e}"))
            })?;
        let encode = |value: &[u8]| {
            STANDARD
                .encode(value)
                .replace('+', "-")
                .replace('=', "_")
                .replace('/', "~")
        };
        let values = [
            (POLICY_COOKIE_NAME, encode(policy.as_bytes())),
            (SIGNATURE_COOKIE_NAME, encode(&signature)),
            (KEY_PAIR_ID_COOKIE_NAME, self.key_pair_id.clone()),
        ];

        let cookies = values
            .into_iter()
            .map(|(name, value)| {
                ContentAccessCookie::new(
                    name.to_string(),
                    value,
                    self.cookie_domain.clone(),
                    self.cookie_path.clone(),
                )
            })
            .collect();

        Ok(ContentAccessConfiguration::new(
            cookies,
            Some(invalid_after),
        ))
    }

    fn clear(&self) -> Result<ContentAccessConfiguration, ContentAccessError> {
        let cookies = COOKIE_NAMES
            .into_iter()
            .map(|name| {
                ContentAccessCookie::new(
                    name.to_string(),
                    String::new(),
                    self.cookie_domain.clone(),
                    self.cookie_path.clone(),
                )
            })
            .collect();

        Ok(ContentAccessConfiguration::new(cookies, None))
    }
}

/// Builds the resource policy and renewal deadline without signing credentials.
fn build_policy(
    resource_url: &str,
    validity_seconds: u64,
    now: SystemTime,
) -> Result<(String, SystemTime), ContentAccessError> {
    let now = now.duration_since(UNIX_EPOCH).map_err(|error| {
        ContentAccessError::new(format!("system clock is before Unix epoch: {error}"))
    })?;
    let expires_at = now
        .as_secs()
        .checked_add(validity_seconds)
        .ok_or_else(|| ContentAccessError::new("content access expiration overflow"))?;
    let invalid_after = UNIX_EPOCH
        .checked_add(Duration::from_secs(expires_at))
        .ok_or_else(|| ContentAccessError::new("content access expiration overflow"))?;
    let policy = serde_json::json!({"Statement": [{
        "Resource": resource_url,
        "Condition": {"DateLessThan": {"AWS:EpochTime": expires_at}}
    }]})
    .to_string();
    Ok((policy, invalid_after))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_configurator_does_not_set_cookies() {
        let configuration = NopContentAccessConfigurator.configure().unwrap();
        assert_eq!(configuration.invalid_after(), None);
        assert!(configuration.cookies().is_empty());
        let cleared_configuration = NopContentAccessConfigurator.clear().unwrap();
        assert_eq!(cleared_configuration.invalid_after(), None);
        assert!(cleared_configuration.cookies().is_empty());
    }

    #[test]
    fn build_policy_scopes_resource_and_matches_the_renewal_deadline() {
        let now = UNIX_EPOCH + Duration::from_secs(1_000);
        let (policy, invalid_after) =
            build_policy("https://cdn.example.com/music/*", 300, now).unwrap();
        let policy: serde_json::Value = serde_json::from_str(&policy).unwrap();
        assert_eq!(
            policy,
            serde_json::json!({"Statement": [{
                "Resource": "https://cdn.example.com/music/*",
                "Condition": {"DateLessThan": {"AWS:EpochTime": 1_300}}
            }]})
        );
        assert_eq!(invalid_after, UNIX_EPOCH + Duration::from_secs(1_300));
    }

    #[test]
    fn build_policy_rejects_invalid_time_and_expiration_overflow() {
        assert!(
            build_policy(
                "https://cdn.example.com/*",
                300,
                UNIX_EPOCH - Duration::from_secs(1)
            )
            .is_err()
        );
        assert!(
            build_policy(
                "https://cdn.example.com/*",
                u64::MAX,
                UNIX_EPOCH + Duration::from_secs(1)
            )
            .is_err()
        );
    }
}

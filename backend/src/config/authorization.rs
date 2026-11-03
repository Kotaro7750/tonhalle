//! Configures OAuth 2.0 JWT validation using provider signing keys.

use aliri::{jwa, jwt};
use aliri_clock::UnixTime;
use aliri_oauth2::{Authority, HasScope, Scope, scope::BasicClaimsWithScope};
use aliri_tower::Oauth2Authorizer;
use axum::{Router, body::Body};
use clap::Args;
use getset::{CopyGetters, Getters};
use serde::Deserialize;
use std::time::Duration;

/// A failure to configure or initialize access-token authorization.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthorizationError {
    /// An enabled authorizer is missing a required setting.
    #[error("authorization.{0} is required when authorization.enabled is true")]
    MissingSetting(&'static str),
    /// Provider signing keys could not be retrieved.
    #[error("retrieving JWKS: {0}")]
    Jwks(String),
}

/// Interval between provider signing-key refreshes.
const JWKS_REFRESH_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// OAuth 2.0 access-token authorization settings.
#[derive(Debug, Default, Args, CopyGetters, Getters)]
pub(crate) struct AuthorizationConfig {
    /// Enables Bearer access-token validation for API routes.
    #[arg(long = "authorization-enabled", env = "TONHALLE_AUTHORIZATION__ENABLED", default_value_t = false, action = clap::ArgAction::Set)]
    #[getset(get_copy = "pub(crate)")]
    enabled: bool,
    /// Required token issuer when authorization is enabled.
    #[arg(long = "authorization-issuer", env = "TONHALLE_AUTHORIZATION__ISSUER")]
    #[getset(get = "pub(crate)")]
    issuer: Option<String>,
    /// Required token audience when authorization is enabled.
    #[arg(
        long = "authorization-audience",
        env = "TONHALLE_AUTHORIZATION__AUDIENCE"
    )]
    #[getset(get = "pub(crate)")]
    audience: Option<String>,
    /// Provider endpoint supplying JWT signing keys.
    #[arg(
        long = "authorization-jwks-url",
        env = "TONHALLE_AUTHORIZATION__JWKS_URL"
    )]
    #[getset(get = "pub(crate)")]
    jwks_url: Option<String>,
}

impl AuthorizationConfig {
    /// Protects API routes and starts hourly provider signing-key refreshes when enabled.
    pub(crate) async fn protect_api(&self, router: Router) -> Result<Router, AuthorizationError> {
        self.validate()?;
        if !self.enabled() {
            return Ok(router);
        }

        let authority = Authority::new_from_url(
            self.jwks_url()
                .as_deref()
                .expect("validated authorization JWKS URL")
                .to_owned(),
            self.validator(),
        )
        .await
        .map_err(|error| AuthorizationError::Jwks(error.to_string()))?;
        authority.spawn_refresh(JWKS_REFRESH_INTERVAL);

        Ok(Self::protect_with_authority(router, authority))
    }

    /// Restricts tokens to the configured issuer, audience, and RS256 signatures.
    fn validator(&self) -> jwt::CoreValidator {
        jwt::CoreValidator::default()
            .add_approved_algorithm(jwa::Algorithm::RS256)
            .add_allowed_audience(jwt::Audience::from(
                self.audience()
                    .as_deref()
                    .expect("validated authorization audience"),
            ))
            .require_issuer(jwt::Issuer::from(
                self.issuer()
                    .as_deref()
                    .expect("validated authorization issuer"),
            ))
    }

    /// Applies the same token policy to every route in the API router.
    fn protect_with_authority(router: Router, authority: Authority) -> Router {
        let authorizer = Oauth2Authorizer::new()
            .with_claims::<AccessTokenClaims>()
            .with_terse_error_handler::<Body>();
        router.layer(authorizer.jwt_layer::<Body>(authority))
    }

    /// Requires provider settings only when access-token authorization is enabled.
    pub(crate) fn validate(&self) -> Result<(), AuthorizationError> {
        if !self.enabled() {
            return Ok(());
        }

        for (name, value) in [
            ("issuer", self.issuer().as_deref()),
            ("audience", self.audience().as_deref()),
            ("jwks_url", self.jwks_url().as_deref()),
        ] {
            if value.is_none_or(str::is_empty) {
                return Err(AuthorizationError::MissingSetting(name));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
/// Represents the claims in an OAuth 2.0 access token
pub(crate) struct AccessTokenClaims {
    #[serde(flatten)]
    /// Standard token claims and OAuth scopes.
    standard: BasicClaimsWithScope,
    #[serde(default, rename = "token_use")]
    /// Cognito-specific claim indicating the type of token. Only `access` is accepted.
    _token_use: Option<TokenUse>,
}

#[derive(Clone, Debug, Deserialize)]
/// Restricts Cognito tokens to the access-token purpose.
enum TokenUse {
    /// An access token rather than an identity token.
    #[serde(rename = "access")]
    Access,
}

impl jwt::CoreClaims for AccessTokenClaims {
    fn nbf(&self) -> Option<UnixTime> {
        jwt::CoreClaims::nbf(&self.standard)
    }

    fn exp(&self) -> Option<UnixTime> {
        jwt::CoreClaims::exp(&self.standard)
    }

    fn aud(&self) -> &jwt::Audiences {
        jwt::CoreClaims::aud(&self.standard)
    }

    fn iss(&self) -> Option<&jwt::IssuerRef> {
        jwt::CoreClaims::iss(&self.standard)
    }

    fn sub(&self) -> Option<&jwt::SubjectRef> {
        jwt::CoreClaims::sub(&self.standard)
    }
}

impl HasScope for AccessTokenClaims {
    fn scope(&self) -> &Scope {
        HasScope::scope(&self.standard)
    }
}

#[cfg(test)]
mod tests {
    use aliri::Jwks;
    use aliri_oauth2::Authority;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    use crate::cmd::server::build_app;

    use super::AuthorizationConfig;

    #[test]
    fn disabled_authorization_does_not_require_provider_settings() {
        assert!(AuthorizationConfig::default().validate().is_ok());
    }

    #[test]
    fn enabled_authorization_requires_all_provider_settings() {
        let config = AuthorizationConfig {
            enabled: true,
            issuer: Some("https://issuer.example".to_string()),
            audience: Some("https://api.tonhalle.example.com".to_string()),
            jwks_url: None,
        };

        assert!(config.validate().is_err());

        let config = AuthorizationConfig {
            enabled: true,
            issuer: Some("https://issuer.example".to_string()),
            audience: None,
            jwks_url: Some("https://issuer.example/jwks.json".to_string()),
        };

        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn protect_api_disabled_allows_requests_without_provider_settings() {
        let router = Router::new().route("/test", get(|| async { "OK" }));
        let app = AuthorizationConfig::default()
            .protect_api(router)
            .await
            .unwrap();
        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protect_api_requires_authentication_while_health_and_preflight_remain_public() {
        let config = AuthorizationConfig {
            enabled: true,
            issuer: Some("https://issuer.example".into()),
            audience: Some("https://api.tonhalle.example.com".into()),
            jwks_url: Some("https://issuer.example/jwks.json".into()),
        };
        let authority = Authority::new(Jwks::default(), config.validator());
        let api = Router::new().route("/test", get(|| async { "OK" }));
        let app = build_app(
            AuthorizationConfig::protect_with_authority(api, authority),
            Some("https://app.example".parse().unwrap()),
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v0/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/v0/test")
                    .header("origin", "https://app.example")
                    .header("access-control-request-method", "GET")
                    .header("access-control-request-headers", "authorization")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["access-control-allow-origin"],
            "https://app.example"
        );
        assert!(
            response.headers()["access-control-allow-headers"]
                .to_str()
                .unwrap()
                .contains("authorization")
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

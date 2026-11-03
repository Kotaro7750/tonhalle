//! Exposes delivery-access configuration through authenticated HTTP routes.

use axum::{
    Router,
    extract::State,
    http::{
        StatusCode,
        header::{CACHE_CONTROL, SET_COOKIE},
    },
    response::{IntoResponse, Response},
    routing::post,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use tonhalle_api::models::ContentAccessConfiguration as ApiContentAccessConfiguration;

use crate::{
    controller::error::ControllerError,
    domain::content_access::{ContentAccessConfiguration, ContentAccessCookie},
    port::content_access::ContentAccessError,
    usecase::content_access::ContentAccessUseCase,
};

/// Configures opaque delivery credentials on the client.
#[derive(Clone)]
pub(crate) struct ContentAccessController {
    /// Delivery-access application workflow.
    access: ContentAccessUseCase,
}

impl ContentAccessController {
    /// Logs provider failures without exposing signing details to clients.
    fn access_error(error: ContentAccessError) -> ControllerError {
        tracing::error!(%error, "Configuring content access failed");
        ControllerError::Internal
    }

    /// Connects delivery-access routes to their application workflow.
    pub(crate) fn new(access: ContentAccessUseCase) -> Self {
        Self { access }
    }

    /// Exposes issue and clear operations at the contract's exact path.
    pub(crate) fn router(self) -> Router {
        Router::new()
            .route("/content-access", post(Self::configure).delete(Self::clear))
            .with_state(self)
    }

    /// Issues delivery credentials and returns their renewal deadline.
    async fn configure(State(this): State<Self>) -> Result<Response, ControllerError> {
        let configuration = this.access.configure().map_err(Self::access_error)?;
        Self::configuration_response(configuration)
    }

    /// Removes delivery credentials using their original cookie scope.
    async fn clear(State(this): State<Self>) -> Result<Response, ControllerError> {
        let configuration = this.access.clear().map_err(Self::access_error)?;
        Self::cookie_response(
            configuration.cookies(),
            true,
            StatusCode::NO_CONTENT.into_response(),
        )
    }

    /// Converts provider-neutral credentials to the API contract.
    fn configuration_response(
        configuration: ContentAccessConfiguration,
    ) -> Result<Response, ControllerError> {
        let mut body = ApiContentAccessConfiguration::new();
        body.invalid_after = configuration.invalid_after().map(Into::into);
        Self::cookie_response(
            configuration.cookies(),
            false,
            axum::Json(body).into_response(),
        )
    }

    /// Appends secure cookies and prevents caching credential responses.
    fn cookie_response(
        cookies: &[ContentAccessCookie],
        removal: bool,
        mut response: Response,
    ) -> Result<Response, ControllerError> {
        for content_access_cookie in cookies {
            let mut cookie = Cookie::build((
                content_access_cookie.name().to_string(),
                content_access_cookie.value().to_string(),
            ))
            .path(content_access_cookie.path().to_string())
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Lax)
            .build();
            if let Some(domain) = content_access_cookie.domain().as_deref() {
                cookie.set_domain(domain.to_string());
            }
            if removal {
                cookie.make_removal();
            }

            response.headers_mut().append(
                SET_COOKIE,
                cookie.to_string().parse().map_err(|e| {
                    tracing::error!(error = %e, "Creating delivery cookie failed");
                    ControllerError::Internal
                })?,
            );
        }

        response
            .headers_mut()
            .insert(CACHE_CONTROL, "no-store".parse().expect("static header"));
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::SET_COOKIE},
    };
    use std::{
        sync::Arc,
        time::{Duration, UNIX_EPOCH},
    };
    use tower::ServiceExt;

    use super::ContentAccessController;
    use crate::{
        cmd::server::build_app,
        domain::content_access::{ContentAccessConfiguration, ContentAccessCookie},
        infrastructure::content_access::NopContentAccessConfigurator,
        port::content_access::{ContentAccessConfigurator, ContentAccessError},
        usecase::content_access::ContentAccessUseCase,
    };

    /// Supplies opaque credentials without invoking a delivery provider or cryptography.
    struct StubContentAccessConfigurator;

    impl ContentAccessConfigurator for StubContentAccessConfigurator {
        fn configure(&self) -> Result<ContentAccessConfiguration, ContentAccessError> {
            Ok(ContentAccessConfiguration::new(
                vec![ContentAccessCookie::new(
                    "delivery-credential".into(),
                    "opaque-value".into(),
                    Some("example.com".into()),
                    "/music".into(),
                )],
                Some(UNIX_EPOCH + Duration::from_secs(2_000_000_000)),
            ))
        }

        fn clear(&self) -> Result<ContentAccessConfiguration, ContentAccessError> {
            Ok(ContentAccessConfiguration::new(
                vec![ContentAccessCookie::new(
                    "delivery-credential".into(),
                    String::new(),
                    Some("example.com".into()),
                    "/music".into(),
                )],
                None,
            ))
        }
    }

    #[tokio::test]
    async fn content_access_issues_and_removes_secure_cookies_with_matching_scopes() {
        let app = build_app(
            ContentAccessController::new(ContentAccessUseCase::new(Arc::new(
                StubContentAccessConfigurator,
            )))
            .router(),
            Some("https://app.example.com".parse().unwrap()),
        );
        for method in ["POST", "DELETE"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/api/v0/content-access")
                        .header("origin", "https://app.example.com")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                if method == "POST" {
                    StatusCode::OK
                } else {
                    StatusCode::NO_CONTENT
                }
            );
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(
                response.headers()["access-control-allow-credentials"],
                "true"
            );
            let cookies: Vec<_> = response
                .headers()
                .get_all(SET_COOKIE)
                .iter()
                .map(|value| value.to_str().unwrap())
                .collect();
            assert_eq!(cookies.len(), 1);
            for cookie in cookies {
                assert!(cookie.starts_with("delivery-credential="));
                assert!(cookie.contains("Secure"));
                assert!(cookie.contains("HttpOnly"));
                assert!(cookie.contains("SameSite=Lax"));
                assert!(cookie.contains("Domain=example.com"));
                assert!(cookie.contains("Path=/music"));
                assert_eq!(cookie.contains("Max-Age=0"), method == "DELETE");
            }
            let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            if method == "POST" {
                let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                assert!(body["invalidAfter"].as_str().is_some());
                assert_eq!(body.as_object().unwrap().len(), 1);
                let mut contract: serde_json::Value =
                    serde_yaml_ng::from_str(include_str!("../../../contracts/api/openapi.yaml"))
                        .unwrap();
                contract["$ref"] =
                    serde_json::json!("#/components/schemas/ContentAccessConfiguration");
                let validator = jsonschema::draft202012::options()
                    .should_validate_formats(true)
                    .build(&contract)
                    .unwrap();
                assert!(validator.is_valid(&body));
            } else {
                assert!(bytes.is_empty());
            }
        }
        for method in ["POST", "DELETE"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("OPTIONS")
                        .uri("/api/v0/content-access")
                        .header("origin", "https://app.example.com")
                        .header("access-control-request-method", method)
                        .header("access-control-request-headers", "authorization")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert!(
                response.headers()["access-control-allow-methods"]
                    .to_str()
                    .unwrap()
                    .contains(method)
            );
        }
    }

    #[tokio::test]
    async fn content_access_nop_has_no_credentials_or_deadline() {
        let provider: Arc<dyn ContentAccessConfigurator> = Arc::new(NopContentAccessConfigurator);
        let app = build_app(
            ContentAccessController::new(ContentAccessUseCase::new(provider)).router(),
            None,
        );
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v0/content-access")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key(SET_COOKIE));
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            "{}"
        );
    }
}

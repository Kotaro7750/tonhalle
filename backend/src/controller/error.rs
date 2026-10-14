//! Maps application failures to HTTP problem responses.

use axum::{
    Json,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use tonhalle_api::models;

use crate::usecase::{GetReleaseError, GetTrackError, ListReleasesError};

/// An HTTP boundary failure rendered as an RFC 9457 problem.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ControllerError {
    /// The request failed boundary validation.
    #[error("bad request: {0}")]
    BadRequest(String),
    /// No resource matches the requested route or identifier.
    #[error("not found")]
    NotFound,
    /// The matched route does not support the requested HTTP method.
    #[error("method not allowed")]
    MethodNotAllowed,
    /// A logged application failure has no client-visible details.
    #[error("internal server error")]
    Internal,
}

impl From<ListReleasesError> for ControllerError {
    /// Maps invalid cursors to bad requests and logs internal listing failures.
    fn from(value: ListReleasesError) -> Self {
        match value {
            ListReleasesError::InvalidCursor => Self::BadRequest("invalid cursor".into()),
            ListReleasesError::Internal(error) => {
                tracing::error!(?error, "Listing releases failed");
                Self::Internal
            }
        }
    }
}

impl From<GetReleaseError> for ControllerError {
    /// Maps missing releases to not found and logs internal lookup failures.
    fn from(value: GetReleaseError) -> Self {
        match value {
            GetReleaseError::NotFound => Self::NotFound,
            GetReleaseError::Internal(error) => {
                tracing::error!(?error, "Getting release failed");
                Self::Internal
            }
        }
    }
}

impl From<GetTrackError> for ControllerError {
    /// Maps missing tracks to not found and logs internal lookup failures.
    fn from(value: GetTrackError) -> Self {
        match value {
            GetTrackError::NotFound => Self::NotFound,
            GetTrackError::Internal(error) => {
                tracing::error!(?error, "Getting track failed");
                Self::Internal
            }
        }
    }
}

impl IntoResponse for ControllerError {
    /// Renders a problem response without exposing internal error causes.
    fn into_response(self) -> Response {
        let (status, detail) = match self {
            Self::BadRequest(detail) => (StatusCode::BAD_REQUEST, Some(detail)),
            Self::NotFound => (StatusCode::NOT_FOUND, None),
            Self::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, None),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, None),
        };

        let problem = models::Problem {
            r_type: "about:blank".into(),
            title: status.canonical_reason().unwrap_or("Error").into(),
            status: i32::from(status.as_u16()),
            detail,
            instance: None,
        };

        (
            status,
            [(CONTENT_TYPE, "application/problem+json")],
            Json(problem),
        )
            .into_response()
    }
}

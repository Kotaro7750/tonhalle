use std::collections::HashMap;

use axum::{body::Body, extract::*, response::Response, routing::*};
use axum_extra::{
    TypedHeader,
    extract::{CookieJar, Query as QueryExtra},
};
use bytes::Bytes;
use headers::Host;
use http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, header::CONTENT_TYPE};
use tracing::error;
use validator::{Validate, ValidationErrors};

#[allow(unused_imports)]
use crate::{apis, models};
use crate::{header, types::*};
#[allow(unused_imports)]
use crate::{
    models::check_xss_map, models::check_xss_map_nested, models::check_xss_map_string,
    models::check_xss_string, models::check_xss_vec_string,
};

/// Setup API Server.
pub fn new<I, A, E>(api_impl: I) -> Router
where
    I: AsRef<A> + Clone + Send + Sync + 'static,
    A: apis::releases::Releases<E> + apis::tracks::Tracks<E> + Send + Sync + 'static,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    // build our application with a route
    Router::new()
        .route("/api/v0/releases", get(list_releases::<I, A, E>))
        .route("/api/v0/releases/{release_id}", get(get_release::<I, A, E>))
        .route("/api/v0/tracks/{track_id}", get(get_track::<I, A, E>))
        .with_state(api_impl)
}

#[tracing::instrument(skip_all)]
fn get_release_validation(
    path_params: models::GetReleasePathParams,
) -> std::result::Result<(models::GetReleasePathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetRelease - GET /api/v0/releases/{releaseId}
#[tracing::instrument(skip_all)]
async fn get_release<I, A, E>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    Path(path_params): Path<models::GetReleasePathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::releases::Releases<E> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_release_validation(path_params))
        .await
        .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .get_release(&method, &host, &cookies, &path_params)
        .await;

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::releases::GetReleaseResponse::Status200(body) => {
                let mut response = Response::builder();
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers
                        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::releases::GetReleaseResponse::Status400(body) => {
                let mut response = Response::builder();
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/problem+json"),
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::releases::GetReleaseResponse::Status404(body) => {
                let mut response = Response::builder();
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/problem+json"),
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::releases::GetReleaseResponse::Status500(body) => {
                let mut response = Response::builder();
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/problem+json"),
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl
                .as_ref()
                .handle_error(&method, &host, &cookies, why)
                .await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn list_releases_validation(
    query_params: models::ListReleasesQueryParams,
) -> std::result::Result<(models::ListReleasesQueryParams,), ValidationErrors> {
    query_params.validate()?;

    Ok((query_params,))
}
/// ListReleases - GET /api/v0/releases
#[tracing::instrument(skip_all)]
async fn list_releases<I, A, E>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    QueryExtra(query_params): QueryExtra<models::ListReleasesQueryParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::releases::Releases<E> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || list_releases_validation(query_params))
        .await
        .unwrap();

    let Ok((query_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .list_releases(&method, &host, &cookies, &query_params)
        .await;

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::releases::ListReleasesResponse::Status200(body) => {
                let mut response = Response::builder();
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers
                        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::releases::ListReleasesResponse::Status400(body) => {
                let mut response = Response::builder();
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/problem+json"),
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::releases::ListReleasesResponse::Status500(body) => {
                let mut response = Response::builder();
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/problem+json"),
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl
                .as_ref()
                .handle_error(&method, &host, &cookies, why)
                .await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[tracing::instrument(skip_all)]
fn get_track_validation(
    path_params: models::GetTrackPathParams,
) -> std::result::Result<(models::GetTrackPathParams,), ValidationErrors> {
    path_params.validate()?;

    Ok((path_params,))
}
/// GetTrack - GET /api/v0/tracks/{trackId}
#[tracing::instrument(skip_all)]
async fn get_track<I, A, E>(
    method: Method,
    TypedHeader(host): TypedHeader<Host>,
    cookies: CookieJar,
    Path(path_params): Path<models::GetTrackPathParams>,
    State(api_impl): State<I>,
) -> Result<Response, StatusCode>
where
    I: AsRef<A> + Send + Sync,
    A: apis::tracks::Tracks<E> + Send + Sync,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    #[allow(clippy::redundant_closure)]
    let validation = tokio::task::spawn_blocking(move || get_track_validation(path_params))
        .await
        .unwrap();

    let Ok((path_params,)) = validation else {
        return Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(validation.unwrap_err().to_string()))
            .map_err(|_| StatusCode::BAD_REQUEST);
    };

    let result = api_impl
        .as_ref()
        .get_track(&method, &host, &cookies, &path_params)
        .await;

    let resp = match result {
        Ok(rsp) => match rsp {
            apis::tracks::GetTrackResponse::Status200(body) => {
                let mut response = Response::builder();
                let mut response = response.status(200);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers
                        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tracks::GetTrackResponse::Status400(body) => {
                let mut response = Response::builder();
                let mut response = response.status(400);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/problem+json"),
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tracks::GetTrackResponse::Status404(body) => {
                let mut response = Response::builder();
                let mut response = response.status(404);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/problem+json"),
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
            apis::tracks::GetTrackResponse::Status500(body) => {
                let mut response = Response::builder();
                let mut response = response.status(500);
                {
                    let mut response_headers = response.headers_mut().unwrap();
                    response_headers.insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/problem+json"),
                    );
                }

                let body_content = tokio::task::spawn_blocking(move || {
                    serde_json::to_vec(&body).map_err(|e| {
                        error!(error = ?e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })
                })
                .await
                .unwrap()?;
                response.body(Body::from(body_content))
            }
        },
        Err(why) => {
            // Application code returned an error. This should not happen, as the implementation should
            // return a valid response.
            return api_impl
                .as_ref()
                .handle_error(&method, &host, &cookies, why)
                .await;
        }
    };

    resp.map_err(|e| {
        error!(error = ?e);
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

#[allow(dead_code)]
#[inline]
fn response_with_status_code_only(code: StatusCode) -> Result<Response, StatusCode> {
    Response::builder()
        .status(code)
        .body(Body::empty())
        .map_err(|_| code)
}

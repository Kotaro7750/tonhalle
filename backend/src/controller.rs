//! Validates HTTP requests and exposes the music browsing API.

use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    routing::get,
};
use tonhalle_api::models;

use crate::{
    domain::{ReleaseId, TrackId},
    usecase::{GetReleaseUseCase, GetTrackUseCase, ListReleasesUseCase},
};

mod domain_type_convert;
mod error;

use error::ControllerError;

/// Handles music browsing requests using application query workflows.
#[derive(Clone)]
pub(crate) struct MusicController {
    list_releases: ListReleasesUseCase,
    get_release: GetReleaseUseCase,
    get_track: GetTrackUseCase,
}

impl MusicController {
    /// Connects HTTP operations to their query workflows.
    pub(crate) fn new(
        list_releases: ListReleasesUseCase,
        get_release: GetReleaseUseCase,
        get_track: GetTrackUseCase,
    ) -> Self {
        Self {
            list_releases,
            get_release,
            get_track,
        }
    }

    /// Installs music API routes and problem responses for unsupported requests.
    pub(crate) fn router(self) -> Router {
        Router::new()
            .route("/releases", get(Self::list_releases))
            .route("/releases/{release_id}", get(Self::get_release))
            .route("/tracks/{track_id}", get(Self::get_track))
            .fallback(|| async { ControllerError::NotFound })
            .method_not_allowed_fallback(|| async { ControllerError::MethodNotAllowed })
            .with_state(self)
    }

    /// Validates pagination input before returning a release page.
    async fn list_releases(
        State(this): State<Self>,
        query: Result<Query<ListReleasesQuery>, QueryRejection>,
    ) -> Result<Json<models::ReleasePage>, ControllerError> {
        let Query(query) = query.map_err(|error| ControllerError::BadRequest(error.to_string()))?;
        let limit = query.limit.unwrap_or(30);

        if !(1..=100).contains(&limit) || query.cursor.as_ref().is_some_and(String::is_empty) {
            return Err(ControllerError::BadRequest(
                "invalid limit or cursor".into(),
            ));
        }

        Ok(Json(
            this.list_releases
                .execute(limit, query.cursor)
                .await?
                .into(),
        ))
    }

    /// Validates a release identifier before returning its metadata and tracks.
    async fn get_release(
        State(this): State<Self>,
        path: Result<Path<String>, PathRejection>,
    ) -> Result<Json<models::Release>, ControllerError> {
        let Path(id) = path.map_err(|error| ControllerError::BadRequest(error.to_string()))?;
        let id = id
            .parse::<ReleaseId>()
            .map_err(|error| ControllerError::BadRequest(error.to_string()))?;

        Ok(Json(this.get_release.execute(&id).await?.into()))
    }

    /// Validates a track identifier before returning its metadata and original audio.
    async fn get_track(
        State(this): State<Self>,
        path: Result<Path<String>, PathRejection>,
    ) -> Result<Json<models::Track>, ControllerError> {
        let Path(id) = path.map_err(|error| ControllerError::BadRequest(error.to_string()))?;
        let id = id
            .parse::<TrackId>()
            .map_err(|error| ControllerError::BadRequest(error.to_string()))?;

        Ok(Json(this.get_track.execute(&id).await?.into()))
    }
}

/// Pagination input awaiting HTTP boundary validation.
#[derive(serde::Deserialize)]
struct ListReleasesQuery {
    cursor: Option<String>,
    limit: Option<u8>,
}

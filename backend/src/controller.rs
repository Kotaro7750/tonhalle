//! Validates HTTP requests and exposes the music browsing API.

use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        rejection::{PathRejection, QueryRejection},
    },
    routing::get,
};
use error::ControllerError;
use tonhalle_api::models;

use crate::{
    domain::id::{ArtistId, ReleaseId, TrackId},
    usecase::{
        GetArtistUseCase, GetReleaseUseCase, GetTrackUseCase, ListArtistsUseCase,
        ListReleasesUseCase,
    },
};

pub(crate) mod content_access;
mod domain_type_convert;
mod error;

/// Handles music browsing requests using application query workflows.
#[derive(Clone)]
pub(crate) struct MusicController {
    list_artists: ListArtistsUseCase,
    get_artist: GetArtistUseCase,
    list_releases: ListReleasesUseCase,
    get_release: GetReleaseUseCase,
    get_track: GetTrackUseCase,
}

impl MusicController {
    /// Connects HTTP operations to their query workflows.
    pub(crate) fn new(
        list_artists: ListArtistsUseCase,
        get_artist: GetArtistUseCase,
        list_releases: ListReleasesUseCase,
        get_release: GetReleaseUseCase,
        get_track: GetTrackUseCase,
    ) -> Self {
        Self {
            list_artists,
            get_artist,
            list_releases,
            get_release,
            get_track,
        }
    }

    /// Installs music API routes and problem responses for unsupported requests.
    pub(crate) fn router(self) -> Router {
        Router::new()
            .route("/artists", get(Self::list_artists))
            .route("/artists/{artist_id}", get(Self::get_artist))
            .route("/releases", get(Self::list_releases))
            .route("/releases/{release_id}", get(Self::get_release))
            .route("/tracks/{track_id}", get(Self::get_track))
            .fallback(|| async { ControllerError::NotFound })
            .method_not_allowed_fallback(|| async { ControllerError::MethodNotAllowed })
            .with_state(self)
    }

    /// Validates pagination input before returning an artist page.
    async fn list_artists(
        State(this): State<Self>,
        query: Result<Query<PaginationQuery>, QueryRejection>,
    ) -> Result<Json<models::ArtistPage>, ControllerError> {
        let Query(query) = query.map_err(|error| ControllerError::BadRequest(error.to_string()))?;
        let limit = query.validated_limit()?;

        Ok(Json(
            this.list_artists.execute(limit, query.cursor).await?.into(),
        ))
    }

    /// Validates an artist identifier before returning its metadata.
    async fn get_artist(
        State(this): State<Self>,
        path: Result<Path<String>, PathRejection>,
    ) -> Result<Json<models::Artist>, ControllerError> {
        let Path(id) = path.map_err(|error| ControllerError::BadRequest(error.to_string()))?;
        let id = id
            .parse::<ArtistId>()
            .map_err(|error| ControllerError::BadRequest(error.to_string()))?;

        Ok(Json(models::Artist::from(
            &this.get_artist.execute(&id).await?,
        )))
    }

    /// Validates pagination input before returning a release page.
    async fn list_releases(
        State(this): State<Self>,
        query: Result<Query<PaginationQuery>, QueryRejection>,
    ) -> Result<Json<models::ReleasePage>, ControllerError> {
        let Query(query) = query.map_err(|error| ControllerError::BadRequest(error.to_string()))?;
        let limit = query.validated_limit()?;

        Ok(Json(
            this.list_releases
                .execute(
                    limit,
                    query.cursor,
                    query
                        .artist_id
                        .map(|id| id.parse::<ArtistId>())
                        .transpose()
                        .map_err(|error| ControllerError::BadRequest(error.to_string()))?,
                )
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

/// Query input awaiting HTTP boundary validation.
#[derive(serde::Deserialize)]
struct PaginationQuery {
    #[serde(rename = "artistId")]
    artist_id: Option<String>,
    cursor: Option<String>,
    limit: Option<u8>,
}

impl PaginationQuery {
    /// Validates the common page bounds and continuation token.
    fn validated_limit(&self) -> Result<u8, ControllerError> {
        let limit = self.limit.unwrap_or(30);
        if !(1..=100).contains(&limit) || self.cursor.as_ref().is_some_and(String::is_empty) {
            return Err(ControllerError::BadRequest(
                "invalid limit or cursor".into(),
            ));
        }
        Ok(limit)
    }
}

#[cfg(test)]
mod tests;

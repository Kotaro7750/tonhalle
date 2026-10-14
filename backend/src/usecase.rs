//! Queries release and track metadata through the index port.

use std::sync::Arc;

use crate::{
    domain::{
        ReleaseId, TrackId,
        primitive::{release::Release, track::Track},
    },
    port::{self, MetadataIndex, ReleasePage},
};

mod build_metadata_index;

pub(crate) use build_metadata_index::{BuildMetadataIndexError, BuildMetadataIndexUseCase};

/// Browses accepted releases through the metadata index.
#[derive(Clone)]
pub(crate) struct ListReleasesUseCase {
    index: Arc<dyn MetadataIndex>,
}

/// A release-list failure expressed independently of the HTTP controller.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ListReleasesError {
    /// The supplied token cannot continue release browsing.
    #[error("invalid cursor")]
    InvalidCursor,
    /// The underlying index failure remains available for diagnostics.
    #[error("listing releases: {0}")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<port::ListReleasesError> for ListReleasesError {
    /// Converts port failures while preserving their cause and application meaning.
    fn from(error: port::ListReleasesError) -> Self {
        match error {
            port::ListReleasesError::InvalidCursor => Self::InvalidCursor,
            port::ListReleasesError::Internal(error) => Self::Internal(error),
        }
    }
}

impl ListReleasesUseCase {
    /// Connects the query workflow to its metadata-index port.
    pub(crate) fn new(index: Arc<dyn MetadataIndex>) -> Self {
        Self { index }
    }

    /// Reads an ordered page using the supplied continuation token.
    pub(crate) async fn execute(
        &self,
        limit: u8,
        cursor: Option<String>,
    ) -> Result<ReleasePage, ListReleasesError> {
        self.index
            .list_releases(limit, cursor)
            .await
            .map_err(Into::into)
    }
}

/// Retrieves an accepted release through the metadata index.
#[derive(Clone)]
pub(crate) struct GetReleaseUseCase {
    index: Arc<dyn MetadataIndex>,
}

/// A release lookup failure expressed independently of HTTP.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GetReleaseError {
    /// The requested entity was not accepted into the index.
    #[error("release not found")]
    NotFound,
    /// The underlying index failure remains available for diagnostics.
    #[error("getting release: {0}")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<port::GetReleaseError> for GetReleaseError {
    /// Converts port failures while preserving their cause and application meaning.
    fn from(error: port::GetReleaseError) -> Self {
        match error {
            port::GetReleaseError::NotFound => Self::NotFound,
            port::GetReleaseError::Internal(error) => Self::Internal(error),
        }
    }
}

impl GetReleaseUseCase {
    /// Connects the query workflow to its metadata-index port.
    pub(crate) fn new(index: Arc<dyn MetadataIndex>) -> Self {
        Self { index }
    }

    /// Retrieves the requested release and its accepted tracks.
    pub(crate) async fn execute(&self, id: &ReleaseId) -> Result<Release, GetReleaseError> {
        self.index.get_release(id).await.map_err(Into::into)
    }
}

/// Retrieves an accepted track through the metadata index.
#[derive(Clone)]
pub(crate) struct GetTrackUseCase {
    index: Arc<dyn MetadataIndex>,
}

/// A track lookup failure expressed independently of HTTP.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GetTrackError {
    /// The requested entity was not accepted into the index.
    #[error("track not found")]
    NotFound,
    /// The underlying index failure remains available for diagnostics.
    #[error("getting track: {0}")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl From<port::GetTrackError> for GetTrackError {
    /// Converts port failures while preserving their cause and application meaning.
    fn from(error: port::GetTrackError) -> Self {
        match error {
            port::GetTrackError::NotFound => Self::NotFound,
            port::GetTrackError::Internal(error) => Self::Internal(error),
        }
    }
}

impl GetTrackUseCase {
    /// Connects the query workflow to its metadata-index port.
    pub(crate) fn new(index: Arc<dyn MetadataIndex>) -> Self {
        Self { index }
    }

    /// Retrieves the requested track and its original audio file.
    pub(crate) async fn execute(&self, id: &TrackId) -> Result<Track, GetTrackError> {
        self.index.get_track(id).await.map_err(Into::into)
    }
}

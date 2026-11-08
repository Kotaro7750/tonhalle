//! Defines indexed music queries and their results.

use async_trait::async_trait;
use std::error;

use crate::domain::{
    id::{AudioFileId, ReleaseId, TrackId},
    primitive::{release::Release, track::Track},
};

/// A release-list slice with an optional continuation token.
#[derive(getset::Getters, Debug, Clone)]
pub(crate) struct ReleasePage {
    /// Exposes the release order chosen by the index.
    #[getset(get = "pub(crate)")]
    items: Vec<Release>,
    next_cursor: Option<String>,
}

impl ReleasePage {
    /// Collects a page and its optional continuation token.
    pub(crate) fn new(items: Vec<Release>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }

    /// Provides an opaque continuation token only when another page exists.
    pub(crate) fn next_cursor(&self) -> Option<&str> {
        self.next_cursor.as_deref()
    }
}

/// A failure to browse the metadata index.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ListReleasesError {
    /// The continuation token cannot resume this release listing.
    #[error("invalid cursor")]
    InvalidCursor,
    /// The index operation failed with a retained source error.
    #[error("listing releases: {0}")]
    Internal(#[from] Box<dyn error::Error + Send + Sync>),
}

/// A failure to retrieve an indexed release.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GetReleaseError {
    /// The requested entity is absent from the index.
    #[error("release not found")]
    NotFound,
    /// The index operation failed with a retained source error.
    #[error("getting release: {0}")]
    Internal(#[from] Box<dyn error::Error + Send + Sync>),
}

/// A failure to retrieve an indexed track.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GetTrackError {
    /// The requested entity is absent from the index.
    #[error("track not found")]
    NotFound,
    /// The index operation failed with a retained source error.
    #[error("getting track: {0}")]
    Internal(#[from] Box<dyn error::Error + Send + Sync>),
}

/// A rejected addition or an index write failure.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AddReleaseError {
    /// A release identity must identify one complete release in the collection.
    #[error("duplicate release identity: {id}")]
    DuplicateRelease { id: ReleaseId },
    /// A track identity cannot belong to different releases.
    #[error("track {track_id} belongs to both {first_release_id} and {release_id}")]
    SharedTrack {
        track_id: TrackId,
        first_release_id: ReleaseId,
        release_id: ReleaseId,
    },
    /// A stored audio identity cannot belong to different releases.
    #[error("audio file {id} belongs to both {first_release_id} and {release_id}")]
    SharedAudioFile {
        id: AudioFileId,
        first_release_id: ReleaseId,
        release_id: ReleaseId,
    },
    /// The index could not perform the write.
    #[error("adding release: {0}")]
    Internal(#[from] Box<dyn error::Error + Send + Sync>),
}

/// Provides asynchronous browsing of accepted music entities.
#[async_trait]
pub(crate) trait MetadataIndex: Send + Sync {
    /// Validates against existing entities and atomically adds a complete release.
    ///
    /// Concurrent reads and writes are supported; rejected additions leave the index unchanged.
    async fn add_release(&self, release: Release) -> Result<(), AddReleaseError>;

    /// Reads releases in identifier order after an optional continuation token.
    async fn list_releases(
        &self,
        limit: u8,
        cursor: Option<String>,
    ) -> Result<ReleasePage, ListReleasesError>;
    /// Retrieves a release with all its accepted tracks.
    async fn get_release(&self, id: &ReleaseId) -> Result<Release, GetReleaseError>;
    /// Retrieves a track with its accepted original audio file.
    async fn get_track(&self, id: &TrackId) -> Result<Track, GetTrackError>;
}

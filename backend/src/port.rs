//! Defines storage and metadata-index contracts independent of infrastructure.

use async_trait::async_trait;

use crate::domain::{
    ReleaseId, TrackId,
    id::ParseReleaseIdError,
    ingestion::IngestTrackError,
    primitive::{
        release::{Release, ReleaseValidationError},
        track::Track,
    },
};

/// A storage enumeration failure that prevents a complete scan.
#[derive(Debug, thiserror::Error)]
#[error("scanning music storage: {0}")]
pub(crate) struct ScanReleasesError(#[from] Box<dyn std::error::Error + Send + Sync>);

impl ScanReleasesError {
    /// Preserves the underlying failure in the error source chain.
    pub(crate) fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Box::new(source))
    }
}

/// One source audio failure identified within its stored release.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AudioScanFailure {
    /// Storage could not obtain valid source bytes and their stored identity.
    #[error("reading {object_key}: {source}")]
    Read {
        object_key: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Domain parsing or validation rejected the source audio.
    #[error("parsing {object_key}: {source}")]
    Parse {
        object_key: String,
        #[source]
        source: IngestTrackError,
    },
}

/// A rejected storage release with the identity available at the failing stage.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReleaseScanFailure {
    /// The storage key could not supply a valid release identity before reading any audio.
    #[error("invalid release identity in {object_key}: {source}")]
    InvalidId {
        object_key: String,
        #[source]
        source: ParseReleaseIdError,
    },
    /// All audio sources were attempted, and at least one failed.
    #[error("scanning audio for {id}: {failures:?}")]
    Audio {
        id: ReleaseId,
        failures: Vec<AudioScanFailure>,
    },
    /// Valid tracks could not form one consistent release.
    #[error("constructing {id}: {source}")]
    InvalidRelease {
        id: ReleaseId,
        #[source]
        source: ReleaseValidationError,
    },
}

/// Scans complete domain releases independently of physical storage representation.
pub(crate) trait MusicStorage: Send + Sync {
    /// Returns individual successes and failures separately from incomplete enumeration.
    fn scan_releases(&self) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError>;
}

/// A release-list slice with an optional continuation token.
#[derive(Debug, Clone)]
pub(crate) struct ReleasePage {
    items: Vec<Release>,
    next_cursor: Option<String>,
}

impl ReleasePage {
    /// Collects a page and its optional continuation token.
    pub(crate) fn new(items: Vec<Release>, next_cursor: Option<String>) -> Self {
        Self { items, next_cursor }
    }

    /// Exposes the release order chosen by the index.
    pub(crate) fn items(&self) -> &[Release] {
        &self.items
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
    Internal(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A failure to retrieve an indexed release.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GetReleaseError {
    /// The requested entity is absent from the index.
    #[error("release not found")]
    NotFound,
    /// The index operation failed with a retained source error.
    #[error("getting release: {0}")]
    Internal(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// A failure to retrieve an indexed track.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GetTrackError {
    /// The requested entity is absent from the index.
    #[error("track not found")]
    NotFound,
    /// The index operation failed with a retained source error.
    #[error("getting track: {0}")]
    Internal(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Provides asynchronous browsing of accepted music entities.
#[async_trait]
pub(crate) trait MetadataIndex: Send + Sync {
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

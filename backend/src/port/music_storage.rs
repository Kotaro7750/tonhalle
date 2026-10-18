//! Defines release-level music storage operations and their outcomes.

use std::error;

use crate::domain::{
    id::{ParseReleaseIdError, ReleaseId},
    import::ImportRelease,
    ingestion::IngestTrackError,
    primitive::release::{Release, ReleaseValidationError},
};

/// The outcome of a non-overwriting release save.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SaveReleaseOutcome {
    /// The complete release was published.
    Saved,
    /// A previously published release was retained.
    AlreadyExists,
}

/// A failure to persist a complete release without replacing existing content.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SaveReleaseError {
    /// The release identity is occupied by an incompatible destination object.
    #[error("conflicting storage object for release {id}")]
    Conflict { id: ReleaseId },
    /// The adapter failed while preparing, writing, or publishing the release.
    #[error("saving release {id}: {source}")]
    Internal {
        id: ReleaseId,
        #[source]
        source: Box<dyn error::Error + Send + Sync>,
    },
}

/// A storage enumeration failure that prevents a complete scan.
#[derive(Debug, thiserror::Error)]
#[error("scanning music storage: {0}")]
pub(crate) struct ScanReleasesError(#[from] Box<dyn error::Error + Send + Sync>);

impl ScanReleasesError {
    /// Preserves the underlying failure in the error source chain.
    pub(crate) fn new(source: impl error::Error + Send + Sync + 'static) -> Self {
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
        source: Box<dyn error::Error + Send + Sync>,
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
    /// Saves a complete release without replacing an existing identity or exposing partial writes.
    fn save_release(&self, release: &ImportRelease)
    -> Result<SaveReleaseOutcome, SaveReleaseError>;

    /// Returns individual successes and failures separately from incomplete enumeration.
    fn scan_releases(&self) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError>;
}

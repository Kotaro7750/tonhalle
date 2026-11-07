//! Builds the startup index from storage releases after domain consistency validation.

use std::sync::Arc;
use tokio::task::{JoinError, spawn_blocking};

use crate::{
    domain::primitive::release::{Release, ReleasesValidationError, validate_releases},
    port::music_storage::{MusicStorage, ScanReleasesError},
};

/// A fatal scan or consistency failure that prevents startup index construction.
#[derive(Debug, thiserror::Error)]
pub(crate) enum BuildMetadataIndexError {
    /// The domain consistency validation worker failed.
    #[error(transparent)]
    Worker(#[from] JoinError),
    /// Storage enumeration did not complete.
    #[error(transparent)]
    Scan(#[from] ScanReleasesError),
    /// Accepted releases contain conflicting entity identities.
    #[error(transparent)]
    InconsistentReleases(#[from] ReleasesValidationError),
}

/// Obtains valid storage releases for startup indexing.
pub(crate) struct BuildMetadataIndexUseCase {
    storage: Arc<dyn MusicStorage>,
}

impl BuildMetadataIndexUseCase {
    /// Connects startup indexing to its read-only music-storage port.
    pub(crate) fn new(storage: Arc<dyn MusicStorage>) -> Self {
        Self { storage }
    }

    /// Reports individual scan failures and validates the accepted collection through Domain.
    pub(crate) async fn execute(&self) -> Result<Vec<Release>, BuildMetadataIndexError> {
        let mut releases = Vec::new();

        for result in self.storage.scan_releases().await? {
            match result {
                Ok(release) => releases.push(release),
                Err(failure) => tracing::warn!(?failure, "Storage release rejected"),
            }
        }

        spawn_blocking(move || {
            validate_releases(&releases)?;
            Ok(releases)
        })
        .await?
    }
}

/// Verifies scan orchestration and propagation of fatal failures.
#[cfg(test)]
mod tests {
    use std::{
        error,
        io::{Error as IoError, ErrorKind},
        iter::successors,
    };

    use super::*;
    use crate::{
        domain::{id::ReleaseId, import::ImportAudio, ingestion::metadata::sample_metadata},
        port::music_storage::{
            DeleteStagingError, ReleaseScanFailure, SaveReleaseError, SaveReleaseOutcome,
        },
        test_support::sample_release,
    };

    /// Simulates a fatal storage enumeration failure.
    struct UnavailableStorage;

    #[async_trait::async_trait]
    impl MusicStorage for UnavailableStorage {
        /// Index-only fixtures must never receive writes.
        async fn stage_audio(&self, _: ImportAudio) -> Result<(), SaveReleaseError> {
            panic!("index construction must not write music storage")
        }

        async fn delete_staging(&self, _: Option<&ReleaseId>) -> Result<(), DeleteStagingError> {
            panic!("index construction must not write music storage")
        }

        async fn commit_release(
            &self,
            _: &ReleaseId,
            _: bool,
        ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
            panic!("index construction must not write music storage")
        }

        /// Returns an I/O failure before individual release results are available.
        async fn scan_releases(
            &self,
        ) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError> {
            Err(ScanReleasesError::new(IoError::new(
                ErrorKind::PermissionDenied,
                "content directory is not readable",
            )))
        }
    }

    /// Supplies accepted releases alongside an individual storage scan failure.
    struct FixtureStorage(Vec<Release>);

    #[async_trait::async_trait]
    impl MusicStorage for FixtureStorage {
        /// Index-only fixtures must never receive writes.
        async fn stage_audio(&self, _: ImportAudio) -> Result<(), SaveReleaseError> {
            panic!("index construction must not write music storage")
        }

        async fn delete_staging(&self, _: Option<&ReleaseId>) -> Result<(), DeleteStagingError> {
            panic!("index construction must not write music storage")
        }

        async fn commit_release(
            &self,
            _: &ReleaseId,
            _: bool,
        ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
            panic!("index construction must not write music storage")
        }

        /// Preserves accepted releases while reporting an independently rejected storage key.
        async fn scan_releases(
            &self,
        ) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError> {
            let mut results = self.0.iter().cloned().map(Ok).collect::<Vec<_>>();

            results.push(Err(ReleaseScanFailure::InvalidId {
                object_key: "invalid-release".into(),
                source: "invalid-release".parse::<ReleaseId>().unwrap_err(),
            }));

            Ok(results)
        }
    }

    #[tokio::test]
    async fn execute_preserves_the_cause_of_scan_failures() {
        let error = BuildMetadataIndexUseCase::new(Arc::new(UnavailableStorage))
            .execute()
            .await
            .unwrap_err();

        assert!(
            successors(Some(&error as &dyn error::Error), |cause| cause.source()).any(|cause| {
                cause
                    .downcast_ref::<IoError>()
                    .is_some_and(|cause| cause.kind() == ErrorKind::PermissionDenied)
            })
        );
    }

    #[tokio::test]
    async fn execute_keeps_valid_releases_when_individual_candidates_fail() {
        let release = sample_release(&sample_metadata());

        let accepted =
            BuildMetadataIndexUseCase::new(Arc::new(FixtureStorage(vec![release.clone()])))
                .execute()
                .await
                .unwrap();

        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].id(), release.id());
    }

    #[tokio::test]
    async fn execute_propagates_collection_conflicts() {
        let release = sample_release(&sample_metadata());
        assert!(matches!(
            BuildMetadataIndexUseCase::new(Arc::new(FixtureStorage(vec![
                release.clone(),
                release
            ])))
            .execute()
            .await,
            Err(BuildMetadataIndexError::InconsistentReleases(
                ReleasesValidationError::DuplicateRelease { .. }
            ))
        ));
    }
}

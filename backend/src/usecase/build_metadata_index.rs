//! Builds the startup index from storage releases after domain consistency validation.

use std::sync::Arc;

use crate::{
    domain::primitive::release::{Release, ReleasesValidationError, validate_releases},
    port::music_storage::{MusicStorage, ScanReleasesError},
};

/// A fatal scan or consistency failure that prevents startup index construction.
#[derive(Debug, thiserror::Error)]
pub(crate) enum BuildMetadataIndexError {
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
    pub(crate) fn execute(&self) -> Result<Vec<Release>, BuildMetadataIndexError> {
        let mut releases = Vec::new();

        for result in self.storage.scan_releases()? {
            match result {
                Ok(release) => releases.push(release),
                Err(failure) => tracing::warn!(?failure, "Storage release rejected"),
            }
        }

        validate_releases(&releases)?;

        Ok(releases)
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
        domain::{id::ReleaseId, import::ImportRelease, ingestion::metadata::sample_metadata},
        port::music_storage::{ReleaseScanFailure, SaveReleaseError, SaveReleaseOutcome},
        test_support::sample_release,
    };

    /// Simulates a fatal storage enumeration failure.
    struct UnavailableStorage;

    impl MusicStorage for UnavailableStorage {
        /// Index-only fixtures must never receive writes.
        fn save_release(&self, _: &ImportRelease) -> Result<SaveReleaseOutcome, SaveReleaseError> {
            panic!("index construction must not write music storage")
        }

        /// Returns an I/O failure before individual release results are available.
        fn scan_releases(
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

    impl MusicStorage for FixtureStorage {
        /// Index-only fixtures must never receive writes.
        fn save_release(&self, _: &ImportRelease) -> Result<SaveReleaseOutcome, SaveReleaseError> {
            panic!("index construction must not write music storage")
        }

        /// Preserves accepted releases while reporting an independently rejected storage key.
        fn scan_releases(
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

    #[test]
    fn execute_preserves_the_cause_of_scan_failures() {
        let error = BuildMetadataIndexUseCase::new(Arc::new(UnavailableStorage))
            .execute()
            .unwrap_err();

        assert!(
            successors(Some(&error as &dyn error::Error), |cause| cause.source()).any(|cause| {
                cause
                    .downcast_ref::<IoError>()
                    .is_some_and(|cause| cause.kind() == ErrorKind::PermissionDenied)
            })
        );
    }

    #[test]
    fn execute_keeps_valid_releases_when_individual_candidates_fail() {
        let release = sample_release(&sample_metadata());

        let accepted =
            BuildMetadataIndexUseCase::new(Arc::new(FixtureStorage(vec![release.clone()])))
                .execute()
                .unwrap();

        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].id(), release.id());
    }

    #[test]
    fn execute_propagates_collection_conflicts() {
        let release = sample_release(&sample_metadata());
        assert!(matches!(
            BuildMetadataIndexUseCase::new(Arc::new(FixtureStorage(vec![
                release.clone(),
                release
            ])))
            .execute(),
            Err(BuildMetadataIndexError::InconsistentReleases(
                ReleasesValidationError::DuplicateRelease { .. }
            ))
        ));
    }
}

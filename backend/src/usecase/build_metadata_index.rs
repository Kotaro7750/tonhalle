//! Incrementally registers accepted storage releases in the shared metadata index.

use futures_util::StreamExt;
use std::sync::Arc;

use crate::port::{
    metadata_index::{AddReleaseError, MetadataIndex},
    music_storage::{MusicStorage, ScanReleasesError},
};

/// A fatal enumeration or index infrastructure failure.
#[derive(Debug, thiserror::Error)]
pub(crate) enum BuildMetadataIndexError {
    /// Storage enumeration could not continue.
    #[error(transparent)]
    Scan(#[from] ScanReleasesError),
    /// The index could not accept writes.
    #[error(transparent)]
    Index(#[from] AddReleaseError),
}

/// Scans releases and publishes each independently validated addition.
pub(crate) struct BuildMetadataIndexUseCase {
    storage: Arc<dyn MusicStorage>,
    index: Arc<dyn MetadataIndex>,
}

impl BuildMetadataIndexUseCase {
    /// Connects the source storage and shared destination index.
    pub(crate) fn new(storage: Arc<dyn MusicStorage>, index: Arc<dyn MetadataIndex>) -> Self {
        Self { storage, index }
    }

    /// Continues past rejected releases while propagating failures that prevent further scanning.
    pub(crate) async fn execute(&self) -> Result<(), BuildMetadataIndexError> {
        let mut releases = self.storage.scan_releases();
        while let Some(result) = releases.next().await {
            let release = match result? {
                Ok(release) => release,
                Err(failure) => {
                    tracing::warn!(?failure, "Storage release rejected");
                    continue;
                }
            };
            match self.index.add_release(release).await {
                Ok(()) => {}
                Err(
                    error @ (AddReleaseError::DuplicateRelease { .. }
                    | AddReleaseError::SharedTrack { .. }
                    | AddReleaseError::SharedAudioFile { .. }),
                ) => {
                    tracing::warn!(?error, "Index release rejected")
                }
                Err(error @ AddReleaseError::Internal(_)) => return Err(error.into()),
            }
        }
        Ok(())
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
    use crate::domain::primitive::release::Release;
    use crate::infrastructure::metadata_index::InMemoryMetadataIndex;
    use crate::{
        domain::{id::ReleaseId, import::ImportAudio, ingestion::metadata::sample_metadata},
        port::music_storage::{
            DeleteStagingError, ReleaseScanFailure, SaveReleaseError, SaveReleaseOutcome,
        },
        test_support::{sample_release, sample_release_with_audio_id},
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
        fn scan_releases(
            &self,
        ) -> futures_util::stream::BoxStream<
            '_,
            Result<Result<Release, ReleaseScanFailure>, ScanReleasesError>,
        > {
            futures_util::stream::once(async {
                Err(ScanReleasesError::new(IoError::new(
                    ErrorKind::PermissionDenied,
                    "content directory is not readable",
                )))
            })
            .boxed()
        }
    }

    /// Supplies accepted releases alongside an individual storage scan failure.
    struct FixtureStorage(Vec<Release>, Option<Arc<tokio::sync::Notify>>);

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
        fn scan_releases(
            &self,
        ) -> futures_util::stream::BoxStream<
            '_,
            Result<Result<Release, ReleaseScanFailure>, ScanReleasesError>,
        > {
            let mut results = self.0.iter().cloned().map(Ok).collect::<Vec<_>>();

            results.push(Err(ReleaseScanFailure::InvalidId {
                object_key: "invalid-release".into(),
                source: "invalid-release".parse::<ReleaseId>().unwrap_err(),
            }));

            let tail = futures_util::stream::once(async move {
                if let Some(gate) = &self.1 {
                    gate.notify_one();
                    std::future::pending::<()>().await;
                }
                None
            })
            .filter_map(|item| async move { item });
            futures_util::stream::iter(results.into_iter().map(Ok))
                .chain(tail)
                .boxed()
        }
    }

    #[tokio::test]
    async fn execute_publishes_releases_before_the_scan_finishes() {
        let release = sample_release(&sample_metadata());
        let index = Arc::new(InMemoryMetadataIndex::new());
        let gate = Arc::new(tokio::sync::Notify::new());
        let scan = BuildMetadataIndexUseCase::new(
            Arc::new(FixtureStorage(vec![release.clone()], Some(gate.clone()))),
            index.clone(),
        );
        let worker = tokio::spawn(async move { scan.execute().await });
        tokio::time::timeout(std::time::Duration::from_secs(5), gate.notified())
            .await
            .unwrap();
        assert!(!worker.is_finished());
        assert_eq!(
            index.get_release(release.id()).await.unwrap().id(),
            release.id()
        );
        worker.abort();
        assert!(worker.await.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn execute_propagates_fatal_scan_failures() {
        let index = Arc::new(InMemoryMetadataIndex::new());
        let error = BuildMetadataIndexUseCase::new(Arc::new(UnavailableStorage), index)
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
    async fn execute_rejects_conflicts_and_continues_to_later_releases() {
        let first = sample_release(&sample_metadata());
        let mut raw = sample_metadata();
        *raw.release_title_mut() = "Later release".into();
        let later = sample_release_with_audio_id(
            &raw,
            "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap(),
        );
        let index = Arc::new(InMemoryMetadataIndex::new());
        BuildMetadataIndexUseCase::new(
            Arc::new(FixtureStorage(
                vec![first.clone(), first.clone(), later.clone()],
                None,
            )),
            index.clone(),
        )
        .execute()
        .await
        .unwrap();
        assert_eq!(
            index.list_releases(100, None).await.unwrap().items().len(),
            2
        );
        assert_eq!(
            index.get_release(first.id()).await.unwrap().id(),
            first.id()
        );
        assert_eq!(
            index.get_release(later.id()).await.unwrap().id(),
            later.id()
        );
    }
}

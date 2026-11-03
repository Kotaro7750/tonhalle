//! Imports independent release candidates without accessing the metadata index.

use std::sync::Arc;
use tokio::task::{JoinError, spawn_blocking};

use crate::{
    domain::import::{ImportRelease, ImportValidationError},
    port::{
        import_source::{ImportSource, ListImportReleasesError, ReadImportAudioError},
        music_storage::{MusicStorage, SaveReleaseError, SaveReleaseOutcome},
    },
};

/// A fatal failure preventing complete import-source enumeration.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportMusicError {
    /// The source enumeration worker failed.
    #[error(transparent)]
    Worker(#[from] JoinError),
    /// The source could not enumerate all release candidates.
    #[error(transparent)]
    ListReleases(#[from] ListImportReleasesError),
}

/// A candidate failure reported while other releases continue importing.
#[derive(Debug, thiserror::Error)]
enum ImportCandidateError {
    /// The source reading and validation worker failed.
    #[error(transparent)]
    Worker(#[from] JoinError),
    /// A source audio object could not be obtained.
    #[error(transparent)]
    Read(#[from] ReadImportAudioError),
    /// Source metadata violates the import constraints.
    #[error(transparent)]
    Validation(#[from] ImportValidationError),
    /// The validated release could not be saved.
    #[error(transparent)]
    Save(#[from] SaveReleaseError),
}

/// Counts all candidate outcomes after a complete source enumeration.
#[derive(Debug, Default, getset::CopyGetters)]
pub(crate) struct ImportSummary {
    /// Number of releases successfully published.
    #[getset(get_copy = "pub(crate)")]
    saved: usize,
    /// Number of already existing releases retained.
    #[getset(get_copy = "pub(crate)")]
    skipped: usize,
    /// Number of candidates rejected during reading, validation, or saving.
    #[getset(get_copy = "pub(crate)")]
    failed: usize,
}

/// Coordinates release-level validation and persistence independently of transport.
pub(crate) struct ImportMusicUseCase {
    source: Arc<dyn ImportSource>,
    storage: Arc<dyn MusicStorage>,
}

impl ImportMusicUseCase {
    /// Connects independent source and destination storage implementations.
    pub(crate) fn new(source: Arc<dyn ImportSource>, storage: Arc<dyn MusicStorage>) -> Self {
        Self { source, storage }
    }

    /// Continues after each candidate failure and reports fatal enumeration failures.
    pub(crate) async fn execute(&self) -> Result<ImportSummary, ImportMusicError> {
        let source = self.source.clone();
        let candidates = spawn_blocking(move || source.list_releases()).await??;
        let mut summary = ImportSummary::default();
        for candidate in candidates {
            let key = candidate.key().clone();
            let source = self.source.clone();
            let result = async {
                let release = spawn_blocking(move || {
                    let bytes = candidate
                        .audio_keys()
                        .iter()
                        .map(|key| source.read_audio(key))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok::<_, ImportCandidateError>(ImportRelease::new(bytes)?)
                })
                .await??;
                Ok::<_, ImportCandidateError>(self.storage.save_release(&release).await?)
            }
            .await;
            match result {
                Ok(SaveReleaseOutcome::Saved) => summary.saved += 1,
                Ok(SaveReleaseOutcome::AlreadyExists) => summary.skipped += 1,
                Err(error) => {
                    summary.failed += 1;
                    tracing::warn!(candidate = %key, error = %error, "Release import failed");
                }
            }
        }
        Ok(summary)
    }
}

/// Verifies import orchestration using controlled source and storage outcomes.
#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, io::Error as IoError, sync::Mutex};

    use super::*;
    use crate::{
        domain::{ingestion::metadata::sample_metadata, primitive::release::Release},
        port::{
            import_source::ImportCandidate,
            music_storage::{ReleaseScanFailure, ScanReleasesError},
        },
        test_support::sample_audio,
    };

    /// Provides opaque candidates and deterministic read outcomes.
    struct Source {
        candidates: Vec<Vec<String>>,
        bytes: Vec<u8>,
        unavailable: bool,
    }

    impl ImportSource for Source {
        /// Returns ordered candidates or a fatal enumeration failure.
        fn list_releases(&self) -> Result<Vec<ImportCandidate>, ListImportReleasesError> {
            if self.unavailable {
                return Err(ListImportReleasesError::new(IoError::other("unavailable")));
            }
            Ok(self
                .candidates
                .iter()
                .enumerate()
                .map(|(i, keys)| ImportCandidate::new(i.to_string(), keys.clone()))
                .collect())
        }

        /// Simulates a read failure, invalid bytes, or valid source audio.
        fn read_audio(&self, key: &str) -> Result<Vec<u8>, ReadImportAudioError> {
            match key {
                "unreadable" => Err(ReadImportAudioError::InvalidObject { key: key.into() }),
                "invalid" => Ok(b"invalid".to_vec()),
                _ => Ok(self.bytes.clone()),
            }
        }
    }

    /// Captures submitted bytes and controls each save outcome.
    struct Storage {
        outcomes: Mutex<VecDeque<Result<SaveReleaseOutcome, ()>>>,
        saved: Mutex<Vec<Vec<Vec<u8>>>>,
    }

    #[async_trait::async_trait]
    impl MusicStorage for Storage {
        /// Records valid releases and returns the configured outcome.
        async fn save_release(
            &self,
            release: &ImportRelease,
        ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
            self.saved.lock().unwrap().push(
                release
                    .audio()
                    .iter()
                    .map(|audio| audio.bytes().to_vec())
                    .collect(),
            );
            self.outcomes
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected save")
                .map_err(|()| SaveReleaseError::Conflict {
                    id: release.id().clone(),
                })
        }

        /// Import must never read the destination index.
        async fn scan_releases(
            &self,
        ) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError> {
            panic!("import must not scan storage")
        }
    }

    #[tokio::test]
    async fn execute_continues_after_candidate_failures_and_counts_save_outcomes() {
        let bytes = sample_audio(&sample_metadata());
        let source = Arc::new(Source {
            candidates: vec![
                vec!["unreadable".into()],
                vec!["invalid".into()],
                vec!["save-failure".into()],
                vec!["existing".into()],
                vec!["success".into()],
            ],
            bytes: bytes.clone(),
            unavailable: false,
        });
        let storage = Arc::new(Storage {
            outcomes: Mutex::new(VecDeque::from([
                Err(()),
                Ok(SaveReleaseOutcome::AlreadyExists),
                Ok(SaveReleaseOutcome::Saved),
            ])),
            saved: Mutex::new(vec![]),
        });
        let result = ImportMusicUseCase::new(source, storage.clone())
            .execute()
            .await
            .unwrap();
        assert_eq!(
            (result.saved(), result.skipped(), result.failed()),
            (1, 1, 3)
        );
        assert_eq!(*storage.saved.lock().unwrap(), vec![vec![bytes]; 3]);
        assert!(storage.outcomes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn execute_propagates_enumeration_failures_without_saving() {
        let source = Arc::new(Source {
            candidates: vec![],
            bytes: vec![],
            unavailable: true,
        });
        let storage = Arc::new(Storage {
            outcomes: Mutex::new(VecDeque::new()),
            saved: Mutex::new(vec![]),
        });
        assert!(matches!(
            ImportMusicUseCase::new(source, storage.clone())
                .execute()
                .await,
            Err(ImportMusicError::ListReleases(_))
        ));
        assert!(storage.saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn execute_returns_an_empty_summary_when_there_are_no_candidates() {
        let source = Arc::new(Source {
            candidates: vec![],
            bytes: vec![],
            unavailable: false,
        });
        let storage = Arc::new(Storage {
            outcomes: Mutex::new(VecDeque::new()),
            saved: Mutex::new(vec![]),
        });
        let result = ImportMusicUseCase::new(source, storage.clone())
            .execute()
            .await
            .unwrap();
        assert_eq!(
            (result.saved(), result.skipped(), result.failed()),
            (0, 0, 0)
        );
        assert!(storage.saved.lock().unwrap().is_empty());
    }
}

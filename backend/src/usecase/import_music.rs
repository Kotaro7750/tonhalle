//! Imports independent release candidates without accessing the metadata index.

use std::{collections::HashSet, sync::Arc};
use tokio::task::{JoinError, spawn_blocking};

use crate::{
    domain::{
        id::ReleaseId,
        import::{ImportRelease, ImportValidationError},
    },
    port::{
        import_source::{ImportSource, ListImportReleasesError, ReadImportAudioError},
        music_storage::{MusicStorage, SaveReleaseOutcome},
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
    /// Another candidate in this execution already claimed the release identity.
    #[error("duplicate release candidate: {0}")]
    DuplicateRelease(ReleaseId),
    /// The source reading and validation worker failed.
    #[error(transparent)]
    Worker(#[from] JoinError),
    /// A source audio object could not be obtained.
    #[error(transparent)]
    Read(#[from] ReadImportAudioError),
    /// Source metadata violates the import constraints.
    #[error(transparent)]
    Validation(#[from] ImportValidationError),
}

/// Counts all candidate outcomes after a complete source enumeration.
#[derive(Debug, Default, getset::CopyGetters)]
pub(crate) struct ImportSummary {
    /// Number of releases saved or treated as saved in a dry run.
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
    pub(crate) async fn execute(&self, dry_run: bool) -> Result<ImportSummary, ImportMusicError> {
        let checked = ImportCheckUseCase::new(self.source.clone())
            .execute()
            .await?;
        let mut summary = ImportSummary {
            failed: checked.failed,
            ..ImportSummary::default()
        };
        for (key, release) in checked.releases {
            let result = self.storage.save_release(&release, dry_run).await;
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

/// Validated source releases and the number of rejected candidates.
#[derive(Default, getset::Getters, getset::CopyGetters)]
pub(crate) struct ImportCheckResult {
    /// Retains source keys alongside validated releases for reporting.
    #[getset(get = "pub(crate)")]
    releases: Vec<(String, ImportRelease)>,
    /// Counts candidates that could not be read or validated.
    #[getset(get_copy = "pub(crate)")]
    failed: usize,
}

/// Reads and validates import candidates without a destination storage.
pub(crate) struct ImportCheckUseCase {
    source: Arc<dyn ImportSource>,
}

impl ImportCheckUseCase {
    /// Connects the read-only source used for validation.
    pub(crate) fn new(source: Arc<dyn ImportSource>) -> Self {
        Self { source }
    }

    /// Collects validated releases while continuing after candidate failures.
    pub(crate) async fn execute(&self) -> Result<ImportCheckResult, ImportMusicError> {
        let source = self.source.clone();
        let candidates = spawn_blocking(move || source.list_releases()).await??;

        let mut result = ImportCheckResult::default();
        let mut planned = HashSet::new();

        for candidate in candidates {
            let key = candidate.key().clone();
            let source = self.source.clone();
            let release = spawn_blocking(move || {
                let bytes = candidate
                    .audio_keys()
                    .iter()
                    .map(|key| source.read_audio(key))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok::<_, ImportCandidateError>(ImportRelease::new(bytes)?)
            })
            .await
            .map_err(ImportCandidateError::from)
            .and_then(|release| release)
            .and_then(|release| {
                if !planned.insert(release.id().clone()) {
                    return Err(ImportCandidateError::DuplicateRelease(release.id().clone()));
                }
                Ok(release)
            });
            match release {
                Ok(release) => result.releases.push((key, release)),
                Err(error) => {
                    result.failed += 1;
                    tracing::warn!(candidate = %key, error = %error, "Release validation failed");
                }
            }
        }
        Ok(result)
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
            music_storage::{ReleaseScanFailure, SaveReleaseError, ScanReleasesError},
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
                "existing" | "success" => {
                    let mut metadata = sample_metadata();
                    metadata.release_title_mut().push_str(key);
                    Ok(sample_audio(&metadata))
                }
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
            _dry_run: bool,
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
            .execute(false)
            .await
            .unwrap();
        assert_eq!(
            (result.saved(), result.skipped(), result.failed()),
            (1, 1, 3)
        );
        let mut existing = sample_metadata();
        existing.release_title_mut().push_str("existing");
        let mut success = sample_metadata();
        success.release_title_mut().push_str("success");
        let saved = storage.saved.lock().unwrap();
        assert_eq!(saved.len(), 3);
        assert!(saved[0] == vec![bytes]);
        for (audio, metadata) in saved[1..].iter().zip([existing, success]) {
            assert_eq!(
                ImportRelease::new(audio.clone()).unwrap().id(),
                ImportRelease::new(vec![sample_audio(&metadata)])
                    .unwrap()
                    .id()
            );
        }
        assert!(storage.outcomes.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn import_check_rejects_duplicate_releases_before_persistence() {
        let source = Arc::new(Source {
            candidates: vec![vec!["first".into()], vec!["duplicate".into()]],
            bytes: sample_audio(&sample_metadata()),
            unavailable: false,
        });
        let checked = ImportCheckUseCase::new(source.clone())
            .execute()
            .await
            .unwrap();
        assert_eq!(checked.releases().len(), 1);
        assert_eq!(checked.releases()[0].0, "0");
        assert_eq!(checked.failed(), 1);

        for outcome in [
            Err(()),
            Ok(SaveReleaseOutcome::AlreadyExists),
            Ok(SaveReleaseOutcome::Saved),
        ] {
            let expected_failed = if outcome.is_err() { 2 } else { 1 };
            let storage = Arc::new(Storage {
                outcomes: Mutex::new(VecDeque::from([outcome])),
                saved: Mutex::new(vec![]),
            });
            let result = ImportMusicUseCase::new(source.clone(), storage.clone())
                .execute(false)
                .await
                .unwrap();
            assert_eq!(storage.saved.lock().unwrap().len(), 1);
            assert_eq!(result.failed(), expected_failed);
        }
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
                .execute(false)
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
            .execute(false)
            .await
            .unwrap();
        assert_eq!(
            (result.saved(), result.skipped(), result.failed()),
            (0, 0, 0)
        );
        assert!(storage.saved.lock().unwrap().is_empty());
    }
}

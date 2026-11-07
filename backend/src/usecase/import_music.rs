//! Imports independent release candidates without accessing the metadata index.

use std::{collections::HashSet, sync::Arc};

use crate::{
    domain::{
        id::ReleaseId,
        import::{ImportValidationError, ImportedReleaseValidator},
        primitive::release::ReleaseMetadata,
    },
    port::{
        import_source::{
            ImportCandidate, ImportSource, ListImportReleasesError, ReadImportAudioError,
        },
        music_storage::{DeleteStagingError, MusicStorage, SaveReleaseError, SaveReleaseOutcome},
    },
};

/// A fatal enumeration or cleanup failure in the import workflow.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportMusicError {
    /// The source could not enumerate all release candidates.
    #[error(transparent)]
    ListReleases(#[from] ListImportReleasesError),
    /// Temporary audio could not be removed after processing all candidates.
    #[error(transparent)]
    Cleanup(#[from] DeleteStagingError),
}

/// A candidate failure reported while other releases continue importing.
#[derive(Debug, thiserror::Error)]
enum ImportCandidateError {
    /// Another candidate in this execution already claimed the release identity.
    #[error("duplicate release candidate: {0}")]
    DuplicateRelease(ReleaseId),
    /// A source audio object could not be obtained.
    #[error(transparent)]
    Read(#[from] ReadImportAudioError),
    /// Source metadata violates the import constraints.
    #[error(transparent)]
    Validation(#[from] ImportValidationError),
    /// An individual audio staging write failed.
    #[error(transparent)]
    Save(#[from] SaveReleaseError),
    /// A previous interrupted import's staging area could not be cleared.
    #[error(transparent)]
    Cleanup(#[from] DeleteStagingError),
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
        let candidates = self.source.list_releases().await?;
        let mut summary = ImportSummary::default();
        let mut planned = HashSet::new();
        for candidate in candidates {
            let key = candidate.key().clone();
            let release = prepare_unique_release(
                self.source.clone(),
                candidate,
                &mut planned,
                if dry_run {
                    None
                } else {
                    Some(self.storage.as_ref())
                },
            )
            .await;
            let Some(release) = release else {
                summary.failed += 1;
                continue;
            };
            match self.storage.commit_release(release.id(), dry_run).await {
                Ok(SaveReleaseOutcome::Saved) => summary.saved += 1,
                Ok(SaveReleaseOutcome::AlreadyExists) => summary.skipped += 1,
                Err(error) => {
                    summary.failed += 1;
                    tracing::warn!(candidate = %key, error = %error, "Release import failed");
                }
            }
        }
        if !dry_run {
            self.storage.delete_staging(None).await?;
        }
        Ok(summary)
    }
}

/// Validated source releases and the number of rejected candidates.
#[derive(Default, getset::Getters, getset::CopyGetters)]
pub(crate) struct ImportCheckResult {
    /// Retains source keys alongside validated releases for reporting.
    #[getset(get = "pub(crate)")]
    releases: Vec<(String, ReleaseMetadata)>,
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
        let candidates = self.source.list_releases().await?;

        let mut result = ImportCheckResult::default();
        let mut planned = HashSet::new();

        for candidate in candidates {
            let key = candidate.key().clone();
            match prepare_unique_release(self.source.clone(), candidate, &mut planned, None).await {
                Some(release) => result.releases.push((key, release)),
                None => result.failed += 1,
            }
        }
        Ok(result)
    }
}

/// Reserves identified releases even when candidate processing fails.
async fn prepare_unique_release(
    source: Arc<dyn ImportSource>,
    candidate: ImportCandidate,
    planned: &mut HashSet<ReleaseId>,
    storage: Option<&dyn MusicStorage>,
) -> Option<ReleaseMetadata> {
    let result = validate_and_stage_candidate(source, &candidate, storage).await;
    if let Some(id) = result.release_id
        && !planned.insert(id.clone())
    {
        let error = ImportCandidateError::DuplicateRelease(id);
        tracing::warn!(candidate = %candidate.key(), error = %error, "Release validation failed");
        return None;
    }
    result.validated_metadata
}

/// Preserves the first identified release independently of candidate success.
struct CandidatePreparationResult {
    /// Reserves the identity discovered before any later failure.
    release_id: Option<ReleaseId>,
    /// Supplies metadata only when every candidate operation succeeded.
    validated_metadata: Option<ReleaseMetadata>,
}

/// Checks every source while staging only the valid prefix of a candidate.
async fn validate_and_stage_candidate(
    source: Arc<dyn ImportSource>,
    candidate: &ImportCandidate,
    storage: Option<&dyn MusicStorage>,
) -> CandidatePreparationResult {
    let mut validator = ImportedReleaseValidator::default();
    let mut release_id = None;
    let mut external_failure = false;

    for audio_key in candidate.audio_keys() {
        let audio = source
            .read_audio(audio_key)
            .await
            .map_err(ImportCandidateError::from)
            .and_then(|bytes| validator.push(bytes).map_err(ImportCandidateError::from));

        let checked = async {
            let audio = audio?;
            let first_audio = release_id.is_none();
            release_id.get_or_insert_with(|| audio.release_id().clone());
            if !external_failure
                && !validator.failed()
                && let Some(storage) = storage
            {
                if first_audio {
                    storage.delete_staging(Some(audio.release_id())).await?;
                }
                storage.stage_audio(audio).await?;
            }
            Ok::<_, ImportCandidateError>(())
        }
        .await;

        if let Err(error) = checked {
            external_failure |= !matches!(error, ImportCandidateError::Validation(_));
            tracing::warn!(candidate = %candidate.key(), audio = %audio_key, error = %error, "Audio import check failed");
        }
    }

    let validated_metadata = match validator.finish() {
        Ok(release) if !external_failure => Some(release),
        Ok(_) => None,
        Err(error) => {
            if !external_failure {
                tracing::warn!(candidate = %candidate.key(), error = %error, "Release validation failed");
            }
            None
        }
    };
    CandidatePreparationResult {
        release_id,
        validated_metadata,
    }
}

/// Verifies incremental validation, persistence boundaries, and cleanup.
#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, io::Error as IoError, sync::Mutex};

    use super::*;
    use crate::{
        domain::{
            import::ImportAudio, ingestion::metadata::sample_metadata, primitive::release::Release,
        },
        port::music_storage::{ReleaseScanFailure, ScanReleasesError},
        test_support::sample_audio,
    };

    /// Supplies ordered candidates and records every attempted source read.
    struct Source {
        candidates: Vec<Vec<String>>,
        reads: Mutex<Vec<String>>,
        events: Arc<Mutex<Vec<&'static str>>>,
        unavailable: bool,
    }

    #[async_trait::async_trait]
    impl ImportSource for Source {
        async fn list_releases(&self) -> Result<Vec<ImportCandidate>, ListImportReleasesError> {
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

        async fn read_audio(&self, key: &str) -> Result<Vec<u8>, ReadImportAudioError> {
            self.reads.lock().unwrap().push(key.into());
            self.events.lock().unwrap().push("read");
            if key == "unreadable" {
                return Err(ReadImportAudioError::InvalidObject { key: key.into() });
            }
            if key == "invalid" {
                return Ok(b"invalid".to_vec());
            }
            let mut metadata = sample_metadata();
            let (release, track) = key.split_once('/').unwrap();
            metadata.release_title_mut().push_str(release);
            *metadata.track_number_mut() = track.into();
            Ok(sample_audio(&metadata))
        }
    }

    /// Records storage boundaries and injects staging, publication, or cleanup failures.
    #[derive(Default)]
    struct Storage {
        events: Arc<Mutex<Vec<&'static str>>>,
        staged: Mutex<Vec<Vec<u8>>>,
        committed: Mutex<Vec<ReleaseId>>,
        deleted: Mutex<Vec<Option<ReleaseId>>>,
        outcomes: Mutex<VecDeque<Result<SaveReleaseOutcome, ()>>>,
        fail_stage: bool,
        fail_cleanup: bool,
    }

    #[async_trait::async_trait]
    impl MusicStorage for Storage {
        async fn stage_audio(&self, audio: ImportAudio) -> Result<(), SaveReleaseError> {
            let id = audio.release_id().clone();
            self.events.lock().unwrap().push("stage");
            self.staged.lock().unwrap().push(audio.into_bytes());
            if self.fail_stage {
                return Err(SaveReleaseError::Conflict { id: id.clone() });
            }
            Ok(())
        }

        async fn commit_release(
            &self,
            id: &ReleaseId,
            _: bool,
        ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
            self.events.lock().unwrap().push("commit");
            self.committed.lock().unwrap().push(id.clone());
            self.outcomes
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(SaveReleaseOutcome::Saved))
                .map_err(|()| SaveReleaseError::Conflict { id: id.clone() })
        }

        async fn delete_staging(&self, id: Option<&ReleaseId>) -> Result<(), DeleteStagingError> {
            self.events.lock().unwrap().push("delete");
            self.deleted.lock().unwrap().push(id.cloned());
            if self.fail_cleanup && id.is_none() {
                return Err(DeleteStagingError::new(IoError::other("cleanup failed")));
            }
            Ok(())
        }

        async fn scan_releases(
            &self,
        ) -> Result<Vec<Result<Release, ReleaseScanFailure>>, ScanReleasesError> {
            panic!("import must not scan storage")
        }
    }

    /// Builds independent source candidates from readable key lists.
    fn source(candidates: &[&[&str]]) -> Arc<Source> {
        Arc::new(Source {
            candidates: candidates
                .iter()
                .map(|keys| keys.iter().map(|key| (*key).into()).collect())
                .collect(),
            reads: Mutex::new(Vec::new()),
            events: Arc::default(),
            unavailable: false,
        })
    }

    #[tokio::test]
    async fn execute_stops_staging_after_failure_but_checks_every_remaining_audio() {
        let source = source(&[
            &["a/1", "invalid", "unreadable", "a/2"],
            &["a/3", "a/4"],
            &["b/1", "b/1", "b/2"],
            &["c/1", "d/2", "c/3"],
            &["e/1", "e/2"],
        ]);
        let storage = Arc::new(Storage {
            events: source.events.clone(),
            ..Storage::default()
        });
        let result = ImportMusicUseCase::new(source.clone(), storage.clone())
            .execute(false)
            .await
            .unwrap();
        assert_eq!(
            (result.saved(), result.skipped(), result.failed()),
            (1, 0, 4)
        );
        assert_eq!(source.reads.lock().unwrap().len(), 14);
        assert_eq!(
            &source.events.lock().unwrap()[..6],
            &["read", "delete", "stage", "read", "read", "read"]
        );
        // Duplicate candidates are staged but never committed.
        let staged = storage.staged.lock().unwrap();
        assert_eq!(staged.len(), 7);
        let titles: Vec<_> = staged
            .iter()
            .map(|bytes| {
                crate::test_support::validate_import_release(vec![bytes.clone()])
                    .unwrap()
                    .title()
                    .clone()
            })
            .collect();
        assert!(titles[0].ends_with('a'));
        assert!(titles[1].ends_with('a') && titles[2].ends_with('a'));
        assert!(titles[3].ends_with('b'));
        assert!(titles[4].ends_with('c'));
        assert!(titles[5].ends_with('e') && titles[6].ends_with('e'));
        assert_eq!(storage.committed.lock().unwrap().len(), 1);
        let deleted = storage.deleted.lock().unwrap();
        assert_eq!(deleted.len(), 6);
        assert!(deleted.last().unwrap().is_none());
    }

    #[tokio::test]
    async fn execute_continues_validation_after_staging_failure_and_cleans_up() {
        let source = source(&[&["a/1", "a/2", "invalid"], &["b/1"]]);
        let storage = Arc::new(Storage {
            fail_stage: true,
            ..Storage::default()
        });
        let result = ImportMusicUseCase::new(source.clone(), storage.clone())
            .execute(false)
            .await
            .unwrap();
        assert_eq!(result.failed(), 2);
        assert_eq!(source.reads.lock().unwrap().len(), 4);
        assert_eq!(storage.staged.lock().unwrap().len(), 2);
        assert!(storage.committed.lock().unwrap().is_empty());
        assert!(storage.deleted.lock().unwrap().last().unwrap().is_none());
    }

    #[tokio::test]
    async fn execute_counts_publication_outcomes_and_reports_cleanup_failure() {
        let storage = Arc::new(Storage {
            outcomes: Mutex::new(VecDeque::from([
                Err(()),
                Ok(SaveReleaseOutcome::AlreadyExists),
                Ok(SaveReleaseOutcome::Saved),
            ])),
            ..Storage::default()
        });
        let result =
            ImportMusicUseCase::new(source(&[&["a/1"], &["b/1"], &["c/1"]]), storage.clone())
                .execute(false)
                .await
                .unwrap();
        assert_eq!(
            (result.saved(), result.skipped(), result.failed()),
            (1, 1, 1)
        );
        assert!(storage.deleted.lock().unwrap().last().unwrap().is_none());
        let storage = Arc::new(Storage {
            fail_cleanup: true,
            ..Storage::default()
        });
        assert!(matches!(
            ImportMusicUseCase::new(source(&[&["a/1"]]), storage)
                .execute(false)
                .await,
            Err(ImportMusicError::Cleanup(_))
        ));
    }

    #[tokio::test]
    async fn import_check_rejects_duplicates_and_keeps_only_release_metadata() {
        let source = source(&[&["a/1", "a/2"], &["a/3"], &[]]);
        let checked = ImportCheckUseCase::new(source).execute().await.unwrap();
        assert_eq!(checked.releases().len(), 1);
        assert_eq!(checked.releases()[0].0, "0");
        assert_eq!(checked.failed(), 2);
    }

    #[tokio::test]
    async fn execute_reserves_failed_candidates_and_commits_only_the_first_successful_candidate() {
        let storage = Arc::new(Storage::default());
        let result = ImportMusicUseCase::new(
            source(&[
                &["a/1"],
                &["a/2"],
                &["unreadable", "b/1"],
                &["b/2"],
                &["invalid", "c/1"],
                &["c/2"],
            ]),
            storage.clone(),
        )
        .execute(false)
        .await
        .unwrap();
        assert_eq!(
            (result.saved(), result.skipped(), result.failed()),
            (1, 0, 5)
        );
        let committed = storage.committed.lock().unwrap();
        assert_eq!(committed.len(), 1);
        let first = crate::test_support::validate_import_release(
            storage.staged.lock().unwrap()[..1].to_vec(),
        )
        .unwrap();
        assert_eq!(&committed[0], first.id());
    }

    #[tokio::test]
    async fn execute_propagates_enumeration_failures_without_storage_operations() {
        let source = Arc::new(Source {
            candidates: vec![],
            reads: Mutex::new(vec![]),
            events: Arc::default(),
            unavailable: true,
        });
        let storage = Arc::new(Storage::default());
        assert!(matches!(
            ImportMusicUseCase::new(source, storage.clone())
                .execute(false)
                .await,
            Err(ImportMusicError::ListReleases(_))
        ));
        assert!(storage.staged.lock().unwrap().is_empty());
        assert!(storage.deleted.lock().unwrap().is_empty());
    }
}

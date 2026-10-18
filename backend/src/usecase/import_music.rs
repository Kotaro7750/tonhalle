//! Imports independent release candidates without accessing the metadata index.

use std::sync::Arc;

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
    /// The source could not enumerate all release candidates.
    #[error(transparent)]
    ListReleases(#[from] ListImportReleasesError),
}

/// A candidate failure reported while other releases continue importing.
#[derive(Debug, thiserror::Error)]
enum ImportCandidateError {
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
    pub(crate) fn execute(&self) -> Result<ImportSummary, ImportMusicError> {
        let candidates = self.source.list_releases()?;
        let mut summary = ImportSummary::default();
        for candidate in candidates {
            let result = (|| -> Result<SaveReleaseOutcome, ImportCandidateError> {
                let bytes = candidate
                    .audio_keys()
                    .iter()
                    .map(|key| self.source.read_audio(key))
                    .collect::<Result<Vec<_>, _>>()?;
                let release = ImportRelease::new(bytes)?;
                Ok(self.storage.save_release(&release)?)
            })();
            match result {
                Ok(SaveReleaseOutcome::Saved) => summary.saved += 1,
                Ok(SaveReleaseOutcome::AlreadyExists) => summary.skipped += 1,
                Err(error) => {
                    summary.failed += 1;
                    tracing::warn!(candidate = %candidate.key(), error = %error, "Release import failed");
                }
            }
        }
        Ok(summary)
    }
}

//! Validates individual import audio and accumulates release constraints without retaining bytes.

use std::collections::HashSet;

use crate::domain::{
    id::{ReleaseId, TrackId},
    ingestion::{IngestTrackError, parse_audio},
    primitive::{audio_file::AudioProperties, release::ReleaseMetadata},
};

/// A rejected import candidate, leaving every source unchanged.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportValidationError {
    /// A release must contain audio.
    #[error("release contains no audio")]
    Empty,
    /// At least one audio failed validation.
    #[error("release contains invalid audio")]
    Failed,
    /// One source failed shared metadata validation.
    #[error(transparent)]
    Audio(#[from] IngestTrackError),
    /// Every source must carry identical release metadata.
    #[error("inconsistent release metadata")]
    MetadataMismatch,
    /// Imports reject multiple originals of the same track.
    #[error("duplicate track: {0}")]
    DuplicateTrack(TrackId),
}

/// A validated source audio retained only until its individual staging write completes.
#[derive(getset::Getters)]
pub(crate) struct ImportAudio {
    /// Identifies the release extracted from this source.
    #[getset(get = "pub(crate)")]
    release_id: ReleaseId,
    #[getset(get = "pub(crate)")]
    /// Describes the validated media format and playback properties.
    properties: AudioProperties,
    #[getset(get = "pub(crate)")]
    /// Preserves the original source for its staging write.
    bytes: Vec<u8>,
}

impl ImportAudio {
    /// Transfers the original bytes to the storage adapter without copying them.
    pub(crate) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Incrementally validates a release while retaining metadata rather than audio bytes.
#[derive(Default, getset::CopyGetters)]
pub(crate) struct ImportedReleaseValidator {
    /// Retains the first accepted source's release metadata.
    metadata: Option<ReleaseMetadata>,
    /// Detects repeated originals without retaining their audio bytes.
    tracks: HashSet<TrackId>,
    /// Prevents completion after any rejected audio.
    #[getset(get_copy = "pub(crate)")]
    failed: bool,
}

impl ImportedReleaseValidator {
    /// Parses one source and remembers failures while allowing subsequent validation.
    pub(crate) fn push(&mut self, bytes: Vec<u8>) -> Result<ImportAudio, ImportValidationError> {
        let result = self.validate(bytes);
        self.failed |= result.is_err();
        result
    }

    /// Checks source metadata before constructing audio eligible for staging.
    fn validate(&mut self, bytes: Vec<u8>) -> Result<ImportAudio, ImportValidationError> {
        let (track, metadata, properties) = parse_audio(&bytes)?.into_parts();
        if self
            .metadata
            .as_ref()
            .is_some_and(|previous| previous != &metadata)
        {
            return Err(ImportValidationError::MetadataMismatch);
        }
        if !self.tracks.insert(track.id().clone()) {
            return Err(ImportValidationError::DuplicateTrack(track.id().clone()));
        }
        let release_id = metadata.id().clone();
        self.metadata.get_or_insert(metadata);
        Ok(ImportAudio {
            release_id,
            properties,
            bytes,
        })
    }

    /// Completes a nonempty release after every source has been checked.
    pub(crate) fn finish(self) -> Result<ReleaseMetadata, ImportValidationError> {
        if self.failed {
            return Err(ImportValidationError::Failed);
        }
        self.metadata.ok_or(ImportValidationError::Empty)
    }
}

/// Verifies release-level import constraints independently of metadata field validation.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::ingestion::metadata::sample_metadata, test_support::sample_audio};

    #[test]
    fn imported_release_validator_rejects_empty_release() {
        let validator = ImportedReleaseValidator::default();

        assert!(matches!(
            validator.finish(),
            Err(ImportValidationError::Empty)
        ));
    }

    #[test]
    fn imported_release_validator_rejects_duplicate_audio() {
        let mut validator = ImportedReleaseValidator::default();
        let audio = sample_audio(&sample_metadata());
        validator.push(audio.clone()).unwrap();

        assert!(matches!(
            validator.push(audio),
            Err(ImportValidationError::DuplicateTrack(_))
        ));
    }

    #[test]
    fn imported_release_validator_rejects_release_after_invalid_then_valid_audio() {
        let mut validator = ImportedReleaseValidator::default();
        assert!(validator.push(b"invalid".to_vec()).is_err());
        validator.push(sample_audio(&sample_metadata())).unwrap();

        assert!(matches!(
            validator.finish(),
            Err(ImportValidationError::Failed)
        ));
    }

    #[test]
    fn imported_release_validator_rejects_mismatched_release_metadata() {
        let first = sample_metadata();
        let mut second = first.clone();
        second.release_title_mut().push_str(" changed");
        let first_audio = sample_audio(&first);
        let second_audio = sample_audio(&second);
        ImportedReleaseValidator::default()
            .push(second_audio.clone())
            .unwrap();
        let mut validator = ImportedReleaseValidator::default();
        validator.push(first_audio).unwrap();

        assert!(matches!(
            validator.push(second_audio),
            Err(ImportValidationError::MetadataMismatch)
        ));
    }
}

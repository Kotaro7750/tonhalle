//! Validates individual import audio and accumulates release constraints without retaining bytes.

use std::{
    collections::HashSet,
    io::{self, Read, Seek},
};

use crate::domain::{
    id::{ReleaseId, TrackId},
    ingestion::{IngestTrackError, parse_audio},
    primitive::{audio_file::AudioProperties, release::ReleaseMetadata},
};

/// A seekable audio source independent of its physical storage.
pub(crate) trait AudioReader: Read + Seek + Send {}

impl<T: Read + Seek + Send> AudioReader for T {}

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
    /// Keeps the source open until its staging transfer completes.
    reader: Box<dyn AudioReader>,
}

impl ImportAudio {
    /// Rewinds and transfers the validated source for streaming its original bytes.
    pub(crate) fn into_reader(mut self) -> io::Result<Box<dyn AudioReader>> {
        self.reader.rewind()?;
        Ok(self.reader)
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
    pub(crate) fn push(
        &mut self,
        reader: impl AudioReader + 'static,
    ) -> Result<ImportAudio, ImportValidationError> {
        let result = self.validate(reader);
        self.failed |= result.is_err();
        result
    }

    /// Checks source metadata before constructing audio eligible for staging.
    fn validate(
        &mut self,
        mut reader: impl AudioReader + 'static,
    ) -> Result<ImportAudio, ImportValidationError> {
        let (track, metadata, properties) = parse_audio(&mut reader)?.into_parts();
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
            reader: Box::new(reader),
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
    use std::io::Cursor;

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
        validator.push(Cursor::new(audio.clone())).unwrap();

        assert!(matches!(
            validator.push(Cursor::new(audio)),
            Err(ImportValidationError::DuplicateTrack(_))
        ));
    }

    #[test]
    fn imported_release_validator_rejects_release_after_invalid_then_valid_audio() {
        let mut validator = ImportedReleaseValidator::default();
        assert!(validator.push(Cursor::new(b"invalid".to_vec())).is_err());
        validator
            .push(Cursor::new(sample_audio(&sample_metadata())))
            .unwrap();

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
            .push(Cursor::new(second_audio.clone()))
            .unwrap();
        let mut validator = ImportedReleaseValidator::default();
        validator.push(Cursor::new(first_audio)).unwrap();

        assert!(matches!(
            validator.push(Cursor::new(second_audio)),
            Err(ImportValidationError::MetadataMismatch)
        ));
    }
}

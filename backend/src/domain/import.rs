//! Validates complete releases before assigning storage identities.

use std::collections::HashSet;

use crate::domain::{
    id::{ReleaseId, TrackId},
    ingestion::{IngestTrackError, parse_audio},
    primitive::{audio_file::AudioProperties, release::ReleaseMetadata, track::TrackMetadata},
};

/// A rejected import candidate, leaving every source unchanged.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportValidationError {
    /// A release must contain audio.
    #[error("release contains no audio")]
    Empty,
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

/// A validated source audio retaining its exact original bytes.
#[derive(getset::Getters)]
pub(crate) struct ImportAudio {
    track_metadata: TrackMetadata,
    /// Provides the validated format and playback properties.
    #[getset(get = "pub(crate)")]
    properties: AudioProperties,
    /// Provides unchanged source bytes.
    #[getset(get = "pub(crate)")]
    bytes: Vec<u8>,
}

/// A complete release eligible for storage with newly issued audio identities.
#[derive(getset::Getters)]
pub(crate) struct ImportRelease {
    metadata: ReleaseMetadata,
    /// Provides every validated source for release-level persistence.
    #[getset(get = "pub(crate)")]
    audio: Vec<ImportAudio>,
}

impl ImportRelease {
    /// Validates all sources with shared domain constructors and rejects duplicate tracks.
    pub(crate) fn new(sources: Vec<Vec<u8>>) -> Result<Self, ImportValidationError> {
        let mut metadata = None;
        let mut audio = Vec::new();
        let mut tracks = HashSet::new();
        for bytes in sources {
            let parsed = parse_audio(&bytes)?;
            let (track_metadata, current, properties) = parsed.into_parts();
            let source = ImportAudio {
                track_metadata,
                properties,
                bytes,
            };
            if metadata
                .as_ref()
                .is_some_and(|previous| previous != &current)
            {
                return Err(ImportValidationError::MetadataMismatch);
            }
            if !tracks.insert(source.track_metadata.id().clone()) {
                return Err(ImportValidationError::DuplicateTrack(
                    source.track_metadata.id().clone(),
                ));
            }
            metadata = Some(current);
            audio.push(source);
        }
        Ok(Self {
            metadata: metadata.ok_or(ImportValidationError::Empty)?,
            audio,
        })
    }

    /// Identifies the release exclusively from embedded metadata.
    pub(crate) fn id(&self) -> &ReleaseId {
        self.metadata.id()
    }
}

/// Verifies import-specific rejection while sharing metadata constraints with indexing.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ingestion::metadata::sample_metadata;
    use crate::test_support::sample_audio;

    #[test]
    fn import_release_rejects_duplicates_inconsistent_metadata_and_invalid_sources() {
        let fixture = sample_metadata();
        let original = sample_audio(&fixture);
        assert!(matches!(
            ImportRelease::new(vec![]),
            Err(ImportValidationError::Empty)
        ));
        assert!(matches!(
            ImportRelease::new(vec![original.clone(), original.clone()]),
            Err(ImportValidationError::DuplicateTrack(_))
        ));

        let mut changed = sample_metadata();
        changed.release_title_mut().push_str(" changed");
        assert!(matches!(
            ImportRelease::new(vec![original.clone(), sample_audio(&changed)]),
            Err(ImportValidationError::MetadataMismatch)
        ));
        for field in 0..4 {
            let mut invalid = sample_metadata();
            match field {
                0 => invalid.track_title_mut().clear(),
                1 => *invalid.release_date_mut() = "2026-02-30".into(),
                2 => *invalid.disc_number_mut() = "0".into(),
                _ => invalid.track_credit_mut().names_mut().clear(),
            }
            assert!(ImportRelease::new(vec![original.clone(), sample_audio(&invalid)]).is_err());
        }
    }
}

//! Validates complete releases before assigning storage identities.

use std::collections::HashSet;

use crate::domain::{
    ReleaseId, TrackId,
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
pub(crate) struct ImportAudio {
    track_metadata: TrackMetadata,
    properties: AudioProperties,
    bytes: Vec<u8>,
}

impl ImportAudio {
    /// Provides the validated format and playback properties.
    pub(crate) fn properties(&self) -> &AudioProperties {
        &self.properties
    }

    /// Provides unchanged source bytes.
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A complete release eligible for storage with newly issued audio identities.
pub(crate) struct ImportRelease {
    metadata: ReleaseMetadata,
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
            let current = parsed.release_metadata;
            let source = ImportAudio {
                track_metadata: parsed.track_metadata,
                properties: parsed.properties,
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

    /// Provides every validated source for release-level persistence.
    pub(crate) fn audio(&self) -> &[ImportAudio] {
        &self.audio
    }
}

/// Verifies import-specific rejection while sharing metadata constraints with indexing.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{sample_audio, sample_fixture};

    #[test]
    fn import_release_rejects_duplicates_inconsistent_metadata_and_invalid_sources() {
        let fixture = sample_fixture();
        let original = sample_audio(&fixture.build());
        assert!(matches!(
            ImportRelease::new(vec![]),
            Err(ImportValidationError::Empty)
        ));
        assert!(matches!(
            ImportRelease::new(vec![original.clone(), original.clone()]),
            Err(ImportValidationError::DuplicateTrack(_))
        ));

        let mut changed = sample_fixture();
        changed.release_title.push_str(" changed");
        assert!(matches!(
            ImportRelease::new(vec![original.clone(), sample_audio(&changed.build())]),
            Err(ImportValidationError::MetadataMismatch)
        ));
        for field in 0..4 {
            let mut invalid = sample_fixture();
            match field {
                0 => invalid.track_title.clear(),
                1 => invalid.release_date = "2026-02-30".into(),
                2 => invalid.disc_number = "0".into(),
                _ => invalid.track_artists.names.clear(),
            }
            assert!(
                ImportRelease::new(vec![original.clone(), sample_audio(&invalid.build())]).is_err()
            );
        }
    }
}

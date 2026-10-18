//! Composes audio extraction and validated entity construction for storage adapters.

use metadata::ExtractMetadataError;
use url::Url;

use crate::domain::{
    id::{AudioFileId, ReleaseId},
    primitive::{
        audio_file::{AudioFile, AudioFileValidationError, AudioProperties},
        release::{
            Release, ReleaseMetadata, ReleaseMetadataValidationError, ReleaseValidationError,
        },
        track::{Track, TrackMetadata, TrackValidationError},
    },
};

pub(crate) mod metadata;

/// A failure to ingest a track or establish its membership in the stored release.
#[derive(Debug, thiserror::Error)]
pub(crate) enum IngestTrackError {
    /// Audio bytes cannot supply readable metadata and playback properties.
    #[error(transparent)]
    ExtractMetadata(#[from] ExtractMetadataError),
    /// Embedded release metadata violates a domain constraint.
    #[error(transparent)]
    InvalidReleaseMetadata(#[from] ReleaseMetadataValidationError),
    /// Measured audio properties violate a domain constraint.
    #[error(transparent)]
    InvalidAudioFile(#[from] AudioFileValidationError),
    /// Embedded track metadata violates a domain constraint.
    #[error(transparent)]
    InvalidTrack(#[from] TrackValidationError),
    /// Embedded metadata identifies a release other than the stored parent.
    #[error("embedded metadata identifies a different release: {actual}")]
    ReleaseIdMismatch { actual: ReleaseId },
}

/// Extracts audio metadata and validates its entities against the parent release identity.
pub(crate) fn ingest_track(
    bytes: &[u8],
    audio_file_id: AudioFileId,
    object_key: String,
    playback_url: Url,
    release_id: &ReleaseId,
) -> Result<(Track, ReleaseMetadata), IngestTrackError> {
    let parsed = parse_audio(bytes)?;
    if parsed.release_metadata.id() != release_id {
        return Err(IngestTrackError::ReleaseIdMismatch {
            actual: parsed.release_metadata.id().clone(),
        });
    }
    let audio =
        AudioFile::from_properties(audio_file_id, object_key, playback_url, parsed.properties);
    Ok((
        Track::new(parsed.track_metadata, audio),
        parsed.release_metadata,
    ))
}

/// Validated embedded metadata and audio properties without stored entities.
pub(crate) struct ParsedAudio {
    /// Describes the track and its metadata-derived identity.
    track_metadata: TrackMetadata,
    /// Describes the release identified by the embedded tags.
    release_metadata: ReleaseMetadata,
    /// Describes audio independently of its storage location.
    properties: AudioProperties,
}

impl ParsedAudio {
    /// Transfers validated track metadata, release metadata, and audio properties to their owner.
    pub(crate) fn into_parts(self) -> (TrackMetadata, ReleaseMetadata, AudioProperties) {
        (self.track_metadata, self.release_metadata, self.properties)
    }
}

/// Extracts and validates audio without requiring stored identity or playback links.
pub(crate) fn parse_audio(bytes: &[u8]) -> Result<ParsedAudio, IngestTrackError> {
    let raw = metadata::extract_metadata(bytes)?;
    let release_metadata = ReleaseMetadata::new(
        raw.release_title(),
        raw.release_date(),
        raw.release_credit().names(),
        raw.release_credit().mbids(),
    )?;
    let properties = AudioProperties::new(
        raw.audio().media_type(),
        raw.audio().sample_rate(),
        raw.audio().bit_depth(),
        raw.audio().channels(),
        raw.audio().bitrate(),
    )?;
    let track_metadata = TrackMetadata::new(
        release_metadata.id(),
        raw.track_title(),
        raw.disc_number(),
        raw.track_number(),
        raw.audio().duration_ms(),
        raw.track_credit().names(),
        raw.track_credit().mbids(),
    )?;
    Ok(ParsedAudio {
        track_metadata,
        release_metadata,
        properties,
    })
}

/// Constructs one release after all of its audio sources have been parsed successfully.
pub(crate) fn build_release(
    id: ReleaseId,
    tracks: Vec<(Track, ReleaseMetadata)>,
) -> Result<Release, ReleaseValidationError> {
    Release::new(id, tracks)
}

/// Verifies metadata constraints and entity identity rules.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MetadataFixture, release_metadata, sample_metadata};

    #[test]
    fn ingest_track_rejects_missing_required_values_without_defaults() {
        let source = sample_metadata();
        let release_id = release_metadata(&source).unwrap().id().clone();

        let mut variants = Vec::new();
        for field in 0..9 {
            let mut invalid = MetadataFixture::from(&source);

            match field {
                0 => invalid.release_title_mut().clear(),
                1 => *invalid.release_date_mut() = "2026-02-29".into(),
                2 => invalid.release_artists_mut().names_mut().clear(),
                3 => *invalid.track_title_mut() = "\t\n".into(),
                4 => invalid.track_artists_mut().names_mut().clear(),
                5 => *invalid.track_number_mut() = "0".into(),
                6 => *invalid.audio_file_mut().sample_rate_mut() = 0,
                7 => *invalid.audio_file_mut().bit_depth_mut() = Some(0),
                8 => *invalid.audio_file_mut().channels_mut() = 0,
                _ => unreachable!(),
            }

            variants.push(invalid);
        }

        for invalid in variants {
            assert!(invalid.ingest_track(&release_id).is_err());
        }
    }

    #[test]
    fn ingest_track_ignores_release_and_recording_mbids() {
        use crate::test_support::{sample_audio, sample_audio_with_mbids};

        let source = sample_metadata();
        let release_id = release_metadata(&source).unwrap().id().clone();
        let storage = MetadataFixture::from(&source).audio_file().clone();
        let parse = |bytes: &[u8]| {
            ingest_track(
                bytes,
                storage.id(),
                storage.object_key().clone(),
                storage.playback_url().clone(),
                &release_id,
            )
        };

        let plain = parse(&sample_audio(&source)).unwrap();
        let tagged = parse(&sample_audio_with_mbids(
            &source,
            Some("7b0857ed-11ee-4426-9899-16f4e415322f"),
            Some("f7b87070-d876-4a11-997a-048cdff1a87d"),
        ))
        .unwrap();

        assert_eq!(plain.0.id(), tagged.0.id());
        assert_eq!(plain.1, tagged.1);

        let mut changed = MetadataFixture::from(&source);
        changed.track_title_mut().push_str(" Revised");

        let changed_track = parse(&sample_audio_with_mbids(
            &changed.build(),
            Some("7b0857ed-11ee-4426-9899-16f4e415322f"),
            Some("f7b87070-d876-4a11-997a-048cdff1a87d"),
        ))
        .unwrap();

        assert_ne!(plain.0.id(), changed_track.0.id());

        changed.release_title_mut().push_str(" Revised");
        let actual = release_metadata(&changed.build()).unwrap().id().clone();

        assert!(matches!(
            parse(&sample_audio_with_mbids(
                &changed.build(),
                Some("7b0857ed-11ee-4426-9899-16f4e415322f"),
                Some("f7b87070-d876-4a11-997a-048cdff1a87d"),
            )),
            Err(IngestTrackError::ReleaseIdMismatch { actual: id }) if id == actual
        ));
    }

    #[test]
    fn ingest_track_preserves_audio_identity_when_release_metadata_changes() {
        let source = sample_metadata();
        let first = release_metadata(&source).unwrap();
        let (original, _) = MetadataFixture::from(&source)
            .ingest_track(first.id())
            .unwrap();

        let mut changed = MetadataFixture::from(&source);

        changed.release_title_mut().push_str(" Revised");

        let second = release_metadata(&changed.build()).unwrap();
        let (updated, _) = changed.ingest_track(second.id()).unwrap();

        assert_ne!(original.id(), updated.id());
        assert_eq!(
            updated.original_audio_file().id(),
            changed.audio_file().id()
        );
    }
}

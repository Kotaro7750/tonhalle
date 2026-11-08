//! Composes audio extraction and validated entity construction for storage adapters.

use metadata::ExtractMetadataError;
use std::io::{Read, Seek};
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
    reader: impl Read + Seek,
    audio_file_id: AudioFileId,
    object_key: String,
    playback_url: Url,
    release_id: &ReleaseId,
) -> Result<(Track, ReleaseMetadata), IngestTrackError> {
    let parsed = parse_audio(reader)?;
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
pub(crate) fn parse_audio(reader: impl Read + Seek) -> Result<ParsedAudio, IngestTrackError> {
    let raw = metadata::extract_metadata(reader)?;
    let release_metadata = ReleaseMetadata::new(
        raw.release_title(),
        raw.release_date().as_deref(),
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
    use std::io::Cursor;

    use super::*;
    use crate::domain::ingestion::metadata::{RawMetadata, sample_metadata};
    use crate::test_support::{release_metadata, sample_audio, sample_audio_file};

    /// Supplies stable storage information to ingestion tests.
    fn ingest_sample(
        raw: &RawMetadata,
        release_id: &ReleaseId,
    ) -> Result<(Track, ReleaseMetadata), IngestTrackError> {
        let audio = sample_audio_file();
        ingest_track(
            Cursor::new(sample_audio(raw)),
            audio.id(),
            audio.object_key().into(),
            audio.playback_url().clone(),
            release_id,
        )
    }

    #[test]
    fn ingest_track_preserves_identity_and_audio_id_when_release_date_changes_or_is_absent() {
        let source = sample_metadata();
        let release_id = release_metadata(&source).unwrap().id().clone();
        let original = ingest_sample(&source, &release_id).unwrap();

        for date in [
            None,
            Some(""),
            Some(" \t　"),
            Some("2026"),
            Some("2026-10"),
            Some("2024-02-29"),
        ] {
            let mut changed = source.clone();
            *changed.release_date_mut() = date.map(str::to_owned);
            let (track, metadata) = ingest_sample(&changed, &release_id).unwrap();
            let release =
                build_release(release_id.clone(), vec![(track.clone(), metadata)]).unwrap();

            assert_eq!(track.id(), original.0.id());
            assert_eq!(
                track.original_audio_file().id(),
                original.0.original_audio_file().id()
            );
            assert_eq!(
                release.release_date().as_deref(),
                date.filter(|value| !value.trim().is_empty())
            );
        }
    }

    #[test]
    fn ingest_track_rejects_missing_required_values_without_defaults() {
        let source = sample_metadata();
        let release_id = release_metadata(&source).unwrap().id().clone();

        let mut variants = Vec::new();
        for field in 0..9 {
            let mut invalid = source.clone();

            match field {
                0 => invalid.release_title_mut().clear(),
                1 => *invalid.release_date_mut() = Some("2026-02-29".into()),
                2 => invalid.release_credit_mut().names_mut().clear(),
                3 => *invalid.track_title_mut() = "\t\n".into(),
                4 => invalid.track_credit_mut().names_mut().clear(),
                5 => *invalid.track_number_mut() = "0".into(),
                6 => *invalid.audio_mut().sample_rate_mut() = 0,
                7 => *invalid.audio_mut().bit_depth_mut() = Some(0),
                8 => *invalid.audio_mut().channels_mut() = 0,
                _ => unreachable!(),
            }

            variants.push(invalid);
        }

        for invalid in variants {
            assert!(ingest_sample(&invalid, &release_id).is_err());
        }
    }

    #[test]
    fn ingest_track_ignores_release_and_recording_mbids() {
        use crate::test_support::{sample_audio, sample_audio_with_mbids};

        let source = sample_metadata();
        let release_id = release_metadata(&source).unwrap().id().clone();
        let storage = sample_audio_file();
        let parse = |bytes: &[u8]| {
            ingest_track(
                Cursor::new(bytes),
                storage.id(),
                storage.object_key().into(),
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

        let mut changed = source.clone();
        changed.track_title_mut().push_str(" Revised");

        let changed_track = parse(&sample_audio_with_mbids(
            &changed,
            Some("7b0857ed-11ee-4426-9899-16f4e415322f"),
            Some("f7b87070-d876-4a11-997a-048cdff1a87d"),
        ))
        .unwrap();

        assert_ne!(plain.0.id(), changed_track.0.id());

        changed.release_title_mut().push_str(" Revised");
        let actual = release_metadata(&changed).unwrap().id().clone();

        assert!(matches!(
            parse(&sample_audio_with_mbids(
                &changed,
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
        let (original, _) = ingest_sample(&source, first.id()).unwrap();

        let mut changed = source.clone();

        changed.release_title_mut().push_str(" Revised");

        let second = release_metadata(&changed).unwrap();
        let (updated, _) = ingest_sample(&changed, second.id()).unwrap();

        assert_ne!(original.id(), updated.id());
        assert_eq!(updated.original_audio_file().id(), sample_audio_file().id());
    }
}

//! Composes audio extraction and validated entity construction for storage adapters.

pub(crate) mod metadata;

use url::Url;

use crate::domain::{
    AudioFileId, ReleaseId,
    primitive::{
        audio_file::{AudioFile, AudioFileValidationError},
        release::{
            Release, ReleaseMetadata, ReleaseMetadataValidationError, ReleaseValidationError,
        },
        track::{Track, TrackValidationError},
    },
};
use metadata::ExtractMetadataError;

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
    let raw = metadata::extract_metadata(bytes)?;

    let metadata = ReleaseMetadata::new(
        raw.release_title(),
        raw.release_date(),
        raw.release_artist_names(),
        raw.release_artist_mbids(),
    )?;

    if metadata.id() != release_id {
        return Err(IngestTrackError::ReleaseIdMismatch {
            actual: metadata.id().clone(),
        });
    }

    let audio_file = AudioFile::new(
        audio_file_id,
        object_key,
        playback_url,
        raw.sample_rate(),
        raw.bit_depth(),
        raw.channels(),
        raw.bitrate(),
    )?;

    let track = Track::new(
        release_id,
        raw.track_title(),
        raw.disc_number(),
        raw.track_number(),
        raw.duration_ms(),
        raw.track_artist_names(),
        raw.track_artist_mbids(),
        audio_file,
    )?;

    Ok((track, metadata))
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
        for field in 0..10 {
            let mut invalid = MetadataFixture::from(&source);

            match field {
                0 => invalid.release_title.clear(),
                1 => invalid.release_date = "2026-02-29".into(),
                2 => invalid.release_artists.names.clear(),
                3 => invalid.track_title = "\t\n".into(),
                4 => invalid.track_artists.names.clear(),
                5 => invalid.track_number = "0".into(),
                6 => invalid.audio_file.sample_rate = 0,
                7 => invalid.audio_file.bit_depth = 0,
                8 => invalid.audio_file.channels = 0,
                9 => invalid.audio_file.bitrate = 0,
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
        let storage = MetadataFixture::from(&source).audio_file;
        let parse = |bytes: &[u8]| {
            ingest_track(
                bytes,
                storage.id,
                storage.object_key.clone(),
                storage.playback_url.clone(),
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
        changed.track_title.push_str(" Revised");

        let changed_track = parse(&sample_audio_with_mbids(
            &changed.build(),
            Some("7b0857ed-11ee-4426-9899-16f4e415322f"),
            Some("f7b87070-d876-4a11-997a-048cdff1a87d"),
        ))
        .unwrap();

        assert_ne!(plain.0.id(), changed_track.0.id());

        changed.release_title.push_str(" Revised");
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

        changed.release_title.push_str(" Revised");

        let second = release_metadata(&changed.build()).unwrap();
        let (updated, _) = changed.ingest_track(second.id()).unwrap();

        assert_ne!(original.id(), updated.id());
        assert_eq!(updated.original_audio_file().id(), changed.audio_file.id);
    }
}

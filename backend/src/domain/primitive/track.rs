//! Defines release tracks and validates their metadata and original audio association.

use serde_json::json;

use super::audio_file::AudioFile;
use crate::domain::artist_credit::{ArtistCreditValidationError, artist_credits};
use crate::domain::{ArtistCredit, ReleaseId, TrackId, normalization};

/// A failure to construct a track from its required metadata.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TrackValidationError {
    /// The track title is empty after identity normalization.
    #[error("invalid metadata: missing track title")]
    MissingTitle,
    /// The disc position is not a positive integer.
    #[error("invalid metadata: invalid disc/track number")]
    InvalidDiscNumber,
    /// The track position is not a positive integer.
    #[error("invalid metadata: invalid disc/track number")]
    InvalidTrackNumber,
    /// The playback duration exceeds the domain's numeric range.
    #[error("invalid metadata: duration overflow")]
    DurationOverflow,
    /// Performer names cannot form valid artist credits.
    #[error(transparent)]
    ArtistCredits(#[from] ArtistCreditValidationError),
}

/// Validated track metadata independent of stored audio identity and location.
#[derive(getset::CopyGetters, getset::Getters, Debug, Clone)]
pub(crate) struct TrackMetadata {
    /// Identifies the track from its normalized release and track metadata.
    #[getset(get = "pub(crate)")]
    id: TrackId,
    /// Connects this track to the release used during identity generation.
    #[getset(get = "pub(crate)")]
    release_id: ReleaseId,
    /// Preserves the track title supplied by the source metadata.
    #[getset(get = "pub(crate)")]
    title: String,
    /// Provides the validated disc position for release ordering.
    #[getset(get_copy = "pub(crate)")]
    disc_number: u32,
    /// Provides the validated track position within its disc.
    #[getset(get_copy = "pub(crate)")]
    track_number: u32,
    /// Provides playback duration in milliseconds, including valid zero durations.
    #[getset(get_copy = "pub(crate)")]
    duration_ms: Option<u32>,
    /// Keeps track credits separate from release credits.
    #[getset(get = "pub(crate)")]
    artist_credits: Vec<ArtistCredit>,
}

impl TrackMetadata {
    /// Validates track fields and derives an identity while preserving display values.
    pub(crate) fn new(
        release_id: &ReleaseId,
        title: &str,
        disc_number: &str,
        track_number: &str,
        duration_ms: Option<u128>,
        artist_names: &[String],
        artist_mbids: &[String],
    ) -> Result<Self, TrackValidationError> {
        let normalized_title =
            normalization::required(title).ok_or(TrackValidationError::MissingTitle)?;
        let disc_number = number(disc_number).ok_or(TrackValidationError::InvalidDiscNumber)?;
        let track_number = number(track_number).ok_or(TrackValidationError::InvalidTrackNumber)?;
        let artist_credits = artist_credits(artist_names, artist_mbids)?;

        let id = TrackId::from_metadata(json!([
            release_id,
            disc_number.to_string(),
            track_number.to_string(),
            normalized_title
        ]));

        let duration_ms = duration_ms
            .map(u32::try_from)
            .transpose()
            .map_err(|_| TrackValidationError::DurationOverflow)?;

        Ok(Self {
            id,
            release_id: release_id.clone(),
            title: title.to_owned(),
            disc_number,
            track_number,
            duration_ms,
            artist_credits,
        })
    }
}

/// A release track associated with its stored original audio file.
#[derive(getset::Getters, Debug, Clone)]
pub(crate) struct Track {
    metadata: TrackMetadata,
    /// Provides the original audio retained during index construction.
    #[getset(get = "pub(crate)")]
    original_audio_file: AudioFile,
}

impl Track {
    /// Combines validated track metadata with its stored original audio.
    pub(crate) fn new(metadata: TrackMetadata, original_audio_file: AudioFile) -> Self {
        Self {
            metadata,
            original_audio_file,
        }
    }

    /// Identifies the track from its normalized release and track metadata.
    pub(crate) fn id(&self) -> &TrackId {
        self.metadata.id()
    }

    /// Connects this track to the release used during identity generation.
    pub(crate) fn release_id(&self) -> &ReleaseId {
        self.metadata.release_id()
    }

    /// Preserves the track title supplied by the source metadata.
    pub(crate) fn title(&self) -> &str {
        self.metadata.title()
    }

    /// Provides the validated disc position for release ordering.
    pub(crate) fn disc_number(&self) -> u32 {
        self.metadata.disc_number()
    }

    /// Provides the validated track position within its disc.
    pub(crate) fn track_number(&self) -> u32 {
        self.metadata.track_number()
    }

    /// Provides playback duration in milliseconds, including valid zero durations.
    pub(crate) fn duration_ms(&self) -> Option<u32> {
        self.metadata.duration_ms()
    }

    /// Keeps track credits separate from release credits.
    pub(crate) fn artist_credits(&self) -> &[ArtistCredit] {
        self.metadata.artist_credits()
    }
}

/// Validates a positive disc or track number while ignoring an optional total.
fn number(value: &str) -> Option<u32> {
    let number = value.split('/').next().unwrap_or_default();

    if number.is_empty() || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    number.parse::<u32>().ok().filter(|n| *n > 0)
}

/// Verifies validated track positions, required display values, and duration limits.
#[cfg(test)]
mod tests {
    use super::TrackValidationError;
    use crate::domain::{artist_credit::ArtistCreditValidationError, ingestion::IngestTrackError};
    use crate::test_support::sample_fixture;

    #[test]
    fn track_metadata_new_requires_positive_positions_and_ignores_totals() {
        let mut fixture = sample_fixture();
        let release = fixture.build_release_metadata().unwrap();

        *fixture.disc_number_mut() = "001/2".into();
        *fixture.track_number_mut() = "002/12".into();

        let (track, _) = fixture.build_track(release.id()).unwrap();

        assert_eq!(track.disc_number(), 1);
        assert_eq!(track.track_number(), 2);

        *fixture.disc_number_mut() = "1".into();
        *fixture.track_number_mut() = "2".into();

        assert_eq!(
            track.id(),
            fixture.build_track(release.id()).unwrap().0.id()
        );

        for field in ["disc", "track"] {
            for invalid in ["", "0", "-1", "+1", "1.0", " 1"] {
                let mut fixture = sample_fixture();

                match field {
                    "disc" => *fixture.disc_number_mut() = invalid.into(),
                    _ => *fixture.track_number_mut() = invalid.into(),
                }

                let error = fixture.build_track(release.id()).unwrap_err();

                match field {
                    "disc" => assert!(matches!(
                        error,
                        IngestTrackError::InvalidTrack(TrackValidationError::InvalidDiscNumber)
                    )),
                    _ => assert!(matches!(
                        error,
                        IngestTrackError::InvalidTrack(TrackValidationError::InvalidTrackNumber)
                    )),
                }
            }
        }
    }

    #[test]
    fn track_metadata_new_rejects_missing_titles_and_artist_credits() {
        let release = sample_fixture().build_release_metadata().unwrap();

        for field in ["title", "credits", "artist_name"] {
            let mut fixture = sample_fixture();

            match field {
                "title" => *fixture.track_title_mut() = " \t".into(),
                "credits" => fixture.track_artists_mut().names_mut().clear(),
                _ => *fixture.track_artists_mut().names_mut() = vec![" \t".into()],
            }

            let error = fixture.build_track(release.id()).unwrap_err();

            assert!(matches!(
                (field, error),
                (
                    "title",
                    IngestTrackError::InvalidTrack(TrackValidationError::MissingTitle)
                ) | (
                    "credits",
                    IngestTrackError::InvalidTrack(TrackValidationError::ArtistCredits(
                        ArtistCreditValidationError::NameMbidMismatch
                    ))
                ) | (
                    "artist_name",
                    IngestTrackError::InvalidTrack(TrackValidationError::ArtistCredits(
                        ArtistCreditValidationError::MissingName
                    ))
                )
            ));
        }
    }

    #[test]
    fn track_metadata_new_accepts_zero_duration_and_rejects_duration_overflow() {
        let mut fixture = sample_fixture();
        let release = fixture.build_release_metadata().unwrap();

        for duration in [0, u128::from(u32::MAX)] {
            *fixture.duration_ms_mut() = Some(duration);

            let (track, _) = fixture.build_track(release.id()).unwrap();

            assert_eq!(track.duration_ms().map(u128::from), Some(duration));
        }

        let known_id = fixture.build_track(release.id()).unwrap().0.id().clone();
        *fixture.duration_ms_mut() = None;
        let unknown = fixture.build_track(release.id()).unwrap().0;
        assert!(unknown.duration_ms().is_none());
        assert_eq!(unknown.id(), &known_id);

        *fixture.duration_ms_mut() = Some(u128::from(u32::MAX) + 1);

        assert!(matches!(
            fixture.build_track(release.id()),
            Err(IngestTrackError::InvalidTrack(
                TrackValidationError::DurationOverflow
            ))
        ));
    }
}

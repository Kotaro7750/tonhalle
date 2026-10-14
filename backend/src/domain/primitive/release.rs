//! Defines releases and validates their metadata, membership, original audio selection, and cross-release identities.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use serde_json::json;

use super::track::Track;
use crate::domain::artist_credit::{ArtistCreditValidationError, artist_credits};
use crate::domain::{ArtistCredit, AudioFileId, ReleaseId, TrackId, normalization};

/// A failure to derive release identity and display metadata.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReleaseMetadataValidationError {
    /// The release title is empty after identity normalization.
    #[error("invalid metadata: missing release title")]
    MissingTitle,
    /// The release date has an invalid shape or calendar value.
    #[error("invalid metadata: invalid release date")]
    InvalidDate,
    /// Album-artist names cannot form valid artist credits.
    #[error(transparent)]
    ArtistCredits(#[from] ArtistCreditValidationError),
}

/// Validated release metadata extracted alongside one track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReleaseMetadata {
    id: ReleaseId,
    title: String,
    release_date: String,
    artist_credits: Vec<ArtistCredit>,
}

impl ReleaseMetadata {
    /// Validates release fields and derives their metadata-based identity.
    pub(crate) fn new(
        title: &str,
        release_date: &str,
        artist_names: &[String],
        artist_mbids: &[String],
    ) -> Result<Self, ReleaseMetadataValidationError> {
        let normalized_title =
            normalization::required(title).ok_or(ReleaseMetadataValidationError::MissingTitle)?;
        validate_release_date(release_date)?;

        let artist_credits = artist_credits(artist_names, artist_mbids)?;
        let artist_ids = artist_credits
            .iter()
            .map(|credit| credit.artist_id())
            .collect::<Vec<_>>();

        let id = ReleaseId::from_metadata(json!([artist_ids, normalized_title, release_date]));

        Ok(Self {
            id,
            title: title.to_owned(),
            release_date: release_date.to_owned(),
            artist_credits,
        })
    }

    /// Identifies the release derived from embedded metadata.
    pub(crate) fn id(&self) -> &ReleaseId {
        &self.id
    }
}

/// A failure to construct a complete, consistent release.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReleaseValidationError {
    /// A release must contain at least one valid track.
    #[error("release contains no tracks")]
    Empty,
    /// Every track and its metadata must belong to the specified release.
    #[error("track {track_id} belongs to a different release")]
    WrongRelease { track_id: TrackId },
    /// Every source must carry identical release metadata, including display values.
    #[error("inconsistent release metadata in {object_key}")]
    MetadataMismatch { object_key: String },
    /// A stored audio identity cannot represent two different tracks.
    #[error("duplicate audio file identity: {id}")]
    DuplicateAudioFile { id: AudioFileId },
}

/// An identity conflict between otherwise valid releases.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReleasesValidationError {
    /// A release identity must identify one complete release in the collection.
    #[error("duplicate release identity: {id}")]
    DuplicateRelease { id: ReleaseId },
    /// A track identity cannot belong to different releases.
    #[error("track {track_id} belongs to both {first_release_id} and {release_id}")]
    SharedTrack {
        track_id: TrackId,
        first_release_id: ReleaseId,
        release_id: ReleaseId,
    },
    /// A stored audio identity cannot belong to different releases.
    #[error("audio file {id} belongs to both {first_release_id} and {release_id}")]
    SharedAudioFile {
        id: AudioFileId,
        first_release_id: ReleaseId,
        release_id: ReleaseId,
    },
}

/// A release with its tracks ordered for display.
#[derive(Debug, Clone)]
pub(crate) struct Release {
    id: ReleaseId,
    title: String,
    release_date: String,
    artist_credits: Vec<ArtistCredit>,
    tracks: Vec<Track>,
}

impl Release {
    /// Validates release membership and metadata agreement before selecting original tracks.
    pub(crate) fn new(
        id: ReleaseId,
        sources: Vec<(Track, ReleaseMetadata)>,
    ) -> Result<Self, ReleaseValidationError> {
        let (_, metadata) = sources.first().ok_or(ReleaseValidationError::Empty)?;

        for (track, source_metadata) in &sources {
            if track.release_id() != &id || source_metadata.id() != &id {
                return Err(ReleaseValidationError::WrongRelease {
                    track_id: track.id().clone(),
                });
            }

            if source_metadata != metadata {
                return Err(ReleaseValidationError::MetadataMismatch {
                    object_key: track.original_audio_file().object_key().to_owned(),
                });
            }
        }

        let metadata = metadata.clone();
        let mut seen_tracks = HashSet::new();
        let mut seen_audio_files = HashSet::new();
        let mut tracks = Vec::new();

        for (track, _) in sources {
            if !seen_tracks.insert(track.id().clone()) {
                tracing::warn!(track_id = %track.id(), object_key = track.original_audio_file().object_key(), "Duplicate original audio excluded");
                continue;
            }

            if !seen_audio_files.insert(track.original_audio_file().id()) {
                return Err(ReleaseValidationError::DuplicateAudioFile {
                    id: track.original_audio_file().id(),
                });
            }

            tracks.push(track);
        }

        tracks.sort_by_key(|track| (track.disc_number(), track.track_number()));

        Ok(Self {
            id,
            title: metadata.title,
            release_date: metadata.release_date,
            artist_credits: metadata.artist_credits,
            tracks,
        })
    }

    /// Exposes the identity shared by the release and its tracks.
    pub(crate) fn id(&self) -> &ReleaseId {
        &self.id
    }

    /// Preserves the accepted release title for display.
    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    /// Preserves the accepted release date precision.
    pub(crate) fn release_date(&self) -> &str {
        &self.release_date
    }

    /// Exposes release credits in display order.
    pub(crate) fn artist_credits(&self) -> &[ArtistCredit] {
        &self.artist_credits
    }

    /// Exposes accepted tracks in disc and track order.
    pub(crate) fn tracks(&self) -> &[Track] {
        &self.tracks
    }
}

/// Rejects shared release, track, and audio identities without dropping valid entities silently.
pub(crate) fn validate_releases(releases: &[Release]) -> Result<(), ReleasesValidationError> {
    let mut seen_releases = HashSet::new();
    let mut seen_tracks = HashMap::new();
    let mut seen_audio_files = HashMap::new();

    for release in releases {
        if !seen_releases.insert(release.id()) {
            return Err(ReleasesValidationError::DuplicateRelease {
                id: release.id().clone(),
            });
        }

        for track in release.tracks() {
            if let Some(first_release_id) = seen_tracks.insert(track.id(), release.id()) {
                return Err(ReleasesValidationError::SharedTrack {
                    track_id: track.id().clone(),
                    first_release_id: first_release_id.clone(),
                    release_id: release.id().clone(),
                });
            }

            if let Some(first_release_id) =
                seen_audio_files.insert(track.original_audio_file().id(), release.id())
            {
                return Err(ReleasesValidationError::SharedAudioFile {
                    id: track.original_audio_file().id(),
                    first_release_id: first_release_id.clone(),
                    release_id: release.id().clone(),
                });
            }
        }
    }

    Ok(())
}

/// Checks calendar validity without filling missing date components.
fn validate_release_date(value: &str) -> Result<(), ReleaseMetadataValidationError> {
    let expanded = match value.len() {
        4 => format!("{value}-01-01"),
        7 => format!("{value}-01"),
        10 => value.to_string(),
        _ => return Err(ReleaseMetadataValidationError::InvalidDate),
    };

    let shape = expanded.bytes().enumerate().all(|(i, b)| {
        if i == 4 || i == 7 {
            b == b'-'
        } else {
            b.is_ascii_digit()
        }
    });

    if !shape || NaiveDate::parse_from_str(&expanded, "%Y-%m-%d").is_err() {
        return Err(ReleaseMetadataValidationError::InvalidDate);
    }

    Ok(())
}

/// Verifies metadata agreement, membership, original-audio selection, and release-collection consistency.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MetadataFixture, sample_fixture, sample_metadata, sample_release};

    #[test]
    fn release_new_keeps_the_first_original_and_orders_tracks_for_display() {
        let source = sample_fixture();
        let id = source.build_release_metadata().unwrap().id().clone();

        let mut second = sample_fixture();
        second.track_number = "2".into();
        second.audio_file.id = "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap();

        let first_original = second.build_track(&id).unwrap();

        second.audio_file.id = "925eda40-54cb-4af4-9579-6811fc7477ac".parse().unwrap();
        let duplicate = second.build_track(&id).unwrap();

        let release = Release::new(
            id,
            vec![
                first_original.clone(),
                duplicate,
                sample_fixture()
                    .build_track(first_original.0.release_id())
                    .unwrap(),
            ],
        )
        .unwrap();

        assert_eq!(release.tracks().len(), 2);
        assert_eq!(release.tracks()[0].track_number(), 1);
        assert_eq!(release.tracks()[1].track_number(), 2);
        assert_eq!(
            release.tracks()[1].original_audio_file().id(),
            first_original.0.original_audio_file().id()
        );
    }

    #[test]
    fn release_new_checks_all_release_metadata_before_excluding_duplicates() {
        let source = sample_fixture();
        let id = source.build_release_metadata().unwrap().id().clone();
        let original = sample_fixture().build_track(&id).unwrap();

        for field in ["title", "credits"] {
            let mut changed = sample_fixture();

            match field {
                "title" => changed.release_title.push(' '),
                _ => changed.release_artists.names[0].push(' '),
            }

            let duplicate = changed.build_track(&id).unwrap();

            assert_eq!(original.0.id(), duplicate.0.id());

            assert!(matches!(
                Release::new(id.clone(), vec![original.clone(), duplicate]),
                Err(ReleaseValidationError::MetadataMismatch { .. })
            ));
        }
    }

    #[test]
    fn release_new_rejects_empty_releases_wrong_membership_and_reused_audio_ids() {
        let source = sample_fixture();
        let id = source.build_release_metadata().unwrap().id().clone();

        assert!(matches!(
            Release::new(id.clone(), vec![]),
            Err(ReleaseValidationError::Empty)
        ));

        let mut other = sample_fixture();
        other.release_title.push_str(" Other");
        let other_id = other.build_release_metadata().unwrap().id().clone();

        assert!(matches!(
            Release::new(id.clone(), vec![other.build_track(&other_id).unwrap()]),
            Err(ReleaseValidationError::WrongRelease { .. })
        ));

        other.release_title = source.release_title.clone();
        other.track_number = "2".into();

        assert!(matches!(
            Release::new(
                id.clone(),
                vec![
                    sample_fixture().build_track(&id).unwrap(),
                    other.build_track(&id).unwrap()
                ]
            ),
            Err(ReleaseValidationError::DuplicateAudioFile { .. })
        ));
    }

    #[test]
    fn validate_release_date_accepts_only_valid_partial_or_complete_dates() {
        for valid in ["2026", "2026-10", "2024-02-29"] {
            assert!(validate_release_date(valid).is_ok());
        }

        for invalid in [
            "",
            "2026-2",
            "2026-02-29",
            "2026-04-31",
            "2026-00",
            "2026-10-13T00:00:00",
        ] {
            assert!(
                matches!(
                    validate_release_date(invalid),
                    Err(ReleaseMetadataValidationError::InvalidDate)
                ),
                "{invalid}"
            );
        }
    }

    #[test]
    fn release_metadata_new_uses_only_release_credits_and_changes_with_release_metadata() {
        let source = sample_fixture();
        let first = source.build_release_metadata().unwrap();

        let mut changed = sample_fixture();
        changed.track_artists.names = vec!["Different Track Performer".into()];

        assert_eq!(first.id(), changed.build_release_metadata().unwrap().id());

        changed.release_title.push_str(" Revised");

        assert_ne!(first.id(), changed.build_release_metadata().unwrap().id());
    }

    #[test]
    fn release_metadata_new_rejects_missing_titles_and_artist_credits() {
        for field in ["title", "credits", "artist_name"] {
            let mut fixture = sample_fixture();

            match field {
                "title" => fixture.release_title = " \t".into(),
                "credits" => fixture.release_artists.names.clear(),
                _ => fixture.release_artists.names = vec![" \t".into()],
            }

            let error = fixture.build_release_metadata().unwrap_err();

            assert!(matches!(
                (field, error),
                ("title", ReleaseMetadataValidationError::MissingTitle)
                    | (
                        "credits",
                        ReleaseMetadataValidationError::ArtistCredits(
                            ArtistCreditValidationError::NameMbidMismatch
                        )
                    )
                    | (
                        "artist_name",
                        ReleaseMetadataValidationError::ArtistCredits(
                            ArtistCreditValidationError::MissingName
                        )
                    )
            ));
        }
    }

    #[test]
    fn validate_releases_rejects_duplicate_releases_and_shared_audio_files() {
        let source = sample_metadata();
        let first = sample_release(&source);

        assert!(matches!(
            validate_releases(&[first.clone(), first.clone()]),
            Err(ReleasesValidationError::DuplicateRelease { .. })
        ));

        let mut other = MetadataFixture::from(&source);
        other.release_title.push_str(" Other");

        assert!(matches!(
            validate_releases(&[first.clone(), other.release()]),
            Err(ReleasesValidationError::SharedAudioFile { .. })
        ));

        other.audio_file.id = "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap();
        let second = other.release();

        assert!(validate_releases(&[first.clone(), second.clone()]).is_ok());
    }
}

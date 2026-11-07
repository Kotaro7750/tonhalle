//! Defines releases and validates their metadata, membership, original audio selection, and cross-release identities.

use chrono::NaiveDate;
use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::domain::{
    artist_credit::{ArtistCredit, ArtistCreditValidationError, artist_credits},
    id::{AudioFileId, ReleaseId, TrackId},
    normalization,
    primitive::track::Track,
};

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
#[derive(getset::Getters, Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReleaseMetadata {
    /// Identifies the release derived from embedded metadata.
    #[getset(get = "pub(crate)")]
    id: ReleaseId,
    /// Preserves the release title for display.
    #[getset(get = "pub(crate)")]
    title: String,
    release_date: Option<String>,
    artist_credits: Vec<ArtistCredit>,
}

impl ReleaseMetadata {
    /// Validates release fields and derives their metadata-based identity.
    pub(crate) fn new(
        title: &str,
        release_date: Option<&str>,
        artist_names: &[String],
        artist_mbids: &[String],
    ) -> Result<Self, ReleaseMetadataValidationError> {
        let normalized_title =
            normalization::required(title).ok_or(ReleaseMetadataValidationError::MissingTitle)?;
        if let Some(date) = release_date {
            validate_release_date(date)?;
        }

        let artist_credits = artist_credits(artist_names, artist_mbids)?;
        let artist_ids = artist_credits
            .iter()
            .map(|credit| credit.artist_id())
            .collect::<Vec<_>>();

        let id = ReleaseId::from_metadata(json!([artist_ids, normalized_title]));

        Ok(Self {
            id,
            title: title.to_owned(),
            release_date: release_date.map(str::to_owned),
            artist_credits,
        })
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
#[derive(getset::Getters, Debug, Clone)]
pub(crate) struct Release {
    /// Exposes the identity shared by the release and its tracks.
    #[getset(get = "pub(crate)")]
    id: ReleaseId,
    /// Preserves the accepted release title for display.
    #[getset(get = "pub(crate)")]
    title: String,
    /// Preserves the accepted release date precision.
    #[getset(get = "pub(crate)")]
    release_date: Option<String>,
    /// Exposes release credits in display order.
    #[getset(get = "pub(crate)")]
    artist_credits: Vec<ArtistCredit>,
    /// Exposes accepted tracks in disc and track order.
    #[getset(get = "pub(crate)")]
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
    use crate::{
        domain::{
            ingestion::metadata::sample_metadata,
            primitive::audio_file::{AudioFile, AudioProperties, MediaType},
        },
        test_support::{release_metadata, sample_audio_file, sample_release, sample_track},
    };

    #[test]
    fn release_new_keeps_the_first_original_and_orders_tracks_for_display() {
        let source = sample_metadata();
        let id = release_metadata(&source).unwrap().id().clone();

        let mut second = sample_metadata();
        *second.track_number_mut() = "2".into();
        let audio_id = "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap();

        let audio = sample_audio_file();
        let first_original = sample_track(
            &second,
            &id,
            AudioFile::from_properties(
                audio_id,
                audio.object_key().into(),
                audio.playback_url().clone(),
                AudioProperties::new(MediaType::Wav, 44100, Some(16), 2, Some(1411200)).unwrap(),
            ),
        );

        let audio_id = "925eda40-54cb-4af4-9579-6811fc7477ac".parse().unwrap();
        let duplicate = sample_track(
            &second,
            &id,
            AudioFile::from_properties(
                audio_id,
                audio.object_key().into(),
                audio.playback_url().clone(),
                AudioProperties::new(MediaType::Wav, 44100, Some(16), 2, Some(1411200)).unwrap(),
            ),
        );

        let release = Release::new(
            id,
            vec![
                first_original.clone(),
                duplicate,
                sample_track(
                    &sample_metadata(),
                    first_original.0.release_id(),
                    sample_audio_file(),
                ),
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
        let source = sample_metadata();
        let id = release_metadata(&source).unwrap().id().clone();
        let original = sample_track(&sample_metadata(), &id, sample_audio_file());

        for field in ["title", "credits", "date", "missing_date"] {
            let mut changed = sample_metadata();

            match field {
                "title" => changed.release_title_mut().push(' '),
                "credits" => changed.release_credit_mut().names_mut()[0].push(' '),
                "date" => *changed.release_date_mut() = Some("2026".into()),
                _ => *changed.release_date_mut() = None,
            }

            let duplicate = sample_track(&changed, &id, sample_audio_file());

            assert_eq!(original.0.id(), duplicate.0.id());

            assert!(matches!(
                Release::new(id.clone(), vec![original.clone(), duplicate]),
                Err(ReleaseValidationError::MetadataMismatch { .. })
            ));
        }
    }

    #[test]
    fn release_new_rejects_empty_releases_wrong_membership_and_reused_audio_ids() {
        let source = sample_metadata();
        let id = release_metadata(&source).unwrap().id().clone();

        assert!(matches!(
            Release::new(id.clone(), vec![]),
            Err(ReleaseValidationError::Empty)
        ));

        let mut other = sample_metadata();
        other.release_title_mut().push_str(" Other");
        let other_id = release_metadata(&other).unwrap().id().clone();

        assert!(matches!(
            Release::new(
                id.clone(),
                vec![sample_track(&other, &other_id, sample_audio_file())]
            ),
            Err(ReleaseValidationError::WrongRelease { .. })
        ));

        *other.release_title_mut() = source.release_title().clone();
        *other.track_number_mut() = "2".into();

        assert!(matches!(
            Release::new(
                id.clone(),
                vec![
                    sample_track(&sample_metadata(), &id, sample_audio_file()),
                    sample_track(&other, &id, sample_audio_file())
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
        let source = sample_metadata();
        let first = release_metadata(&source).unwrap();

        let mut changed = sample_metadata();
        *changed.track_credit_mut().names_mut() = vec!["Different Track Performer".into()];

        assert_eq!(first.id(), release_metadata(&changed).unwrap().id());

        changed.release_title_mut().push_str(" Revised");

        assert_ne!(first.id(), release_metadata(&changed).unwrap().id());
    }

    #[test]
    fn release_metadata_new_rejects_missing_titles_and_artist_credits() {
        assert!(matches!(
            ReleaseMetadata::new(" \t", Some("2026"), &["Artist".into()], &[]),
            Err(ReleaseMetadataValidationError::MissingTitle)
        ));
        assert!(matches!(
            ReleaseMetadata::new("Release", Some("2026"), &[], &[]),
            Err(ReleaseMetadataValidationError::ArtistCredits(
                ArtistCreditValidationError::NameMbidMismatch
            ))
        ));
        assert!(matches!(
            ReleaseMetadata::new("Release", Some("2026"), &[" \t".into()], &[]),
            Err(ReleaseMetadataValidationError::ArtistCredits(
                ArtistCreditValidationError::MissingName
            ))
        ));
    }

    #[test]
    fn validate_releases_rejects_duplicate_releases_and_shared_audio_files() {
        let source = sample_metadata();
        let first = sample_release(&source);

        assert!(matches!(
            validate_releases(&[first.clone(), first.clone()]),
            Err(ReleasesValidationError::DuplicateRelease { .. })
        ));

        let mut other = source.clone();
        other.release_title_mut().push_str(" Other");

        assert!(matches!(
            validate_releases(&[first.clone(), sample_release(&other)]),
            Err(ReleasesValidationError::SharedAudioFile { .. })
        ));

        let audio_id = "229fc612-8a56-45d0-bf61-55b36ff3dcba".parse().unwrap();
        let audio = sample_audio_file();
        let id = release_metadata(&other).unwrap().id().clone();
        let audio = AudioFile::from_properties(
            audio_id,
            audio.object_key().into(),
            audio.playback_url().clone(),
            AudioProperties::new(MediaType::Wav, 44100, Some(16), 2, Some(1411200)).unwrap(),
        );
        let second = Release::new(id.clone(), vec![sample_track(&other, &id, audio)]).unwrap();

        assert!(validate_releases(&[first.clone(), second.clone()]).is_ok());
    }
}

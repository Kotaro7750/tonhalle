//! Converts validated domain entities to OpenAPI response models.

use domain::artist_credit::ArtistCredit;
use tonhalle_api::{models, types::Nullable};

use crate::{
    domain,
    domain::primitive::{audio_file::AudioFile, release::Release, track::Track},
    port::metadata_index,
};

impl From<&ArtistCredit> for models::ArtistCredit {
    /// Preserves artist display spelling and represents absent separators as JSON null.
    fn from(value: &ArtistCredit) -> Self {
        Self {
            artist_id: value.artist_id().to_string(),
            name: value.name().to_owned(),
            join_phrase: value
                .join_phrase()
                .map(str::to_owned)
                .map_or(Nullable::Null, Nullable::Present),
        }
    }
}

impl From<&Release> for models::ReleaseSummary {
    /// Builds a release-list summary without including track details.
    fn from(value: &Release) -> Self {
        Self {
            id: value.id().to_string(),
            title: value.title().to_owned(),
            release_date: value
                .release_date()
                .clone()
                .map_or(Nullable::Null, Nullable::Present),
            artist_credits: value.artist_credits().iter().map(Into::into).collect(),
        }
    }
}

impl From<Release> for models::Release {
    /// Builds release details with tracks in domain display order.
    fn from(value: Release) -> Self {
        Self {
            id: value.id().to_string(),
            title: value.title().to_owned(),
            release_date: value
                .release_date()
                .clone()
                .map_or(Nullable::Null, Nullable::Present),
            artist_credits: value.artist_credits().iter().map(Into::into).collect(),
            tracks: value.tracks().iter().map(Into::into).collect(),
        }
    }
}

impl From<&Track> for models::TrackSummary {
    /// Builds a track summary without including audio delivery details.
    fn from(value: &Track) -> Self {
        Self {
            id: value.id().to_string(),
            release_id: value.release_id().to_string(),
            title: value.title().to_owned(),
            disc_number: value.disc_number(),
            track_number: value.track_number(),
            duration_ms: value
                .duration_ms()
                .map_or(Nullable::Null, Nullable::Present),
            artist_credits: value.artist_credits().iter().map(Into::into).collect(),
        }
    }
}

impl From<Track> for models::Track {
    /// Builds track details with the accepted original audio file.
    fn from(value: Track) -> Self {
        Self {
            id: value.id().to_string(),
            release_id: value.release_id().to_string(),
            title: value.title().to_owned(),
            disc_number: value.disc_number(),
            track_number: value.track_number(),
            duration_ms: value
                .duration_ms()
                .map_or(Nullable::Null, Nullable::Present),
            artist_credits: value.artist_credits().iter().map(Into::into).collect(),
            original_audio_file: value.original_audio_file().into(),
        }
    }
}

impl From<&AudioFile> for models::AudioFile {
    /// Builds audio playback metadata with the retained storage UUID.
    fn from(value: &AudioFile) -> Self {
        Self {
            id: value.id().to_string(),
            media_type: value.media_type().as_str().into(),
            playback_url: value.playback_url().as_str().to_owned(),
            sample_rate: value.sample_rate(),
            bit_depth: value.bit_depth().map_or(Nullable::Null, Nullable::Present),
            channels: value.channels(),
            bitrate: value.bitrate().map_or(Nullable::Null, Nullable::Present),
        }
    }
}

impl From<metadata_index::ReleasePage> for models::ReleasePage {
    /// Builds a release page and omits the continuation token at the end.
    fn from(value: metadata_index::ReleasePage) -> Self {
        Self {
            items: value.items().iter().map(Into::into).collect(),
            next_cursor: value.next_cursor().map(str::to_owned),
        }
    }
}

impl From<&ArtistCredit> for models::Artist {
    /// Exposes an artist identity and its retained display name.
    fn from(value: &ArtistCredit) -> Self {
        Self {
            id: value.artist_id().to_string(),
            name: value.name().to_owned(),
        }
    }
}

impl From<metadata_index::ArtistPage> for models::ArtistPage {
    /// Converts an artist page and omits the final continuation token.
    fn from(value: metadata_index::ArtistPage) -> Self {
        Self {
            items: value.items().iter().map(Into::into).collect(),
            next_cursor: value.next_cursor().map(str::to_owned),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, to_value};

    use super::*;
    use crate::{
        domain::{
            ingestion::metadata::sample_metadata,
            primitive::audio_file::{AudioProperties, MediaType},
        },
        test_support::{release_metadata, sample_audio_file, sample_release, sample_track},
    };

    #[test]
    fn release_conversion_preserves_dates_or_serializes_null_in_details_and_summaries() {
        for date in [None, Some("2026"), Some("2026-10"), Some("2026-10-13")] {
            let mut raw = sample_metadata();
            *raw.release_date_mut() = date.map(str::to_owned);
            let release = sample_release(&raw);
            let summary = to_value(models::ReleaseSummary::from(&release)).unwrap();
            let detail = to_value(models::Release::from(release)).unwrap();
            let expected = to_value(date).unwrap();
            assert_eq!(summary.get("releaseDate"), Some(&expected));
            assert_eq!(detail.get("releaseDate"), Some(&expected));
        }
    }

    #[test]
    fn track_conversion_serializes_missing_duration_as_null_in_details_and_summaries() {
        let mut raw = sample_metadata();
        *raw.audio_mut().duration_ms_mut() = None;
        let id = release_metadata(&raw).unwrap().id().clone();
        let (track, _) = sample_track(&raw, &id, sample_audio_file());

        let summary = to_value(models::TrackSummary::from(&track)).unwrap();
        let detail = to_value(models::Track::from(track)).unwrap();

        assert_eq!(summary.get("durationMs"), Some(&Value::Null));
        assert_eq!(detail.get("durationMs"), Some(&Value::Null));
    }

    #[test]
    fn audio_file_conversion_preserves_optional_properties_or_serializes_them_as_null() {
        for (bit_depth, bitrate) in [(Some(16), Some(1411200)), (None, None)] {
            let audio = AudioFile::from_properties(
                sample_audio_file().id(),
                "audio.wav".into(),
                "https://example.com/audio.wav".parse().unwrap(),
                AudioProperties::new(MediaType::Wav, 44100, bit_depth, 2, bitrate).unwrap(),
            );
            let result = to_value(models::AudioFile::from(&audio)).unwrap();

            assert_eq!(result.get("bitDepth"), Some(&to_value(bit_depth).unwrap()));
            assert_eq!(result.get("bitrate"), Some(&to_value(bitrate).unwrap()));
        }
    }

    #[test]
    fn release_page_conversion_preserves_the_cursor_or_omits_it_at_the_end() {
        for cursor in [Some("next-token".to_owned()), None] {
            let page = metadata_index::ReleasePage::new(
                vec![sample_release(&sample_metadata())],
                cursor.clone(),
            );
            let result = to_value(models::ReleasePage::from(page)).unwrap();

            assert_eq!(result.get("nextCursor"), cursor.map(Value::String).as_ref());
        }
    }
}

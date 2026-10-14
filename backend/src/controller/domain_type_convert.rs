//! Converts validated domain entities to OpenAPI response models.

use tonhalle_api::{models, types::Nullable};

use crate::domain::primitive::{audio_file::AudioFile, release::Release, track::Track};
use crate::{domain, port};

impl From<&domain::ArtistCredit> for models::ArtistCredit {
    /// Preserves artist display spelling and represents absent separators as JSON null.
    fn from(value: &domain::ArtistCredit) -> Self {
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
            release_date: value.release_date().to_owned(),
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
            release_date: value.release_date().to_owned(),
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
            duration_ms: value.duration_ms(),
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
            duration_ms: value.duration_ms(),
            artist_credits: value.artist_credits().iter().map(Into::into).collect(),
            original_audio_file: value.original_audio_file().into(),
        }
    }
}

impl From<&AudioFile> for models::AudioFile {
    /// Builds WAV playback metadata with the retained storage UUID.
    fn from(value: &AudioFile) -> Self {
        Self {
            id: value.id().to_string(),
            media_type: "audio/wav".into(),
            playback_url: value.playback_url().as_str().to_owned(),
            sample_rate: value.sample_rate(),
            bit_depth: value.bit_depth(),
            channels: value.channels(),
            bitrate: value.bitrate(),
        }
    }
}

impl From<port::ReleasePage> for models::ReleasePage {
    /// Builds a release page and omits the continuation token at the end.
    fn from(value: port::ReleasePage) -> Self {
        Self {
            items: value.items().iter().map(Into::into).collect(),
            next_cursor: value.next_cursor().map(str::to_owned),
        }
    }
}

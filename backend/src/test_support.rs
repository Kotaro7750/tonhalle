//! Builds synthetic source metadata for domain and workflow unit tests.

use crate::domain::{
    AudioFileId, ReleaseId,
    ingestion::{IngestTrackError, build_release, metadata::RawMetadata},
    primitive::{
        audio_file::{AudioFile, AudioFileValidationError},
        release::{Release, ReleaseMetadata, ReleaseMetadataValidationError},
        track::Track,
    },
};

/// Derives a release identity for metadata fixtures through the production validation path.
pub(crate) fn release_metadata(
    raw: &RawMetadata,
) -> Result<ReleaseMetadata, ReleaseMetadataValidationError> {
    ReleaseMetadata::new(
        raw.release_title(),
        raw.release_date(),
        raw.release_artist_names(),
        raw.release_artist_mbids(),
    )
}

/// Builds valid source metadata without reading external audio files.
pub(crate) fn sample_metadata() -> RawMetadata {
    sample_fixture().build()
}

/// Builds editable metadata and storage values for constructor and ingestion tests.
pub(crate) fn sample_fixture() -> MetadataFixture {
    MetadataFixture {
        release_title: "Example Release".into(),
        release_date: "2026-10-13".into(),
        release_artists: CreditFixture {
            names: vec!["Example Artist".into()],
            mbids: vec![],
        },
        track_title: "Example Track".into(),
        track_artists: CreditFixture {
            names: vec!["Example Artist".into()],
            mbids: vec![],
        },
        disc_number: "1".into(),
        track_number: "1".into(),
        duration_ms: 1000,
        audio_file: AudioFixture {
            id: "2f1c75fc-55de-4d4b-8992-76050f65dff0".parse().unwrap(),
            object_key: "release/example.wav".into(),
            playback_url: "https://example.com/example.wav".parse().unwrap(),
            sample_rate: 44100,
            bit_depth: 16,
            channels: 2,
            bitrate: 1411200,
        },
    }
}

/// Constructs a valid release through the production Domain generation path.
pub(crate) fn sample_release(source: &RawMetadata) -> Release {
    MetadataFixture::from(source).release()
}

/// Editable artist values used to exercise credit-validation boundaries.
#[derive(Clone)]
pub(crate) struct CreditFixture {
    pub(crate) names: Vec<String>,
    pub(crate) mbids: Vec<String>,
}

/// Editable audio properties used to exercise required-value validation.
pub(crate) struct AudioFixture {
    pub(crate) id: AudioFileId,
    pub(crate) object_key: String,
    pub(crate) playback_url: url::Url,
    pub(crate) sample_rate: u32,
    pub(crate) bit_depth: u32,
    pub(crate) channels: u32,
    pub(crate) bitrate: u64,
}

impl AudioFixture {
    /// Builds stored audio through its validating constructor.
    pub(crate) fn build(&self) -> Result<AudioFile, AudioFileValidationError> {
        AudioFile::new(
            self.id,
            self.object_key.clone(),
            self.playback_url.clone(),
            self.sample_rate,
            self.bit_depth,
            self.channels,
            self.bitrate,
        )
    }
}

/// An editable copy of source metadata for domain and indexing regressions.
pub(crate) struct MetadataFixture {
    pub(crate) release_title: String,
    pub(crate) release_date: String,
    pub(crate) release_artists: CreditFixture,
    pub(crate) track_title: String,
    pub(crate) track_artists: CreditFixture,
    pub(crate) disc_number: String,
    pub(crate) track_number: String,
    pub(crate) duration_ms: u128,
    pub(crate) audio_file: AudioFixture,
}

impl From<&RawMetadata> for MetadataFixture {
    /// Copies source metadata so tests can vary one validation input.
    fn from(source: &RawMetadata) -> Self {
        Self {
            release_title: source.release_title().to_owned(),
            release_date: source.release_date().to_owned(),
            release_artists: CreditFixture {
                names: source.release_artist_names().to_vec(),
                mbids: source.release_artist_mbids().to_vec(),
            },
            track_title: source.track_title().to_owned(),
            track_artists: CreditFixture {
                names: source.track_artist_names().to_vec(),
                mbids: source.track_artist_mbids().to_vec(),
            },
            disc_number: source.disc_number().to_owned(),
            track_number: source.track_number().to_owned(),
            duration_ms: source.duration_ms(),
            audio_file: AudioFixture {
                id: "2f1c75fc-55de-4d4b-8992-76050f65dff0".parse().unwrap(),
                object_key: "release/example.wav".into(),
                playback_url: "https://example.com/example.wav".parse().unwrap(),
                sample_rate: source.sample_rate(),
                bit_depth: source.bit_depth(),
                channels: source.channels(),
                bitrate: source.bitrate(),
            },
        }
    }
}

impl MetadataFixture {
    /// Constructs source metadata from the current fixture values.
    pub(crate) fn build(&self) -> RawMetadata {
        self.into()
    }

    /// Builds validated release metadata from flat fixture fields.
    pub(crate) fn build_release_metadata(
        &self,
    ) -> Result<ReleaseMetadata, ReleaseMetadataValidationError> {
        ReleaseMetadata::new(
            &self.release_title,
            &self.release_date,
            &self.release_artists.names,
            &self.release_artists.mbids,
        )
    }

    /// Builds a track and release metadata through their validating constructors.
    pub(crate) fn build_track(
        &self,
        release_id: &ReleaseId,
    ) -> Result<(Track, ReleaseMetadata), IngestTrackError> {
        let metadata = self.build_release_metadata()?;
        let audio = self.audio_file.build()?;

        let track = Track::new(
            release_id,
            &self.track_title,
            &self.disc_number,
            &self.track_number,
            self.duration_ms,
            &self.track_artists.names,
            &self.track_artists.mbids,
            audio,
        )?;

        Ok((track, metadata))
    }

    /// Ingests fixture WAV bytes with separately supplied storage information.
    pub(crate) fn ingest_track(
        &self,
        release_id: &ReleaseId,
    ) -> Result<(Track, ReleaseMetadata), IngestTrackError> {
        crate::domain::ingestion::ingest_track(
            &sample_audio(&self.build()),
            self.audio_file.id,
            self.audio_file.object_key.clone(),
            self.audio_file.playback_url.clone(),
            release_id,
        )
    }

    /// Constructs a release through the production validation path with fixture storage identity.
    pub(crate) fn release(&self) -> Release {
        let id = release_metadata(&self.build()).unwrap().id().clone();
        let track = self.ingest_track(&id).unwrap();

        build_release(id, vec![track]).unwrap()
    }
}

/// Encodes fixture metadata in WAV bytes to exercise the production ingestion boundary.
pub(crate) fn sample_audio(source: &RawMetadata) -> Vec<u8> {
    sample_audio_with_mbids(source, None, None)
}

/// Encodes optional recording and release MBIDs to test that they do not affect identities.
pub(crate) fn sample_audio_with_mbids(
    source: &RawMetadata,
    release_mbid: Option<&str>,
    recording_mbid: Option<&str>,
) -> Vec<u8> {
    use lofty::{
        config::WriteOptions,
        tag::{ItemKey, ItemValue, Tag, TagExt, TagItem, TagType},
    };

    let mut tag = Tag::new(TagType::Id3v2);

    for (key, value) in [
        (ItemKey::AlbumTitle, Some(source.release_title())),
        (ItemKey::ReleaseDate, Some(source.release_date())),
        (ItemKey::MusicBrainzReleaseId, release_mbid),
        (ItemKey::TrackTitle, Some(source.track_title())),
        (ItemKey::MusicBrainzRecordingId, recording_mbid),
        (ItemKey::DiscNumber, Some(source.disc_number())),
        (ItemKey::TrackNumber, Some(source.track_number())),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            tag.insert_text(key, value.to_owned());
        }
    }

    for (key, values) in [
        (ItemKey::AlbumArtist, source.release_artist_names()),
        (
            ItemKey::MusicBrainzReleaseArtistId,
            source.release_artist_mbids(),
        ),
        (ItemKey::TrackArtist, source.track_artist_names()),
        (ItemKey::MusicBrainzArtistId, source.track_artist_mbids()),
    ] {
        for value in values {
            tag.push(TagItem::new(key, ItemValue::Text(value.clone())));
        }
    }

    let mut tags = Vec::new();
    tag.dump_to(&mut tags, WriteOptions::new()).unwrap();

    let block_align = (source.bit_depth() / 8 * source.channels()) as u16;
    let byte_rate = source.bitrate() / 8;
    let data_len = (u128::from(byte_rate) * source.duration_ms() / 1000) as usize;

    let mut format = Vec::new();
    format.extend_from_slice(&1_u16.to_le_bytes());
    format.extend_from_slice(&(source.channels() as u16).to_le_bytes());
    format.extend_from_slice(&source.sample_rate().to_le_bytes());
    format.extend_from_slice(&u32::try_from(byte_rate).unwrap().to_le_bytes());
    format.extend_from_slice(&block_align.to_le_bytes());
    format.extend_from_slice(&(source.bit_depth() as u16).to_le_bytes());

    let mut bytes = b"RIFF\0\0\0\0WAVE".to_vec();

    for (key, payload) in [
        (b"fmt ", format),
        (b"data", vec![0; data_len]),
        (b"id3 ", tags),
    ] {
        bytes.extend_from_slice(key);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&payload);
        if payload.len() % 2 != 0 {
            bytes.push(0);
        }
    }

    let riff_len = bytes.len() as u32 - 8;
    bytes[4..8].copy_from_slice(&riff_len.to_le_bytes());

    bytes
}

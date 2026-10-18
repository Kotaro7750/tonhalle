//! Builds synthetic source metadata for domain and workflow unit tests.

use crate::domain::{
    AudioFileId, ReleaseId,
    ingestion::{IngestTrackError, build_release, metadata::RawMetadata},
    primitive::{
        audio_file::{AudioFile, AudioFileValidationError, AudioProperties},
        release::{Release, ReleaseMetadata, ReleaseMetadataValidationError},
        track::{Track, TrackMetadata},
    },
};

/// Derives a release identity for metadata fixtures through the production validation path.
pub(crate) fn release_metadata(
    raw: &RawMetadata,
) -> Result<ReleaseMetadata, ReleaseMetadataValidationError> {
    ReleaseMetadata::new(
        raw.release_title(),
        raw.release_date(),
        raw.release_credit().names(),
        raw.release_credit().mbids(),
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
        duration_ms: Some(1000),
        audio_file: AudioFixture {
            id: "2f1c75fc-55de-4d4b-8992-76050f65dff0".parse().unwrap(),
            object_key: "release/example.wav".into(),
            playback_url: "https://example.com/example.wav".parse().unwrap(),
            sample_rate: 44100,
            bit_depth: Some(16),
            media_type: "audio/wav",
            channels: 2,
            bitrate: Some(1411200),
        },
    }
}

/// Constructs a valid release through the production Domain generation path.
pub(crate) fn sample_release(source: &RawMetadata) -> Release {
    MetadataFixture::from(source).release()
}

/// Editable artist values used to exercise credit-validation boundaries.
#[derive(Clone, getset::Getters, getset::MutGetters)]
pub(crate) struct CreditFixture {
    /// Artist spellings supplied to credit validation.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    names: Vec<String>,
    /// Artist identifiers supplied to credit validation.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    mbids: Vec<String>,
}

/// Editable audio properties used to exercise required-value validation.
#[derive(Clone, getset::Getters, getset::CopyGetters, getset::MutGetters)]
pub(crate) struct AudioFixture {
    /// Stored audio identity used by the fixture.
    #[getset(get_copy = "pub(crate)", get_mut = "pub(crate)")]
    id: AudioFileId,
    /// Relative location of the stored audio fixture.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    object_key: String,
    /// Delivery endpoint of the stored audio fixture.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    playback_url: url::Url,
    /// Sampling frequency in hertz supplied to audio validation.
    #[getset(get_copy = "pub(crate)", get_mut = "pub(crate)")]
    sample_rate: u32,
    /// Optional sample precision supplied to audio validation.
    #[getset(get_copy = "pub(crate)", get_mut = "pub(crate)")]
    bit_depth: Option<u32>,
    /// Detected content type represented by the fixture.
    #[getset(get_copy = "pub(crate)", get_mut = "pub(crate)")]
    media_type: &'static str,
    /// Channel count supplied to audio validation.
    #[getset(get_copy = "pub(crate)", get_mut = "pub(crate)")]
    channels: u32,
    /// Optional bitrate in bits per second supplied to audio validation.
    #[getset(get_copy = "pub(crate)", get_mut = "pub(crate)")]
    bitrate: Option<u64>,
}

impl AudioFixture {
    /// Builds stored audio through its validating constructor.
    pub(crate) fn build(&self) -> Result<AudioFile, AudioFileValidationError> {
        let properties = AudioProperties::new(
            self.media_type,
            self.sample_rate,
            self.bit_depth,
            self.channels,
            self.bitrate,
        )?;
        Ok(AudioFile::from_properties(
            self.id,
            self.object_key.clone(),
            self.playback_url.clone(),
            properties,
        ))
    }
}

/// An editable copy of source metadata for domain and indexing regressions.
#[derive(getset::Getters, getset::CopyGetters, getset::MutGetters)]
pub(crate) struct MetadataFixture {
    /// Release spelling supplied to metadata validation.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    release_title: String,
    /// Partial or complete date supplied to release validation.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    release_date: String,
    /// Artist credit inputs associated with the release.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    release_artists: CreditFixture,
    /// Track spelling supplied to metadata validation.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    track_title: String,
    /// Artist credit inputs associated with the track.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    track_artists: CreditFixture,
    /// Raw disc position supplied to metadata validation.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    disc_number: String,
    /// Raw track position supplied to metadata validation.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    track_number: String,
    /// Optional playback duration in milliseconds supplied to track validation.
    #[getset(get_copy = "pub(crate)", get_mut = "pub(crate)")]
    duration_ms: Option<u128>,
    /// Stored identity and playback inputs used to build audio.
    #[getset(get = "pub(crate)", get_mut = "pub(crate)")]
    audio_file: AudioFixture,
}

impl From<&RawMetadata> for MetadataFixture {
    /// Copies source metadata so tests can vary one validation input.
    fn from(source: &RawMetadata) -> Self {
        Self {
            release_title: source.release_title().to_owned(),
            release_date: source.release_date().to_owned(),
            release_artists: CreditFixture {
                names: source.release_credit().names().to_vec(),
                mbids: source.release_credit().mbids().to_vec(),
            },
            track_title: source.track_title().to_owned(),
            track_artists: CreditFixture {
                names: source.track_credit().names().to_vec(),
                mbids: source.track_credit().mbids().to_vec(),
            },
            disc_number: source.disc_number().to_owned(),
            track_number: source.track_number().to_owned(),
            duration_ms: source.audio().duration_ms(),
            audio_file: AudioFixture {
                id: "2f1c75fc-55de-4d4b-8992-76050f65dff0".parse().unwrap(),
                object_key: "release/example.wav".into(),
                playback_url: "https://example.com/example.wav".parse().unwrap(),
                sample_rate: source.audio().sample_rate(),
                bit_depth: source.audio().bit_depth(),
                media_type: source.audio().media_type(),
                channels: source.audio().channels(),
                bitrate: source.audio().bitrate(),
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

        let track_metadata = TrackMetadata::new(
            release_id,
            &self.track_title,
            &self.disc_number,
            &self.track_number,
            self.duration_ms,
            &self.track_artists.names,
            &self.track_artists.mbids,
        )?;

        Ok((Track::new(track_metadata, audio), metadata))
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
        (ItemKey::AlbumTitle, Some(source.release_title().as_str())),
        (ItemKey::ReleaseDate, Some(source.release_date().as_str())),
        (ItemKey::MusicBrainzReleaseId, release_mbid),
        (ItemKey::TrackTitle, Some(source.track_title().as_str())),
        (ItemKey::MusicBrainzRecordingId, recording_mbid),
        (ItemKey::DiscNumber, Some(source.disc_number().as_str())),
        (ItemKey::TrackNumber, Some(source.track_number().as_str())),
    ] {
        if let Some(value) = value.filter(|value| !value.is_empty()) {
            tag.insert_text(key, value.to_owned());
        }
    }

    for (key, values) in [
        (ItemKey::AlbumArtist, source.release_credit().names()),
        (
            ItemKey::MusicBrainzReleaseArtistId,
            source.release_credit().mbids(),
        ),
        (ItemKey::TrackArtist, source.track_credit().names()),
        (ItemKey::MusicBrainzArtistId, source.track_credit().mbids()),
    ] {
        for value in values {
            tag.push(TagItem::new(key, ItemValue::Text(value.clone())));
        }
    }

    let mut tags = Vec::new();
    tag.dump_to(&mut tags, WriteOptions::new()).unwrap();

    let block_align =
        (source.audio().bit_depth().unwrap_or_default() / 8 * source.audio().channels()) as u16;
    let byte_rate = source.audio().bitrate().unwrap_or_default() / 8;
    let data_len =
        (u128::from(byte_rate) * source.audio().duration_ms().unwrap_or_default() / 1000) as usize;

    let mut format = Vec::new();
    format.extend_from_slice(&1_u16.to_le_bytes());
    format.extend_from_slice(&(source.audio().channels() as u16).to_le_bytes());
    format.extend_from_slice(&source.audio().sample_rate().to_le_bytes());
    format.extend_from_slice(&u32::try_from(byte_rate).unwrap().to_le_bytes());
    format.extend_from_slice(&block_align.to_le_bytes());
    format
        .extend_from_slice(&(source.audio().bit_depth().unwrap_or_default() as u16).to_le_bytes());

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

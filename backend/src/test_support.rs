//! Creates valid domain values and tagged audio bytes for tests.

use std::{
    fs::File,
    io::{Cursor, Seek, SeekFrom, Write},
};

use crate::domain::{
    id::ReleaseId,
    import::{ImportAudio, ImportValidationError, ImportedReleaseValidator},
    ingestion::metadata::RawMetadata,
    primitive::{
        audio_file::{AudioFile, AudioProperties, MediaType},
        release::{Release, ReleaseMetadata, ReleaseMetadataValidationError},
        track::{Track, TrackMetadata},
    },
};

/// Builds release metadata through its validating constructor.
pub(crate) fn release_metadata(
    raw: &RawMetadata,
) -> Result<ReleaseMetadata, ReleaseMetadataValidationError> {
    ReleaseMetadata::new(
        raw.release_title(),
        raw.release_date().as_deref(),
        raw.release_credit().names(),
        raw.release_credit().mbids(),
    )
}

/// Validates test sources through the incremental import path.
pub(crate) fn validate_import_release(
    sources: Vec<Vec<u8>>,
) -> Result<ReleaseMetadata, ImportValidationError> {
    let mut builder = ImportedReleaseValidator::default();
    for bytes in sources {
        builder.push(Cursor::new(bytes))?;
    }
    builder.finish()
}

/// Creates staging audio through the release validator.
pub(crate) fn sample_import_audio(bytes: Vec<u8>) -> ImportAudio {
    ImportedReleaseValidator::default()
        .push(Cursor::new(bytes))
        .unwrap()
}

/// Creates valid storage information without parsing audio.
pub(crate) fn sample_audio_file() -> AudioFile {
    AudioFile::from_properties(
        "2f1c75fc-55de-4d4b-8992-76050f65dff0".parse().unwrap(),
        "release/example.wav".into(),
        "https://example.com/example.wav".parse().unwrap(),
        AudioProperties::new(MediaType::Wav, 44100, Some(16), 2, Some(1411200)).unwrap(),
    )
}

/// Combines valid track inputs with separately supplied storage information.
pub(crate) fn sample_track(
    raw: &RawMetadata,
    release_id: &ReleaseId,
    audio: AudioFile,
) -> (Track, ReleaseMetadata) {
    let metadata = TrackMetadata::new(
        release_id,
        raw.track_title(),
        raw.disc_number(),
        raw.track_number(),
        raw.audio().duration_ms(),
        raw.track_credit().names(),
        raw.track_credit().mbids(),
    )
    .unwrap();
    (Track::new(metadata, audio), release_metadata(raw).unwrap())
}

/// Builds a valid release without encoding or parsing audio.
pub(crate) fn sample_release(raw: &RawMetadata) -> Release {
    let id = release_metadata(raw).unwrap().id().clone();
    Release::new(
        id.clone(),
        vec![sample_track(raw, &id, sample_audio_file())],
    )
    .unwrap()
}

/// Builds a release with a distinct stored audio identity for cross-release tests.
pub(crate) fn sample_release_with_audio_id(
    raw: &RawMetadata,
    audio_id: crate::domain::id::AudioFileId,
) -> Release {
    let id = release_metadata(raw).unwrap().id().clone();
    let source = sample_audio_file();
    let audio = AudioFile::from_properties(
        audio_id,
        source.object_key().into(),
        source.playback_url().clone(),
        AudioProperties::new(MediaType::Wav, 44100, Some(16), 2, Some(1411200)).unwrap(),
    );
    Release::new(id.clone(), vec![sample_track(raw, &id, audio)]).unwrap()
}

/// Encodes source metadata in WAV bytes to exercise the production ingestion boundary.
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
        (ItemKey::ReleaseDate, source.release_date().as_deref()),
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

/// Writes a sparse WAV with tags after a large audio payload without allocating that payload.
pub(crate) fn write_sparse_audio(file: &mut File, raw: &RawMetadata, payload_length: u32) {
    let mut header = sample_audio(raw);
    let data = header
        .windows(4)
        .position(|chunk| chunk == b"data")
        .unwrap();
    let original = u32::from_le_bytes(header[data + 4..data + 8].try_into().unwrap());
    let tail = header[data + 8 + original as usize..].to_vec();
    let tail_offset = data as u64 + 8 + u64::from(payload_length);
    let length = tail_offset + tail.len() as u64;
    header[4..8].copy_from_slice(&((length - 8) as u32).to_le_bytes());
    header[data + 4..data + 8].copy_from_slice(&payload_length.to_le_bytes());
    file.write_all(&header[..data + 8]).unwrap();
    file.seek(SeekFrom::Start(tail_offset)).unwrap();
    file.write_all(&tail).unwrap();
    file.rewind().unwrap();
}

//! Extracts unvalidated embedded tags and audio properties from stored audio bytes.

use lofty::{
    config::ParseOptions,
    error::FileParseError,
    file::{AudioFile as _, FileType, TaggedFile, TaggedFileExt},
    iff::wav::{WavFile, WavFormat, WavProperties},
    probe::Probe,
    tag::ItemKey,
};
use std::io::{Error as IoError, Read, Seek};

use crate::domain::primitive::audio_file::MediaType;

/// Embedded tags and audio properties awaiting domain validation.
#[cfg_attr(test, derive(getset::MutGetters))]
#[derive(getset::Getters, Debug, Clone)]
pub(crate) struct RawMetadata {
    /// Preserves source spelling before release identity normalization.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    release_title: String,
    /// Preserves date precision for release validation.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    release_date: Option<String>,
    /// Provides the unvalidated release artist names and MBIDs.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    release_credit: RawCreditMetadata,
    /// Preserves source spelling before track identity normalization.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    track_title: String,
    /// Provides the unvalidated track artist names and MBIDs.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    track_credit: RawCreditMetadata,
    /// Preserves raw disc notation for positive-number validation.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    disc_number: String,
    /// Preserves raw track notation for positive-number validation.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    track_number: String,
    /// Provides the extracted technical information before entity construction.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    audio: RawAudioMetadata,
}

/// Artist name and MBID lists whose correspondence has not yet been validated.
#[cfg_attr(test, derive(getset::MutGetters))]
#[derive(getset::Getters, Debug, Clone)]
pub(crate) struct RawCreditMetadata {
    /// Preserves explicit artist name boundaries and source spelling.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    names: Vec<String>,
    /// Preserves raw MBID order for later name correspondence.
    #[getset(get = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    mbids: Vec<String>,
}

/// Audio properties and container identity awaiting domain validation.
#[cfg_attr(test, derive(getset::MutGetters))]
#[derive(getset::CopyGetters, Debug, Clone)]
pub(crate) struct RawAudioMetadata {
    /// Provides the measured playback duration before track construction.
    #[getset(get_copy = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    duration_ms: Option<u128>,
    /// Provides the sampling frequency in hertz reported by the audio format.
    #[getset(get_copy = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    sample_rate: u32,
    /// Provides the number of bits used for each audio sample when available.
    #[getset(get_copy = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    bit_depth: Option<u32>,
    /// Provides the content type detected from audio bytes.
    #[getset(get_copy = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    media_type: MediaType,
    /// Provides the channel count reported by the audio format.
    #[getset(get_copy = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    channels: u32,
    /// Provides the audio bitrate in bits per second when available.
    #[getset(get_copy = "pub(crate)")]
    #[cfg_attr(test, getset(get_mut = "pub(crate)"))]
    bitrate: Option<u64>,
}

/// A failure to extract embedded metadata and playback properties from audio bytes.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ExtractMetadataError {
    /// The container cannot supply readable metadata.
    #[error("reading audio: {0}")]
    Parse(#[source] FileParseError),
    /// The detected container is not a supported playback format.
    #[error("unsupported audio format: {0:?}")]
    Unsupported(FileType),
    /// Format detection could not read the audio bytes.
    #[error("detecting audio format: {0}")]
    Detect(#[source] IoError),
}

/// Extracts tags and playback properties from a seekable source without retaining audio data.
pub(crate) fn extract_metadata(
    reader: impl Read + Seek,
) -> Result<RawMetadata, ExtractMetadataError> {
    let options = ParseOptions::new().read_cover_art(false);
    let probe = Probe::new(reader)
        .options(options)
        .guess_file_type()
        .map_err(ExtractMetadataError::Detect)?;
    let (file, wav_bitrate): (TaggedFile, Option<u64>) = if probe.file_type() == Some(FileType::Wav)
    {
        let wav = WavFile::read_from(&mut probe.into_inner(), options)
            .map_err(ExtractMetadataError::Parse)?;
        let bitrate = bitrate(wav.properties());
        (wav.into(), bitrate)
    } else {
        (probe.read().map_err(ExtractMetadataError::Parse)?, None)
    };
    let properties = file.properties();
    // Lofty uses zero for unknown duration, while WAV with a known byte rate can round a short duration to zero.
    let duration_ms = (!properties.duration().is_zero()
        || (file.file_type() == FileType::Wav && wav_bitrate.is_some()))
    .then(|| properties.duration().as_millis());
    let media_type = match file.file_type() {
        FileType::Wav => MediaType::Wav,
        FileType::Flac => MediaType::Flac,
        FileType::Aac => MediaType::Aac,
        FileType::Mp4 => MediaType::Mp4,
        FileType::Mpeg => MediaType::Mpeg,
        FileType::Vorbis | FileType::Opus | FileType::Speex => MediaType::Ogg,
        format => return Err(ExtractMetadataError::Unsupported(format)),
    };

    Ok(RawMetadata {
        release_title: first(&file, ItemKey::AlbumTitle).unwrap_or_default(),
        release_date: first(&file, ItemKey::ReleaseDate)
            .or_else(|| first(&file, ItemKey::RecordingDate)),
        release_credit: RawCreditMetadata {
            names: values(&file, ItemKey::AlbumArtist),
            mbids: values(&file, ItemKey::MusicBrainzReleaseArtistId),
        },
        track_title: first(&file, ItemKey::TrackTitle).unwrap_or_default(),
        track_credit: RawCreditMetadata {
            names: values(&file, ItemKey::TrackArtist),
            mbids: values(&file, ItemKey::MusicBrainzArtistId),
        },
        disc_number: first(&file, ItemKey::DiscNumber).unwrap_or_default(),
        track_number: first(&file, ItemKey::TrackNumber).unwrap_or_default(),
        audio: RawAudioMetadata {
            duration_ms,
            sample_rate: properties.sample_rate().unwrap_or_default(),
            bit_depth: properties.bit_depth().map(u32::from),
            media_type,
            channels: properties.channels().map(u32::from).unwrap_or_default(),
            bitrate: wav_bitrate.or_else(|| {
                properties
                    .audio_bitrate()
                    .filter(|value| *value > 0)
                    .map(|value| u64::from(value) * 1000)
            }),
        },
    })
}

/// Retains exact PCM bitrates when consistent with the container, otherwise converts the reported kbit/s to bit/s.
fn bitrate(properties: &WavProperties) -> Option<u64> {
    let pcm_bitrate = u64::from(properties.sample_rate())
        * u64::from(properties.bit_depth())
        * u64::from(properties.channels());

    if matches!(properties.format(), WavFormat::PCM | WavFormat::IEEE_FLOAT)
        && (pcm_bitrate + 500) / 1000 == u64::from(properties.bitrate())
    {
        Some(pcm_bitrate).filter(|value| *value > 0)
    } else {
        Some(u64::from(properties.bitrate()) * 1000).filter(|value| *value > 0)
    }
}

/// Reads values from the primary tag, falling back to other tags when the field is absent.
fn values(file: &TaggedFile, key: ItemKey) -> Vec<String> {
    file.primary_tag()
        .into_iter()
        .chain(file.tags().iter())
        .find_map(|tag| {
            let values = tag.get_strings(key).map(str::to_string).collect::<Vec<_>>();

            (!values.is_empty()).then_some(values)
        })
        .unwrap_or_default()
}

/// Selects the first embedded value for single-valued source metadata.
fn first(file: &TaggedFile, key: ItemKey) -> Option<String> {
    values(file, key).into_iter().next()
}

/// Creates unvalidated input data without a duplicate test model.
#[cfg(test)]
pub(crate) fn sample_metadata() -> RawMetadata {
    RawMetadata {
        release_title: "Example Release".into(),
        release_date: Some("2026-10-13".into()),
        release_credit: RawCreditMetadata {
            names: vec!["Example Artist".into()],
            mbids: vec![],
        },
        track_title: "Example Track".into(),
        track_credit: RawCreditMetadata {
            names: vec!["Example Artist".into()],
            mbids: vec![],
        },
        disc_number: "1".into(),
        track_number: "1".into(),
        audio: RawAudioMetadata {
            duration_ms: Some(1000),
            sample_rate: 44100,
            bit_depth: Some(16),
            media_type: MediaType::Wav,
            channels: 2,
            bitrate: Some(1411200),
        },
    }
}

/// Verifies extraction preserves source values before domain validation.
#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::test_support::sample_audio;

    /// Rejects extraction that reads through a large audio payload instead of seeking past it.
    struct LimitedRead<R> {
        inner: R,
        remaining: usize,
    }

    impl<R: Read> Read for LimitedRead<R> {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = self.inner.read(buffer)?;
            self.remaining = self
                .remaining
                .checked_sub(count)
                .expect("audio payload was read");
            Ok(count)
        }
    }

    impl<R: Seek> Seek for LimitedRead<R> {
        fn seek(&mut self, position: std::io::SeekFrom) -> std::io::Result<u64> {
            self.inner.seek(position)
        }
    }

    #[test]
    fn extract_metadata_skips_large_audio_payload() {
        use std::io::{SeekFrom, Write};

        let source = sample_metadata();
        let mut bytes = sample_audio(&source);
        let data = bytes.windows(4).position(|chunk| chunk == b"data").unwrap();
        let original_len = u32::from_le_bytes(bytes[data + 4..data + 8].try_into().unwrap());
        let tail = bytes[data + 8 + original_len as usize..].to_vec();
        let payload_len = 256_u32 * 1024 * 1024;
        bytes[data + 4..data + 8].copy_from_slice(&payload_len.to_le_bytes());
        let tail_offset = data as u64 + 8 + u64::from(payload_len);
        let file_len = tail_offset + tail.len() as u64;
        bytes[4..8].copy_from_slice(&((file_len - 8) as u32).to_le_bytes());
        let mut file = tempfile::tempfile().unwrap();
        file.write_all(&bytes[..data + 8]).unwrap();
        file.seek(SeekFrom::Start(tail_offset)).unwrap();
        file.write_all(&tail).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();

        let extracted = extract_metadata(LimitedRead {
            inner: file,
            remaining: 64 * 1024,
        })
        .unwrap();

        assert_eq!(extracted.release_title(), source.release_title());
        assert_eq!(extracted.track_title(), source.track_title());
        assert_eq!(
            extracted.audio().sample_rate(),
            source.audio().sample_rate()
        );
    }

    #[test]
    fn extract_metadata_preserves_missing_and_invalid_metadata_without_validation() {
        let source = sample_metadata();
        let mut invalid = source.clone();
        invalid.release_title_mut().clear();
        *invalid.release_date_mut() = Some("2026-02-29".into());
        invalid.release_credit_mut().names_mut().clear();
        *invalid.release_credit_mut().mbids_mut() =
            vec!["7b0857ed-11ee-4426-9899-16f4e415322f".into()];
        *invalid.track_title_mut() = " \t".into();
        *invalid.track_credit_mut().names_mut() = vec![" One ".into(), "Two".into()];
        *invalid.track_credit_mut().mbids_mut() = vec![
            "f7b87070-d876-4a11-997a-048cdff1a87d".into(),
            "7b0857ed-11ee-4426-9899-16f4e415322f".into(),
        ];
        *invalid.disc_number_mut() = "0".into();

        let audio = sample_audio(&invalid);
        let extracted = extract_metadata(Cursor::new(&audio)).unwrap();

        assert!(extracted.release_title().is_empty());
        assert_eq!(extracted.release_date().as_deref(), Some("2026-02-29"));
        assert!(extracted.release_credit().names().is_empty());
        assert_eq!(
            extracted.release_credit().mbids(),
            invalid.release_credit().mbids()
        );
        assert_eq!(extracted.track_title(), " \t");
        assert_eq!(
            extracted.track_credit().names(),
            invalid.track_credit().names()
        );
        assert_eq!(
            extracted.track_credit().mbids(),
            invalid.track_credit().mbids()
        );
        assert_eq!(extracted.disc_number(), "0");
    }

    #[test]
    fn extract_metadata_preserves_exact_pcm_bitrate_and_converts_reported_rates() {
        let source = sample_metadata();

        for (stored_bitrate, expected_bitrate) in [(1_411_200, 1_411_200), (2_116_800, 2_117_000)] {
            let mut fixture = source.clone();
            *fixture.audio_mut().bitrate_mut() = Some(stored_bitrate);

            let extracted = extract_metadata(Cursor::new(sample_audio(&fixture))).unwrap();

            assert_eq!(
                extracted.audio().sample_rate(),
                source.audio().sample_rate()
            );
            assert_eq!(extracted.audio().bit_depth(), source.audio().bit_depth());
            assert_eq!(extracted.audio().channels(), source.audio().channels());
            assert_eq!(extracted.audio().bitrate(), Some(expected_bitrate));
        }
    }
    #[test]
    fn extract_metadata_reads_supported_formats_and_optional_properties() {
        use lofty::{
            config::WriteOptions,
            file::FileType,
            tag::{ItemKey, ItemValue, Tag, TagExt, TagItem},
        };
        use std::fs::{read, write};

        let fixture = sample_metadata();
        for (extension, bytes, media_type, bit_depth) in [
            (
                "flac",
                include_bytes!("../../../../test_fixtures/tone.flac").as_slice(),
                "audio/flac",
                Some(16),
            ),
            (
                "aac",
                include_bytes!("../../../../test_fixtures/tone.aac").as_slice(),
                "audio/aac",
                None,
            ),
            (
                "m4a",
                include_bytes!("../../../../test_fixtures/tone.m4a").as_slice(),
                "audio/mp4",
                None,
            ),
            (
                "ogg",
                include_bytes!("../../../../test_fixtures/tone.ogg").as_slice(),
                "audio/ogg",
                None,
            ),
            (
                "opus",
                include_bytes!("../../../../test_fixtures/tone.opus").as_slice(),
                "audio/ogg",
                None,
            ),
            (
                "spx",
                include_bytes!("../../../../test_fixtures/tone.spx").as_slice(),
                "audio/ogg",
                None,
            ),
            (
                "mp3",
                include_bytes!("../../../../test_fixtures/tone.mp3").as_slice(),
                "audio/mpeg",
                None,
            ),
        ] {
            let root = tempfile::tempdir().unwrap();
            let directory = root.path();
            let path = directory.join(format!("{}.{}", "tone", extension.to_uppercase()));
            write(&path, bytes).unwrap();
            let mut tag = Tag::new(FileType::from_ext(extension).unwrap().primary_tag_type());
            for (key, value) in [
                (ItemKey::AlbumTitle, fixture.release_title().as_str()),
                (
                    ItemKey::ReleaseDate,
                    fixture.release_date().as_deref().unwrap(),
                ),
                (ItemKey::AlbumArtist, "Example Artist"),
                (ItemKey::TrackTitle, fixture.track_title().as_str()),
                (ItemKey::DiscNumber, "1"),
                (ItemKey::TrackNumber, "1"),
            ] {
                tag.insert_text(key, value.to_owned());
            }
            for name in ["First Artist", "Second Artist"] {
                tag.push(TagItem::new(
                    ItemKey::TrackArtist,
                    ItemValue::Text(name.into()),
                ));
            }
            tag.save_to_path(&path, WriteOptions::new()).unwrap();

            let extracted = extract_metadata(Cursor::new(read(&path).unwrap())).unwrap();
            assert_eq!(
                extracted.track_title(),
                fixture.track_title(),
                "{extension}"
            );
            assert_eq!(
                extracted.track_credit().names(),
                &["First Artist", "Second Artist"]
            );
            assert_eq!(
                extracted.audio().media_type().as_str(),
                media_type,
                "{extension}"
            );
            assert_eq!(extracted.audio().bit_depth(), bit_depth, "{extension}");
            assert_eq!(
                extracted.audio().sample_rate(),
                if extension == "opus" { 24000 } else { 32000 }
            );
            assert_eq!(extracted.audio().channels(), 1);
            assert!(extracted.audio().bitrate().unwrap() > 0);
            assert!(
                extracted.audio().duration_ms().unwrap() >= 900,
                "{extension}"
            );

            if extension == "flac" {
                // FLAC permits an unknown total sample count in STREAMINFO.
                let mut bytes = read(&path).unwrap();
                assert_eq!(&bytes[..4], b"fLaC");
                assert_eq!(bytes[4] & 0x7f, 0);
                bytes[21] &= 0xf0;
                bytes[22..26].fill(0);
                write(&path, bytes).unwrap();
                let unknown = extract_metadata(Cursor::new(read(&path).unwrap())).unwrap();
                assert_eq!(unknown.audio().duration_ms(), None);
                assert_eq!(unknown.audio().bitrate(), None);
            }
        }
    }
}

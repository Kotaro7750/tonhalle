//! Extracts unvalidated embedded tags and audio properties from stored audio bytes.

use std::io::Cursor;

use lofty::{
    config::ParseOptions,
    file::{AudioFile as _, FileType, TaggedFile, TaggedFileExt},
    iff::wav::{WavFile, WavFormat, WavProperties},
    probe::Probe,
    tag::ItemKey,
};

/// Embedded tags and audio properties awaiting domain validation.
#[derive(Debug, Clone)]
pub(crate) struct RawMetadata {
    release_title: String,
    release_date: String,
    release_credit: RawCreditMetadata,
    track_title: String,
    track_credit: RawCreditMetadata,
    disc_number: String,
    track_number: String,
    audio: RawAudioMetadata,
}

impl RawMetadata {
    /// Preserves source spelling before release identity normalization.
    pub(crate) fn release_title(&self) -> &str {
        &self.release_title
    }

    /// Preserves date precision for release validation.
    pub(crate) fn release_date(&self) -> &str {
        &self.release_date
    }

    /// Provides the unvalidated release artist names and MBIDs.
    pub(crate) fn release_credit(&self) -> &RawCreditMetadata {
        &self.release_credit
    }

    /// Preserves source spelling before track identity normalization.
    pub(crate) fn track_title(&self) -> &str {
        &self.track_title
    }

    /// Provides the unvalidated track artist names and MBIDs.
    pub(crate) fn track_credit(&self) -> &RawCreditMetadata {
        &self.track_credit
    }

    /// Preserves raw disc notation for positive-number validation.
    pub(crate) fn disc_number(&self) -> &str {
        &self.disc_number
    }

    /// Preserves raw track notation for positive-number validation.
    pub(crate) fn track_number(&self) -> &str {
        &self.track_number
    }

    /// Provides the extracted technical information before entity construction.
    pub(crate) fn audio(&self) -> &RawAudioMetadata {
        &self.audio
    }
}

/// Artist name and MBID lists whose correspondence has not yet been validated.
#[derive(Debug, Clone)]
pub(crate) struct RawCreditMetadata {
    names: Vec<String>,
    mbids: Vec<String>,
}

impl RawCreditMetadata {
    /// Preserves explicit artist name boundaries and source spelling.
    pub(crate) fn names(&self) -> &[String] {
        &self.names
    }

    /// Preserves raw MBID order for later name correspondence.
    pub(crate) fn mbids(&self) -> &[String] {
        &self.mbids
    }
}

/// Audio properties and container identity awaiting domain validation.
#[derive(Debug, Clone)]
pub(crate) struct RawAudioMetadata {
    duration_ms: Option<u128>,
    sample_rate: u32,
    bit_depth: Option<u32>,
    media_type: &'static str,
    channels: u32,
    bitrate: Option<u64>,
}

impl RawAudioMetadata {
    /// Provides the measured playback duration before track construction.
    pub(crate) fn duration_ms(&self) -> Option<u128> {
        self.duration_ms
    }

    /// Provides the sampling frequency in hertz reported by the audio format.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Provides the number of bits used for each audio sample when available.
    pub(crate) fn bit_depth(&self) -> Option<u32> {
        self.bit_depth
    }

    /// Provides the content type detected from audio bytes.
    pub(crate) fn media_type(&self) -> &'static str {
        self.media_type
    }

    /// Provides the channel count reported by the audio format.
    pub(crate) fn channels(&self) -> u32 {
        self.channels
    }

    /// Provides the audio bitrate in bits per second when available.
    pub(crate) fn bitrate(&self) -> Option<u64> {
        self.bitrate
    }
}

/// A failure to extract embedded metadata and playback properties from audio bytes.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ExtractMetadataError {
    /// The container cannot supply readable metadata.
    #[error("reading audio: {0}")]
    Parse(#[source] lofty::error::FileParseError),
    /// The detected container is not a supported playback format.
    #[error("unsupported audio format: {0:?}")]
    Unsupported(FileType),
    /// Format detection could not read the audio bytes.
    #[error("detecting audio format: {0}")]
    Detect(#[source] std::io::Error),
}

/// Extracts embedded metadata from audio bytes without accessing physical storage.
pub(crate) fn extract_metadata(bytes: &[u8]) -> Result<RawMetadata, ExtractMetadataError> {
    let probe = Probe::new(Cursor::new(bytes))
        .guess_file_type()
        .map_err(ExtractMetadataError::Detect)?;
    let (file, wav_bitrate): (TaggedFile, Option<u64>) = if probe.file_type() == Some(FileType::Wav)
    {
        let wav = WavFile::read_from(&mut Cursor::new(bytes), ParseOptions::new())
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
        FileType::Wav => "audio/wav",
        FileType::Flac => "audio/flac",
        FileType::Aac => "audio/aac",
        FileType::Mp4 => "audio/mp4",
        FileType::Mpeg => "audio/mpeg",
        FileType::Vorbis | FileType::Opus | FileType::Speex => "audio/ogg",
        format => return Err(ExtractMetadataError::Unsupported(format)),
    };

    Ok(RawMetadata {
        release_title: first(&file, ItemKey::AlbumTitle).unwrap_or_default(),
        release_date: first(&file, ItemKey::ReleaseDate)
            .or_else(|| first(&file, ItemKey::RecordingDate))
            .unwrap_or_default(),
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

/// Verifies extraction preserves source values before domain validation.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{MetadataFixture, sample_audio, sample_metadata};

    impl From<&MetadataFixture> for RawMetadata {
        /// Builds raw metadata directly from editable test values.
        fn from(fixture: &MetadataFixture) -> Self {
            Self {
                release_title: fixture.release_title.clone(),
                release_date: fixture.release_date.clone(),
                release_credit: RawCreditMetadata {
                    names: fixture.release_artists.names.clone(),
                    mbids: fixture.release_artists.mbids.clone(),
                },
                track_title: fixture.track_title.clone(),
                track_credit: RawCreditMetadata {
                    names: fixture.track_artists.names.clone(),
                    mbids: fixture.track_artists.mbids.clone(),
                },
                disc_number: fixture.disc_number.clone(),
                track_number: fixture.track_number.clone(),
                audio: RawAudioMetadata {
                    duration_ms: fixture.duration_ms,
                    sample_rate: fixture.audio_file.sample_rate,
                    bit_depth: fixture.audio_file.bit_depth,
                    media_type: fixture.audio_file.media_type,
                    channels: fixture.audio_file.channels,
                    bitrate: fixture.audio_file.bitrate,
                },
            }
        }
    }

    #[test]
    fn extract_metadata_preserves_missing_and_invalid_metadata_without_validation() {
        let source = sample_metadata();
        let mut invalid = MetadataFixture::from(&source);
        invalid.release_title.clear();
        invalid.release_date = "2026-02-29".into();
        invalid.release_artists.names.clear();
        invalid.release_artists.mbids = vec!["7b0857ed-11ee-4426-9899-16f4e415322f".into()];
        invalid.track_title = " \t".into();
        invalid.track_artists.names = vec![" One ".into(), "Two".into()];
        invalid.track_artists.mbids = vec![
            "f7b87070-d876-4a11-997a-048cdff1a87d".into(),
            "7b0857ed-11ee-4426-9899-16f4e415322f".into(),
        ];
        invalid.disc_number = "0".into();

        let audio = sample_audio(&invalid.build());
        let extracted = extract_metadata(&audio).unwrap();

        assert!(extracted.release_title().is_empty());
        assert_eq!(extracted.release_date(), "2026-02-29");
        assert!(extracted.release_credit().names().is_empty());
        assert_eq!(
            extracted.release_credit().mbids(),
            invalid.release_artists.mbids
        );
        assert_eq!(extracted.track_title(), " \t");
        assert_eq!(
            extracted.track_credit().names(),
            invalid.track_artists.names
        );
        assert_eq!(
            extracted.track_credit().mbids(),
            invalid.track_artists.mbids
        );
        assert_eq!(extracted.disc_number(), "0");
    }

    #[test]
    fn extract_metadata_preserves_exact_pcm_bitrate_and_converts_reported_rates() {
        let source = sample_metadata();

        for (stored_bitrate, expected_bitrate) in [(1_411_200, 1_411_200), (2_116_800, 2_117_000)] {
            let mut fixture = MetadataFixture::from(&source);
            fixture.audio_file.bitrate = Some(stored_bitrate);

            let extracted = extract_metadata(&sample_audio(&fixture.build())).unwrap();

            assert_eq!(
                extracted.audio().sample_rate(),
                source.audio().sample_rate()
            );
            assert_eq!(extracted.audio().bit_depth(), source.audio().bit_depth());
            assert_eq!(extracted.audio().channels(), source.audio().channels());
            assert_eq!(extracted.audio().bitrate(), Some(expected_bitrate));
        }
    }
}

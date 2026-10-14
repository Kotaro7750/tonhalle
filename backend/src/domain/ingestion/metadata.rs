//! Extracts unvalidated embedded tags and audio properties from stored audio bytes.

use std::io::Cursor;

use lofty::{
    config::ParseOptions,
    file::{AudioFile as _, TaggedFile, TaggedFileExt},
    iff::wav::{WavFile, WavFormat, WavProperties},
    tag::ItemKey,
};

/// Embedded tags and audio properties awaiting domain validation.
#[derive(Debug, Clone)]
pub(crate) struct RawMetadata {
    release_title: String,
    release_date: String,
    release_artist_names: Vec<String>,
    release_artist_mbids: Vec<String>,
    track_title: String,
    track_artist_names: Vec<String>,
    track_artist_mbids: Vec<String>,
    disc_number: String,
    track_number: String,
    duration_ms: u128,
    sample_rate: u32,
    bit_depth: u32,
    channels: u32,
    bitrate: u64,
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

    /// Preserves explicit album-artist name boundaries and source spelling.
    pub(crate) fn release_artist_names(&self) -> &[String] {
        &self.release_artist_names
    }

    /// Preserves album-artist MBID order for later name correspondence.
    pub(crate) fn release_artist_mbids(&self) -> &[String] {
        &self.release_artist_mbids
    }

    /// Preserves source spelling before track identity normalization.
    pub(crate) fn track_title(&self) -> &str {
        &self.track_title
    }

    /// Preserves explicit performer name boundaries and source spelling.
    pub(crate) fn track_artist_names(&self) -> &[String] {
        &self.track_artist_names
    }

    /// Preserves performer MBID order for later name correspondence.
    pub(crate) fn track_artist_mbids(&self) -> &[String] {
        &self.track_artist_mbids
    }

    /// Preserves raw disc notation for positive-number validation.
    pub(crate) fn disc_number(&self) -> &str {
        &self.disc_number
    }

    /// Preserves raw track notation for positive-number validation.
    pub(crate) fn track_number(&self) -> &str {
        &self.track_number
    }

    /// Provides the measured playback duration before track construction.
    pub(crate) fn duration_ms(&self) -> u128 {
        self.duration_ms
    }

    /// Provides the decoded sampling frequency in hertz.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Provides the number of bits used for each audio sample.
    pub(crate) fn bit_depth(&self) -> u32 {
        self.bit_depth
    }

    /// Provides the channel count used to describe playback.
    pub(crate) fn channels(&self) -> u32 {
        self.channels
    }

    /// Provides the audio bitrate in bits per second.
    pub(crate) fn bitrate(&self) -> u64 {
        self.bitrate
    }
}

/// A failure to extract embedded metadata and playback properties from audio bytes.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ExtractMetadataError {
    /// The container cannot supply readable metadata.
    #[error("reading WAV: {0}")]
    Parse(#[source] lofty::error::FileParseError),
}

/// Extracts embedded metadata from WAV bytes without accessing physical storage.
pub(crate) fn extract_metadata(bytes: &[u8]) -> Result<RawMetadata, ExtractMetadataError> {
    let mut bytes = Cursor::new(bytes);
    let wav =
        WavFile::read_from(&mut bytes, ParseOptions::new()).map_err(ExtractMetadataError::Parse)?;
    let properties = *wav.properties();
    let file: TaggedFile = wav.into();

    Ok(RawMetadata {
        release_title: first(&file, ItemKey::AlbumTitle).unwrap_or_default(),
        release_date: first(&file, ItemKey::ReleaseDate)
            .or_else(|| first(&file, ItemKey::RecordingDate))
            .unwrap_or_default(),
        release_artist_names: values(&file, ItemKey::AlbumArtist),
        release_artist_mbids: values(&file, ItemKey::MusicBrainzReleaseArtistId),
        track_title: first(&file, ItemKey::TrackTitle).unwrap_or_default(),
        track_artist_names: values(&file, ItemKey::TrackArtist),
        track_artist_mbids: values(&file, ItemKey::MusicBrainzArtistId),
        disc_number: first(&file, ItemKey::DiscNumber).unwrap_or_default(),
        track_number: first(&file, ItemKey::TrackNumber).unwrap_or_default(),
        duration_ms: properties.duration().as_millis(),
        sample_rate: properties.sample_rate(),
        bit_depth: u32::from(properties.bit_depth()),
        channels: u32::from(properties.channels()),
        bitrate: bitrate(&properties),
    })
}

/// Retains exact PCM bitrates when consistent with the container, otherwise converts the reported kbit/s to bit/s.
fn bitrate(properties: &WavProperties) -> u64 {
    let pcm_bitrate = u64::from(properties.sample_rate())
        * u64::from(properties.bit_depth())
        * u64::from(properties.channels());

    if matches!(properties.format(), WavFormat::PCM | WavFormat::IEEE_FLOAT)
        && (pcm_bitrate + 500) / 1000 == u64::from(properties.bitrate())
    {
        pcm_bitrate
    } else {
        u64::from(properties.bitrate()) * 1000
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
                release_artist_names: fixture.release_artists.names.clone(),
                release_artist_mbids: fixture.release_artists.mbids.clone(),
                track_title: fixture.track_title.clone(),
                track_artist_names: fixture.track_artists.names.clone(),
                track_artist_mbids: fixture.track_artists.mbids.clone(),
                disc_number: fixture.disc_number.clone(),
                track_number: fixture.track_number.clone(),
                duration_ms: fixture.duration_ms,
                sample_rate: fixture.audio_file.sample_rate,
                bit_depth: fixture.audio_file.bit_depth,
                channels: fixture.audio_file.channels,
                bitrate: fixture.audio_file.bitrate,
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
        assert!(extracted.release_artist_names().is_empty());
        assert_eq!(
            extracted.release_artist_mbids(),
            invalid.release_artists.mbids
        );
        assert_eq!(extracted.track_title(), " \t");
        assert_eq!(extracted.track_artist_names(), invalid.track_artists.names);
        assert_eq!(extracted.track_artist_mbids(), invalid.track_artists.mbids);
        assert_eq!(extracted.disc_number(), "0");
    }

    #[test]
    fn extract_metadata_preserves_exact_pcm_bitrate_and_converts_reported_rates() {
        let source = sample_metadata();

        for (stored_bitrate, expected_bitrate) in [(1_411_200, 1_411_200), (2_116_800, 2_117_000)] {
            let mut fixture = MetadataFixture::from(&source);
            fixture.audio_file.bitrate = stored_bitrate;

            let extracted = extract_metadata(&sample_audio(&fixture.build())).unwrap();

            assert_eq!(extracted.sample_rate(), source.sample_rate());
            assert_eq!(extracted.bit_depth(), source.bit_depth());
            assert_eq!(extracted.channels(), source.channels());
            assert_eq!(extracted.bitrate(), expected_bitrate);
        }
    }
}

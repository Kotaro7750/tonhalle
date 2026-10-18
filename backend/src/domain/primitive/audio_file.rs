//! Defines stored audio entities and validates their playback properties.

use url::Url;

use crate::domain::id::AudioFileId;

/// A supported audio content type independent of its storage location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MediaType {
    /// Waveform audio.
    Wav,
    /// Free Lossless Audio Codec audio.
    Flac,
    /// Advanced Audio Coding audio.
    Aac,
    /// Audio in an MPEG-4 container.
    Mp4,
    /// MPEG audio.
    Mpeg,
    /// Audio in an Ogg container.
    Ogg,
}

impl MediaType {
    /// Recognizes supported filename extensions without a leading dot, ignoring ASCII case.
    pub(crate) fn from_extension(extension: &str) -> Option<Self> {
        match extension.to_ascii_lowercase().as_str() {
            "wav" => Some(Self::Wav),
            "flac" => Some(Self::Flac),
            "aac" => Some(Self::Aac),
            "m4a" | "mp4" => Some(Self::Mp4),
            "mp3" => Some(Self::Mpeg),
            "ogg" | "oga" | "opus" | "spx" => Some(Self::Ogg),
            _ => None,
        }
    }

    /// Provides the MIME representation for external delivery.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Wav => "audio/wav",
            Self::Flac => "audio/flac",
            Self::Aac => "audio/aac",
            Self::Mp4 => "audio/mp4",
            Self::Mpeg => "audio/mpeg",
            Self::Ogg => "audio/ogg",
        }
    }

    /// Provides the conventional audio filename extension without a leading dot.
    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
            Self::Aac => "aac",
            Self::Mp4 => "m4a",
            Self::Mpeg => "mp3",
            Self::Ogg => "ogg",
        }
    }
}

/// A failure to construct an audio file with valid playback properties.
#[derive(Debug, thiserror::Error)]
pub(crate) enum AudioFileValidationError {
    /// Required audio properties are zero.
    #[error("invalid metadata: missing audio properties")]
    MissingProperties,
    /// The audio bitrate exceeds the domain's numeric range.
    #[error("invalid metadata: audio bitrate overflow")]
    BitrateOverflow,
}

/// The retained identity, storage location, and playback properties of an audio file.
#[derive(Debug, Clone)]
pub(crate) struct AudioFile {
    id: AudioFileId,
    object_key: String,
    playback_url: Url,
    properties: AudioProperties,
}

/// Validated audio properties independent of storage identity and location.
#[derive(getset::CopyGetters, Debug, Clone)]
pub(crate) struct AudioProperties {
    /// Provides the validated media type.
    #[getset(get_copy = "pub(crate)")]
    media_type: MediaType,
    /// Provides the validated sample rate.
    #[getset(get_copy = "pub(crate)")]
    sample_rate: u32,
    /// Provides the validated bit depth.
    #[getset(get_copy = "pub(crate)")]
    bit_depth: Option<u32>,
    /// Provides the validated channels.
    #[getset(get_copy = "pub(crate)")]
    channels: u32,
    /// Provides the validated bitrate.
    #[getset(get_copy = "pub(crate)")]
    bitrate: Option<u32>,
}

impl AudioFile {
    /// Attaches storage identity and delivery information to validated properties.
    pub(crate) fn from_properties(
        id: AudioFileId,
        object_key: String,
        playback_url: Url,
        properties: AudioProperties,
    ) -> Self {
        Self {
            id,
            object_key,
            playback_url,
            properties,
        }
    }

    /// Retains the UUID issued when the audio file was stored.
    pub(crate) fn id(&self) -> AudioFileId {
        self.id
    }

    /// Locates the original audio in content storage.
    pub(crate) fn object_key(&self) -> &str {
        &self.object_key
    }

    /// Provides the external delivery URL for this audio file.
    pub(crate) fn playback_url(&self) -> &Url {
        &self.playback_url
    }

    /// Provides the detected audio content type.
    pub(crate) fn media_type(&self) -> MediaType {
        self.properties.media_type()
    }

    /// Provides the decoded sampling frequency in hertz.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.properties.sample_rate()
    }

    /// Provides the number of bits used for each audio sample.
    pub(crate) fn bit_depth(&self) -> Option<u32> {
        self.properties.bit_depth()
    }

    /// Provides the channel count used to describe playback.
    pub(crate) fn channels(&self) -> u32 {
        self.properties.channels()
    }

    /// Provides the audio bitrate in bits per second.
    pub(crate) fn bitrate(&self) -> Option<u32> {
        self.properties.bitrate()
    }
}

impl AudioProperties {
    /// Validates audio properties through the same path for imports and stored audio.
    pub(crate) fn new(
        media_type: MediaType,
        sample_rate: u32,
        bit_depth: Option<u32>,
        channels: u32,
        bitrate: Option<u64>,
    ) -> Result<Self, AudioFileValidationError> {
        if [sample_rate, channels].contains(&0) || bit_depth == Some(0) || bitrate == Some(0) {
            return Err(AudioFileValidationError::MissingProperties);
        }

        let bitrate = bitrate
            .map(u32::try_from)
            .transpose()
            .map_err(|_| AudioFileValidationError::BitrateOverflow)?;

        Ok(Self {
            media_type,
            sample_rate,
            bit_depth,
            channels,
            bitrate,
        })
    }
}

/// Verifies required properties and the bitrate range at audio-file construction.
#[cfg(test)]
mod tests {
    use super::{AudioFileValidationError, MediaType};
    use crate::test_support::sample_fixture;

    #[test]
    fn audio_file_new_rejects_missing_properties_and_bitrate_overflow() {
        for field in 0..5 {
            let mut invalid = sample_fixture().audio_file().clone();

            match field {
                0 => *invalid.sample_rate_mut() = 0,
                1 => *invalid.bit_depth_mut() = Some(0),
                2 => *invalid.channels_mut() = 0,
                3 => *invalid.bitrate_mut() = Some(0),
                4 => *invalid.bitrate_mut() = Some(u64::from(u32::MAX) + 1),
                _ => unreachable!(),
            }

            let error = invalid.build().unwrap_err();

            match field {
                4 => assert!(matches!(error, AudioFileValidationError::BitrateOverflow)),
                _ => assert!(matches!(error, AudioFileValidationError::MissingProperties)),
            }
        }

        for media_type in [
            MediaType::Wav,
            MediaType::Flac,
            MediaType::Mpeg,
            MediaType::Aac,
            MediaType::Mp4,
            MediaType::Ogg,
        ] {
            let mut missing_precision = sample_fixture().audio_file().clone();
            *missing_precision.media_type_mut() = media_type;
            *missing_precision.bit_depth_mut() = None;
            *missing_precision.bitrate_mut() = None;
            let audio = missing_precision.build().unwrap();
            assert!(audio.bit_depth().is_none());
            assert!(audio.bitrate().is_none());
            assert_eq!(audio.id(), missing_precision.id());
        }

        let mut valid = sample_fixture().audio_file().clone();
        *valid.bitrate_mut() = Some(u64::from(u32::MAX));

        let audio = valid.build().unwrap();

        assert_eq!(audio.bitrate(), Some(u32::MAX));
        assert_eq!(audio.id(), valid.id());
        assert_eq!(audio.object_key(), valid.object_key());
        assert_eq!(audio.playback_url(), valid.playback_url());
    }
}

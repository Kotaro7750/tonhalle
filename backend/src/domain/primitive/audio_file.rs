//! Defines stored audio entities and validates their playback properties.

use url::Url;

use crate::domain::AudioFileId;

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
#[derive(Debug, Clone)]
pub(crate) struct AudioProperties {
    media_type: &'static str,
    sample_rate: u32,
    bit_depth: Option<u32>,
    channels: u32,
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
    pub(crate) fn media_type(&self) -> &'static str {
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
        media_type: &'static str,
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
    /// Provides the validated media type.
    pub(crate) fn media_type(&self) -> &'static str {
        self.media_type
    }
    /// Provides the validated sample rate.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    /// Provides the validated bit depth.
    pub(crate) fn bit_depth(&self) -> Option<u32> {
        self.bit_depth
    }
    /// Provides the validated channels.
    pub(crate) fn channels(&self) -> u32 {
        self.channels
    }
    /// Provides the validated bitrate.
    pub(crate) fn bitrate(&self) -> Option<u32> {
        self.bitrate
    }
}

/// Verifies required properties and the bitrate range at audio-file construction.
#[cfg(test)]
mod tests {
    use super::AudioFileValidationError;
    use crate::test_support::sample_fixture;

    #[test]
    fn audio_file_new_rejects_missing_properties_and_bitrate_overflow() {
        for field in 0..5 {
            let mut invalid = sample_fixture().audio_file;

            match field {
                0 => invalid.sample_rate = 0,
                1 => invalid.bit_depth = Some(0),
                2 => invalid.channels = 0,
                3 => invalid.bitrate = Some(0),
                4 => invalid.bitrate = Some(u64::from(u32::MAX) + 1),
                _ => unreachable!(),
            }

            let error = invalid.build().unwrap_err();

            match field {
                4 => assert!(matches!(error, AudioFileValidationError::BitrateOverflow)),
                _ => assert!(matches!(error, AudioFileValidationError::MissingProperties)),
            }
        }

        for media_type in [
            "audio/wav",
            "audio/flac",
            "audio/mpeg",
            "audio/aac",
            "audio/mp4",
            "audio/ogg",
        ] {
            let mut missing_precision = sample_fixture().audio_file;
            missing_precision.media_type = media_type;
            missing_precision.bit_depth = None;
            missing_precision.bitrate = None;
            let audio = missing_precision.build().unwrap();
            assert!(audio.bit_depth().is_none());
            assert!(audio.bitrate().is_none());
            assert_eq!(audio.id(), missing_precision.id);
        }

        let mut valid = sample_fixture().audio_file;
        valid.bitrate = Some(u64::from(u32::MAX));

        let audio = valid.build().unwrap();

        assert_eq!(audio.bitrate(), Some(u32::MAX));
        assert_eq!(audio.id(), valid.id);
        assert_eq!(audio.object_key(), valid.object_key);
        assert_eq!(audio.playback_url(), &valid.playback_url);
    }
}

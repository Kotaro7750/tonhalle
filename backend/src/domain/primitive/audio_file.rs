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
    sample_rate: u32,
    bit_depth: u32,
    channels: u32,
    bitrate: u32,
}

impl AudioFile {
    /// Validates playback properties and retains the existing storage identity.
    pub(crate) fn new(
        id: AudioFileId,
        object_key: String,
        playback_url: Url,
        sample_rate: u32,
        bit_depth: u32,
        channels: u32,
        bitrate: u64,
    ) -> Result<Self, AudioFileValidationError> {
        if [sample_rate, bit_depth, channels].contains(&0) || bitrate == 0 {
            return Err(AudioFileValidationError::MissingProperties);
        }

        let bitrate = bitrate
            .try_into()
            .map_err(|_| AudioFileValidationError::BitrateOverflow)?;

        Ok(Self {
            id,
            object_key,
            playback_url,
            sample_rate,
            bit_depth,
            channels,
            bitrate,
        })
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
    pub(crate) fn bitrate(&self) -> u32 {
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
                1 => invalid.bit_depth = 0,
                2 => invalid.channels = 0,
                3 => invalid.bitrate = 0,
                4 => invalid.bitrate = u64::from(u32::MAX) + 1,
                _ => unreachable!(),
            }

            let error = invalid.build().unwrap_err();

            match field {
                4 => assert!(matches!(error, AudioFileValidationError::BitrateOverflow)),
                _ => assert!(matches!(error, AudioFileValidationError::MissingProperties)),
            }
        }

        let mut valid = sample_fixture().audio_file;
        valid.bitrate = u64::from(u32::MAX);

        let audio = valid.build().unwrap();

        assert_eq!(audio.bitrate(), u32::MAX);
        assert_eq!(audio.id(), valid.id);
        assert_eq!(audio.object_key(), valid.object_key);
        assert_eq!(audio.playback_url(), &valid.playback_url);
    }
}

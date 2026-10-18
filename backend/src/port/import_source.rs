//! Defines read-only source access without exposing physical storage types.

/// A failure to enumerate the complete set of import candidates.
#[derive(Debug, thiserror::Error)]
#[error("listing import releases: {0}")]
pub(crate) struct ListImportReleasesError(#[source] Box<dyn std::error::Error + Send + Sync>);

impl ListImportReleasesError {
    /// Preserves the concrete adapter failure without exposing its physical representation.
    pub(crate) fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self(Box::new(source))
    }
}

/// A failure to obtain one source audio object.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ReadImportAudioError {
    /// The locator cannot address an object within the configured source.
    #[error("invalid import audio key: {key}")]
    InvalidKey { key: String },
    /// The referenced audio object no longer exists.
    #[error("import audio not found: {key}: {source}")]
    NotFound {
        key: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The referenced object is not eligible for reading as source audio.
    #[error("invalid import audio object: {key}")]
    InvalidObject { key: String },
    /// The adapter failed while resolving or reading the object.
    #[error("reading import audio {key}: {source}")]
    Internal {
        key: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// A source-defined release grouping with opaque audio locators.
#[derive(getset::Getters)]
pub(crate) struct ImportCandidate {
    /// Identifies the candidate for diagnostics.
    #[getset(get = "pub(crate)")]
    key: String,
    /// Preserves the source adapter's audio order.
    #[getset(get = "pub(crate)")]
    audio_keys: Vec<String>,
}

impl ImportCandidate {
    /// Groups opaque audio locators under their diagnostic identity.
    pub(crate) fn new(key: String, audio_keys: Vec<String>) -> Self {
        Self { key, audio_keys }
    }
}

/// Provides candidates and bytes from independent read-only storage.
pub(crate) trait ImportSource: Send + Sync {
    /// Enumerates release groupings within the configured scope.
    fn list_releases(&self) -> Result<Vec<ImportCandidate>, ListImportReleasesError>;
    /// Reads the original audio bytes using an opaque locator.
    fn read_audio(&self, key: &str) -> Result<Vec<u8>, ReadImportAudioError>;
}

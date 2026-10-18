//! Defines import settings and validates the selected storage pair.

use clap::Args;

use super::{
    content_storage::ContentStorageConfig,
    import_storages::{ImportConfigError, ImportSourceConfig},
};

/// Settings for independently configured import source and destination storage.
#[derive(Debug, Args)]
pub(crate) struct ImportConfig {
    /// Read-only source storage settings.
    #[command(flatten)]
    source: ImportSourceConfig,
    /// Destination music-storage settings.
    #[command(flatten)]
    storage: ContentStorageConfig,
}

impl ImportConfig {
    /// Validates compatibility using the configured storage implementations.
    pub(crate) fn validate(&self) -> Result<(), ImportConfigError> {
        self.source.validate_destination(&self.storage)
    }

    /// Provides the factory for the configured import-source port.
    pub(crate) fn source(&self) -> &ImportSourceConfig {
        &self.source
    }

    /// Provides the factory for the configured music-storage port.
    pub(crate) fn storage(&self) -> &ContentStorageConfig {
        &self.storage
    }
}

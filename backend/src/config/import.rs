//! Defines import settings and validates the selected storage pair.

use clap::Args;

use crate::config::{
    content_storage::ContentStorageConfig,
    import_storages::{ImportConfigError, ImportSourceConfig},
};

/// Settings for independently configured import source and destination storage.
#[derive(getset::Getters, Debug, Args)]
pub(crate) struct ImportConfig {
    /// Read-only source storage settings.
    #[command(flatten)]
    #[getset(get = "pub(crate)")]
    source: ImportSourceConfig,
    /// Destination music-storage settings.
    #[command(flatten)]
    #[getset(get = "pub(crate)")]
    storage: ContentStorageConfig,
}

impl ImportConfig {
    /// Validates compatibility using the configured storage implementations.
    pub(crate) fn validate(&self) -> Result<(), ImportConfigError> {
        self.source.validate_destination(&self.storage)
    }
}

//! Constructs import ports from independently scoped storage settings.

use clap::Args;
use std::sync::Arc;

use crate::{
    config::content_storage::ContentStorageConfig,
    infrastructure::import_source::filesystem::{
        FileSystemImportCompatibilityError, FileSystemImportSourceConfig,
        FileSystemImportSourceConstructionError,
    },
    port::import_source::ImportSource,
};

/// A failure to construct or validate the configured import adapters.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportConfigError {
    /// The configured source cannot be constructed.
    #[error("constructing import source: {0}")]
    Source(#[from] FileSystemImportSourceConstructionError),
    /// The selected adapters cannot safely be used together.
    #[error("validating import storages: {0}")]
    Compatibility(#[from] FileSystemImportCompatibilityError),
}

/// Settings selecting and constructing the read-only import source.
#[derive(Debug, Args)]
pub(crate) struct ImportSourceConfig {
    /// Filesystem source settings owned by the concrete adapter.
    #[command(flatten)]
    file_system: FileSystemImportSourceConfig,
}

impl ImportSourceConfig {
    /// Constructs a source exposed through its storage-independent port.
    pub(crate) fn construct_import_source(
        &self,
    ) -> Result<Arc<dyn ImportSource>, ImportConfigError> {
        Ok(Arc::new(self.file_system.construct_import_source()?))
    }

    /// Delegates compatibility checks to the configured concrete storage settings.
    pub(super) fn validate_destination(
        &self,
        destination: &ContentStorageConfig,
    ) -> Result<(), ImportConfigError> {
        self.file_system
            .validate_destination(destination.root_path())?;
        Ok(())
    }
}

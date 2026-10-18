//! Defines shared stored-audio configuration independently of the consuming command.

use crate::{
    infrastructure::music_storage::filesystem::{
        FileSystemContentStorageConfig, FileSystemContentStorageConstructionError,
        FileSystemStorageConstructionError,
    },
    port::music_storage::MusicStorage,
};
use clap::Args;
use std::{path::Path, sync::Arc};

/// Settings selecting and constructing content storage for servers and imports.
#[derive(Debug, Args)]
pub(crate) struct ContentStorageConfig {
    /// Filesystem settings owned by the concrete storage adapter.
    #[command(flatten)]
    file_system: FileSystemContentStorageConfig,
}

impl ContentStorageConfig {
    /// Provides the destination for adapter-specific compatibility validation.
    pub(crate) fn root_path(&self) -> &Path {
        self.file_system.root_path()
    }

    /// Requires the delivery configuration needed when serving playable releases.
    pub(crate) fn validate_playback_url(&self) -> Result<(), FileSystemStorageConstructionError> {
        self.file_system.validate_playback_url()
    }

    /// Constructs the shared storage port with its optional delivery configuration.
    pub(crate) fn construct_music_storage(
        &self,
    ) -> Result<Arc<dyn MusicStorage>, FileSystemContentStorageConstructionError> {
        Ok(Arc::new(self.file_system.construct_music_storage()?))
    }
}

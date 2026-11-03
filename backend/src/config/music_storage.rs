//! Defines shared stored-audio configuration independently of the consuming command.

use clap::Args;
use std::{io::Error as IoError, path::PathBuf, sync::Arc};
use url::Url;

use crate::{
    infrastructure::music_storage::filesystem::{
        FileSystemMusicStorage, FileSystemStorageConstructionError, normalize_playback_url,
    },
    port::music_storage::MusicStorage,
};

/// A failure to select, validate, or construct music storage.
#[derive(Debug, thiserror::Error)]
pub(crate) enum MusicStorageConfigError {
    /// The selected implementation has no settings.
    #[error("implementation settings are required for the selected music storage mode")]
    MissingSettings,
    /// Filesystem settings are invalid or cannot construct storage.
    #[error(transparent)]
    FileSystem(#[from] FileSystemMusicStorageConfigError),
}

/// Supported music-storage implementations.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(super) enum MusicStorageMode {
    /// Stores audio in a local directory tree.
    Filesystem,
}

/// Settings selecting and constructing music storage for servers and imports.
#[derive(getset::Getters, getset::CopyGetters, Debug, Args)]
pub(crate) struct MusicStorageConfig {
    /// Storage implementation selected independently of its settings.
    #[arg(
        id = "music_storage_mode",
        long = "music-storage--mode",
        env = "TONHALLE_MUSIC_STORAGE__MODE",
        value_enum,
        default_value = "filesystem"
    )]
    #[getset(get_copy = "pub(super)")]
    mode: MusicStorageMode,
    /// Optional settings for the filesystem implementation.
    #[command(flatten)]
    #[getset(get = "pub(super)")]
    file_system: Option<FileSystemMusicStorageConfig>,
}

/// Defines validation and construction required of music-storage settings.
trait MusicStorageImplementation {
    /// Validates music-storage settings independently of the consuming command.
    fn validate(&self) -> Result<(), MusicStorageConfigError>;

    /// Constructs storage exposed through its implementation-independent port.
    fn construct(&self) -> Result<Arc<dyn MusicStorage>, MusicStorageConfigError>;
}

impl MusicStorageConfig {
    /// Selects the configured implementation and requires its settings.
    fn settings(&self) -> Result<&dyn MusicStorageImplementation, MusicStorageConfigError> {
        match self.mode {
            MusicStorageMode::Filesystem => self
                .file_system
                .as_ref()
                .map(|settings| settings as &dyn MusicStorageImplementation)
                .ok_or(MusicStorageConfigError::MissingSettings),
        }
    }

    /// Validates settings for the selected storage implementation.
    pub(crate) fn validate(&self) -> Result<(), MusicStorageConfigError> {
        self.settings()?.validate()
    }

    /// Constructs the selected storage implementation.
    pub(crate) fn construct_music_storage(
        &self,
    ) -> Result<Arc<dyn MusicStorage>, MusicStorageConfigError> {
        self.settings()?.construct()
    }
}

/// A failure to validate filesystem music-storage settings or construct storage.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FileSystemMusicStorageConfigError {
    /// Filesystem storage requires a root directory.
    #[error("FileSystemMusicStorageConfig.root_path is required")]
    MissingRootPath,
    /// The configured destination cannot be resolved.
    #[error("resolving music storage root: {0}")]
    ResolveRoot(#[from] IoError),
    /// The resolved destination does not satisfy storage requirements.
    #[error(transparent)]
    Storage(#[from] FileSystemStorageConstructionError),
}

/// CLI settings and construction rules shared by filesystem music-storage consumers.
#[derive(getset::Getters, Debug, clap::Args)]
pub(crate) struct FileSystemMusicStorageConfig {
    /// Existing directory holding stored audio releases.
    #[arg(
        id = "music_storage_root_path",
        long = "music-storage--root-path",
        env = "TONHALLE_MUSIC_STORAGE__ROOT_PATH"
    )]
    #[getset(get = "pub(super)")]
    root_path: Option<PathBuf>,
    /// Optional delivery endpoint used when reading stored releases.
    #[arg(
        id = "music_storage_playback_url_base",
        long = "music-storage--playback-url-base",
        env = "TONHALLE_MUSIC_STORAGE__PLAYBACK_URL_BASE"
    )]
    #[getset(get = "pub(super)")]
    playback_url_base: Option<Url>,
}

impl MusicStorageImplementation for FileSystemMusicStorageConfig {
    fn validate(&self) -> Result<(), MusicStorageConfigError> {
        if self.root_path.is_none() {
            return Err(FileSystemMusicStorageConfigError::MissingRootPath.into());
        }
        if let Some(url) = &self.playback_url_base {
            normalize_playback_url(&mut url.clone())
                .map_err(FileSystemMusicStorageConfigError::from)?;
        }
        Ok(())
    }

    fn construct(&self) -> Result<Arc<dyn MusicStorage>, MusicStorageConfigError> {
        self.validate()?;
        let root = self
            .root_path
            .as_ref()
            .ok_or(FileSystemMusicStorageConfigError::MissingRootPath)?
            .canonicalize()
            .map_err(FileSystemMusicStorageConfigError::from)?;
        let storage = match &self.playback_url_base {
            Some(url) => FileSystemMusicStorage::new(root, url.clone()),
            None => FileSystemMusicStorage::for_import(root),
        }
        .map_err(FileSystemMusicStorageConfigError::from)?;
        Ok(Arc::new(storage))
    }
}

/// Verifies playback requirements independently of server initialization.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_music_storage_config_validate_rejects_invalid_delivery_endpoints() {
        for (url, valid) in [
            ("https://example.com/audio", true),
            ("http://localhost:8000/", true),
            ("ftp://example.com/", false),
            ("https://example.com/?token=value", false),
            ("https://example.com/#fragment", false),
        ] {
            let config = FileSystemMusicStorageConfig {
                root_path: Some(PathBuf::from("unused")),
                playback_url_base: Some(url.parse().unwrap()),
            };
            let result = config.validate();
            if valid {
                assert!(result.is_ok(), "{url}");
            } else {
                assert!(
                    matches!(
                        result,
                        Err(MusicStorageConfigError::FileSystem(
                            FileSystemMusicStorageConfigError::Storage(
                                FileSystemStorageConstructionError::InvalidPlaybackUrlBase
                            )
                        ))
                    ),
                    "{url}"
                );
            }
        }
    }
}

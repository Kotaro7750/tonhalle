//! Defines import settings and validates the selected storage pair.

use clap::Args;
use std::{io::Error as IoError, path::Path};

use crate::config::{
    import_source::{
        FileSystemImportSourceConfigError, ImportSourceConfig, ImportSourceConfigError,
        ImportSourceMode,
    },
    music_storage::{
        FileSystemMusicStorageConfigError, MusicStorageConfig, MusicStorageConfigError,
        MusicStorageMode,
    },
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
    storage: MusicStorageConfig,
}

impl ImportConfig {
    /// Validates compatibility using the configured storage implementations.
    pub(crate) fn validate(&self) -> Result<(), ImportConfigError> {
        self.source.validate()?;
        self.storage.validate()?;

        match (self.source.mode(), self.storage.mode()) {
            (ImportSourceMode::Filesystem, MusicStorageMode::S3) => {}
            (ImportSourceMode::Filesystem, MusicStorageMode::Filesystem) => {
                let source = self
                    .source
                    .file_system()
                    .as_ref()
                    .ok_or(ImportSourceConfigError::MissingSettings)?;
                let destination = self
                    .storage
                    .file_system()
                    .as_ref()
                    .ok_or(MusicStorageConfigError::MissingSettings)?;
                let source =
                    source
                        .root_path()
                        .as_deref()
                        .ok_or(ImportSourceConfigError::FileSystem(
                            FileSystemImportSourceConfigError::MissingRootPath,
                        ))?;
                let destination = destination.root_path().as_deref().ok_or(
                    MusicStorageConfigError::FileSystem(
                        FileSystemMusicStorageConfigError::MissingRootPath,
                    ),
                )?;
                Self::validate_filesystem_compatibility(source, destination)
                    .map_err(ImportCompatibilityError::from)?;
            }
        }
        Ok(())
    }

    /// Rejects filesystem roots that would allow imports to modify their own source tree.
    fn validate_filesystem_compatibility(
        source: &Path,
        destination: &Path,
    ) -> Result<(), FileSystemImportCompatibilityError> {
        let source = source.canonicalize()?;
        let destination = destination.canonicalize()?;
        if source.starts_with(&destination) || destination.starts_with(&source) {
            return Err(FileSystemImportCompatibilityError::OverlappingRoots);
        }
        Ok(())
    }
}

/// A failure to validate the import command's source, destination, or storage pair.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportConfigError {
    /// The selected source settings are invalid.
    #[error(transparent)]
    Source(#[from] ImportSourceConfigError),
    /// The selected destination settings are invalid.
    #[error(transparent)]
    Destination(#[from] MusicStorageConfigError),
    /// The selected adapters cannot safely be used together.
    #[error("validating import source and music storage compatibility: {0}")]
    Compatibility(#[from] ImportCompatibilityError),
}

/// A failure to validate compatibility between selected storage implementations.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportCompatibilityError {
    /// The filesystem source and destination are incompatible.
    #[error(transparent)]
    FileSystem(#[from] FileSystemImportCompatibilityError),
}

/// A failure to validate filesystem source and destination compatibility.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FileSystemImportCompatibilityError {
    /// One configured root cannot be resolved.
    #[error("resolving import source and music storage roots: {0}")]
    ResolveRoot(#[from] IoError),
    /// Source and destination must not share a directory tree.
    #[error("import source and music storage directories must not overlap")]
    OverlappingRoots,
}

/// Verifies filesystem source and destination compatibility.
#[cfg(test)]
mod tests {
    use std::fs::create_dir;

    use super::*;

    #[test]
    fn validate_filesystem_compatibility_rejects_equal_and_nested_roots_and_accepts_disjoint_roots()
    {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let nested = source.join("nested");
        let destination = root.path().join("destination");
        create_dir(&source).unwrap();
        create_dir(&nested).unwrap();
        create_dir(&destination).unwrap();
        for (path, overlapping) in [
            (source.as_path(), true),
            (nested.as_path(), true),
            (root.path(), true),
            (destination.as_path(), false),
        ] {
            let result = ImportConfig::validate_filesystem_compatibility(&source, path);
            if overlapping {
                assert!(
                    matches!(
                        result,
                        Err(FileSystemImportCompatibilityError::OverlappingRoots)
                    ),
                    "{path:?}: {result:?}"
                );
            } else {
                assert!(result.is_ok(), "{path:?}: {result:?}");
            }
        }
    }
}

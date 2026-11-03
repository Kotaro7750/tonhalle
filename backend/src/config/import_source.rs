//! Validates import-source settings and constructs the selected source port.

use clap::Args;
use std::{path::PathBuf, sync::Arc};

use crate::{
    infrastructure::import_source::filesystem::{
        FileSystemImportSource, FileSystemImportSourceConstructionError,
    },
    port::import_source::ImportSource,
};

/// Settings selecting and constructing the read-only import source.
#[derive(getset::Getters, getset::CopyGetters, Debug, Args)]
pub(crate) struct ImportSourceConfig {
    /// Import-source implementation selected independently of its settings.
    #[arg(
        id = "import_source_mode",
        long = "import-source--mode",
        env = "TONHALLE_IMPORT_SOURCE__MODE",
        value_enum,
        default_value = "filesystem"
    )]
    #[getset(get_copy = "pub(super)")]
    mode: ImportSourceMode,
    /// Optional settings for the filesystem implementation.
    #[command(flatten)]
    #[getset(get = "pub(super)")]
    file_system: Option<FileSystemImportSourceConfig>,
}

impl ImportSourceConfig {
    /// Selects the configured implementation and requires its settings.
    fn settings(&self) -> Result<&dyn ImportSourceImplementation, ImportSourceConfigError> {
        match self.mode {
            ImportSourceMode::Filesystem => self
                .file_system
                .as_ref()
                .map(|settings| settings as &dyn ImportSourceImplementation)
                .ok_or(ImportSourceConfigError::MissingSettings),
        }
    }

    /// Constructs the selected import-source implementation.
    pub(crate) fn construct_import_source(
        &self,
    ) -> Result<Arc<dyn ImportSource>, ImportSourceConfigError> {
        self.settings()?.construct()
    }

    /// Validates settings for the selected source implementation.
    pub(super) fn validate(&self) -> Result<(), ImportSourceConfigError> {
        self.settings()?.validate()
    }
}

/// A failure to select, validate, or construct an import source.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ImportSourceConfigError {
    /// The selected implementation has no settings.
    #[error("implementation settings are required for the selected import source mode")]
    MissingSettings,
    /// Filesystem source settings are invalid or cannot construct a source.
    #[error(transparent)]
    FileSystem(#[from] FileSystemImportSourceConfigError),
}

/// Supported import-source implementations.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(super) enum ImportSourceMode {
    /// Reads releases from a local directory tree.
    Filesystem,
}

/// Defines validation and construction required of import-source settings.
trait ImportSourceImplementation {
    /// Validates the source settings independently of any destination.
    fn validate(&self) -> Result<(), ImportSourceConfigError>;

    /// Constructs a source exposed through its implementation-independent port.
    fn construct(&self) -> Result<Arc<dyn ImportSource>, ImportSourceConfigError>;
}

/// CLI settings and construction rules for the filesystem import source.
#[derive(getset::Getters, Debug, clap::Args)]
pub(crate) struct FileSystemImportSourceConfig {
    /// Directory tree containing release candidates.
    #[arg(
        id = "import_source_root_path",
        long = "import-source--root-path",
        env = "TONHALLE_IMPORT_SOURCE__ROOT_PATH"
    )]
    #[getset(get = "pub(super)")]
    root_path: Option<PathBuf>,
}

/// A failure specific to filesystem import-source configuration.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FileSystemImportSourceConfigError {
    /// Filesystem imports require a source directory.
    #[error("FileSystemImportSourceConfig.root_path is required")]
    MissingRootPath,
    /// The configured source cannot be constructed.
    #[error("constructing import source: {0}")]
    Source(#[from] FileSystemImportSourceConstructionError),
}

impl ImportSourceImplementation for FileSystemImportSourceConfig {
    fn construct(&self) -> Result<Arc<dyn ImportSource>, ImportSourceConfigError> {
        let source = FileSystemImportSource::new(
            self.root_path
                .as_ref()
                .ok_or(FileSystemImportSourceConfigError::MissingRootPath)?
                .clone(),
        )
        .map_err(FileSystemImportSourceConfigError::from)?;
        Ok(Arc::new(source))
    }

    fn validate(&self) -> Result<(), ImportSourceConfigError> {
        if self.root_path.is_none() {
            return Err(FileSystemImportSourceConfigError::MissingRootPath.into());
        }
        Ok(())
    }
}

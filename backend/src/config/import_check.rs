//! Defines source-only settings for import validation.

use clap::Args;

use crate::config::import_source::{ImportSourceConfig, ImportSourceConfigError};

/// Settings for checking releases without configuring a destination.
#[derive(Debug, Args, getset::Getters)]
pub(crate) struct ImportCheckConfig {
    /// Read-only source storage settings.
    #[command(flatten)]
    #[getset(get = "pub(crate)")]
    source: ImportSourceConfig,
}

impl ImportCheckConfig {
    /// Validates the selected source independently of destination settings.
    pub(crate) fn validate(&self) -> Result<(), ImportSourceConfigError> {
        self.source.validate()
    }
}

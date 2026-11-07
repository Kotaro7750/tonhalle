//! Defines command-line inputs and subcommand startup handlers.

use clap::{Parser, Subcommand};

use crate::config::{import::ImportConfig, import_check::ImportCheckConfig, server::ServerConfig};

pub(crate) mod import;
pub(crate) mod import_check;
pub(crate) mod server;

/// Command-line interface for backend operations.
#[derive(Debug, Parser)]
#[command(version, about)]
#[derive(getset::Getters)]
pub(crate) struct Cli {
    /// Selects the operation to execute.
    #[command(subcommand)]
    #[getset(get = "pub(crate)")]
    command: Command,
}

/// Supported backend operations.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Start the API server.
    Server(ServerConfig),
    /// Import audio releases without updating the metadata index.
    Import(ImportConfig),
    /// Read and validate source releases without destination storage.
    ImportCheck(ImportCheckConfig),
}

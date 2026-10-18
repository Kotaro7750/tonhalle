//! Defines command-line inputs and subcommand startup handlers.

pub(crate) mod import;
pub(crate) mod server;

use crate::config::{ImportConfig, ServerConfig};
use clap::{Parser, Subcommand};

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
}

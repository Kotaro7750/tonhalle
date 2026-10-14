//! Defines command-line inputs and subcommand startup handlers.

pub(crate) mod server;

use crate::config::ServerConfig;
use clap::{Parser, Subcommand};

/// Command-line interface for backend operations.
#[derive(Debug, Parser)]
#[command(version, about)]
pub(crate) struct Cli {
    /// Selects the operation to execute.
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// Supported backend operations.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Start the API server.
    Server(ServerConfig),
}

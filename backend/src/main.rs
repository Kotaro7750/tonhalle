//! Parses backend commands and dispatches their startup handlers.

use clap::Parser;

use crate::cmd::{import::run as run_import, server::run as run_server};

mod cmd;
mod config;
mod controller;
mod domain;
mod infrastructure;
mod observability;
mod port;
mod usecase;

/// Parses the selected command and runs it until completion.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cmd::Cli::parse();
    observability::init_logging();

    match cli.command() {
        cmd::Command::Server(config) => run_server(config).await?,
        cmd::Command::Import(config) => run_import(config).await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_support;

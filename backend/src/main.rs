//! Parses backend commands and dispatches their startup handlers.

use clap::Parser;

use crate::{
    cmd::{
        import::run as run_import, import_check::run as run_import_check, server::run as run_server,
    },
    observability::LogMode,
};

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
    let mode = match cli.command() {
        cmd::Command::Server(_) => LogMode::Server,
        cmd::Command::Import(_) | cmd::Command::ImportCheck(_) => LogMode::Cli,
    };
    observability::init_logging(mode);

    match cli.command() {
        cmd::Command::Server(config) => run_server(config).await?,
        cmd::Command::Import(config) => run_import(config).await?,
        cmd::Command::ImportCheck(config) => run_import_check(config).await?,
    }
    Ok(())
}

#[cfg(test)]
mod test_support;

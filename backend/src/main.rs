//! Parses backend commands and dispatches their startup handlers.

mod cmd;
mod config;
mod controller;
mod domain;
mod infrastructure;
mod observability;
mod port;
mod usecase;

use clap::Parser;

/// Parses the selected command and runs it until completion.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = cmd::Cli::parse();
    observability::init_logging();

    match cli.command {
        cmd::Command::Server(config) => cmd::server::run(config).await?,
    }
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_support;

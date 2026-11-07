//! Configures application logging for server and CLI commands.

use std::io::stdout;
use tracing_subscriber::EnvFilter;

/// Selects logging behavior for the command's intended use.
pub(crate) enum LogMode {
    /// Emits structured JSON logs for server operation.
    Server,
    /// Emits concise text logs for interactive CLI use.
    Cli,
}

/// Installs command-specific logging with RUST_LOG filtering and an info-level fallback.
pub(crate) fn init_logging(mode: LogMode) {
    let builder = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_writer(stdout)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        );

    match mode {
        LogMode::Server => builder.json().init(),
        LogMode::Cli => builder.compact().without_time().with_target(false).init(),
    }
}

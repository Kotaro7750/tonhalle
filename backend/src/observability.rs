//! Configures structured application logging.

use tracing_subscriber::EnvFilter;

/// Installs JSON logging with RUST_LOG filtering and an info-level fallback.
pub(crate) fn init_logging() {
    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_writer(std::io::stdout)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();
}

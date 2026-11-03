//! Runs imports using storage-independent configuration factories.

use crate::{config::import::ImportConfig, usecase::import_music::ImportMusicUseCase};

/// Constructs configured storage ports, runs the import worker, and reports its outcome.
pub(crate) async fn run(config: &ImportConfig) -> anyhow::Result<()> {
    config.validate()?;
    let source = config.source().construct_import_source()?;
    let storage = config.storage().construct_music_storage().await?;
    let summary = ImportMusicUseCase::new(source, storage).execute().await?;
    println!(
        "saved: {}, skipped: {}, failed: {}",
        summary.saved(),
        summary.skipped(),
        summary.failed()
    );
    anyhow::ensure!(
        summary.failed() == 0,
        "{} release imports failed",
        summary.failed()
    );
    Ok(())
}

//! Reports validated source releases without accessing destination storage.

use crate::{config::import_check::ImportCheckConfig, usecase::import_music::ImportCheckUseCase};

/// Reads source releases, prints their identities and titles, and reports validation failures.
pub(crate) async fn run(config: &ImportCheckConfig) -> anyhow::Result<()> {
    config.validate()?;
    let source = config.source().construct_import_source()?;
    let result = ImportCheckUseCase::new(source).execute().await?;
    for (_, release) in result.releases() {
        println!("{}\t{}", release.id(), release.title());
    }
    anyhow::ensure!(
        result.failed() == 0,
        "{} release checks failed",
        result.failed()
    );
    Ok(())
}

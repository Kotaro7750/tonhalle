//! Starts the API server and wires storage, indexing, and HTTP services.

mod config;
mod controller;
mod domain;
mod infrastructure;
mod observability;
mod port;
mod usecase;

use std::{net::SocketAddr, sync::Arc};

use axum::{Router, http::Method};
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

use crate::{
    config::TonhalleConfig,
    controller::MusicController,
    infrastructure::{
        metadata_index::InMemoryMetadataIndex, music_storage::FileSystemMusicStorage,
    },
    usecase::{BuildMetadataIndexUseCase, GetReleaseUseCase, GetTrackUseCase, ListReleasesUseCase},
};

/// Builds the startup index and serves the configured API until shutdown.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::init_logging();

    let config = TonhalleConfig::load()?;
    let cors_origin = config.cors_origin()?;

    let storage = Arc::new(FileSystemMusicStorage::new(
        config.content_storage().root_path().to_path_buf(),
        config.content_storage().playback_url_base().clone(),
    )?);

    tracing::info!("Scanning music storage");
    let releases =
        tokio::task::spawn_blocking(move || BuildMetadataIndexUseCase::new(storage).execute())
            .await??;
    tracing::info!(
        release_count = releases.len(),
        track_count = releases
            .iter()
            .map(|release| release.tracks().len())
            .sum::<usize>(),
        "Startup index built"
    );

    let index = Arc::new(InMemoryMetadataIndex::new(releases));
    let controller = MusicController::new(
        ListReleasesUseCase::new(index.clone()),
        GetReleaseUseCase::new(index.clone()),
        GetTrackUseCase::new(index),
    );

    let mut app = Router::new()
        .nest("/api/v0", controller.router())
        .route("/health", axum::routing::get(|| async { "OK" }))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        );

    if let Some(origin) = cors_origin {
        app = app.layer(
            CorsLayer::new()
                .allow_origin(origin)
                .allow_methods([Method::GET, Method::HEAD]),
        );
    }

    let listener = tokio::net::TcpListener::bind(config.listen_address()).await?;
    tracing::info!(address = %listener.local_addr()?, "Tonhalle server listening");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

/// Waits for Ctrl+C or SIGTERM to initiate graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Signal received; shutting down");
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_support;

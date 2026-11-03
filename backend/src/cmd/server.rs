//! Initializes storage and serves the HTTP API.

use axum::{Router, http::Method, routing::get};
use std::{io::Error as IoError, net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    signal::ctrl_c,
    task::{JoinError, spawn_blocking},
};
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

use crate::{
    config::{
        music_storage::MusicStorageConfigError,
        server::{CorsOriginError, ServerConfig, ServerConfigError},
    },
    controller::MusicController,
    infrastructure::metadata_index::InMemoryMetadataIndex,
    usecase::{
        GetReleaseUseCase, GetTrackUseCase, ListReleasesUseCase,
        build_metadata_index::{BuildMetadataIndexError, BuildMetadataIndexUseCase},
    },
};

/// A failure to initialize or run the API server.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerError {
    /// Command-specific configuration requirements are not satisfied.
    #[error(transparent)]
    Config(#[from] ServerConfigError),
    /// The configured browser origin is invalid.
    #[error(transparent)]
    Cors(#[from] CorsOriginError),
    /// Music storage cannot be initialized.
    #[error(transparent)]
    Storage(#[from] MusicStorageConfigError),
    /// Startup indexing failed.
    #[error(transparent)]
    Index(#[from] BuildMetadataIndexError),
    /// The indexing worker failed.
    #[error(transparent)]
    Worker(#[from] JoinError),
    /// Network or server I/O failed.
    #[error(transparent)]
    Io(#[from] IoError),
}

/// Builds the startup index and serves the configured API until shutdown.
pub(crate) async fn run(config: &ServerConfig) -> Result<(), ServerError> {
    config.validate()?;
    let cors_origin = config.cors_origin()?;

    let storage = config.music_storage().construct_music_storage()?;

    tracing::info!("Scanning music storage");
    let releases =
        spawn_blocking(move || BuildMetadataIndexUseCase::new(storage).execute()).await??;
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
        .route("/health", get(|| async { "OK" }))
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

    let listener = TcpListener::bind(config.listen_address()).await?;
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
        ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};

        signal(SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = {
        use std::future::pending;

        pending::<()>()
    };

    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("Signal received; shutting down");
}

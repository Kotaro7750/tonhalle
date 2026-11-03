//! Initializes storage and serves the HTTP API.

use axum::{
    Router,
    http::{
        HeaderValue, Method,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    routing::get,
};
use std::{io::Error as IoError, net::SocketAddr, sync::Arc};
use tokio::{net::TcpListener, signal::ctrl_c};
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

use crate::{
    config::{
        authorization::AuthorizationError,
        content_access::ContentAccessConfigError,
        music_storage::MusicStorageConfigError,
        server::{CorsOriginError, ServerConfig, ServerConfigError},
    },
    controller::{MusicController, content_access::ContentAccessController},
    infrastructure::metadata_index::InMemoryMetadataIndex,
    usecase::{
        GetReleaseUseCase, GetTrackUseCase, ListReleasesUseCase,
        build_metadata_index::{BuildMetadataIndexError, BuildMetadataIndexUseCase},
        content_access::ContentAccessUseCase,
    },
};

/// A failure to initialize or run the API server.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ServerError {
    /// Delivery-access provider cannot be initialized.
    #[error(transparent)]
    ContentAccess(#[from] ContentAccessConfigError),
    /// Provider signing keys cannot be loaded.
    #[error(transparent)]
    Authorization(#[from] AuthorizationError),
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
    /// Network or server I/O failed.
    #[error(transparent)]
    Io(#[from] IoError),
}

/// Builds the startup index and serves the configured API until shutdown.
pub(crate) async fn run(config: &ServerConfig) -> Result<(), ServerError> {
    config.validate()?;
    let cors_origin = config.cors_origin()?;

    let storage = config.music_storage().construct_music_storage().await?;

    tracing::info!("Scanning music storage");
    let releases = BuildMetadataIndexUseCase::new(storage).execute().await?;
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

    let content_access = ContentAccessController::new(ContentAccessUseCase::new(
        config.content_access().construct()?,
    ));
    let api_router = controller.router().merge(content_access.router());
    let api = config.authorization().protect_api(api_router).await?;

    let app = build_app(api, cors_origin);

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

/// Mounts protected API routes alongside public health and CORS handling.
pub(crate) fn build_app(api: Router, cors_origin: Option<HeaderValue>) -> Router {
    let mut app = Router::new()
        .nest("/api/v0", api)
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
                .allow_methods([Method::GET, Method::HEAD, Method::POST, Method::DELETE])
                .allow_credentials(true)
                .allow_headers([AUTHORIZATION, CONTENT_TYPE]),
        );
    }

    app
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

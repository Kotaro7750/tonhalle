//! Initializes storage and serves the HTTP API.

use axum::{
    Router,
    http::{
        HeaderValue, Method,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    routing::get,
};
use std::{future::IntoFuture, io::Error as IoError, net::SocketAddr, sync::Arc};
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
        build_metadata_index::BuildMetadataIndexUseCase, content_access::ContentAccessUseCase,
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
    /// Network or server I/O failed.
    #[error(transparent)]
    Io(#[from] IoError),
}

/// Serves the API while indexing stored releases in the background.
pub(crate) async fn run(config: &ServerConfig) -> Result<(), ServerError> {
    config.validate()?;
    let cors_origin = config.cors_origin()?;

    let storage = config.music_storage().construct_music_storage().await?;

    let index = Arc::new(InMemoryMetadataIndex::new());
    let scan = BuildMetadataIndexUseCase::new(storage, index.clone());
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

    serve_with_scan(listener, app, scan, shutdown_signal()).await?;

    Ok(())
}

/// Supervises indexing alongside HTTP and cancels further scan work on shutdown.
async fn serve_with_scan(
    listener: TcpListener,
    app: Router,
    scan: BuildMetadataIndexUseCase,
    shutdown: impl std::future::Future<Output = ()> + Send + 'static,
) -> Result<(), IoError> {
    let mut worker = tokio::spawn(async move {
        tracing::info!("Music storage scan started");
        scan.execute().await
    });
    let abort = worker.abort_handle();
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown.await;
        abort.abort();
    })
    .into_future();
    tokio::pin!(server);
    tokio::select! {
        result = &mut server => {
            worker.abort();
            let _ = worker.await;
            result
        }
        result = &mut worker => {
            match result {
                Ok(Ok(())) => tracing::info!("Music storage scan completed"),
                Ok(Err(error)) => tracing::error!(?error, "Music storage scan failed"),
                Err(error) => tracing::error!(?error, "Music storage scan worker failed"),
            }
            server.await
        }
    }
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

/// Exercises HTTP availability and shutdown with unfinished or failed scans.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{id::ReleaseId, import::ImportAudio, primitive::release::Release},
        port::music_storage::{
            DeleteStagingError, MusicStorage, ReleaseScanFailure, SaveReleaseError,
            SaveReleaseOutcome, ScanReleasesError,
        },
    };
    use futures_util::{
        StreamExt,
        stream::{self, BoxStream},
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
        sync::{Notify, oneshot},
        time::{Duration, timeout},
    };

    /// Signals when the scan is polled, then waits, fails, or panics.
    struct TestStorage {
        started: Arc<Notify>,
        mode: u8,
    }

    #[async_trait::async_trait]
    impl MusicStorage for TestStorage {
        async fn stage_audio(&self, _: ImportAudio) -> Result<(), SaveReleaseError> {
            unreachable!()
        }
        async fn commit_release(
            &self,
            _: &ReleaseId,
            _: bool,
        ) -> Result<SaveReleaseOutcome, SaveReleaseError> {
            unreachable!()
        }
        async fn delete_staging(&self, _: Option<&ReleaseId>) -> Result<(), DeleteStagingError> {
            unreachable!()
        }
        fn scan_releases(
            &self,
        ) -> BoxStream<'_, Result<Result<Release, ReleaseScanFailure>, ScanReleasesError>> {
            stream::once(async move {
                self.started.notify_one();
                match self.mode {
                    0 => std::future::pending().await,
                    1 => Err(ScanReleasesError::new(IoError::other("scan failed"))),
                    _ => panic!("scan worker panic"),
                }
            })
            .boxed()
        }
    }

    #[tokio::test]
    async fn serve_with_scan_keeps_http_available_during_scanning_and_after_failure() {
        for mode in 0..3 {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let started = Arc::new(Notify::new());
            let scan = BuildMetadataIndexUseCase::new(
                Arc::new(TestStorage {
                    started: started.clone(),
                    mode,
                }),
                Arc::new(InMemoryMetadataIndex::new()),
            );
            let (stop, stopped) = oneshot::channel();
            let server = tokio::spawn(serve_with_scan(
                listener,
                build_app(Router::new(), None),
                scan,
                async {
                    let _ = stopped.await;
                },
            ));
            timeout(Duration::from_secs(5), started.notified())
                .await
                .unwrap();
            let response = timeout(Duration::from_secs(5), async {
                let mut client = TcpStream::connect(address).await.unwrap();
                client
                    .write_all(
                        b"GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .await
                    .unwrap();
                let mut response = String::new();
                client.read_to_string(&mut response).await.unwrap();
                response
            })
            .await
            .unwrap();
            assert!(response.starts_with("HTTP/1.1 200"), "{response}");
            stop.send(()).unwrap();
            timeout(Duration::from_secs(5), server)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
        }
    }
}

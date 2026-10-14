//! Supports API integration tests with sample audio and OpenAPI response validation.

use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use async_trait::async_trait;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::{
    controller::MusicController,
    domain::{
        ArtistId, ReleaseId, TrackId,
        primitive::{release::Release, track::Track},
    },
    infrastructure::{
        metadata_index::InMemoryMetadataIndex, music_storage::FileSystemMusicStorage,
    },
    port::{self, MetadataIndex, ReleasePage},
    usecase::{BuildMetadataIndexUseCase, GetReleaseUseCase, GetTrackUseCase, ListReleasesUseCase},
};

mod errors;
mod reads;

/// Opens repository audio fixtures through the production storage adapter.
fn sample_storage() -> FileSystemMusicStorage {
    FileSystemMusicStorage::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../sample"),
        "https://media.example.com/music/".parse().unwrap(),
    )
    .unwrap()
}

/// Builds the production index from repository audio fixtures.
fn sample_releases() -> Vec<Release> {
    BuildMetadataIndexUseCase::new(Arc::new(sample_storage()))
        .execute()
        .unwrap()
}

/// Wires HTTP routes to the index supplied by an integration test.
fn router(index: Arc<dyn MetadataIndex>) -> Router {
    Router::new().nest(
        "/api/v0",
        MusicController::new(
            ListReleasesUseCase::new(index.clone()),
            GetReleaseUseCase::new(index.clone()),
            GetTrackUseCase::new(index),
        )
        .router(),
    )
}

/// Sends an HTTP request and validates response media type and body against OpenAPI.
async fn request(app: Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();

    assert_eq!(
        response.headers()["content-type"],
        if status.is_success() {
            "application/json"
        } else {
            "application/problem+json"
        }
    );

    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let schema = match status {
        StatusCode::OK if uri.split('?').next() == Some("/api/v0/releases") => "ReleasePage",
        StatusCode::OK if uri.starts_with("/api/v0/releases/") => "Release",
        StatusCode::OK => "Track",
        StatusCode::BAD_REQUEST => "BadRequestProblem",
        StatusCode::NOT_FOUND => "NotFoundProblem",
        StatusCode::INTERNAL_SERVER_ERROR => "InternalServerErrorProblem",
        _ => panic!("unexpected status: {status}"),
    };

    let errors = CONTRACT_VALIDATORS[schema]
        .iter_errors(&body)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();

    assert!(errors.is_empty(), "{uri}: {}", errors.join("; "));

    (status, body)
}

/// Validates success and error responses against the current API contract.
static CONTRACT_VALIDATORS: LazyLock<BTreeMap<&'static str, jsonschema::Validator>> =
    LazyLock::new(|| {
        let contract: Value =
            serde_yaml_ng::from_str(include_str!("../../contracts/api/openapi.yaml")).unwrap();

        [
            "ReleasePage",
            "Release",
            "Track",
            "BadRequestProblem",
            "NotFoundProblem",
            "InternalServerErrorProblem",
        ]
        .into_iter()
        .map(|name| {
            let mut schema = contract.clone();
            schema["$ref"] = json!(format!("#/components/schemas/{name}"));

            let validator = jsonschema::draft202012::options()
                .should_validate_formats(true)
                .build(&schema)
                .unwrap();

            (name, validator)
        })
        .collect()
    });

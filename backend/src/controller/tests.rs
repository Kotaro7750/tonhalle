//! Verifies HTTP boundary behavior with controlled metadata-index results.

use async_trait::async_trait;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use jsonschema::draft202012::options;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::Error as IoError,
    sync::{Arc, LazyLock},
};
use tower::ServiceExt;

use crate::{
    controller::MusicController,
    domain::{
        id::{ReleaseId, TrackId},
        ingestion::metadata::sample_metadata,
        primitive::{release::Release, track::Track},
    },
    port::metadata_index::{self, MetadataIndex, ReleasePage},
    test_support::sample_release,
    usecase::{GetReleaseUseCase, GetTrackUseCase, ListReleasesUseCase},
};

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
            serde_yaml_ng::from_str(include_str!("../../../contracts/api/openapi.yaml")).unwrap();

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

            let validator = options()
                .should_validate_formats(true)
                .build(&schema)
                .unwrap();

            (name, validator)
        })
        .collect()
    });

/// Returns controlled query results without storage or pagination logic.
struct FixtureIndex {
    release: Release,
}

impl FixtureIndex {
    /// Supplies one directly constructed domain release.
    fn new() -> Self {
        Self {
            release: sample_release(&sample_metadata()),
        }
    }
}

#[async_trait]
impl MetadataIndex for FixtureIndex {
    async fn add_release(&self, _: Release) -> Result<(), metadata_index::AddReleaseError> {
        panic!("read-only fixture")
    }

    /// Exposes a fixed page while checking HTTP argument interpretation.
    async fn list_releases(
        &self,
        limit: u8,
        cursor: Option<String>,
    ) -> Result<ReleasePage, metadata_index::ListReleasesError> {
        match cursor.as_deref() {
            Some("invalid") => Err(metadata_index::ListReleasesError::InvalidCursor),
            Some("opaque-token") => {
                assert_eq!(limit, 7);
                Ok(ReleasePage::new(
                    vec![self.release.clone()],
                    Some("next-token".into()),
                ))
            }
            None => {
                assert_eq!(limit, 30);
                Ok(ReleasePage::new(vec![self.release.clone()], None))
            }
            other => panic!("unexpected cursor: {other:?}"),
        }
    }

    /// Returns a release only for its requested identity.
    async fn get_release(
        &self,
        id: &ReleaseId,
    ) -> Result<Release, metadata_index::GetReleaseError> {
        if id == self.release.id() {
            Ok(self.release.clone())
        } else {
            Err(metadata_index::GetReleaseError::NotFound)
        }
    }

    /// Returns a track only for its requested identity.
    async fn get_track(&self, id: &TrackId) -> Result<Track, metadata_index::GetTrackError> {
        self.release
            .tracks()
            .iter()
            .find(|track| track.id() == id)
            .cloned()
            .ok_or(metadata_index::GetTrackError::NotFound)
    }
}

#[tokio::test]
async fn list_releases_passes_pagination_arguments() {
    let app = router(Arc::new(FixtureIndex::new()));
    let (status, _) = request(app, "/api/v0/releases?limit=7&cursor=opaque-token").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn music_controller_serves_each_read_route_with_the_requested_entity() {
    let index = FixtureIndex::new();
    let release = index.release.clone();
    let track = &release.tracks()[0];
    let app = router(Arc::new(index));
    let (status, page) = request(app.clone(), "/api/v0/releases").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["items"][0]["id"], release.id().to_string());

    let (status, detail) =
        request(app.clone(), &format!("/api/v0/releases/{}", release.id())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["id"], release.id().to_string());

    let (status, detail) = request(app, &format!("/api/v0/tracks/{}", track.id())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail["id"], track.id().to_string());
}

#[tokio::test]
async fn list_releases_rejects_out_of_range_limits_and_empty_cursors() {
    let app = router(Arc::new(FixtureIndex::new()));

    for uri in [
        "/api/v0/releases?limit=0",
        "/api/v0/releases?limit=101",
        "/api/v0/releases?cursor=",
    ] {
        let (status, body) = request(app.clone(), uri).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
        assert_eq!(body["detail"], "invalid limit or cursor");
    }
}

#[tokio::test]
async fn list_releases_maps_query_rejection_to_a_bad_request_problem() {
    let (status, body) = request(
        router(Arc::new(FixtureIndex::new())),
        "/api/v0/releases?limit=abc",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], 400);
    assert_eq!(body["type"], "about:blank");
    assert_eq!(body["title"], "Bad Request");
}

#[tokio::test]
async fn list_releases_maps_invalid_cursor_errors_to_bad_requests() {
    let (status, body) = request(
        router(Arc::new(FixtureIndex::new())),
        "/api/v0/releases?cursor=invalid",
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["detail"], "invalid cursor");
}

#[tokio::test]
async fn get_release_and_get_track_map_id_parse_errors_to_bad_requests() {
    let app = router(Arc::new(FixtureIndex::new()));

    for uri in ["/api/v0/releases/invalid", "/api/v0/tracks/invalid"] {
        let (status, _) = request(app.clone(), uri).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn get_release_and_get_track_map_path_rejection_to_bad_requests() {
    let app = router(Arc::new(FixtureIndex::new()));

    for uri in ["/api/v0/releases/%FF", "/api/v0/tracks/%FF"] {
        let (status, _) = request(app.clone(), uri).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
async fn get_release_and_get_track_map_missing_entities_to_not_found() {
    let app = router(Arc::new(FixtureIndex::new()));

    for (kind, route) in [("release", "releases"), ("track", "tracks")] {
        let (status, body) = request(
            app.clone(),
            &format!("/api/v0/{route}/{kind}-{}", "0".repeat(64)),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body["status"], 404);
    }
}

/// Simulates index failures containing details that must remain private.
struct FailingIndex;

#[async_trait]
impl MetadataIndex for FailingIndex {
    async fn add_release(&self, _: Release) -> Result<(), metadata_index::AddReleaseError> {
        panic!("read-only fixture")
    }

    /// Simulates an internal listing failure with private infrastructure details.
    async fn list_releases(
        &self,
        _: u8,
        _: Option<String>,
    ) -> Result<ReleasePage, metadata_index::ListReleasesError> {
        Err(metadata_index::ListReleasesError::Internal(Box::new(
            IoError::other("secret filesystem path"),
        )))
    }

    /// Simulates an internal release lookup failure with private infrastructure details.
    async fn get_release(&self, _: &ReleaseId) -> Result<Release, metadata_index::GetReleaseError> {
        Err(metadata_index::GetReleaseError::Internal(Box::new(
            IoError::other("secret filesystem path"),
        )))
    }

    /// Simulates an internal track lookup failure with private infrastructure details.
    async fn get_track(&self, _: &TrackId) -> Result<Track, metadata_index::GetTrackError> {
        Err(metadata_index::GetTrackError::Internal(Box::new(
            IoError::other("secret filesystem path"),
        )))
    }
}

#[tokio::test]
async fn music_api_hides_infrastructure_details_in_internal_errors() {
    let app = router(Arc::new(FailingIndex));

    for uri in [
        "/api/v0/releases".into(),
        format!("/api/v0/releases/release-{}", "a".repeat(64)),
        format!("/api/v0/tracks/track-{}", "a".repeat(64)),
    ] {
        let (status, body) = request(app.clone(), &uri).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            body,
            json!({"type": "about:blank", "title": "Internal Server Error", "status": 500})
        );
    }
}

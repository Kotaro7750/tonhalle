//! Verifies HTTP request rejection and safe problem responses.

use super::*;

#[tokio::test]
async fn music_api_returns_problem_json_for_invalid_requests_and_missing_entities() {
    let app = router(Arc::new(InMemoryMetadataIndex::new(sample_releases())));

    for uri in [
        "/api/v0/releases?limit=0",
        "/api/v0/releases?limit=101",
        "/api/v0/releases?limit=-1",
        "/api/v0/releases?limit=256",
        "/api/v0/releases?limit=abc",
        "/api/v0/releases?limit=1.5",
        "/api/v0/releases?limit=",
        "/api/v0/releases?limit=1&limit=2",
        "/api/v0/releases?cursor=",
        "/api/v0/releases?cursor=invalid",
        "/api/v0/releases/invalid",
        "/api/v0/tracks/invalid",
        "/api/v0/releases/%FF",
        "/api/v0/tracks/%FF",
    ] {
        let (status, body) = request(app.clone(), uri).await;

        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
        assert_eq!(body["status"], 400);
        assert_eq!(body["type"], "about:blank");
        assert_eq!(body["title"], "Bad Request");
    }

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

#[tokio::test]
async fn music_api_rejects_wrong_id_kinds_and_noncanonical_hashes() {
    let app = router(Arc::new(InMemoryMetadataIndex::new(sample_releases())));

    for (route, kind, other) in [
        ("releases", "release", "track"),
        ("tracks", "track", "release"),
    ] {
        for id in [
            format!("{other}-{}", "a".repeat(64)),
            format!("{kind}-{}", "A".repeat(64)),
            format!("{kind}-{}", "a".repeat(63)),
            format!("{kind}-{}", "a".repeat(65)),
        ] {
            let (status, problem) = request(app.clone(), &format!("/api/v0/{route}/{id}")).await;

            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(problem["status"], 400);
        }
    }
}

/// Simulates index failures containing details that must remain private.
struct FailingIndex;

#[async_trait]
impl MetadataIndex for FailingIndex {
    /// Simulates an internal listing failure with private infrastructure details.
    async fn list_releases(
        &self,
        _: u8,
        _: Option<String>,
    ) -> Result<ReleasePage, port::ListReleasesError> {
        Err(port::ListReleasesError::Internal(Box::new(
            std::io::Error::other("secret filesystem path"),
        )))
    }

    /// Simulates an internal release lookup failure with private infrastructure details.
    async fn get_release(&self, _: &ReleaseId) -> Result<Release, port::GetReleaseError> {
        Err(port::GetReleaseError::Internal(Box::new(
            std::io::Error::other("secret filesystem path"),
        )))
    }

    /// Simulates an internal track lookup failure with private infrastructure details.
    async fn get_track(&self, _: &TrackId) -> Result<Track, port::GetTrackError> {
        Err(port::GetTrackError::Internal(Box::new(
            std::io::Error::other("secret filesystem path"),
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

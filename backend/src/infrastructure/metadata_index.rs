//! Stores the startup music index and implements cursor-based browsing.

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{
    collections::BTreeMap,
    ops::Bound::{Excluded, Unbounded},
};

use crate::{
    domain::{
        id::{ReleaseId, TrackId},
        primitive::{release::Release, track::Track},
    },
    port::metadata_index::{
        GetReleaseError, GetTrackError, ListReleasesError, MetadataIndex, ReleasePage,
    },
};

/// An identifier-ordered snapshot of accepted releases and tracks.
pub(crate) struct InMemoryMetadataIndex {
    releases: BTreeMap<ReleaseId, Release>,
    tracks: BTreeMap<TrackId, Track>,
}

impl InMemoryMetadataIndex {
    /// Indexes accepted entities for release browsing and direct track lookup.
    pub(crate) fn new(releases: Vec<Release>) -> Self {
        let tracks = releases
            .iter()
            .flat_map(|release| release.tracks().iter())
            .map(|track| (track.id().to_owned(), track.clone()))
            .collect();

        Self {
            releases: releases
                .into_iter()
                .map(|release| (release.id().to_owned(), release))
                .collect(),
            tracks,
        }
    }

    /// Accepts a versioned continuation token only when its release is in this index.
    fn decode_cursor(&self, cursor: String) -> Result<ReleaseId, ListReleasesError> {
        let bytes = URL_SAFE_NO_PAD
            .decode(cursor)
            .map_err(|_| ListReleasesError::InvalidCursor)?;
        let value = String::from_utf8(bytes).map_err(|_| ListReleasesError::InvalidCursor)?;
        let id = value
            .strip_prefix("tonhalle-release-cursor-v0:")
            .ok_or(ListReleasesError::InvalidCursor)?;
        let id = id
            .parse::<ReleaseId>()
            .map_err(|_| ListReleasesError::InvalidCursor)?;

        if !self.releases.contains_key(&id) {
            return Err(ListReleasesError::InvalidCursor);
        }

        Ok(id)
    }
}

#[async_trait]
impl MetadataIndex for InMemoryMetadataIndex {
    /// Reads an identifier-ordered page and encodes its continuation position.
    async fn list_releases(
        &self,
        limit: u8,
        cursor: Option<String>,
    ) -> Result<ReleasePage, ListReleasesError> {
        let cursor = cursor
            .map(|cursor| self.decode_cursor(cursor))
            .transpose()?;
        let start = cursor.as_ref().map_or(Unbounded, Excluded);

        let mut items = self
            .releases
            .range::<ReleaseId, _>((start, Unbounded))
            .take(usize::from(limit) + 1)
            .map(|(_, release)| release.clone())
            .collect::<Vec<_>>();

        let next_cursor = if items.len() > usize::from(limit) {
            items.pop();
            items.last().map(|release| {
                URL_SAFE_NO_PAD.encode(format!("tonhalle-release-cursor-v0:{}", release.id()))
            })
        } else {
            None
        };

        Ok(ReleasePage::new(items, next_cursor))
    }

    /// Looks up an accepted release in the startup snapshot.
    async fn get_release(&self, id: &ReleaseId) -> Result<Release, GetReleaseError> {
        self.releases
            .get(id)
            .cloned()
            .ok_or(GetReleaseError::NotFound)
    }

    /// Looks up an accepted track in the startup snapshot.
    async fn get_track(&self, id: &TrackId) -> Result<Track, GetTrackError> {
        self.tracks.get(id).cloned().ok_or(GetTrackError::NotFound)
    }
}

/// Verifies index lookup, ordering, and cursor continuation independently of HTTP.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::ingestion::metadata::sample_metadata, test_support::sample_release};

    /// Builds distinct releases without parsing audio files.
    fn releases() -> Vec<Release> {
        (0..3)
            .map(|i| {
                let mut fixture = sample_metadata();
                *fixture.release_title_mut() = format!("Release {i}");
                sample_release(&fixture)
            })
            .collect()
    }

    #[tokio::test]
    async fn list_releases_preserves_order_and_resumes_without_duplicates_when_limit_changes() {
        let index = InMemoryMetadataIndex::new(releases());
        let all = index.list_releases(100, None).await.unwrap();
        assert!(
            all.items()
                .windows(2)
                .all(|pair| pair[0].id() < pair[1].id())
        );
        let first = index.list_releases(1, None).await.unwrap();
        assert_eq!(first.items().len(), 1);
        let rest = index
            .list_releases(100, first.next_cursor().map(str::to_owned))
            .await
            .unwrap();
        assert!(rest.next_cursor().is_none());
        let ids: Vec<_> = first
            .items()
            .iter()
            .chain(rest.items())
            .map(|r| r.id())
            .collect();
        assert_eq!(ids, all.items().iter().map(|r| r.id()).collect::<Vec<_>>());

        let mut cursor = None;
        let mut ids = Vec::new();
        loop {
            let page = index.list_releases(1, cursor).await.unwrap();
            ids.extend(page.items().iter().map(|r| r.id().clone()));
            cursor = page.next_cursor().map(str::to_owned);
            if cursor.is_none() {
                break;
            }
        }
        assert_eq!(
            ids,
            all.items()
                .iter()
                .map(|r| r.id().clone())
                .collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn list_releases_rejects_malformed_and_unknown_cursor_positions() {
        let index = InMemoryMetadataIndex::new(releases());
        for cursor in [
            "invalid".to_owned(),
            URL_SAFE_NO_PAD.encode("wrong-version"),
            URL_SAFE_NO_PAD.encode([0xff]),
            URL_SAFE_NO_PAD.encode(format!(
                "tonhalle-release-cursor-v0:release-{}",
                "0".repeat(64)
            )),
        ] {
            assert!(matches!(
                index.list_releases(1, Some(cursor)).await,
                Err(ListReleasesError::InvalidCursor)
            ));
        }
    }

    #[tokio::test]
    async fn metadata_index_returns_empty_pages_and_not_found_for_absent_entities() {
        let release = releases().remove(0);
        let index = InMemoryMetadataIndex::new(vec![]);
        let page = index.list_releases(30, None).await.unwrap();
        assert!(page.items().is_empty());
        assert!(page.next_cursor().is_none());
        assert!(matches!(
            index.get_release(release.id()).await,
            Err(GetReleaseError::NotFound)
        ));
        assert!(matches!(
            index.get_track(release.tracks()[0].id()).await,
            Err(GetTrackError::NotFound)
        ));
    }

    #[tokio::test]
    async fn metadata_index_looks_up_the_requested_release_and_track() {
        let releases = releases();
        let index = InMemoryMetadataIndex::new(releases.clone());
        for release in releases {
            assert_eq!(
                index.get_release(release.id()).await.unwrap().id(),
                release.id()
            );
            for track in release.tracks() {
                assert_eq!(index.get_track(track.id()).await.unwrap().id(), track.id());
            }
        }
    }
}

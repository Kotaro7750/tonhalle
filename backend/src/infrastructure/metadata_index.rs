//! Stores the startup music index and implements cursor-based browsing.

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{
    collections::{BTreeMap, HashMap},
    ops::Bound::{Excluded, Unbounded},
    sync::RwLock,
};

use crate::{
    domain::{
        artist_credit::ArtistCredit,
        id::{ArtistId, AudioFileId, ReleaseId, TrackId},
        primitive::{release::Release, track::Track},
    },
    port::metadata_index::{
        AddReleaseError, ArtistPage, GetArtistError, GetReleaseError, GetTrackError,
        ListArtistsError, ListReleasesError, MetadataIndex, ReleasePage,
    },
};

/// A concurrently readable index of accepted releases.
#[derive(Default)]
pub(crate) struct InMemoryMetadataIndex {
    state: RwLock<IndexState>,
}

/// Owns releases and lookup positions under one lock.
#[derive(Default)]
struct IndexState {
    artists: BTreeMap<ArtistId, ArtistCredit>,
    releases: BTreeMap<ReleaseId, Release>,
    tracks: BTreeMap<TrackId, (ReleaseId, usize)>,
    audio: HashMap<AudioFileId, ReleaseId>,
}

impl InMemoryMetadataIndex {
    /// Creates an empty index for incremental construction.
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

impl IndexState {
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
    async fn add_release(&self, release: Release) -> Result<(), AddReleaseError> {
        let mut state = self.state.write().unwrap();
        if state.releases.contains_key(release.id()) {
            return Err(AddReleaseError::DuplicateRelease {
                id: release.id().clone(),
            });
        }
        for track in release.tracks() {
            if let Some((owner, _)) = state.tracks.get(track.id()) {
                return Err(AddReleaseError::SharedTrack {
                    track_id: track.id().clone(),
                    first_release_id: owner.clone(),
                    release_id: release.id().clone(),
                });
            }
            let id = track.original_audio_file().id();
            if let Some(owner) = state.audio.get(&id) {
                return Err(AddReleaseError::SharedAudioFile {
                    id,
                    first_release_id: owner.clone(),
                    release_id: release.id().clone(),
                });
            }
        }
        for (position, track) in release.tracks().iter().enumerate() {
            state
                .tracks
                .insert(track.id().clone(), (release.id().clone(), position));
            state
                .audio
                .insert(track.original_audio_file().id(), release.id().clone());
        }
        for credit in release.artist_credits() {
            state
                .artists
                .entry(credit.artist_id().clone())
                .or_insert_with(|| credit.clone());
        }
        state.releases.insert(release.id().clone(), release);
        Ok(())
    }

    /// Reads an identifier-ordered page and encodes its continuation position.
    async fn list_releases(
        &self,
        limit: u8,
        cursor: Option<String>,
        artist_id: Option<ArtistId>,
    ) -> Result<ReleasePage, ListReleasesError> {
        let state = self.state.read().unwrap();
        let cursor = cursor
            .map(|cursor| state.decode_cursor(cursor))
            .transpose()?;
        let start = cursor.as_ref().map_or(Unbounded, Excluded);

        let mut items = state
            .releases
            .range::<ReleaseId, _>((start, Unbounded))
            .filter(|(_, release)| {
                artist_id.as_ref().is_none_or(|id| {
                    release
                        .artist_credits()
                        .iter()
                        .any(|credit| credit.artist_id() == id)
                })
            })
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

    async fn list_artists(
        &self,
        limit: u8,
        cursor: Option<String>,
    ) -> Result<ArtistPage, ListArtistsError> {
        let state = self.state.read().unwrap();
        let cursor = cursor
            .map(|cursor| {
                let bytes = URL_SAFE_NO_PAD
                    .decode(cursor)
                    .map_err(|_| ListArtistsError::InvalidCursor)?;
                let value =
                    String::from_utf8(bytes).map_err(|_| ListArtistsError::InvalidCursor)?;
                let id = value
                    .strip_prefix("tonhalle-artist-cursor-v0:")
                    .ok_or(ListArtistsError::InvalidCursor)?
                    .parse::<ArtistId>()
                    .map_err(|_| ListArtistsError::InvalidCursor)?;
                if !state.artists.contains_key(&id) {
                    return Err(ListArtistsError::InvalidCursor);
                }
                Ok(id)
            })
            .transpose()?;
        let start = cursor.as_ref().map_or(Unbounded, Excluded);
        let mut items = state
            .artists
            .range::<ArtistId, _>((start, Unbounded))
            .take(usize::from(limit) + 1)
            .map(|(_, artist)| artist.clone())
            .collect::<Vec<_>>();
        let next_cursor = if items.len() > usize::from(limit) {
            items.pop();
            items.last().map(|artist| {
                URL_SAFE_NO_PAD.encode(format!("tonhalle-artist-cursor-v0:{}", artist.artist_id()))
            })
        } else {
            None
        };
        Ok(ArtistPage::new(items, next_cursor))
    }

    async fn get_artist(&self, id: &ArtistId) -> Result<ArtistCredit, GetArtistError> {
        self.state
            .read()
            .unwrap()
            .artists
            .get(id)
            .cloned()
            .ok_or(GetArtistError::NotFound)
    }

    /// Looks up an accepted release in the startup snapshot.
    async fn get_release(&self, id: &ReleaseId) -> Result<Release, GetReleaseError> {
        self.state
            .read()
            .unwrap()
            .releases
            .get(id)
            .cloned()
            .ok_or(GetReleaseError::NotFound)
    }

    /// Looks up an accepted track in the startup snapshot.
    async fn get_track(&self, id: &TrackId) -> Result<Track, GetTrackError> {
        let state = self.state.read().unwrap();
        let (release, position) = state.tracks.get(id).ok_or(GetTrackError::NotFound)?;
        Ok(state.releases[release].tracks()[*position].clone())
    }
}

/// Verifies index lookup, ordering, and cursor continuation independently of HTTP.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::ingestion::metadata::sample_metadata,
        test_support::{sample_release, sample_release_with_audio_id},
    };

    async fn index_with(releases: Vec<Release>) -> InMemoryMetadataIndex {
        let index = InMemoryMetadataIndex::new();
        for release in releases {
            index.add_release(release).await.unwrap();
        }
        index
    }

    #[tokio::test]
    async fn add_release_rejects_shared_audio_without_partial_registration() {
        let first = releases().remove(0);
        let mut raw = sample_metadata();
        *raw.release_title_mut() = "Conflicting release".into();
        let second = sample_release(&raw);
        let index = index_with(vec![first.clone()]).await;
        assert!(matches!(
            index.add_release(second.clone()).await,
            Err(AddReleaseError::SharedAudioFile { .. })
        ));
        assert!(matches!(
            index.get_release(second.id()).await,
            Err(GetReleaseError::NotFound)
        ));
        assert!(matches!(
            index.get_track(second.tracks()[0].id()).await,
            Err(GetTrackError::NotFound)
        ));
        assert_eq!(
            index
                .list_releases(100, None, None)
                .await
                .unwrap()
                .items()
                .len(),
            1
        );
        assert_eq!(
            index.get_track(first.tracks()[0].id()).await.unwrap().id(),
            first.tracks()[0].id()
        );
    }

    #[tokio::test]
    async fn add_release_allows_only_one_concurrent_registration_of_an_identity() {
        let index = InMemoryMetadataIndex::new();
        let release = releases().remove(0);
        let (first, second) = tokio::join!(
            index.add_release(release.clone()),
            index.add_release(release)
        );
        assert_ne!(first.is_ok(), second.is_ok());
        assert_eq!(
            index
                .list_releases(100, None, None)
                .await
                .unwrap()
                .items()
                .len(),
            1
        );
    }

    /// Builds distinct releases without parsing audio files.
    fn releases() -> Vec<Release> {
        (0..3)
            .map(|i| {
                let mut fixture = sample_metadata();
                *fixture.release_title_mut() = format!("Release {i}");
                sample_release_with_audio_id(
                    &fixture,
                    format!("2f1c75fc-55de-4d4b-8992-76050f65dff{i}")
                        .parse()
                        .unwrap(),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn list_releases_preserves_order_and_resumes_without_duplicates_when_limit_changes() {
        let index = index_with(releases()).await;
        let all = index.list_releases(100, None, None).await.unwrap();
        assert!(
            all.items()
                .windows(2)
                .all(|pair| pair[0].id() < pair[1].id())
        );
        let first = index.list_releases(1, None, None).await.unwrap();
        assert_eq!(first.items().len(), 1);
        let rest = index
            .list_releases(100, first.next_cursor().map(str::to_owned), None)
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
            let page = index.list_releases(1, cursor, None).await.unwrap();
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
        let index = index_with(releases()).await;
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
                index.list_releases(1, Some(cursor), None).await,
                Err(ListReleasesError::InvalidCursor)
            ));
        }
    }

    #[tokio::test]
    async fn metadata_index_returns_empty_pages_and_not_found_for_absent_entities() {
        let release = releases().remove(0);
        let index = InMemoryMetadataIndex::new();
        let page = index.list_releases(30, None, None).await.unwrap();
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
        let index = index_with(releases.clone()).await;
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

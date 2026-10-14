//! Stores the startup music index and implements cursor-based browsing.

use std::{
    collections::BTreeMap,
    ops::Bound::{Excluded, Unbounded},
};

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

use crate::{
    domain::{
        ReleaseId, TrackId,
        primitive::{release::Release, track::Track},
    },
    port::{GetReleaseError, GetTrackError, ListReleasesError, MetadataIndex, ReleasePage},
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

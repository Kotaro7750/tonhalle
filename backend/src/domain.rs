//! Defines music entities and their identifiers.

pub(crate) mod artist_credit;
pub(crate) mod id;
pub(crate) mod ingestion;
pub(crate) mod normalization;
pub(crate) mod primitive;

pub(crate) use artist_credit::ArtistCredit;
pub(crate) use id::{ArtistId, AudioFileId, ReleaseId, TrackId};

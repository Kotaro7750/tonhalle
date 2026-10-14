//! Validated identifiers for releases, tracks, artists, and stored audio files.
//!
//! Entity identifiers follow the hashing rules in `docs/domain.md`.
//! Audio-file identifiers retain the UUID issued when the file was stored.

use std::{fmt, str::FromStr};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::{Uuid, fmt::Hyphenated};

/// Defines a distinct entity ID type and its string-conversion error.
macro_rules! entity_id {
    ($name:ident, $error:ident, $kind:literal) => {
        #[doc = concat!("A validated `", $kind, "-` identifier with a 64-digit lowercase hex hash.")]
        ///
        /// Parsing guarantees the identifier's format and entity kind.
        /// The relationship to source metadata is established during ID generation.
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

        #[doc = concat!("An error indicating an invalid `", $kind, "` identifier string.")]
        #[derive(Debug, thiserror::Error)]
        #[error("invalid {} ID", $kind)]
        pub(crate) struct $error;

        impl TryFrom<String> for $name {
            /// Failure to satisfy this entity's prefix and hash format.
            type Error = $error;

            /// Validates an owned string's prefix, hash length, and lowercase hex digits.
            fn try_from(value: String) -> Result<Self, Self::Error> {
                let valid = value.strip_prefix(concat!($kind, "-")).is_some_and(|hash| {
                    hash.len() == 64
                        && hash
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                });

                if valid { Ok(Self(value)) } else { Err($error) }
            }
        }

        impl FromStr for $name {
            /// Failure to satisfy this entity's prefix and hash format.
            type Err = $error;

            /// Parses a canonical identifier without trimming whitespace or changing case.
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.to_owned().try_into()
            }
        }

        impl AsRef<str> for $name {
            /// Borrows the canonical identifier string.
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            /// Writes the canonical identifier string.
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

entity_id!(ReleaseId, ParseReleaseIdError, "release");
entity_id!(TrackId, ParseTrackIdError, "track");
entity_id!(ArtistId, ParseArtistIdError, "artist");

impl ArtistId {
    /// Creates an artist ID from its MBID or normalized name metadata.
    pub(crate) fn from_metadata(mbid: Option<Uuid>, metadata: Value) -> Self {
        Self(hash_id("artist", mbid, metadata))
    }
}

impl ReleaseId {
    /// Creates a release ID exclusively from normalized release metadata.
    pub(crate) fn from_metadata(metadata: Value) -> Self {
        Self(hash_id("release", None, metadata))
    }
}

impl TrackId {
    /// Creates a track ID exclusively from normalized release and track metadata.
    pub(crate) fn from_metadata(metadata: Value) -> Self {
        Self(hash_id("track", None, metadata))
    }
}

/// Hashes a versioned MBID or metadata input into an entity identifier.
///
/// The input uses the `tonhalle-id-v0` namespace, compact UTF-8 JSON, and SHA-256.
/// Callers must normalize the entity-specific metadata array before hashing.
fn hash_id(kind: &str, mbid: Option<Uuid>, metadata: Value) -> String {
    let input = match mbid {
        Some(id) => json!(["tonhalle-id-v0", kind, "mbid", id.to_string()]),
        None => json!(["tonhalle-id-v0", kind, "metadata", metadata]),
    };

    // serde_json emits the compact UTF-8 JSON and escapes required by domain.md.
    let digest = Sha256::digest(serde_json::to_vec(&input).expect("JSON values serialize"));
    format!("{kind}-{}", hex::encode(digest))
}

/// A stored audio file's identity, backed by a UUID v4 with the RFC variant.
///
/// Parsing requires hyphenated UUIDs and accepts either hex case.
/// Display uses lowercase digits.
/// The issued identity is retained when metadata or indexes are rebuilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AudioFileId(Uuid);

/// Failure to satisfy the audio-file UUID's format, version, or variant requirements.
#[derive(Debug, thiserror::Error)]
#[error("AudioFile ID must be a hyphenated UUID v4 with the RFC variant")]
pub(crate) struct ParseAudioFileIdError;

impl TryFrom<Uuid> for AudioFileId {
    /// Failure to satisfy the UUID version or RFC variant requirements.
    type Error = ParseAudioFileIdError;

    /// Accepts a UUID only when its version is four and its variant is RFC-compatible.
    fn try_from(value: Uuid) -> Result<Self, Self::Error> {
        if value.get_version_num() == 4 && value.get_variant() == uuid::Variant::RFC4122 {
            Ok(Self(value))
        } else {
            Err(ParseAudioFileIdError)
        }
    }
}

impl FromStr for AudioFileId {
    /// Failure to satisfy the UUID string's format, version, or variant requirements.
    type Err = ParseAudioFileIdError;

    /// Parses a hyphenated UUID string and validates its version and variant.
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let id = value
            .parse::<Hyphenated>()
            .map_err(|_| ParseAudioFileIdError)?;

        id.into_uuid().try_into()
    }
}

impl TryFrom<String> for AudioFileId {
    /// Failure to satisfy the UUID string's format, version, or variant requirements.
    type Error = ParseAudioFileIdError;

    /// Validates an owned UUID string using the same rules as borrowed-string parsing.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl fmt::Display for AudioFileId {
    /// Writes the UUID in lowercase, hyphenated form.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Regression tests for identifier parsing, generation, and canonical representation.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_id_rejects_noncanonical_hashes_and_wrong_kinds() {
        for kind in ["release", "track", "artist"] {
            let valid = format!("{kind}-{}", "0123456789abcdef".repeat(4));
            let accepts = |value: &str| match kind {
                "release" => value.parse::<ReleaseId>().is_ok(),
                "track" => value.parse::<TrackId>().is_ok(),
                "artist" => value.parse::<ArtistId>().is_ok(),
                _ => unreachable!(),
            };

            assert!(accepts(&valid));

            for invalid in [
                String::new(),
                format!("{kind}-{}", "a".repeat(63)),
                format!("{kind}-{}", "a".repeat(65)),
                format!("{kind}-{}", "A".repeat(64)),
                format!("{kind}-{}", "g".repeat(64)),
                format!(" {valid}"),
                format!("{valid}\n"),
                format!("{kind}-{}", "é".repeat(32)),
            ] {
                assert!(!accepts(&invalid), "{invalid:?}");
            }

            for other in ["release", "track", "artist"]
                .into_iter()
                .filter(|other| *other != kind)
            {
                assert!(!accepts(&format!("{other}-{}", "a".repeat(64))));
            }
        }

        let value = format!("release-{}", "f".repeat(64));
        let id = ReleaseId::try_from(value.clone()).unwrap();

        assert_eq!(id.to_string(), value);
        assert_eq!(serde_json::to_value(id).unwrap(), json!(value));
    }

    #[test]
    fn hash_id_matches_the_documented_json_hash() {
        assert_eq!(
            ArtistId::from_metadata(None, json!(["Kimiko Ishizaka"])).to_string(),
            "artist-8b552d5cdfb7606d535ac02c537cebfeffab0d4076095656073b4409c8ebe336"
        );
    }

    #[test]
    fn audio_file_id_requires_hyphenation_version_four_and_rfc_variant() {
        let value = "2f1c75fc-55de-4d4b-8992-76050f65dff0";
        let id: AudioFileId = value.parse().unwrap();

        assert_eq!(id.to_string(), value);
        assert_eq!(
            AudioFileId::try_from(value.parse::<Uuid>().unwrap()).unwrap(),
            id
        );
        assert_eq!(AudioFileId::try_from(value.to_uppercase()).unwrap(), id);

        for invalid in [
            "2f1c75fc55de4d4b899276050f65dff0",
            "{2f1c75fc-55de-4d4b-8992-76050f65dff0}",
            "urn:uuid:2f1c75fc-55de-4d4b-8992-76050f65dff0",
            "2f1c75fc-55de-5d4b-8992-76050f65dff0",
            "2f1c75fc-55de-4d4b-0992-76050f65dff0",
            "2f1c75fc-55de-4d4b-c992-76050f65dff0",
            "",
        ] {
            assert!(invalid.parse::<AudioFileId>().is_err(), "{invalid}");
        }

        assert!(AudioFileId::try_from(Uuid::nil()).is_err());
    }
}

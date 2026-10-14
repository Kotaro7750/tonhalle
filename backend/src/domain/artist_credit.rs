//! Defines artist credits and resolves names and MBIDs into ordered attributions.

use std::sync::LazyLock;

use itertools::Itertools;
use regex::Regex;
use serde_json::json;

use super::{ArtistId, normalization};

/// A failure to resolve artist names and optional MBIDs into valid credits.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ArtistCreditValidationError {
    /// An artist name is empty after identity normalization.
    #[error("invalid metadata: missing artist name")]
    MissingName,
    /// Artist names cannot be paired with their valid MBIDs.
    #[error("invalid metadata: artist names and MBIDs do not correspond")]
    NameMbidMismatch,
    /// More than one split matches the expected artist count.
    #[error("invalid metadata: ambiguous artist split")]
    AmbiguousSplit,
    /// No split matches the expected artist count.
    #[error("invalid metadata: cannot split artist names")]
    CannotSplitNames,
}

/// An artist attribution in the source credit order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtistCredit {
    artist_id: ArtistId,
    name: String,
    join_phrase: Option<String>,
}

impl ArtistCredit {
    /// Associates an artist identity with its source spelling and credit separator.
    fn new(artist_id: ArtistId, name: String, join_phrase: Option<String>) -> Self {
        Self {
            artist_id,
            name,
            join_phrase,
        }
    }

    /// Identifies the credited artist independently of its display spelling.
    pub(crate) fn artist_id(&self) -> &ArtistId {
        &self.artist_id
    }

    /// Preserves the artist spelling supplied by the source metadata.
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    /// Preserves the separator used to join this credit to the next artist.
    pub(crate) fn join_phrase(&self) -> Option<&str> {
        self.join_phrase.as_deref()
    }
}

/// Recognizes supported separators without changing their display spelling.
static ARTIST_SEPARATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i: feat\. | feat | ft\. | ft )| & | / |; ").unwrap());

/// Resolves ordered credits only when artist names and valid MBIDs correspond.
pub(crate) fn artist_credits(
    names: &[String],
    mbids: &[String],
) -> Result<Vec<ArtistCredit>, ArtistCreditValidationError> {
    let mbids = mbids
        .iter()
        .filter_map(|value| normalization::mbid(value))
        .collect::<Vec<_>>();

    let (names, joins) = if names.len() == 1 && mbids.len() > 1 {
        split_artists(&names[0], mbids.len())?
    } else {
        (names.to_vec(), vec![None; names.len()])
    };

    if names.is_empty() || (!mbids.is_empty() && names.len() != mbids.len()) {
        return Err(ArtistCreditValidationError::NameMbidMismatch);
    }

    names
        .into_iter()
        .zip(joins)
        .enumerate()
        .map(|(position, (name, join_phrase))| {
            let normalized =
                normalization::required(&name).ok_or(ArtistCreditValidationError::MissingName)?;

            Ok(ArtistCredit::new(
                ArtistId::from_metadata(mbids.get(position).copied(), json!([normalized])),
                name,
                join_phrase,
            ))
        })
        .collect()
}

/// Accepts a single name split only when the expected artist count makes it unique.
fn split_artists(
    value: &str,
    count: usize,
) -> Result<(Vec<String>, Vec<Option<String>>), ArtistCreditValidationError> {
    let boundaries = ARTIST_SEPARATOR.find_iter(value).collect::<Vec<_>>();
    let mut candidates = boundaries
        .iter()
        .combinations(count - 1)
        .filter_map(|selected| {
            let mut start = 0;
            let mut names = Vec::new();
            let mut joins = Vec::new();

            for boundary in selected {
                names.push(value[start..boundary.start()].to_string());
                joins.push(Some(boundary.as_str().to_string()));
                start = boundary.end();
            }

            names.push(value[start..].to_string());
            joins.push(None);

            names
                .iter()
                .all(|name| !normalization::normalize(name).is_empty())
                .then_some((names, joins))
        });

    let first = candidates.next();
    if candidates.next().is_some() {
        return Err(ArtistCreditValidationError::AmbiguousSplit);
    }

    first.ok_or(ArtistCreditValidationError::CannotSplitNames)
}

/// Verifies artist correspondence and retained display separators.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artist_credits_requires_unambiguous_correspondence_and_preserves_join_phrases() {
        let mbids = vec![
            "7b0857ed-11ee-4426-9899-16f4e415322f".into(),
            "f7b87070-d876-4a11-997a-048cdff1a87d".into(),
        ];

        let credits = artist_credits(&["One FeAt. Two".into()], &mbids).unwrap();

        assert_eq!(credits.len(), 2);
        assert_eq!(credits[0].join_phrase(), Some(" FeAt. "));
        assert_eq!(credits[1].join_phrase(), None);

        assert!(matches!(
            artist_credits(&["One & Two / Three".into()], &mbids),
            Err(ArtistCreditValidationError::AmbiguousSplit)
        ));

        assert!(matches!(
            artist_credits(&["One".into()], &mbids),
            Err(ArtistCreditValidationError::CannotSplitNames)
        ));

        assert!(matches!(
            artist_credits(&["One".into(), "Two".into(), "Three".into()], &mbids),
            Err(ArtistCreditValidationError::NameMbidMismatch)
        ));

        assert_eq!(
            artist_credits(&["One & Two".into()], &["invalid".into()])
                .unwrap()
                .len(),
            1
        );

        let explicit = artist_credits(&["One & Band".into(), "Two".into()], &mbids).unwrap();

        assert_eq!(explicit[0].name(), "One & Band");

        assert!(matches!(
            artist_credits(&["\t".into()], &[]),
            Err(ArtistCreditValidationError::MissingName)
        ));
    }
}

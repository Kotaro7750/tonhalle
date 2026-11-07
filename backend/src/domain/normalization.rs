//! Normalizes metadata text and identity inputs for music entities.

use unicode_normalization::UnicodeNormalization;
use uuid::{Uuid, fmt::Hyphenated};

/// Trims surrounding Unicode whitespace while preserving internal text.
pub(crate) fn text(value: &str) -> &str {
    value.trim()
}

/// Trims optional metadata text and treats empty values as absent.
pub(crate) fn optional(value: Option<&str>) -> Option<&str> {
    value.map(text).filter(|value| !value.is_empty())
}

/// Trims Unicode whitespace and applies NFC for identity generation.
pub(crate) fn normalize(value: &str) -> String {
    text(value).nfc().collect()
}

/// Normalizes required identity inputs and omits values that become empty.
pub(crate) fn required(value: &str) -> Option<String> {
    let value = normalize(value);

    (!value.is_empty()).then_some(value)
}

/// Accepts normalized hyphenated UUIDs as optional MusicBrainz identities.
pub(crate) fn mbid(value: &str) -> Option<Uuid> {
    normalize(value)
        .parse::<Hyphenated>()
        .ok()
        .map(|id| id.into_uuid())
}

/// Verifies normalization and optional MusicBrainz identity parsing.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_unicode_whitespace_and_applies_nfc() {
        assert_eq!(normalize("\tCafe\u{301} \r\n"), "Café");
        assert_eq!(normalize("\u{a0}name\u{3000}"), "name");
    }

    #[test]
    fn mbid_accepts_hyphenated_uuids_without_case_sensitivity() {
        let upper = "7B0857ED-11EE-4426-9899-16F4E415322F";

        assert_eq!(mbid(upper), mbid(&upper.to_lowercase()));
        assert!(mbid(upper).is_some());
        assert!(mbid("7b0857ed11ee4426989916f4e415322f").is_none());
    }
}

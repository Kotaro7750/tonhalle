//! Defines delivery-access settings independently of the delivery provider.

use std::time::SystemTime;

/// Cookies and optional expiration returned by a delivery-access provider.
#[derive(Debug, Clone, getset::Getters, getset::CopyGetters)]
pub(crate) struct ContentAccessConfiguration {
    /// Opaque delivery credentials.
    #[getset(get = "pub(crate)")]
    cookies: Vec<ContentAccessCookie>,
    /// Latest time at which clients should renew their credentials.
    #[getset(get_copy = "pub(crate)")]
    invalid_after: Option<SystemTime>,
}

impl ContentAccessConfiguration {
    /// Creates delivery credentials with their renewal deadline.
    pub(crate) fn new(
        cookies: Vec<ContentAccessCookie>,
        invalid_after: Option<SystemTime>,
    ) -> Self {
        Self {
            cookies,
            invalid_after,
        }
    }
}

/// An opaque cookie scoped to a delivery endpoint.
#[derive(Debug, Clone, getset::Getters)]
#[getset(get = "pub(crate)")]
pub(crate) struct ContentAccessCookie {
    /// Cookie name understood by the delivery provider.
    name: String,
    /// Opaque credential value.
    value: String,
    /// Optional domain shared by the API and delivery endpoint.
    domain: Option<String>,
    /// Delivery path covered by the cookie.
    path: String,
}

impl ContentAccessCookie {
    /// Creates a cookie scoped to the configured delivery endpoint.
    pub(crate) fn new(name: String, value: String, domain: Option<String>, path: String) -> Self {
        Self {
            name,
            value,
            domain,
            path,
        }
    }
}

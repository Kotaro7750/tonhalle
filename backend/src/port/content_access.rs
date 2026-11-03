//! Defines the boundary for issuing and clearing delivery credentials.

use crate::domain::content_access::ContentAccessConfiguration;

/// A delivery-provider failure retained for server-side diagnostics.
#[derive(Debug, thiserror::Error)]
#[error("configuring content access: {0}")]
pub(crate) struct ContentAccessError(String);

impl ContentAccessError {
    /// Retains provider diagnostics for server-side logging.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

/// Issues and clears opaque credentials for a separate delivery endpoint.
pub(crate) trait ContentAccessConfigurator: Send + Sync {
    /// Issues credentials and their renewal deadline.
    fn configure(&self) -> Result<ContentAccessConfiguration, ContentAccessError>;
    /// Produces cookies with matching scopes for client-side removal.
    fn clear(&self) -> Result<ContentAccessConfiguration, ContentAccessError>;
}

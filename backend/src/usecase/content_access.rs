//! Configures delivery access through the provider-independent port.

use std::sync::Arc;

use crate::{
    domain::content_access::ContentAccessConfiguration,
    port::content_access::{ContentAccessConfigurator, ContentAccessError},
};

/// Issues delivery credentials and clears them when requested.
#[derive(Clone)]
pub(crate) struct ContentAccessUseCase {
    configurator: Arc<dyn ContentAccessConfigurator>,
}

impl ContentAccessUseCase {
    /// Connects delivery-access operations to their provider.
    pub(crate) fn new(configurator: Arc<dyn ContentAccessConfigurator>) -> Self {
        Self { configurator }
    }

    /// Issues delivery credentials with a renewal deadline.
    pub(crate) fn configure(&self) -> Result<ContentAccessConfiguration, ContentAccessError> {
        self.configurator.configure()
    }

    /// Clears client credentials without revoking previously issued signatures.
    pub(crate) fn clear(&self) -> Result<ContentAccessConfiguration, ContentAccessError> {
        self.configurator.clear()
    }
}

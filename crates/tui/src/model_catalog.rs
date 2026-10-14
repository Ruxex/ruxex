//! TUI model and collaboration inventories; refreshing models preserves the server mode catalog.

use ruxex_app_server_protocol::AppServerIdentity;
use ruxex_protocol::config_types::CollaborationModeMask;
use ruxex_protocol::model_types::ModelPreset;
use std::convert::Infallible;

/// Identity of the AppServer context that produced provider-derived catalog data.
///
/// The provider kind is intentionally not part of this key. A server signature and
/// protocol version are the stable boundary for a client cache; provider details stay
/// behind the server contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppServerCacheScope {
    pub(crate) signature: String,
    pub(crate) version: String,
}

impl AppServerCacheScope {
    pub(crate) fn from_identity(identity: Option<&AppServerIdentity>) -> Option<Self> {
        identity.map(|identity| Self {
            signature: identity.signature.clone(),
            version: identity.version.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ModelCatalog {
    pub(crate) models: Vec<ModelPreset>,
    pub(crate) collaboration_modes: Vec<CollaborationModeMask>,
    pub(crate) server_scope: Option<AppServerCacheScope>,
}

impl ModelCatalog {
    pub(crate) fn new(models: Vec<ModelPreset>) -> Self {
        Self {
            models,
            collaboration_modes: Vec::new(),
            server_scope: None,
        }
    }

    pub(crate) fn with_server_identity(mut self, identity: Option<&AppServerIdentity>) -> Self {
        self.server_scope = AppServerCacheScope::from_identity(identity);
        self
    }

    pub(crate) fn with_collaboration_modes(mut self, modes: Vec<CollaborationModeMask>) -> Self {
        self.collaboration_modes = modes;
        self
    }

    pub(crate) fn try_list_models(&self) -> Result<Vec<ModelPreset>, Infallible> {
        Ok(self.models.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(signature: &str, version: &str) -> AppServerIdentity {
        AppServerIdentity {
            signature: signature.to_string(),
            version: version.to_string(),
            build_version: None,
        }
    }

    #[test]
    fn cache_scope_uses_only_server_signature_and_protocol_version() {
        let first = AppServerCacheScope::from_identity(Some(&identity("server-a", "app-server/1")));
        let same_context =
            AppServerCacheScope::from_identity(Some(&identity("server-a", "app-server/1")));
        let other_server =
            AppServerCacheScope::from_identity(Some(&identity("server-b", "app-server/1")));
        let other_version =
            AppServerCacheScope::from_identity(Some(&identity("server-a", "app-server/2")));

        assert_eq!(first, same_context);
        assert_ne!(first, other_server);
        assert_ne!(first, other_version);
    }
}

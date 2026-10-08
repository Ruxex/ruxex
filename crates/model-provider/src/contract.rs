//! Provider-neutral boundary for Genex hosts and provider implementations.
//!
//! The runtime adapter still lives in [`crate::provider`].  This module is intentionally free of
//! credential-store and wire-client types so a provider implementation can be supplied without
//! making those implementation details part of the Genex contract.

use std::collections::BTreeMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use genex_http_client::HttpClientFactory;
use genex_protocol::account::{ProviderAccount, ProviderAuthStatus};
use genex_protocol::model_types::ModelCatalog;
use genex_protocol::models::ResponseItem;
use genex_protocol::protocol::RateLimitSnapshot;
use serde_json::Value;

/// Remote context-compaction protocols supported by a model provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCompactionSupport {
    Unsupported,
    V2,
}

/// Optional provider-backed features that Genex may expose at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub model_listing: bool,
    pub api_key_model_discovery: bool,
    pub command_auth: bool,
    pub streaming: bool,
    pub usage: bool,
    pub interactive_login: bool,
    pub namespace_tools: bool,
    pub web_search: bool,
    pub standalone_web_search: bool,
    pub external_web_access: bool,
    pub history_notes: bool,
    pub remote_compaction: RemoteCompactionSupport,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            model_listing: true,
            api_key_model_discovery: false,
            command_auth: false,
            streaming: true,
            usage: false,
            interactive_login: false,
            namespace_tools: true,
            web_search: true,
            standalone_web_search: false,
            external_web_access: true,
            history_notes: false,
            remote_compaction: RemoteCompactionSupport::Unsupported,
        }
    }
}

/// Stable, non-secret provider identity used by the mounted data-root marker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ProviderDescriptor {
    pub provider_kind: String,
    pub provider_protocol_version: String,
}

impl ProviderDescriptor {
    pub fn new(
        provider_kind: impl Into<String>,
        provider_protocol_version: impl Into<String>,
    ) -> Self {
        Self {
            provider_kind: provider_kind.into(),
            provider_protocol_version: provider_protocol_version.into(),
        }
    }
}

pub const CURRENT_PROVIDER_PROTOCOL_VERSION: &str = "1";

/// Result of a provider-owned usage capability.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderUsage {
    Unsupported,
    Available(RateLimitSnapshot),
}

pub type ProviderUsageResult = Result<ProviderUsage, ProviderError>;

/// Provider-neutral model catalog together with cache validation metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderModelCatalog {
    pub catalog: ModelCatalog,
    pub identity: String,
    pub etag: Option<String>,
}

/// Provider-neutral error classes exposed to a Genex host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// The provider does not implement the requested operation.
    Unsupported,
    /// The provider has no usable credentials.
    Unauthenticated,
    /// The provider rejected the supplied credentials or request authorization.
    Unauthorized,
    /// The request does not satisfy the provider's input constraints.
    InvalidRequest(String),
    /// The provider quota or rate limit was reached.
    RateLimited { retry_after_seconds: Option<u64> },
    /// A retryable provider-side failure.
    Transient(String),
    /// A connection or streaming failure.
    Transport(String),
    /// A provider-specific failure without a more precise common classification.
    Provider(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("provider operation is unsupported"),
            Self::Unauthenticated => f.write_str("provider is unauthenticated"),
            Self::Unauthorized => f.write_str("provider rejected the request"),
            Self::InvalidRequest(message)
            | Self::Transient(message)
            | Self::Transport(message)
            | Self::Provider(message) => f.write_str(message),
            Self::RateLimited { .. } => f.write_str("provider rate limit reached"),
        }
    }
}

impl std::error::Error for ProviderError {}

pub type ProviderResult<T> = Result<T, ProviderError>;
pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type ProviderChatStream = Pin<Box<dyn Stream<Item = ProviderResult<ProviderChatEvent>> + Send>>;
pub type SharedModelProvider = Arc<dyn ModelProvider>;

/// Current provider authentication state. Credential values are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAuthState {
    pub status: ProviderAuthStatus,
    pub account_id: Option<String>,
    pub display_name: Option<String>,
}

/// A provider-owned credential field exposed to a host login form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAuthField {
    pub id: String,
    pub label: String,
    pub secret: bool,
}

impl ProviderAuthField {
    pub fn new(id: impl Into<String>, label: impl Into<String>, secret: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            secret,
        }
    }
}

/// Provider-owned account and usage state suitable for host/UI projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderAccountState {
    pub provider_id: String,
    pub account: Option<ProviderAccount>,
    pub account_id: Option<String>,
    pub display_name: Option<String>,
    pub auth_status: ProviderAuthStatus,
    pub usage: Option<RateLimitSnapshot>,
}

/// Input to a provider model-catalog operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListModelsRequest {
    pub client_version: String,
    pub http_client_factory: HttpClientFactory,
}

impl ListModelsRequest {
    pub fn new(client_version: impl Into<String>, http_client_factory: HttpClientFactory) -> Self {
        Self {
            client_version: client_version.into(),
            http_client_factory,
        }
    }
}

/// Input to a provider usage operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UsageRequest {
    pub model: Option<String>,
}

/// Provider-neutral chat input. Wire request construction remains inside the provider.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    pub instructions: Option<String>,
    pub input: Vec<ResponseItem>,
    pub tools: Vec<Value>,
    pub metadata: BTreeMap<String, Value>,
}

/// Provider-neutral streamed chat events.
#[derive(Debug, Clone, PartialEq)]
pub enum ProviderChatEvent {
    TextDelta(String),
    OutputItem(ResponseItem),
    Usage(RateLimitSnapshot),
    Completed { response_id: Option<String> },
    Metadata(Value),
}

/// Provider-owned model execution, catalog, authentication, and usage contract.
pub trait ModelProvider: fmt::Debug + Send + Sync + 'static {
    fn descriptor(&self) -> ProviderDescriptor;

    /// Stable cache identity for provider-owned model catalog data.
    fn model_catalog_identity(&self) -> ProviderResult<String> {
        Ok(self.descriptor().provider_kind)
    }

    fn capabilities(&self) -> ProviderCapabilities;

    fn chat(&self, request: ChatRequest) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>>;

    fn list_models(
        &self,
        request: ListModelsRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderModelCatalog>>;

    fn auth_state(&self) -> ProviderAuthState;

    fn auth_fields(&self) -> Vec<ProviderAuthField>;

    fn account_state(&self) -> ProviderResult<ProviderAccountState>;

    fn login(
        &self,
        credentials: BTreeMap<String, String>,
    ) -> ProviderFuture<'_, ProviderResult<()>>;

    fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>>;

    fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>>;

    fn usage(&self, request: UsageRequest) -> ProviderFuture<'_, ProviderResult<ProviderUsage>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct FakeProvider;

    impl ModelProvider for FakeProvider {
        fn descriptor(&self) -> ProviderDescriptor {
            ProviderDescriptor::new("fake", "1")
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }

        fn chat(
            &self,
            _request: ChatRequest,
        ) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn list_models(
            &self,
            _request: ListModelsRequest,
        ) -> ProviderFuture<'_, ProviderResult<ProviderModelCatalog>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn auth_state(&self) -> ProviderAuthState {
            ProviderAuthState {
                status: ProviderAuthStatus::Unauthenticated,
                account_id: None,
                display_name: None,
            }
        }

        fn auth_fields(&self) -> Vec<ProviderAuthField> {
            vec![ProviderAuthField::new("api_key", "API key", true)]
        }

        fn account_state(&self) -> ProviderResult<ProviderAccountState> {
            Ok(ProviderAccountState {
                provider_id: "fake".to_string(),
                account: None,
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::Unauthenticated,
                usage: None,
            })
        }

        fn login(
            &self,
            _credentials: BTreeMap<String, String>,
        ) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Ok(()) })
        }

        fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Ok(()) })
        }

        fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>> {
            Box::pin(async { Ok(false) })
        }

        fn usage(
            &self,
            _request: UsageRequest,
        ) -> ProviderFuture<'_, ProviderResult<ProviderUsage>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }
    }

    #[test]
    fn error_classification_is_explicit() {
        assert_eq!(
            ProviderError::Unsupported.to_string(),
            "provider operation is unsupported"
        );
        assert!(matches!(
            ProviderError::RateLimited {
                retry_after_seconds: Some(5)
            },
            ProviderError::RateLimited { .. }
        ));
    }

    #[test]
    fn contract_is_object_safe_for_host_injection() {
        let provider: SharedModelProvider = Arc::new(FakeProvider);
        assert_eq!(provider.descriptor().provider_kind, "fake");
        assert_eq!(
            provider.auth_state().status,
            ProviderAuthStatus::Unauthenticated
        );
    }
}

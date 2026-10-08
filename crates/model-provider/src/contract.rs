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
use genex_protocol::account::ProviderAuthStatus;
use genex_protocol::model_types::ModelCatalog;
use genex_protocol::models::ResponseItem;
use genex_protocol::protocol::RateLimitSnapshot;
use serde_json::Value;

use crate::provider::ProviderCapabilities;
use crate::provider::ProviderDescriptor;

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

/// Input to a provider model-catalog operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListModelsRequest {
    pub client_version: String,
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
    fn capabilities(&self) -> ProviderCapabilities;

    fn chat(&self, request: ChatRequest) -> ProviderFuture<'_, ProviderResult<ProviderChatStream>>;

    fn list_models(
        &self,
        request: ListModelsRequest,
    ) -> ProviderFuture<'_, ProviderResult<ModelCatalog>>;

    fn auth_state(&self) -> ProviderAuthState;

    fn login(
        &self,
        credentials: BTreeMap<String, String>,
    ) -> ProviderFuture<'_, ProviderResult<()>>;

    fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>>;

    fn usage(
        &self,
        request: UsageRequest,
    ) -> ProviderFuture<'_, ProviderResult<crate::provider::ProviderUsage>>;
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
        ) -> ProviderFuture<'_, ProviderResult<ModelCatalog>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn auth_state(&self) -> ProviderAuthState {
            ProviderAuthState {
                status: ProviderAuthStatus::Unauthenticated,
                account_id: None,
                display_name: None,
            }
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

        fn usage(
            &self,
            _request: UsageRequest,
        ) -> ProviderFuture<'_, ProviderResult<crate::provider::ProviderUsage>> {
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

//! Reusable semantic conformance checks for external Ruxex model providers.
//!
//! The checks in this crate validate provider-neutral invariants. Provider-specific
//! HTTP payloads, credentials, and successful authenticated requests remain the
//! responsibility of the provider implementation.

use std::fmt;

use ruxex_model_provider::{
    ChatRequest, ListModelsRequest, ModelProvider, ModelProviderFactory, ProviderCapabilities,
    ProviderDescriptor, ProviderError, ProviderInit, ProviderResult, ProviderUsage, UsageRequest,
};
use thiserror::Error;

/// Configuration for the provider-neutral contract checks.
#[derive(Debug, Clone)]
pub struct ProviderContractConfig {
    /// The stable identity the provider is expected to expose.
    pub expected_descriptor: ProviderDescriptor,
    /// When present, the provider must report exactly these capabilities.
    pub expected_capabilities: Option<ProviderCapabilities>,
    /// When present, the provider must expose this model-catalog cache identity.
    pub expected_model_catalog_identity: Option<String>,
    /// Optional model-listing operation probe.
    pub list_models: Option<ListModelsProbe>,
    /// Optional usage operation probe.
    pub usage: Option<UsageProbe>,
    /// Optional chat operation probe.
    pub chat: Option<ChatProbe>,
}

impl ProviderContractConfig {
    pub fn new(expected_descriptor: ProviderDescriptor) -> Self {
        Self {
            expected_descriptor,
            expected_capabilities: None,
            expected_model_catalog_identity: None,
            list_models: None,
            usage: None,
            chat: None,
        }
    }

    pub fn with_expected_capabilities(mut self, capabilities: ProviderCapabilities) -> Self {
        self.expected_capabilities = Some(capabilities);
        self
    }

    pub fn with_expected_model_catalog_identity(mut self, identity: impl Into<String>) -> Self {
        self.expected_model_catalog_identity = Some(identity.into());
        self
    }

    pub fn with_list_models(
        mut self,
        request: ListModelsRequest,
        expected: OperationExpectation,
    ) -> Self {
        self.list_models = Some(ListModelsProbe { request, expected });
        self
    }

    pub fn with_usage(mut self, request: UsageRequest, expected: OperationExpectation) -> Self {
        self.usage = Some(UsageProbe { request, expected });
        self
    }

    pub fn with_chat(mut self, request: ChatRequest, expected: OperationExpectation) -> Self {
        self.chat = Some(ChatProbe { request, expected });
        self
    }
}

#[derive(Debug, Clone)]
pub struct ListModelsProbe {
    pub request: ListModelsRequest,
    pub expected: OperationExpectation,
}

#[derive(Debug, Clone)]
pub struct UsageProbe {
    pub request: UsageRequest,
    pub expected: OperationExpectation,
}

#[derive(Debug, Clone)]
pub struct ChatProbe {
    pub request: ChatRequest,
    pub expected: OperationExpectation,
}

/// Whether an optional operation is expected to be implemented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationExpectation {
    Supported,
    Unsupported,
}

#[derive(Debug, Error)]
pub enum ProviderContractError {
    #[error("expected descriptor field `{field}` must not be empty")]
    EmptyDescriptorField { field: &'static str },
    #[error("{area} descriptor mismatch: expected {expected:?}, actual {actual:?}")]
    DescriptorMismatch {
        area: &'static str,
        expected: ProviderDescriptor,
        actual: ProviderDescriptor,
    },
    #[error("{field} is not stable across repeated reads")]
    UnstableValue { field: &'static str },
    #[error("provider capabilities mismatch: expected {expected:?}, actual {actual:?}")]
    CapabilitiesMismatch {
        expected: ProviderCapabilities,
        actual: ProviderCapabilities,
    },
    #[error("model catalog identity failed: {error}")]
    ModelCatalogIdentityFailed { error: ProviderError },
    #[error("model catalog identity is not stable across repeated reads")]
    ModelCatalogIdentityUnstable,
    #[error("model catalog identity mismatch: expected {expected:?}, actual {actual:?}")]
    ModelCatalogIdentityMismatch {
        expected: Option<String>,
        actual: String,
    },
    #[error("request snapshot failed: {error}")]
    RequestSnapshotFailed { error: ProviderError },
    #[error("request snapshot provider kind mismatch: expected {expected}, actual {actual}")]
    RequestSnapshotProviderMismatch { expected: String, actual: String },
    #[error("account state failed: {error}")]
    AccountStateFailed { error: ProviderError },
    #[error("account state provider id mismatch: expected {expected}, actual {actual}")]
    AccountProviderMismatch { expected: String, actual: String },
    #[error("operation `{operation}` unexpectedly reported unsupported")]
    UnexpectedUnsupported { operation: &'static str },
    #[error("operation `{operation}` was expected to be unsupported but returned a result")]
    ExpectedUnsupported { operation: &'static str },
    #[error("operation `{operation}` returned an unexpected provider error: {error}")]
    UnexpectedOperationError {
        operation: &'static str,
        error: ProviderError,
    },
}

/// Constructs a provider through its public factory and checks factory identity consistency.
pub fn construct_and_assert_model_provider_factory<P: ModelProviderFactory>(
    init: ProviderInit,
    expected_descriptor: &ProviderDescriptor,
) -> Result<P, ProviderContractError> {
    validate_descriptor(expected_descriptor)?;

    let factory_descriptor = P::provider_descriptor();
    if factory_descriptor != *expected_descriptor {
        return Err(ProviderContractError::DescriptorMismatch {
            area: "factory",
            expected: expected_descriptor.clone(),
            actual: factory_descriptor,
        });
    }
    if P::provider_descriptor() != factory_descriptor {
        return Err(ProviderContractError::UnstableValue {
            field: "factory descriptor",
        });
    }

    let provider = P::new(init);
    let instance_descriptor = provider.descriptor();
    if instance_descriptor != *expected_descriptor {
        return Err(ProviderContractError::DescriptorMismatch {
            area: "constructed provider",
            expected: expected_descriptor.clone(),
            actual: instance_descriptor,
        });
    }

    Ok(provider)
}

/// Checks provider-neutral semantic invariants and any configured operation probes.
pub async fn assert_model_provider_contract<P: ModelProvider + ?Sized>(
    provider: &P,
    config: &ProviderContractConfig,
) -> Result<(), ProviderContractError> {
    validate_descriptor(&config.expected_descriptor)?;

    let descriptor = provider.descriptor();
    if descriptor != config.expected_descriptor {
        return Err(ProviderContractError::DescriptorMismatch {
            area: "provider",
            expected: config.expected_descriptor.clone(),
            actual: descriptor,
        });
    }
    if provider.descriptor() != descriptor {
        return Err(ProviderContractError::UnstableValue {
            field: "provider descriptor",
        });
    }

    let request_format = provider.request_format();
    if provider.request_format() != request_format {
        return Err(ProviderContractError::UnstableValue {
            field: "request format",
        });
    }

    let capabilities = provider.capabilities();
    if provider.capabilities() != capabilities {
        return Err(ProviderContractError::UnstableValue {
            field: "capabilities",
        });
    }
    if let Some(expected) = config.expected_capabilities
        && capabilities != expected
    {
        return Err(ProviderContractError::CapabilitiesMismatch {
            expected,
            actual: capabilities,
        });
    }

    let identity = provider
        .model_catalog_identity()
        .map_err(|error| ProviderContractError::ModelCatalogIdentityFailed { error })?;
    if provider
        .model_catalog_identity()
        .map_err(|error| ProviderContractError::ModelCatalogIdentityFailed { error })?
        != identity
    {
        return Err(ProviderContractError::ModelCatalogIdentityUnstable);
    }
    if let Some(expected) = &config.expected_model_catalog_identity
        && expected != &identity
    {
        return Err(ProviderContractError::ModelCatalogIdentityMismatch {
            expected: Some(expected.clone()),
            actual: identity,
        });
    }

    let snapshot = provider
        .request_snapshot()
        .await
        .map_err(|error| ProviderContractError::RequestSnapshotFailed { error })?;
    if snapshot.connection_key.provider_kind != descriptor.provider_kind {
        return Err(ProviderContractError::RequestSnapshotProviderMismatch {
            expected: descriptor.provider_kind.clone(),
            actual: snapshot.connection_key.provider_kind,
        });
    }

    let account = provider
        .account_state()
        .map_err(|error| ProviderContractError::AccountStateFailed { error })?;
    if account.provider_id != descriptor.provider_kind {
        return Err(ProviderContractError::AccountProviderMismatch {
            expected: descriptor.provider_kind.clone(),
            actual: account.provider_id,
        });
    }

    if let Some(probe) = &config.list_models {
        assert_operation(
            "list_models",
            probe.expected,
            provider
                .list_models(probe.request.clone())
                .await
                .map(|_| ()),
        )?;
    }
    if let Some(probe) = &config.chat {
        assert_operation(
            "chat",
            probe.expected,
            provider.chat(probe.request.clone()).await.map(|_| ()),
        )?;
    }
    if let Some(probe) = &config.usage {
        assert_usage_operation(probe.expected, provider.usage(probe.request.clone()).await)?;
    }

    Ok(())
}

fn validate_descriptor(descriptor: &ProviderDescriptor) -> Result<(), ProviderContractError> {
    if descriptor.provider_kind.is_empty() {
        return Err(ProviderContractError::EmptyDescriptorField {
            field: "provider_kind",
        });
    }
    if descriptor.provider_protocol_version.is_empty() {
        return Err(ProviderContractError::EmptyDescriptorField {
            field: "provider_protocol_version",
        });
    }
    Ok(())
}

fn assert_operation<T>(
    operation: &'static str,
    expected: OperationExpectation,
    result: ProviderResult<T>,
) -> Result<(), ProviderContractError> {
    match (expected, result) {
        (OperationExpectation::Supported, Err(ProviderError::Unsupported)) => {
            Err(ProviderContractError::UnexpectedUnsupported { operation })
        }
        (OperationExpectation::Supported, Ok(_)) | (OperationExpectation::Supported, Err(_)) => {
            Ok(())
        }
        (OperationExpectation::Unsupported, Ok(_)) => {
            Err(ProviderContractError::ExpectedUnsupported { operation })
        }
        (OperationExpectation::Unsupported, Err(ProviderError::Unsupported)) => Ok(()),
        (OperationExpectation::Unsupported, Err(error)) => {
            Err(ProviderContractError::UnexpectedOperationError { operation, error })
        }
    }
}

fn assert_usage_operation(
    expected: OperationExpectation,
    result: ProviderResult<ProviderUsage>,
) -> Result<(), ProviderContractError> {
    match (expected, result) {
        (OperationExpectation::Supported, Err(ProviderError::Unsupported))
        | (OperationExpectation::Supported, Ok(ProviderUsage::Unsupported)) => {
            Err(ProviderContractError::UnexpectedUnsupported { operation: "usage" })
        }
        (OperationExpectation::Supported, Ok(ProviderUsage::Available(_)))
        | (OperationExpectation::Supported, Err(_)) => Ok(()),
        (OperationExpectation::Unsupported, Ok(ProviderUsage::Unsupported))
        | (OperationExpectation::Unsupported, Err(ProviderError::Unsupported)) => Ok(()),
        (OperationExpectation::Unsupported, Ok(ProviderUsage::Available(_))) => {
            Err(ProviderContractError::ExpectedUnsupported { operation: "usage" })
        }
        (OperationExpectation::Unsupported, Err(error)) => {
            Err(ProviderContractError::UnexpectedOperationError {
                operation: "usage",
                error,
            })
        }
    }
}

impl fmt::Display for OperationExpectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Supported => f.write_str("supported"),
            Self::Unsupported => f.write_str("unsupported"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use ruxex_http_client::{HttpClientFactory, OutboundProxyPolicy};
    use ruxex_login::{CredentialManager, RuxexAuth};
    use ruxex_model_provider::{
        ChatRequest, ListModelsRequest, ProviderAccountState, ProviderAuthState,
        ProviderChatStream, ProviderCredentialSnapshot, ProviderFuture, ProviderInit,
        ProviderModelCatalog, ProviderRequestSnapshot, ProviderResult, ProviderUsage, UsageRequest,
    };
    use ruxex_model_provider_info::ModelProviderInfo;
    use ruxex_protocol::account::ProviderAuthStatus;

    use super::*;

    #[derive(Debug)]
    struct FakeCredentialSnapshot;

    impl ProviderCredentialSnapshot for FakeCredentialSnapshot {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[derive(Debug)]
    struct FakeProvider {
        descriptor: ProviderDescriptor,
        snapshot_provider_kind: String,
        account_provider_id: String,
        capabilities: ProviderCapabilities,
        catalog_identity: String,
    }

    impl FakeProvider {
        fn valid() -> Self {
            Self {
                descriptor: ProviderDescriptor::new("test-provider", "1"),
                snapshot_provider_kind: "test-provider".to_string(),
                account_provider_id: "test-provider".to_string(),
                capabilities: ProviderCapabilities::default(),
                catalog_identity: "test-catalog".to_string(),
            }
        }
    }

    impl ModelProvider for FakeProvider {
        fn descriptor(&self) -> ProviderDescriptor {
            self.descriptor.clone()
        }

        fn capabilities(&self) -> ProviderCapabilities {
            self.capabilities
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

        fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>> {
            let provider_kind = self.snapshot_provider_kind.clone();
            Box::pin(async move {
                Ok(ProviderRequestSnapshot {
                    auth: ruxex_model_provider::ProviderRequestAuth {
                        authenticated: false,
                        managed_auth: false,
                        auth_header_attached: false,
                        auth_header_name: None,
                        revision: None,
                        owner_generation: None,
                    },
                    connection_key: ruxex_model_provider::ProviderConnectionKey {
                        provider_kind,
                        auth_revision: None,
                    },
                    credential_snapshot: Arc::new(FakeCredentialSnapshot),
                })
            })
        }

        fn model_catalog_identity(&self) -> ProviderResult<String> {
            Ok(self.catalog_identity.clone())
        }

        fn account_state(&self) -> ProviderResult<ProviderAccountState> {
            Ok(ProviderAccountState {
                provider_id: self.account_provider_id.clone(),
                account: None,
                account_id: None,
                display_name: None,
                auth_status: ProviderAuthStatus::Unauthenticated,
                usage: None,
            })
        }

        fn auth_state(&self) -> ProviderAuthState {
            ProviderAuthState {
                status: ProviderAuthStatus::Unauthenticated,
                account_id: None,
                display_name: None,
                managed_auth: false,
                revision: None,
            }
        }

        fn auth_fields(&self) -> Vec<ruxex_model_provider::ProviderAuthField> {
            Vec::new()
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
            Box::pin(async { Ok(ProviderUsage::Unsupported) })
        }
    }

    impl ModelProviderFactory for FakeProvider {
        fn provider_descriptor() -> ProviderDescriptor {
            ProviderDescriptor::new("test-provider", "1")
        }

        fn new(_init: ProviderInit) -> Self {
            Self::valid()
        }
    }

    async fn provider_init() -> ProviderInit {
        ProviderInit {
            provider_info: ModelProviderInfo::create_responses_provider(None),
            credential_manager: CredentialManager::from_auth_for_testing(RuxexAuth::from_api_key(
                "test-key",
            )),
            http_client_factory: HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        }
    }

    fn config() -> ProviderContractConfig {
        ProviderContractConfig::new(ProviderDescriptor::new("test-provider", "1"))
            .with_expected_capabilities(ProviderCapabilities::default())
            .with_expected_model_catalog_identity("test-catalog")
    }

    #[test]
    fn config_preserves_expected_provider_contract() {
        let descriptor = ProviderDescriptor::new("test-provider", "1");
        let capabilities = ProviderCapabilities::default();
        let config = ProviderContractConfig::new(descriptor.clone())
            .with_expected_capabilities(capabilities)
            .with_expected_model_catalog_identity("test-catalog");

        assert_eq!(config.expected_descriptor, descriptor);
        assert_eq!(config.expected_capabilities, Some(capabilities));
        assert_eq!(
            config.expected_model_catalog_identity.as_deref(),
            Some("test-catalog")
        );
    }

    #[tokio::test]
    async fn valid_provider_satisfies_mandatory_contract() {
        assert_model_provider_contract(&FakeProvider::valid(), &config())
            .await
            .expect("valid provider should satisfy the contract");
    }

    #[tokio::test]
    async fn valid_factory_constructs_a_provider_with_consistent_identity() {
        let provider = construct_and_assert_model_provider_factory::<FakeProvider>(
            provider_init().await,
            &ProviderDescriptor::new("test-provider", "1"),
        )
        .expect("valid factory should construct a provider");

        assert_model_provider_contract(&provider, &config())
            .await
            .expect("factory-created provider should satisfy the contract");
    }

    #[tokio::test]
    async fn factory_descriptor_mismatch_is_reported_before_construction() {
        let error = construct_and_assert_model_provider_factory::<FakeProvider>(
            provider_init().await,
            &ProviderDescriptor::new("wrong-provider", "1"),
        )
        .expect_err("factory mismatch should fail");

        assert!(matches!(
            error,
            ProviderContractError::DescriptorMismatch {
                area: "factory",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn provider_identity_mismatch_is_reported() {
        let mut provider = FakeProvider::valid();
        provider.account_provider_id = "wrong-provider".to_string();

        let error = assert_model_provider_contract(&provider, &config())
            .await
            .expect_err("identity mismatch should fail");
        assert!(matches!(
            error,
            ProviderContractError::AccountProviderMismatch { .. }
        ));
    }

    #[tokio::test]
    async fn supported_operation_cannot_report_unsupported() {
        let config = config().with_usage(UsageRequest::default(), OperationExpectation::Supported);
        let error = assert_model_provider_contract(&FakeProvider::valid(), &config)
            .await
            .expect_err("unsupported usage should fail a supported probe");
        assert!(matches!(
            error,
            ProviderContractError::UnexpectedUnsupported { operation: "usage" }
        ));
    }

    #[tokio::test]
    async fn unsupported_usage_can_use_the_neutral_success_variant() {
        let config =
            config().with_usage(UsageRequest::default(), OperationExpectation::Unsupported);
        assert_model_provider_contract(&FakeProvider::valid(), &config)
            .await
            .expect("neutral unsupported usage should satisfy an unsupported probe");
    }
}

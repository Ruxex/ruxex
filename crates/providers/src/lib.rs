//! Built-in model-provider implementations for Genex.
//!
//! This crate contains concrete adapters only. The provider-neutral contract lives in
//! [`genex_model_provider`], and hosts choose one concrete implementation and inject it into an
//! `AppServerRuntime`.

pub mod cloudflare;
mod provider;

pub use cloudflare::CloudflareModelProvider;
pub use cloudflare::create_cloudflare_provider;
pub use genex_model_provider::SharedModelProvider;
pub use provider::create_model_provider;
pub use provider::create_model_provider_with_http_client_factory;
pub use genex_model_provider_info::ModelProviderInfo;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use genex_model_provider::ChatRequest;
    use genex_model_provider::ListModelsRequest;
    use genex_model_provider::ModelProvider;
    use genex_model_provider::ProviderAccountState;
    use genex_model_provider::ProviderAuthEnvironment;
    use genex_model_provider::ProviderAuthField;
    use genex_model_provider::ProviderAuthState;
    use genex_model_provider::ProviderCapabilities;
    use genex_model_provider::ProviderChatStream;
    use genex_model_provider::ProviderDescriptor;
    use genex_model_provider::ProviderError;
    use genex_model_provider::ProviderFuture;
    use genex_model_provider::ProviderModelCatalog;
    use genex_model_provider::ProviderRequestSnapshot;
    use genex_model_provider::ProviderResult;
    use genex_model_provider::ProviderUsage;
    use genex_model_provider::SharedModelProvider;
    use genex_model_provider::UsageRequest;
    use genex_model_provider_info::ModelProviderInfo;

    use super::create_model_provider;

    #[derive(Debug)]
    struct ExternalProvider;

    impl ModelProvider for ExternalProvider {
        fn descriptor(&self) -> ProviderDescriptor {
            ProviderDescriptor::new("external-test", "1")
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
                status: genex_protocol::account::ProviderAuthStatus::Unauthenticated,
                account_id: None,
                display_name: None,
                managed_auth: false,
                revision: None,
            }
        }

        fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn auth_environment(&self) -> ProviderAuthEnvironment {
            ProviderAuthEnvironment::default()
        }

        fn auth_fields(&self) -> Vec<ProviderAuthField> {
            Vec::new()
        }

        fn account_state(&self) -> ProviderResult<ProviderAccountState> {
            Err(ProviderError::Unsupported)
        }

        fn login(
            &self,
            _credentials: BTreeMap<String, String>,
        ) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }

        fn usage(
            &self,
            _request: UsageRequest,
        ) -> ProviderFuture<'_, ProviderResult<ProviderUsage>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }
    }

    #[test]
    fn an_external_provider_implements_the_neutral_contract() {
        let provider: SharedModelProvider = Arc::new(ExternalProvider);

        assert_eq!(provider.descriptor().provider_kind, "external-test");
        assert!(matches!(
            provider.account_state(),
            Err(ProviderError::Unsupported)
        ));
    }

    #[test]
    fn built_in_provider_factory_is_composed_by_this_crate() {
        let provider = create_model_provider(ModelProviderInfo::default(), None);

        assert_eq!(provider.descriptor().provider_kind, "Responses provider");
    }
}

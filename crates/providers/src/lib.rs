//! Built-in model-provider implementations for Ruxex.
//!
//! This crate contains concrete adapters only. The provider-neutral contract lives in
//! [`ruxex_model_provider`], and hosts choose one concrete implementation and inject it into an
//! `AppServerRuntime`.

pub mod cloudflare;
pub mod cloudflare_auth;
mod provider;

pub use cloudflare::CloudflareModelProvider;
pub use cloudflare::create_cloudflare_provider;
pub use cloudflare_auth::CloudflareCredentialResolver;
pub use cloudflare_auth::CloudflareCredentials;
pub use cloudflare_auth::create_cloudflare_credential_resolver;
pub use provider::create_model_provider;
pub use provider::create_model_provider_with_http_client_factory;
pub use ruxex_model_provider::SharedModelProvider;
pub use ruxex_model_provider_info::ModelProviderInfo;

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use ruxex_model_provider::ChatRequest;
    use ruxex_model_provider::ListModelsRequest;
    use ruxex_model_provider::ModelProvider;
    use ruxex_model_provider::ProviderAccountState;
    use ruxex_model_provider::ProviderAuthEnvironment;
    use ruxex_model_provider::ProviderAuthField;
    use ruxex_model_provider::ProviderAuthState;
    use ruxex_model_provider::ProviderCapabilities;
    use ruxex_model_provider::ProviderChatStream;
    use ruxex_model_provider::ProviderDescriptor;
    use ruxex_model_provider::ProviderError;
    use ruxex_model_provider::ProviderFuture;
    use ruxex_model_provider::ProviderModelCatalog;
    use ruxex_model_provider::ProviderRequestSnapshot;
    use ruxex_model_provider::ProviderResult;
    use ruxex_model_provider::ProviderUsage;
    use ruxex_model_provider::SharedModelProvider;
    use ruxex_model_provider::UsageRequest;
    use ruxex_model_provider_info::ModelProviderInfo;

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
                status: ruxex_protocol::account::ProviderAuthStatus::Unauthenticated,
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
        let provider =
            create_model_provider(ModelProviderInfo::create_responses_provider(None), None);

        assert_eq!(provider.descriptor().provider_kind, "Responses provider");
    }
}

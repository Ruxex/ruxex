use ruxex_provider_auth::ProviderCredentialResolverRegistry;
use ruxex_provider_auth::SharedProviderCredentialResolver;

pub(crate) fn resolver_for_provider(
    registry: Option<&ProviderCredentialResolverRegistry>,
    provider_id: &str,
) -> Option<SharedProviderCredentialResolver> {
    registry.and_then(|registry| registry.resolver_for(provider_id))
}

#[cfg(test)]
mod tests {
    use ruxex_model_provider::ProviderDescriptor;
    use ruxex_model_provider::ProviderError;
    use ruxex_model_provider::ProviderFuture;
    use ruxex_model_provider::ProviderResult;
    use ruxex_provider_auth::CredentialInput;
    use ruxex_provider_auth::ErasedCredentialFlow;
    use ruxex_provider_auth::ErasedProviderCredentialResolver;
    use ruxex_provider_auth::ProviderCredentialResolverRegistry;
    use std::sync::Arc;

    use super::resolver_for_provider;

    struct FakeResolver(&'static str);

    impl ErasedProviderCredentialResolver for FakeResolver {
        fn provider_id(&self) -> &'static str {
            self.0
        }

        fn begin(
            &self,
            _input: CredentialInput,
        ) -> ProviderFuture<'_, ProviderResult<Box<dyn ErasedCredentialFlow>>> {
            Box::pin(async { Err(ProviderError::Unsupported) })
        }
    }

    #[test]
    fn selects_resolver_for_matching_provider_descriptor() {
        let mut registry = ProviderCredentialResolverRegistry::new();
        registry
            .register(Arc::new(FakeResolver("test-provider")))
            .unwrap();
        let descriptor = ProviderDescriptor::new("test-provider", "1");

        assert!(resolver_for_provider(Some(&registry), &descriptor.provider_kind).is_some());
    }

    #[test]
    fn rejects_resolver_for_another_provider() {
        let mut registry = ProviderCredentialResolverRegistry::new();
        registry
            .register(Arc::new(FakeResolver("test-provider")))
            .unwrap();
        let descriptor = ProviderDescriptor::new("other", "1");

        assert!(resolver_for_provider(Some(&registry), &descriptor.provider_kind).is_none());
    }
}

use std::collections::BTreeMap;

use ruxex_model_provider::{ProviderFuture, ProviderResult};
use ruxex_provider_auth::{
    CredentialFlow, CredentialInput, CredentialStep, CredentialValue, ProviderAuthIntegration,
    ProviderCredentialResolver, ProviderCredentialSink, ProviderCredentials,
};

#[derive(Debug)]
struct TestCredentials {
    api_key: String,
}

impl ProviderCredentials for TestCredentials {
    const PROVIDER_ID: &'static str = "test";

    fn into_fields(self) -> BTreeMap<String, String> {
        BTreeMap::from([(String::from("api_key"), self.api_key)])
    }
}

#[derive(Debug)]
struct TestProvider;

impl ProviderCredentialSink<TestCredentials> for TestProvider {
    fn provider_id(&self) -> &str {
        "test"
    }

    fn login_typed(&self, _credentials: TestCredentials) -> ProviderFuture<'_, ProviderResult<()>> {
        Box::pin(async { Ok(()) })
    }
}

struct TestFlow {
    finished: bool,
}

impl CredentialFlow<TestCredentials> for TestFlow {
    fn current_step(&self) -> CredentialStep {
        if self.finished {
            CredentialStep::Complete
        } else {
            CredentialStep::input("api_key", "API key", true)
        }
    }

    fn submit(
        &mut self,
        _value: CredentialValue,
    ) -> ProviderFuture<'_, ProviderResult<CredentialStep>> {
        self.finished = true;
        Box::pin(async { Ok(CredentialStep::Complete) })
    }

    fn finish(self: Box<Self>) -> ProviderResult<TestCredentials> {
        Ok(TestCredentials {
            api_key: "secret".to_string(),
        })
    }
}

#[derive(Debug)]
struct TestResolver;

impl ProviderCredentialResolver<TestCredentials> for TestResolver {
    fn begin(
        &self,
        _input: CredentialInput,
    ) -> ProviderFuture<'_, ProviderResult<Box<dyn CredentialFlow<TestCredentials>>>> {
        Box::pin(async { Ok(Box::new(TestFlow { finished: false }) as Box<dyn CredentialFlow<_>>) })
    }
}

#[tokio::test]
async fn typed_integration_erases_resolver_without_erasing_its_credential_contract() {
    let integration = ProviderAuthIntegration::new(TestProvider, TestResolver).unwrap();
    let resolver = integration.resolver();
    let mut flow = resolver.begin(CredentialInput::empty()).await.unwrap();

    assert_eq!(
        flow.current_step(),
        CredentialStep::input("api_key", "API key", true)
    );
    assert_eq!(
        flow.submit(CredentialValue::secret("secret"))
            .await
            .unwrap(),
        CredentialStep::Complete
    );
    let fields = flow.finish().unwrap();

    assert_eq!(fields["api_key"], "secret");
    assert_eq!(resolver.provider_id(), "test");
}

#[test]
fn typed_integration_rejects_a_provider_id_mismatch() {
    struct WrongProvider;

    impl ProviderCredentialSink<TestCredentials> for WrongProvider {
        fn provider_id(&self) -> &str {
            "wrong"
        }

        fn login_typed(
            &self,
            _credentials: TestCredentials,
        ) -> ProviderFuture<'_, ProviderResult<()>> {
            Box::pin(async { Ok(()) })
        }
    }

    assert!(ProviderAuthIntegration::new(WrongProvider, TestResolver).is_err());
}

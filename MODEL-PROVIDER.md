# Model provider

`ModelProvider` is the host application's adapter for its LLM service. Your
implementation owns the provider-specific endpoint URLs, authentication,
model catalog, request/response format, usage data, and errors.

For the CLI/TUI, implement both `ModelProvider` and `ModelProviderFactory`.
The CLI creates the provider and App Server internally from the factory. With
the default product identity, pass the type to
`ruxex::run::<MyProvider>`. When supplying a custom `ProductIdentity`, pass the
factory to `ruxex::run_with_provider_factory_and_identity`; the CLI still
invokes the factory and owns the App Server lifecycle.

If you embed the App Server directly, you can instead construct a
`SharedModelProvider` and pass it to its runtime builder. Threads and rollouts
use the provider selected when the runtime starts; they do not select
providers or store credentials.

See the [`ModelProvider` contract](crates/model-provider/src/contract.rs) for
the required interface.

## Conformance tests

Compilation proves that a type implements the trait, but it does not prove
that the provider identity and runtime state agree. Ruxex publishes
`ruxex-model-provider-contract-tests` for provider-owned test suites. Add it as
a development dependency and run the helpers against the exact instance
created by `ModelProviderFactory`:

```rust
use ruxex_model_provider_contract_tests::{
    assert_model_provider_contract, construct_and_assert_model_provider_factory,
    OperationExpectation, ProviderContractConfig,
};
use ruxex_model_provider::{ProviderDescriptor, UsageRequest};

#[tokio::test]
async fn provider_contract() {
    let expected = ProviderDescriptor::new("my-provider", "1");
    let provider = construct_and_assert_model_provider_factory::<MyProvider>(
        test_provider_init(),
        &expected,
    )
    .expect("factory and provider descriptors must agree");

    let config = ProviderContractConfig::new(expected)
        .with_usage(UsageRequest::default(), OperationExpectation::Unsupported);
    assert_model_provider_contract(&provider, &config)
        .await
        .expect("provider must satisfy Ruxex's neutral contract");
}
```

The test crate checks provider-neutral semantics: stable identity, capability
reporting, model-catalog identity, request-snapshot identity, account-state
identity, and explicitly configured operation support. It does not assume a
particular HTTP payload or credential source. Successful authenticated calls,
stream event sequences, and provider-specific error mapping remain the
provider's own tests.

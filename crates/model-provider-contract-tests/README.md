# ruxex-model-provider-contract-tests

Reusable semantic conformance checks for custom Ruxex `ModelProvider`
implementations.

Add this crate as a development dependency in a provider implementation's
tests. It checks provider-neutral invariants such as descriptor identity,
factory consistency, stable capabilities, request-snapshot identity, and
account-state identity. Optional operation probes check that an operation does
not incorrectly report `Unsupported` (or incorrectly claim support).

```toml
[dev-dependencies]
ruxex-model-provider-contract-tests = { git = "https://github.com/Ruxex/ruxex" }
```

```rust
use ruxex_model_provider_contract_tests::{
    assert_model_provider_contract, construct_and_assert_model_provider_factory,
    OperationExpectation, ProviderContractConfig,
};
use ruxex_model_provider::{ListModelsRequest, ProviderDescriptor, UsageRequest};

#[tokio::test]
async fn provider_contract() {
    let expected = ProviderDescriptor::new("my-provider", "1");
    let provider = construct_and_assert_model_provider_factory::<MyProvider>(
        test_provider_init(),
        &expected,
    )
    .expect("factory must preserve the provider identity");

    let config = ProviderContractConfig::new(expected)
        .with_list_models(
            ListModelsRequest::new("contract-test"),
            OperationExpectation::Supported,
        )
        .with_usage(UsageRequest::default(), OperationExpectation::Unsupported);

    assert_model_provider_contract(&provider, &config)
        .await
        .expect("provider must satisfy the neutral Ruxex contract");
}
```

`test_provider_init` is provider-owned because it supplies the provider's
configuration, credential manager, and HTTP client policy. A `Supported` probe
only rejects `ProviderError::Unsupported`; authentication and successful
provider-specific wire behavior should be covered by additional provider-owned
tests.

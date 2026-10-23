# ruxex-model-provider-contract-tests

Reusable semantic conformance checks for custom Ruxex `ModelProvider`
implementations.

Add this crate as a development dependency in a provider implementation's
tests. The offline checks validate provider-neutral invariants such as
descriptor identity, factory consistency, stable capabilities, request-
snapshot identity, account-state identity, model catalog metadata, and chat
stream event ordering. The live chat runner calls the provider's real
`ModelProvider::chat` implementation and feeds the returned stream through the
same event validator.

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

## Offline chat contract

Use `validate_chat_events` for deterministic tests of an already constructed
provider stream. The validator requires this successful lifecycle:

```text
OutputItemAdded -> OutputTextDelta* -> OutputItemDone -> Completed
```

It rejects deltas without an active item, completion with an active item,
events after a stream error, and successful responses without assistant text.
For non-streamed tool/function responses, a standalone `OutputItemDone` is
also valid because no delta item needs to be kept active.
Use `ChatContractConfig::default().allow_stream_error()` when testing a
provider's intentional partial-disconnect behavior.

## Live provider checks

The conformance crate does not discover credentials or provider-specific
environment variables. A provider-owned integration test should construct the
real provider and pass it to `assert_provider_chat_contract`:

```rust
#[tokio::test]
#[ignore = "requires provider credentials and an API request"]
async fn live_chat_stream_satisfies_the_contract() {
    let provider = make_provider_from_real_environment().await;
    let request = make_small_contract_chat_request();

    let report = assert_provider_chat_contract(
        &provider,
        request,
        &ChatContractConfig::default()
            .with_live_timeout(std::time::Duration::from_secs(30)),
    )
    .await
    .expect("real provider stream must satisfy the Ruxex contract");

    assert!(!report.assistant_text.is_empty());
}
```

Live tests assert structural behavior rather than exact model wording. Keep
them ignored or behind a provider-owned feature and run them explicitly with
the provider's normal credential setup; they must never run accidentally as
part of the default workspace test suite.

The `ruxex-provider-host-example` includes a concrete ignored Cloudflare test.
Run it only when `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` are set:

```text
cargo test -p ruxex-provider-host-example cloudflare_live_chat_satisfies_the_shared_contract -- --ignored
```

Set `RUXEX_TEST_MODEL` to override the default Workers AI model slug.

The dummy provider also exercises `assert_provider_chat_contract` with a
locally constructed authenticated provider, so the live-runner path can be
tested without network access.

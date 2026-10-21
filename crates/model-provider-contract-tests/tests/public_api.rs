use ruxex_model_provider::ProviderDescriptor;
#[allow(unused_imports)]
use ruxex_model_provider_contract_tests::{
    OperationExpectation, ProviderContractConfig, assert_model_provider_contract,
    construct_and_assert_model_provider_factory,
};

#[test]
fn contract_helpers_are_publicly_importable() {
    let config = ProviderContractConfig::new(ProviderDescriptor::new("provider", "1")).with_usage(
        ruxex_model_provider::UsageRequest::default(),
        OperationExpectation::Unsupported,
    );

    let _ = config;
}

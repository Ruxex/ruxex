use ruxex_model_provider::ProviderDescriptor;
#[allow(unused_imports)]
use ruxex_model_provider_contract_tests::{
    ChatContractConfig, ChatContractError, ChatContractReport, OperationExpectation,
    ProviderContractConfig, ProviderContractError, assert_model_catalog_contract,
    assert_model_provider_contract, assert_provider_chat_contract,
    construct_and_assert_model_provider_factory, validate_chat_events,
};

#[test]
fn contract_helpers_are_publicly_importable() {
    let config = ProviderContractConfig::new(ProviderDescriptor::new("provider", "1")).with_usage(
        ruxex_model_provider::UsageRequest::default(),
        OperationExpectation::Unsupported,
    );

    let _ = config;
}

#[test]
fn chat_contract_helpers_are_publicly_importable() {
    let _: fn(
        Vec<ruxex_model_provider::ProviderResult<ruxex_model_provider::ProviderChatEvent>>,
        &ChatContractConfig,
    ) -> Result<ChatContractReport, ChatContractError> = validate_chat_events;

    let _ = assert_provider_chat_contract::<dyn ruxex_model_provider::ModelProvider>;
    let _ = assert_model_catalog_contract;
    let _ = ProviderContractError::InvalidModelCatalog {
        reason: "test".to_string(),
    };
}

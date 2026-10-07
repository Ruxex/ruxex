//! Provider and authentication dimensions that partition model catalogs.

use super::*;

#[test]
fn cache_identity_tracks_provider_routing_and_effective_api_credentials() {
    let auth = GenexAuth::from_api_key("first-key");
    let provider =
        ModelProviderInfo::create_responses_provider(Some("https://one.example/v1".into()));
    let key = identity(Some("provider-a"), &provider, Some(&auth)).unwrap();
    let other_key = GenexAuth::from_api_key("second-key");
    assert_ne!(
        identity(Some("provider-a"), &provider, Some(&other_key)).unwrap(),
        key
    );
    let mut other_provider = provider.clone();
    other_provider.base_url = Some("https://two.example/v1".into());
    assert_ne!(
        identity(Some("provider-a"), &other_provider, Some(&auth)).unwrap(),
        key
    );
    other_provider = provider.clone();
    other_provider.experimental_bearer_token = Some("provider-key".into());
    assert_ne!(
        identity(Some("provider-a"), &other_provider, Some(&auth)).unwrap(),
        key
    );
    other_provider = provider;
    other_provider.http_headers = Some(std::collections::HashMap::from([(
        "provider-project".into(),
        "another-project".into(),
    )]));
    assert_ne!(
        identity(Some("provider-b"), &other_provider, Some(&auth)).unwrap(),
        key
    );
}

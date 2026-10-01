//! Provider and authentication dimensions that partition model catalogs.

use super::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use pretty_assertions::assert_eq;

#[test]
fn cache_identity_tracks_provider_routing_and_effective_api_credentials() {
    let auth = CodexAuth::from_api_key("first-key");
    let provider =
        ModelProviderInfo::create_responses_provider(Some("https://one.example/v1".into()));
    let key = identity(&provider, Some(&auth)).unwrap();
    let other_key = CodexAuth::from_api_key("second-key");
    assert_ne!(identity(&provider, Some(&other_key)).unwrap(), key);
    let mut other_provider = provider.clone();
    other_provider.base_url = Some("https://two.example/v1".into());
    assert_ne!(identity(&other_provider, Some(&auth)).unwrap(), key);
    other_provider = provider.clone();
    other_provider.experimental_bearer_token = Some("provider-key".into());
    assert_ne!(identity(&other_provider, Some(&auth)).unwrap(), key);
    other_provider = provider;
    other_provider.http_headers = Some(std::collections::HashMap::from([(
        "provider-project".into(),
        "another-project".into(),
    )]));
    assert_ne!(identity(&other_provider, Some(&auth)).unwrap(), key);
}

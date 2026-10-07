//! Fixtures for integration tests that seed the provider's model cache.

use genex_login::GenexAuth;
use genex_model_provider_info::ModelProviderInfo;
use genex_models_manager::cache::ModelsCacheEntry;
use genex_protocol::model_types::ModelInfo;

/// Constructs a cache fixture matching the supplied test provider and authentication.
pub fn models_cache_entry(
    provider_info: &ModelProviderInfo,
    auth: Option<&GenexAuth>,
    models: Vec<ModelInfo>,
) -> ModelsCacheEntry {
    ModelsCacheEntry {
        fetched_at: std::time::SystemTime::now().into(),
        etag: None,
        client_version: Some(genex_models_manager::client_version_to_whole()),
        identity: crate::models_identity::identity(None, provider_info, auth).ok(),
        models,
    }
}

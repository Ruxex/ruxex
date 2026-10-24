use super::*;
use crate::ModelsManagerConfig;
use crate::cache::FileModelsCache;
use crate::cache::ModelsCache;
use crate::cache::ModelsCacheEntry;
use crate::cache::ModelsCacheError;
use crate::cache::ModelsCacheFuture;
use chrono::Utc;
use pretty_assertions::assert_eq;
use ruxex_http_client::HttpClientFactory;
use ruxex_http_client::OutboundProxyPolicy;
use ruxex_protocol::error::RuxexErr;
use ruxex_protocol::model_types::ModelCatalog;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tempfile::tempdir;

#[path = "api_key_discovery_tests.rs"]
mod api_key_discovery_tests;

#[path = "cache_identity_tests.rs"]
mod cache_identity_tests;

#[path = "model_info_overrides_tests.rs"]
mod model_info_overrides_tests;

const DEFAULT_HTTP_CLIENT_FACTORY: HttpClientFactory =
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault);

fn remote_model(slug: &str, display: &str, priority: i32) -> ModelInfo {
    remote_model_with_visibility(slug, display, priority, "list")
}

fn remote_model_with_visibility(
    slug: &str,
    display: &str,
    priority: i32,
    visibility: &str,
) -> ModelInfo {
    serde_json::from_value(json!({
            "slug": slug,
            "display_name": display,
            "description": format!("{display} desc"),
            "default_reasoning_level": "medium",
            "supported_reasoning_levels": [{"effort": "low", "description": "low"}, {"effort": "medium", "description": "medium"}],
            "shell_type": "shell_command",
            "visibility": visibility,
            "minimal_client_version": [0, 1, 0],
            "supported_in_api": true,
            "priority": priority,
            "upgrade": null,
            "model_messages": {
                "instructions_template": "base instructions",
                "approvals": null,
                "auto_review": null,
                "permissions": null
            },
            "support_verbosity": false,
            "default_verbosity": null,
            "apply_patch_tool_type": null,
            "truncation_policy": {"mode": "bytes", "limit": 10_000},
            "supports_image_detail_original": false,
            "context_window": 272_000,
            "max_context_window": 272_000,
            "experimental_supported_tools": [],
        }))
        .expect("valid model")
}

fn assert_models_contain(actual: &[ModelInfo], expected: &[ModelInfo]) {
    for model in expected {
        assert!(
            actual.iter().any(|candidate| candidate.slug == model.slug),
            "expected model {} in cached list",
            model.slug
        );
    }
}

#[derive(Debug)]
struct TestModelCatalogSource {
    has_command_auth: bool,
    uses_managed_auth: bool,
    has_api_key_auth: bool,
    responses: Mutex<VecDeque<Vec<ModelInfo>>>,
    etag: Option<String>,
    fetch_count: AtomicUsize,
    observed_proxy_policy: Mutex<Option<OutboundProxyPolicy>>,
}

#[derive(Debug)]
struct TestModelsCache {
    entry: Mutex<Option<ModelsCacheEntry>>,
    load_error: bool,
    store_error: bool,
    stored_entries: Mutex<Vec<ModelsCacheEntry>>,
}

impl TestModelsCache {
    fn with_entry(entry: ModelsCacheEntry) -> Arc<Self> {
        Arc::new(Self {
            entry: Mutex::new(Some(entry)),
            load_error: false,
            store_error: false,
            stored_entries: Mutex::new(Vec::new()),
        })
    }
}

impl ModelsCache for TestModelsCache {
    fn load<'a>(
        &'a self,
        _client_version: &'a str,
    ) -> ModelsCacheFuture<'a, Result<Option<ModelsCacheEntry>, ModelsCacheError>> {
        Box::pin(async move {
            if self.load_error {
                return Err(ModelsCacheError::new("test load failure"));
            }
            Ok(self
                .entry
                .lock()
                .expect("cache entry lock should not be poisoned")
                .clone())
        })
    }

    fn store<'a>(
        &'a self,
        entry: &'a ModelsCacheEntry,
    ) -> ModelsCacheFuture<'a, Result<(), ModelsCacheError>> {
        Box::pin(async move {
            if self.store_error {
                return Err(ModelsCacheError::new("test store failure"));
            }
            self.stored_entries
                .lock()
                .expect("stored entries lock should not be poisoned")
                .push(entry.clone());
            Ok(())
        })
    }

    fn refresh_ttl<'a>(
        &'a self,
        _client_version: &'a str,
        _identity: &'a str,
        _etag: &'a str,
    ) -> ModelsCacheFuture<'a, Result<(), ModelsCacheError>> {
        Box::pin(async move {
            let refreshed = {
                let mut entry = self
                    .entry
                    .lock()
                    .expect("cache entry lock should not be poisoned");
                let entry = entry
                    .as_mut()
                    .ok_or_else(|| ModelsCacheError::new("cache not found"))?;
                entry.fetched_at = Utc::now();
                entry.clone()
            };
            self.store(&refreshed).await
        })
    }
}

impl TestModelCatalogSource {
    fn new(responses: Vec<Vec<ModelInfo>>) -> Arc<Self> {
        Arc::new(Self {
            has_command_auth: false,
            uses_managed_auth: true,
            has_api_key_auth: false,
            responses: Mutex::new(responses.into()),
            etag: None,
            fetch_count: AtomicUsize::new(0),
            observed_proxy_policy: Mutex::new(None),
        })
    }

    fn without_refresh(responses: Vec<Vec<ModelInfo>>) -> Arc<Self> {
        Arc::new(Self {
            has_command_auth: false,
            uses_managed_auth: false,
            has_api_key_auth: false,
            responses: Mutex::new(responses.into()),
            etag: None,
            fetch_count: AtomicUsize::new(0),
            observed_proxy_policy: Mutex::new(None),
        })
    }

    fn with_api_key_auth(responses: Vec<Vec<ModelInfo>>) -> Arc<Self> {
        Arc::new(Self {
            has_command_auth: false,
            uses_managed_auth: false,
            has_api_key_auth: true,
            responses: Mutex::new(responses.into()),
            etag: None,
            fetch_count: AtomicUsize::new(0),
            observed_proxy_policy: Mutex::new(None),
        })
    }

    fn fetch_count(&self) -> usize {
        self.fetch_count.load(Ordering::SeqCst)
    }

    fn observed_proxy_policy(&self) -> Option<OutboundProxyPolicy> {
        *self
            .observed_proxy_policy
            .lock()
            .expect("observed proxy policy lock should not be poisoned")
    }

    async fn list_models(&self) -> CoreResult<ModelCatalogSnapshot> {
        self.fetch_count.fetch_add(1, Ordering::SeqCst);
        let models = self
            .responses
            .lock()
            .expect("responses lock should not be poisoned")
            .pop_front()
            .unwrap_or_default();
        Ok(ModelCatalogSnapshot {
            catalog: ModelCatalog { models },
            etag: self.etag.clone(),
            identity: self.identity().expect("test endpoint identity"),
        })
    }
}

impl ModelCatalogSource for TestModelCatalogSource {
    fn auth_context(&self) -> ModelCatalogAuthContext {
        ModelCatalogAuthContext {
            uses_managed_auth: self.uses_managed_auth,
            has_api_key_auth: self.has_api_key_auth,
        }
    }

    fn supports_api_key_models(&self) -> bool {
        true
    }

    fn identity(&self) -> Option<String> {
        Some("test-provider".to_string())
    }

    fn has_command_auth(&self) -> bool {
        self.has_command_auth
    }

    fn uses_managed_auth(&self) -> ModelCatalogFuture<'_, bool> {
        Box::pin(async { self.uses_managed_auth })
    }

    fn list_models<'a>(
        &'a self,
        _client_version: &'a str,
        http_client_factory: HttpClientFactory,
    ) -> ModelCatalogFuture<'a, CoreResult<ModelCatalogSnapshot>> {
        Box::pin(async move {
            *self
                .observed_proxy_policy
                .lock()
                .expect("observed proxy policy lock should not be poisoned") =
                Some(http_client_factory.outbound_proxy_policy());
            TestModelCatalogSource::list_models(self).await
        })
    }
}

fn provider_manager_for_tests(
    ruxex_home: std::path::PathBuf,
    catalog_source: Arc<dyn ModelCatalogSource>,
) -> RemoteModelsManager {
    let manager = RemoteModelsManager::new(ruxex_home, catalog_source);
    manager.set_api_key_model_discovery_enabled(true);
    manager
}

async fn mutate_file_cache_for_test<F>(ruxex_home: &std::path::Path, f: F)
where
    F: FnOnce(&mut ModelsCacheEntry),
{
    let client_version = crate::client_version_to_whole();
    let cache = FileModelsCache::new(ruxex_home.join(MODEL_CACHE_FILE), DEFAULT_MODEL_CACHE_TTL);
    let mut entry = cache
        .load(&client_version)
        .await
        .expect("cache load succeeds")
        .expect("cache entry exists");
    f(&mut entry);
    cache.store(&entry).await.expect("cache store succeeds");
}

fn static_manager_for_tests(model_catalog: ModelCatalog) -> StaticModelsManager {
    StaticModelsManager::new(ModelCatalogAuthContext::default(), model_catalog)
}

#[derive(Debug)]
struct FailingModelCatalogSource;

impl ModelCatalogSource for FailingModelCatalogSource {
    fn identity(&self) -> Option<String> {
        Some("failing-provider".to_string())
    }

    fn has_command_auth(&self) -> bool {
        false
    }

    fn uses_managed_auth(&self) -> ModelCatalogFuture<'_, bool> {
        Box::pin(async { false })
    }

    fn supports_remote_models(&self) -> bool {
        true
    }

    fn list_models<'a>(
        &'a self,
        _client_version: &'a str,
        _http_client_factory: HttpClientFactory,
    ) -> ModelCatalogFuture<'a, CoreResult<ModelCatalogSnapshot>> {
        Box::pin(async {
            Err(RuxexErr::InvalidRequest(
                "model catalog request failed".to_string(),
            ))
        })
    }
}

#[tokio::test]
async fn list_models_result_propagates_catalog_source_errors() {
    let manager = RemoteModelsManager::new_without_cache(Arc::new(FailingModelCatalogSource));

    let result = manager
        .list_models_result(RefreshStrategy::Online, DEFAULT_HTTP_CLIENT_FACTORY)
        .await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().to_string(),
        "model catalog request failed"
    );
}

#[tokio::test]
async fn shared_factory_uses_static_catalog_without_remote_fetch() {
    let source = TestModelCatalogSource::new(Vec::new());
    let catalog = ModelCatalog {
        models: vec![remote_model("configured-model", "Configured", 0)],
    };
    let manager = build_models_manager_without_cache(source.clone(), Some(catalog.clone()));

    let actual = manager
        .raw_model_catalog(RefreshStrategy::Online, DEFAULT_HTTP_CLIENT_FACTORY)
        .await;

    assert_eq!(actual, catalog);
    assert_eq!(source.fetch_count(), 0);
}

#[tokio::test]
async fn shared_factory_uses_remote_catalog_source_without_static_catalog() {
    let model = remote_model("remote-model", "Remote", 0);
    let source = TestModelCatalogSource::new(vec![vec![model.clone()]]);
    let manager =
        build_models_manager_without_cache(source.clone(), /*config_model_catalog*/ None);

    let actual = manager
        .raw_model_catalog(RefreshStrategy::Online, DEFAULT_HTTP_CLIENT_FACTORY)
        .await;

    assert_eq!(actual.models, vec![model]);
    assert_eq!(source.fetch_count(), 1);
}

#[tokio::test]
async fn static_manager_preserves_supported_requested_model_when_fallback_is_allowed() {
    let manager = static_manager_for_tests(ModelCatalog {
        models: vec![
            remote_model("provider-default", "Default", /*priority*/ 0),
            remote_model("provider-supported", "Supported", /*priority*/ 1),
        ],
    });
    let requested_model = Some("provider-supported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_model_fallback*/ true,
            RefreshStrategy::Offline,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "provider-supported");
}

#[tokio::test]
async fn static_manager_falls_back_from_unsupported_requested_model_when_allowed() {
    let manager = static_manager_for_tests(ModelCatalog {
        models: vec![
            remote_model("provider-default", "Default", /*priority*/ 0),
            remote_model("provider-supported", "Supported", /*priority*/ 1),
        ],
    });
    let requested_model = Some("unsupported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_model_fallback*/ true,
            RefreshStrategy::Offline,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "provider-default");
}

#[tokio::test]
async fn static_manager_preserves_unsupported_requested_model_when_fallback_is_disabled() {
    let manager = static_manager_for_tests(ModelCatalog {
        models: vec![remote_model(
            "provider-default",
            "Default",
            /*priority*/ 0,
        )],
    });
    let requested_model = Some("unsupported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_model_fallback*/ false,
            RefreshStrategy::Offline,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "unsupported");
}

#[tokio::test]
async fn static_manager_uses_empty_default_when_fallback_is_allowed_and_catalog_is_empty() {
    let manager = static_manager_for_tests(ModelCatalog { models: Vec::new() });
    let requested_model = Some("unsupported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_model_fallback*/ true,
            RefreshStrategy::Offline,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "");
}

#[tokio::test]
async fn dynamic_manager_preserves_requested_model_when_fallback_is_allowed() {
    let ruxex_home = tempdir().expect("temp dir");
    let endpoint = TestModelCatalogSource::new(Vec::new());
    let manager = provider_manager_for_tests(ruxex_home.path().to_path_buf(), endpoint.clone());
    let requested_model = Some("unsupported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_model_fallback*/ true,
            RefreshStrategy::Online,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "unsupported");
    assert_eq!(endpoint.fetch_count(), 0);
}

#[tokio::test]
async fn get_model_info_tracks_fallback_usage() {
    let config = ModelsManagerConfig::default();
    let manager = static_manager_for_tests(
        crate::test_support::test_models_response().expect("test model catalog should parse"),
    );
    let known_slug = manager
        .get_remote_models()
        .await
        .first()
        .expect("bundled models should include at least one model")
        .slug
        .clone();

    let known = manager.get_model_info(known_slug.as_str(), &config).await;
    assert!(!known.used_fallback_model_metadata);
    assert_eq!(known.slug, known_slug);

    let unknown = manager
        .get_model_info("model-that-does-not-exist", &config)
        .await;
    assert!(unknown.used_fallback_model_metadata);
    assert_eq!(unknown.slug, "model-that-does-not-exist");
}

#[tokio::test]
async fn get_model_info_applies_long_context_override_to_bundled_model_5_6_models() {
    let manager = static_manager_for_tests(
        crate::test_support::test_models_response().expect("test model catalog should parse"),
    );
    let config = ModelsManagerConfig {
        model_context_window: Some(1_000_000),
        ..Default::default()
    };

    for slug in ["provider-model-b", "provider-model-d", "provider-model-c"] {
        let model_info = manager.get_model_info(slug, &config).await;
        let mut expected = manager
            .get_model_info(slug, &ModelsManagerConfig::default())
            .await;
        expected.context_window = Some(872_000);

        assert_eq!(model_info, expected);
    }
}

#[tokio::test]
async fn get_model_info_uses_custom_catalog() {
    let config = ModelsManagerConfig::default();
    let mut overlay = remote_model("model-overlay", "Overlay", /*priority*/ 0);
    overlay.supports_image_detail_original = true;

    let manager = static_manager_for_tests(ModelCatalog {
        models: vec![overlay],
    });

    let model_info = manager
        .get_model_info("model-overlay-experiment", &config)
        .await;

    assert_eq!(model_info.slug, "model-overlay-experiment");
    assert_eq!(model_info.display_name, "Overlay");
    assert_eq!(model_info.context_window, Some(272_000));
    assert!(model_info.supports_image_detail_original);
    assert!(!model_info.used_fallback_model_metadata);
}

#[tokio::test]
async fn get_model_info_matches_namespaced_suffix() {
    let config = ModelsManagerConfig::default();
    let mut remote = remote_model("model-image", "Image", /*priority*/ 0);
    remote.supports_image_detail_original = true;
    let manager = static_manager_for_tests(ModelCatalog {
        models: vec![remote],
    });
    let namespaced_model = "custom/model-image".to_string();

    let model_info = manager.get_model_info(&namespaced_model, &config).await;

    assert_eq!(model_info.slug, namespaced_model);
    assert!(model_info.supports_image_detail_original);
    assert!(!model_info.used_fallback_model_metadata);
}

#[tokio::test]
async fn get_model_info_matches_hyphenated_provider_namespace_suffix() {
    let config = ModelsManagerConfig::default();
    let remote = remote_model("model-image", "Image", /*priority*/ 0);
    let manager = static_manager_for_tests(ModelCatalog {
        models: vec![remote],
    });
    let namespaced_model = "provider-codex/model-image".to_string();

    let model_info = manager.get_model_info(&namespaced_model, &config).await;

    assert_eq!(model_info.slug, namespaced_model);
    assert!(!model_info.used_fallback_model_metadata);
}

#[tokio::test]
async fn get_model_info_rejects_multi_segment_namespace_suffix_matching() {
    let config = ModelsManagerConfig::default();
    let manager = static_manager_for_tests(
        crate::test_support::test_models_response().expect("test model catalog should parse"),
    );
    let known_slug = manager
        .get_remote_models()
        .await
        .first()
        .expect("bundled models should include at least one model")
        .slug
        .clone();
    let namespaced_model = format!("ns1/ns2/{known_slug}");

    let model_info = manager.get_model_info(&namespaced_model, &config).await;

    assert_eq!(model_info.slug, namespaced_model);
    assert!(model_info.used_fallback_model_metadata);
}

#[tokio::test]
async fn refresh_available_models_sorts_by_priority() {
    let remote_models = vec![
        remote_model("priority-low", "Low", /*priority*/ 1),
        remote_model("priority-high", "High", /*priority*/ 0),
    ];
    let ruxex_home = tempdir().expect("temp dir");
    let endpoint = TestModelCatalogSource::new(vec![remote_models.clone()]);
    let manager = provider_manager_for_tests(ruxex_home.path().to_path_buf(), endpoint.clone());

    let available = manager
        .list_models(
            RefreshStrategy::Online,
            HttpClientFactory::new(OutboundProxyPolicy::RespectSystemProxy),
        )
        .await;
    assert_models_contain(&manager.get_remote_models().await, &remote_models);
    assert_eq!(
        endpoint.observed_proxy_policy(),
        Some(OutboundProxyPolicy::RespectSystemProxy)
    );
    let high_idx = available
        .iter()
        .position(|model| model.model == "priority-high")
        .expect("priority-high should be listed");
    let low_idx = available
        .iter()
        .position(|model| model.model == "priority-low")
        .expect("priority-low should be listed");
    assert!(
        high_idx < low_idx,
        "higher priority should be listed before lower priority"
    );
    assert_eq!(endpoint.fetch_count(), 1, "expected a single model fetch");
}

#[tokio::test]
async fn refresh_available_models_uses_only_dynamic_models_for_custom_api_auth() {
    let remote_models = vec![remote_model(
        "api-auth-visible-remote",
        "API Auth Visible",
        /*priority*/ 0,
    )];
    let ruxex_home = tempdir().expect("temp dir");
    let endpoint = Arc::new(TestModelCatalogSource {
        has_command_auth: true,
        uses_managed_auth: false,
        has_api_key_auth: true,
        responses: Mutex::new(vec![remote_models.clone()].into()),
        etag: None,
        fetch_count: AtomicUsize::new(0),
        observed_proxy_policy: Mutex::new(None),
    });
    let manager = provider_manager_for_tests(ruxex_home.path().to_path_buf(), endpoint.clone());
    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("refresh succeeds");

    assert_eq!(manager.get_remote_models().await, remote_models);
    assert_eq!(endpoint.fetch_count(), 1, "expected a single model fetch");
}

#[tokio::test]
async fn refresh_available_models_uses_cache_when_fresh() {
    let remote_models = vec![remote_model("cached", "Cached", /*priority*/ 5)];
    let ruxex_home = tempdir().expect("temp dir");
    let endpoint = TestModelCatalogSource::new(vec![remote_models.clone()]);
    let manager = provider_manager_for_tests(ruxex_home.path().to_path_buf(), endpoint.clone());

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("first refresh succeeds");
    assert_models_contain(&manager.get_remote_models().await, &remote_models);

    // Second call should read from cache and avoid the network.
    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("cached refresh succeeds");
    assert_models_contain(&manager.get_remote_models().await, &remote_models);
    assert_eq!(
        endpoint.fetch_count(),
        1,
        "cache hit should avoid a second model fetch"
    );
}

#[tokio::test]
async fn refresh_available_models_refetches_when_cache_stale() {
    let initial_models = vec![remote_model("stale", "Stale", /*priority*/ 1)];
    let ruxex_home = tempdir().expect("temp dir");
    let updated_models = vec![remote_model("fresh", "Fresh", /*priority*/ 9)];
    let endpoint =
        TestModelCatalogSource::new(vec![initial_models.clone(), updated_models.clone()]);
    let manager = provider_manager_for_tests(ruxex_home.path().to_path_buf(), endpoint.clone());

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("initial refresh succeeds");

    // Rewrite cache with an old timestamp so it is treated as stale.
    mutate_file_cache_for_test(ruxex_home.path(), |cache| {
        cache.fetched_at = Utc::now() - chrono::Duration::hours(1);
    })
    .await;

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("second refresh succeeds");
    assert_models_contain(&manager.get_remote_models().await, &updated_models);
    assert_eq!(
        endpoint.fetch_count(),
        2,
        "stale cache refresh should fetch models again"
    );
}

#[tokio::test]
async fn refresh_available_models_refetches_when_version_mismatch() {
    let initial_models = vec![remote_model("old", "Old", /*priority*/ 1)];
    let ruxex_home = tempdir().expect("temp dir");
    let updated_models = vec![remote_model("new", "New", /*priority*/ 2)];
    let endpoint =
        TestModelCatalogSource::new(vec![initial_models.clone(), updated_models.clone()]);
    let manager = provider_manager_for_tests(ruxex_home.path().to_path_buf(), endpoint.clone());

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("initial refresh succeeds");

    mutate_file_cache_for_test(ruxex_home.path(), |cache| {
        let client_version = crate::client_version_to_whole();
        cache.client_version = Some(format!("{client_version}-mismatch"));
    })
    .await;

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("second refresh succeeds");
    assert_models_contain(&manager.get_remote_models().await, &updated_models);
    assert_eq!(
        endpoint.fetch_count(),
        2,
        "version mismatch should fetch models again"
    );
}

#[tokio::test]
async fn refresh_available_models_drops_removed_remote_models() {
    let initial_models = vec![remote_model(
        "remote-old",
        "Remote Old",
        /*priority*/ 1,
    )];
    let ruxex_home = tempdir().expect("temp dir");
    let refreshed_models = vec![remote_model(
        "remote-new",
        "Remote New",
        /*priority*/ 1,
    )];
    let endpoint = TestModelCatalogSource::new(vec![initial_models, refreshed_models]);
    let manager = RemoteModelsManager::new_with_cache(
        Arc::new(FileModelsCache::new(
            ruxex_home.path().join(MODEL_CACHE_FILE),
            Duration::ZERO,
        )),
        endpoint.clone(),
    );
    manager.set_api_key_model_discovery_enabled(true);

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("initial refresh succeeds");

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("second refresh succeeds");

    let available = manager
        .try_list_models()
        .expect("models should be available");
    assert!(
        available.iter().any(|preset| preset.model == "remote-new"),
        "new remote model should be listed"
    );
    assert!(
        !available.iter().any(|preset| preset.model == "remote-old"),
        "removed remote model should not be listed"
    );
    assert_eq!(
        endpoint.fetch_count(),
        2,
        "second refresh should fetch models again"
    );
}

#[tokio::test]
async fn refresh_available_models_skips_network_without_auth() {
    let dynamic_slug = "dynamic-model-only-for-test-noauth";
    let ruxex_home = tempdir().expect("temp dir");
    let endpoint = TestModelCatalogSource::without_refresh(vec![vec![remote_model(
        dynamic_slug,
        "No Auth",
        /*priority*/ 1,
    )]]);
    let manager = provider_manager_for_tests(ruxex_home.path().to_path_buf(), endpoint.clone());

    manager
        .refresh_available_models(RefreshStrategy::Online, &DEFAULT_HTTP_CLIENT_FACTORY)
        .await
        .expect("refresh should no-op without auth");
    let cached_remote = manager.get_remote_models().await;
    assert!(
        !cached_remote
            .iter()
            .any(|candidate| candidate.slug == dynamic_slug),
        "remote refresh should be skipped without auth"
    );
    assert_eq!(
        endpoint.fetch_count(),
        0,
        "endpoint that cannot refresh should avoid model fetches"
    );
}

#[test]
fn build_available_models_picks_default_after_hiding_hidden_models() {
    let manager = static_manager_for_tests(ModelCatalog { models: Vec::new() });

    let hidden_model =
        remote_model_with_visibility("hidden", "Hidden", /*priority*/ 0, "hide");
    let visible_model =
        remote_model_with_visibility("visible", "Visible", /*priority*/ 1, "list");

    let expected_hidden = ModelPreset::from(hidden_model.clone());
    let mut expected_visible = ModelPreset::from(visible_model.clone());
    expected_visible.is_default = true;

    let available = manager.build_available_models(vec![hidden_model, visible_model]);

    assert_eq!(available, vec![expected_hidden, expected_visible]);
}

#[test]
fn bundled_models_json_roundtrips() {
    let response = crate::test_support::test_models_response()
        .unwrap_or_else(|err| panic!("test model catalog should parse: {err}"));

    let serialized = serde_json::to_string(&response).expect("test model catalog should serialize");
    let roundtripped: ModelCatalog = serde_json::from_str(&serialized)
        .expect("serialized test model catalog should deserialize");

    assert_eq!(
        response, roundtripped,
        "test model catalog should round trip through serde"
    );
    assert!(
        !response.models.is_empty(),
        "test model catalog should contain at least one model"
    );
}

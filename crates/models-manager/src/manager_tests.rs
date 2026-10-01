use super::*;
use crate::ModelsManagerConfig;
use crate::cache::FileModelsCache;
use crate::cache::ModelsCache;
use crate::cache::ModelsCacheEntry;
use crate::cache::ModelsCacheError;
use crate::cache::ModelsCacheFuture;
use chrono::Utc;
use codex_http_client::HttpClientFactory;
use codex_http_client::OutboundProxyPolicy;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_protocol::model_types::ModelAccessPrograms;
use codex_protocol::model_types::ModelsResponse;
use codex_protocol::turn_input::CyberAccessProgram;
use pretty_assertions::assert_eq;
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
struct TestModelsEndpoint {
    has_command_auth: bool,
    uses_managed_auth: bool,
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

    fn failing(load_error: bool, store_error: bool) -> Arc<Self> {
        Arc::new(Self {
            entry: Mutex::new(None),
            load_error,
            store_error,
            stored_entries: Mutex::new(Vec::new()),
        })
    }

    fn stored_entries(&self) -> Vec<ModelsCacheEntry> {
        self.stored_entries
            .lock()
            .expect("stored entries lock should not be poisoned")
            .clone()
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

impl TestModelsEndpoint {
    fn new(responses: Vec<Vec<ModelInfo>>) -> Arc<Self> {
        Arc::new(Self {
            has_command_auth: false,
            uses_managed_auth: true,
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

    async fn list_models(&self) -> CoreResult<ModelsEndpointResponse> {
        self.fetch_count.fetch_add(1, Ordering::SeqCst);
        let models = self
            .responses
            .lock()
            .expect("responses lock should not be poisoned")
            .pop_front()
            .unwrap_or_default();
        Ok(ModelsEndpointResponse {
            models,
            etag: self.etag.clone(),
            identity: self.identity().expect("test endpoint identity"),
        })
    }
}

impl ModelsEndpointClient for TestModelsEndpoint {
    fn supports_api_key_models(&self) -> bool {
        true
    }

    fn identity(&self) -> Option<String> {
        Some("test-provider".to_string())
    }

    fn has_command_auth(&self) -> bool {
        self.has_command_auth
    }

    fn uses_managed_auth(&self) -> ModelsEndpointFuture<'_, bool> {
        Box::pin(async { self.uses_managed_auth })
    }

    fn list_models<'a>(
        &'a self,
        _client_version: &'a str,
        http_client_factory: HttpClientFactory,
    ) -> ModelsEndpointFuture<'a, CoreResult<ModelsEndpointResponse>> {
        Box::pin(async move {
            *self
                .observed_proxy_policy
                .lock()
                .expect("observed proxy policy lock should not be poisoned") =
                Some(http_client_factory.outbound_proxy_policy());
            TestModelsEndpoint::list_models(self).await
        })
    }
}

fn provider_manager_for_tests(
    codex_home: std::path::PathBuf,
    endpoint_client: Arc<dyn ModelsEndpointClient>,
) -> RemoteModelsManager {
    provider_manager_for_tests_with_auth(
        codex_home,
        endpoint_client,
        Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
            "test-api-key",
        ))),
    )
}

fn provider_manager_for_tests_with_auth(
    codex_home: std::path::PathBuf,
    endpoint_client: Arc<dyn ModelsEndpointClient>,
    auth_manager: Option<Arc<AuthManager>>,
) -> RemoteModelsManager {
    RemoteModelsManager::new(codex_home, endpoint_client, auth_manager)
}

async fn mutate_file_cache_for_test<F>(codex_home: &std::path::Path, f: F)
where
    F: FnOnce(&mut ModelsCacheEntry),
{
    let client_version = crate::client_version_to_whole();
    let cache = FileModelsCache::new(codex_home.join(MODEL_CACHE_FILE), DEFAULT_MODEL_CACHE_TTL);
    let mut entry = cache
        .load(&client_version)
        .await
        .expect("cache load succeeds")
        .expect("cache entry exists");
    f(&mut entry);
    cache.store(&entry).await.expect("cache store succeeds");
}

fn static_manager_for_tests(model_catalog: ModelsResponse) -> StaticModelsManager {
    StaticModelsManager::new(/*auth_manager*/ None, model_catalog)
}

#[tokio::test]
async fn static_manager_preserves_supported_requested_model_when_fallback_is_allowed() {
    let manager = static_manager_for_tests(ModelsResponse {
        models: vec![
            remote_model("provider-default", "Default", /*priority*/ 0),
            remote_model("provider-supported", "Supported", /*priority*/ 1),
        ],
    });
    let requested_model = Some("provider-supported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_provider_model_fallback*/ true,
            RefreshStrategy::Offline,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "provider-supported");
}

#[tokio::test]
async fn static_manager_falls_back_from_unsupported_requested_model_when_allowed() {
    let manager = static_manager_for_tests(ModelsResponse {
        models: vec![
            remote_model("provider-default", "Default", /*priority*/ 0),
            remote_model("provider-supported", "Supported", /*priority*/ 1),
        ],
    });
    let requested_model = Some("unsupported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_provider_model_fallback*/ true,
            RefreshStrategy::Offline,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "provider-default");
}

#[tokio::test]
async fn static_manager_preserves_unsupported_requested_model_when_fallback_is_disabled() {
    let manager = static_manager_for_tests(ModelsResponse {
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
            /*allow_provider_model_fallback*/ false,
            RefreshStrategy::Offline,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "unsupported");
}

#[tokio::test]
async fn static_manager_uses_empty_default_when_fallback_is_allowed_and_catalog_is_empty() {
    let manager = static_manager_for_tests(ModelsResponse { models: Vec::new() });
    let requested_model = Some("unsupported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_provider_model_fallback*/ true,
            RefreshStrategy::Offline,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "");
}

#[tokio::test]
async fn dynamic_manager_preserves_requested_model_when_fallback_is_allowed() {
    let codex_home = tempdir().expect("temp dir");
    let endpoint = TestModelsEndpoint::new(Vec::new());
    let manager = provider_manager_for_tests(codex_home.path().to_path_buf(), endpoint.clone());
    let requested_model = Some("unsupported".to_string());

    let model = manager
        .get_default_model(
            &requested_model,
            /*allow_provider_model_fallback*/ true,
            RefreshStrategy::Online,
            DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await;

    assert_eq!(model, "unsupported");
    assert_eq!(endpoint.fetch_count(), 0);
}

#[tokio::test]
async fn get_model_info_tracks_fallback_usage() {
    let codex_home = tempdir().expect("temp dir");
    let config = ModelsManagerConfig::default();
    let manager = provider_manager_for_tests(
        codex_home.path().to_path_buf(),
        TestModelsEndpoint::new(Vec::new()),
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
async fn get_model_info_applies_long_context_override_to_bundled_gpt_5_6_models() {
    let codex_home = tempdir().expect("temp dir");
    let manager = provider_manager_for_tests(
        codex_home.path().to_path_buf(),
        TestModelsEndpoint::new(Vec::new()),
    );
    let config = ModelsManagerConfig {
        model_context_window: Some(1_000_000),
        ..Default::default()
    };

    for slug in ["gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna"] {
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
    let mut overlay = remote_model("gpt-overlay", "Overlay", /*priority*/ 0);
    overlay.supports_image_detail_original = true;

    let manager = static_manager_for_tests(ModelsResponse {
        models: vec![overlay],
    });

    let model_info = manager
        .get_model_info("gpt-overlay-experiment", &config)
        .await;

    assert_eq!(model_info.slug, "gpt-overlay-experiment");
    assert_eq!(model_info.display_name, "Overlay");
    assert_eq!(model_info.context_window, Some(272_000));
    assert!(model_info.supports_image_detail_original);
    assert!(!model_info.used_fallback_model_metadata);
}

#[tokio::test]
async fn get_model_info_matches_namespaced_suffix() {
    let config = ModelsManagerConfig::default();
    let mut remote = remote_model("gpt-image", "Image", /*priority*/ 0);
    remote.supports_image_detail_original = true;
    let manager = static_manager_for_tests(ModelsResponse {
        models: vec![remote],
    });
    let namespaced_model = "custom/gpt-image".to_string();

    let model_info = manager.get_model_info(&namespaced_model, &config).await;

    assert_eq!(model_info.slug, namespaced_model);
    assert!(model_info.supports_image_detail_original);
    assert!(!model_info.used_fallback_model_metadata);
}

#[tokio::test]
async fn get_model_info_matches_hyphenated_provider_namespace_suffix() {
    let config = ModelsManagerConfig::default();
    let remote = remote_model("gpt-image", "Image", /*priority*/ 0);
    let manager = static_manager_for_tests(ModelsResponse {
        models: vec![remote],
    });
    let namespaced_model = "provider-codex/gpt-image".to_string();

    let model_info = manager.get_model_info(&namespaced_model, &config).await;

    assert_eq!(model_info.slug, namespaced_model);
    assert!(!model_info.used_fallback_model_metadata);
}

#[tokio::test]
async fn get_model_info_rejects_multi_segment_namespace_suffix_matching() {
    let codex_home = tempdir().expect("temp dir");
    let config = ModelsManagerConfig::default();
    let manager = provider_manager_for_tests(
        codex_home.path().to_path_buf(),
        TestModelsEndpoint::new(Vec::new()),
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
    let codex_home = tempdir().expect("temp dir");
    let endpoint = TestModelsEndpoint::new(vec![remote_models.clone()]);
    let manager = provider_manager_for_tests(codex_home.path().to_path_buf(), endpoint.clone());

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
    let codex_home = tempdir().expect("temp dir");
    let endpoint = Arc::new(TestModelsEndpoint {
        has_command_auth: true,
        uses_managed_auth: false,
        responses: Mutex::new(vec![remote_models.clone()].into()),
        etag: None,
        fetch_count: AtomicUsize::new(0),
        observed_proxy_policy: Mutex::new(None),
    });
    let manager = provider_manager_for_tests_with_auth(
        codex_home.path().to_path_buf(),
        endpoint.clone(),
        Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
            "test-api-key",
        ))),
    );
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
    let codex_home = tempdir().expect("temp dir");
    let endpoint = TestModelsEndpoint::new(vec![remote_models.clone()]);
    let manager = provider_manager_for_tests(codex_home.path().to_path_buf(), endpoint.clone());

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
async fn online_refresh_updates_access_programs_with_unchanged_etag() {
    let codex_home = tempdir().expect("temp dir");
    let granted_model = ModelInfo {
        available_access_programs: Some(ModelAccessPrograms {
            cyber: vec![
                CyberAccessProgram::Standard,
                CyberAccessProgram::DaybreakBlue,
            ],
        }),
        ..remote_model("access-programs", "Access Programs", /*priority*/ 0)
    };
    let revoked_model = ModelInfo {
        available_access_programs: Some(ModelAccessPrograms { cyber: Vec::new() }),
        ..granted_model.clone()
    };
    let responses = vec![vec![granted_model.clone()], vec![revoked_model.clone()]];
    let endpoint = Arc::new(TestModelsEndpoint {
        has_command_auth: false,
        uses_managed_auth: true,
        responses: Mutex::new(responses.into()),
        etag: Some("stable-catalog-etag".to_string()),
        fetch_count: AtomicUsize::new(0),
        observed_proxy_policy: Mutex::new(None),
    });
    let manager = provider_manager_for_tests(codex_home.path().to_path_buf(), endpoint.clone());

    // Caller-specific access can change without changing the catalog ETag.
    for model in [granted_model, revoked_model] {
        let mut expected = ModelPreset::from(model.clone());
        expected.is_default = true;
        assert_eq!(
            manager
                .list_models(RefreshStrategy::Online, DEFAULT_HTTP_CLIENT_FACTORY)
                .await,
            vec![expected.clone()]
        );
        assert_eq!(manager.get_remote_models().await, vec![model]);
        assert_eq!(
            manager.remote_models.read().await.etag.as_deref(),
            Some("stable-catalog-etag")
        );

        // A new manager must read the latest access metadata from the disk cache.
        let cache_endpoint = TestModelsEndpoint::new(Vec::new());
        let cache_manager =
            provider_manager_for_tests(codex_home.path().to_path_buf(), cache_endpoint.clone());
        assert_eq!(
            cache_manager
                .list_models(
                    RefreshStrategy::OnlineIfUncached,
                    DEFAULT_HTTP_CLIENT_FACTORY
                )
                .await,
            vec![expected]
        );
        assert_eq!(cache_endpoint.fetch_count(), 0);
        assert_eq!(
            cache_manager.remote_models.read().await.etag.as_deref(),
            Some("stable-catalog-etag")
        );
    }
    assert_eq!(endpoint.fetch_count(), 2);
}

#[tokio::test]
async fn refresh_available_models_refetches_when_cache_stale() {
    let initial_models = vec![remote_model("stale", "Stale", /*priority*/ 1)];
    let codex_home = tempdir().expect("temp dir");
    let updated_models = vec![remote_model("fresh", "Fresh", /*priority*/ 9)];
    let endpoint = TestModelsEndpoint::new(vec![initial_models.clone(), updated_models.clone()]);
    let manager = provider_manager_for_tests(codex_home.path().to_path_buf(), endpoint.clone());

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("initial refresh succeeds");

    // Rewrite cache with an old timestamp so it is treated as stale.
    mutate_file_cache_for_test(codex_home.path(), |cache| {
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
    let codex_home = tempdir().expect("temp dir");
    let updated_models = vec![remote_model("new", "New", /*priority*/ 2)];
    let endpoint = TestModelsEndpoint::new(vec![initial_models.clone(), updated_models.clone()]);
    let manager = provider_manager_for_tests(codex_home.path().to_path_buf(), endpoint.clone());

    manager
        .refresh_available_models(
            RefreshStrategy::OnlineIfUncached,
            &DEFAULT_HTTP_CLIENT_FACTORY,
        )
        .await
        .expect("initial refresh succeeds");

    mutate_file_cache_for_test(codex_home.path(), |cache| {
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
    let codex_home = tempdir().expect("temp dir");
    let refreshed_models = vec![remote_model(
        "remote-new",
        "Remote New",
        /*priority*/ 1,
    )];
    let endpoint = TestModelsEndpoint::new(vec![initial_models, refreshed_models]);
    let manager = RemoteModelsManager::new_with_cache(
        Arc::new(FileModelsCache::new(
            codex_home.path().join(MODEL_CACHE_FILE),
            Duration::ZERO,
        )),
        endpoint.clone(),
        Some(AuthManager::from_auth_for_testing(CodexAuth::from_api_key(
            "test-api-key",
        ))),
    );

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
    let codex_home = tempdir().expect("temp dir");
    let endpoint = TestModelsEndpoint::without_refresh(vec![vec![remote_model(
        dynamic_slug,
        "No Auth",
        /*priority*/ 1,
    )]]);
    let manager = provider_manager_for_tests_with_auth(
        codex_home.path().to_path_buf(),
        endpoint.clone(),
        /*auth_manager*/ None,
    );

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

#[derive(Debug)]
struct TestAuthAwareModelsEndpoint {
    auth_manager: Option<Arc<AuthManager>>,
    responses: Mutex<VecDeque<Vec<ModelInfo>>>,
    fetch_count: AtomicUsize,
}

impl TestAuthAwareModelsEndpoint {
    fn new(auth_manager: Option<Arc<AuthManager>>, responses: Vec<Vec<ModelInfo>>) -> Arc<Self> {
        Arc::new(Self {
            auth_manager,
            responses: Mutex::new(responses.into()),
            fetch_count: AtomicUsize::new(0),
        })
    }

    fn fetch_count(&self) -> usize {
        self.fetch_count.load(Ordering::SeqCst)
    }

    async fn uses_managed_auth(&self) -> bool {
        match self.auth_manager.as_ref() {
            Some(auth_manager) => auth_manager
                .auth()
                .await
                .as_ref()
                .is_some_and(CodexAuth::uses_managed_auth),
            None => false,
        }
    }

    async fn list_models(&self) -> CoreResult<ModelsEndpointResponse> {
        self.fetch_count.fetch_add(1, Ordering::SeqCst);
        let models = self
            .responses
            .lock()
            .expect("responses lock should not be poisoned")
            .pop_front()
            .unwrap_or_default();
        Ok(ModelsEndpointResponse {
            models,
            etag: None,
            identity: self.identity().expect("test endpoint identity"),
        })
    }
}

impl ModelsEndpointClient for TestAuthAwareModelsEndpoint {
    fn supports_api_key_models(&self) -> bool {
        true
    }

    fn identity(&self) -> Option<String> {
        self.auth_manager
            .as_ref()
            .and_then(|manager| manager.auth_mode())
            .map(|mode| format!("{mode:?}"))
    }

    fn has_command_auth(&self) -> bool {
        false
    }

    fn uses_managed_auth(&self) -> ModelsEndpointFuture<'_, bool> {
        Box::pin(TestAuthAwareModelsEndpoint::uses_managed_auth(self))
    }

    fn list_models<'a>(
        &'a self,
        _client_version: &'a str,
        _http_client_factory: HttpClientFactory,
    ) -> ModelsEndpointFuture<'a, CoreResult<ModelsEndpointResponse>> {
        Box::pin(TestAuthAwareModelsEndpoint::list_models(self))
    }
}

#[test]
fn build_available_models_picks_default_after_hiding_hidden_models() {
    let manager = static_manager_for_tests(ModelsResponse { models: Vec::new() });

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
    let roundtripped: ModelsResponse = serde_json::from_str(&serialized)
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

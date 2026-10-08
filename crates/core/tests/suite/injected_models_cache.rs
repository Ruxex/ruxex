//! Integration coverage for model catalogs supplied through the public cache interface.

use genex_core::TurnInputRequest;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use anyhow::Result;
use base64::Engine;
use chrono::Utc;
use core_test_support::responses;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::sse;
use core_test_support::submit_thread_settings;
use core_test_support::test_genex::test_genex;
use core_test_support::wait_for_event;
use genex_http_client::HttpClientFactory;
use genex_login::CredentialManager;
use genex_login::GenexAuth;
use genex_model_provider::ModelProvider;
use genex_model_provider::ProviderModelCatalogSource;
use genex_model_provider::create_model_provider;
use genex_models_manager::cache::ModelsCache;
use genex_models_manager::cache::ModelsCacheEntry;
use genex_models_manager::cache::ModelsCacheError;
use genex_models_manager::cache::ModelsCacheFuture;
use genex_models_manager::manager::ModelCatalogFuture;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_models_manager::manager::ModelCatalogSource;
use genex_models_manager::manager::RefreshStrategy;
use genex_models_manager::manager::RemoteModelsManager;
use genex_models_manager::manager::SharedModelsManager;
use genex_models_manager::manager::build_models_manager_with_cache;
use genex_models_manager::model_info::model_info_from_slug;
use genex_protocol::config_types::ReasoningSummary;
use genex_protocol::error::Result as CoreResult;
use genex_protocol::model_types::ModelCatalog;
use genex_protocol::model_types::ModelInfo;
use genex_protocol::model_types::ModelVisibility;
use genex_protocol::protocol::EventMsg;
use genex_protocol::protocol::ThreadSettingsOverrides;
use genex_protocol::user_input::UserInput;
use pretty_assertions::assert_eq;
use tempfile::TempDir;
use tokio::sync::oneshot;
use tokio::time::Duration;
use tokio::time::timeout;

#[derive(Debug)]
struct StoreGate {
    entered: oneshot::Sender<()>,
    release: oneshot::Receiver<()>,
}

#[derive(Debug)]
struct TestModelsCache {
    entry: Option<ModelsCacheEntry>,
    load_error: bool,
    stored_entries: Mutex<Vec<ModelsCacheEntry>>,
    store_gate: Mutex<Option<StoreGate>>,
}

impl ModelsCache for TestModelsCache {
    fn load<'a>(
        &'a self,
        _client_version: &'a str,
    ) -> ModelsCacheFuture<'a, Result<Option<ModelsCacheEntry>, ModelsCacheError>> {
        Box::pin(async move {
            if self.load_error {
                Err(ModelsCacheError::new("test load failure"))
            } else {
                Ok(self
                    .stored_entries
                    .lock()
                    .expect("stored entries lock should not be poisoned")
                    .last()
                    .cloned()
                    .or_else(|| self.entry.clone()))
            }
        })
    }

    fn store<'a>(
        &'a self,
        entry: &'a ModelsCacheEntry,
    ) -> ModelsCacheFuture<'a, Result<(), ModelsCacheError>> {
        Box::pin(async move {
            self.stored_entries
                .lock()
                .expect("stored entries lock should not be poisoned")
                .push(entry.clone());
            let gate = self
                .store_gate
                .lock()
                .expect("store gate lock should not be poisoned")
                .take();
            if let Some(StoreGate { entered, release }) = gate {
                let _ = entered.send(());
                let _ = release.await;
            }
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
            let mut entry = self
                .entry
                .clone()
                .ok_or_else(|| ModelsCacheError::new("cache not found"))?;
            entry.fetched_at = Utc::now();
            self.stored_entries
                .lock()
                .expect("stored entries lock should not be poisoned")
                .push(entry);
            Ok(())
        })
    }
}

#[derive(Debug)]
struct TestModelsEndpoint {
    models: Vec<ModelInfo>,
    fetch_count: AtomicUsize,
}

impl TestModelsEndpoint {
    fn new(models: Vec<ModelInfo>) -> Arc<Self> {
        Arc::new(Self {
            models,
            fetch_count: AtomicUsize::new(0),
        })
    }
}

impl ModelCatalogSource for TestModelsEndpoint {
    fn identity(&self) -> Option<String> {
        Some("test-provider".to_string())
    }

    fn has_command_auth(&self) -> bool {
        false
    }

    fn uses_managed_auth(&self) -> ModelCatalogFuture<'_, bool> {
        Box::pin(async { true })
    }

    fn list_models<'a>(
        &'a self,
        _client_version: &'a str,
        _http_client_factory: HttpClientFactory,
    ) -> ModelCatalogFuture<'a, CoreResult<ModelCatalogSnapshot>> {
        Box::pin(async move {
            self.fetch_count.fetch_add(1, Ordering::SeqCst);
            Ok(ModelCatalogSnapshot {
                catalog: ModelCatalog {
                    models: self.models.clone(),
                },
                etag: None,
                identity: self.identity().expect("test endpoint identity"),
            })
        })
    }
}

fn remote_model(slug: &str) -> ModelInfo {
    ModelInfo {
        visibility: ModelVisibility::List,
        used_fallback_model_metadata: false,
        ..model_info_from_slug(slug)
    }
}

fn models_manager(
    cache: Arc<dyn ModelsCache>,
    endpoint: Arc<TestModelsEndpoint>,
) -> SharedModelsManager {
    Arc::new(RemoteModelsManager::new_with_cache(
        cache,
        endpoint,
        Some(CredentialManager::from_auth_for_testing(GenexAuth::from_api_key(
            "test-api-key",
        ))),
    ))
}

async fn run_agent_with_model(models_manager: SharedModelsManager, model_slug: &str) -> Result<()> {
    let server = responses::start_mock_server().await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let mut builder = test_genex()
        .with_auth(GenexAuth::from_api_key("test-api-key"))
        .with_models_manager(models_manager);
    let test = builder.build(&server).await?;
    let available_models = test
        .thread_manager
        .get_models_manager()
        .list_models(
            RefreshStrategy::OnlineIfUncached,
            genex_core::test_support::default_http_client_factory(),
        )
        .await;
    assert!(
        available_models
            .iter()
            .any(|model| model.model == model_slug)
    );

    submit_thread_settings(
        &test.genex,
        ThreadSettingsOverrides {
            model: Some(model_slug.to_string()),
            ..Default::default()
        },
    )
    .await?;
    test.genex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "hello".to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;
    loop {
        if matches!(
            wait_for_event(&test.genex, |_| true).await,
            EventMsg::TurnComplete(_)
        ) {
            break;
        }
    }

    assert_eq!(
        response_mock.single_request().body_json()["model"],
        model_slug
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn injected_cache_hit_drives_agent_model_selection() -> Result<()> {
    let model_slug = "injected-cache-model";
    let cache = Arc::new(TestModelsCache {
        entry: Some(ModelsCacheEntry {
            identity: Some("test-provider".to_string()),
            fetched_at: Utc::now(),
            etag: None,
            client_version: Some(genex_models_manager::client_version_to_whole()),
            models: vec![remote_model(model_slug)],
        }),
        load_error: false,
        stored_entries: Mutex::new(Vec::new()),
        store_gate: Mutex::new(None),
    });
    let endpoint = TestModelsEndpoint::new(Vec::new());

    run_agent_with_model(models_manager(cache, endpoint.clone()), model_slug).await?;

    assert_eq!(endpoint.fetch_count.load(Ordering::SeqCst), 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn injected_cache_error_falls_back_for_agent_model_selection() -> Result<()> {
    let model_slug = "injected-cache-fallback-model";
    let cache = Arc::new(TestModelsCache {
        entry: None,
        load_error: true,
        stored_entries: Mutex::new(Vec::new()),
        store_gate: Mutex::new(None),
    });
    let endpoint = TestModelsEndpoint::new(vec![remote_model(model_slug)]);

    run_agent_with_model(models_manager(cache.clone(), endpoint.clone()), model_slug).await?;

    assert!(endpoint.fetch_count.load(Ordering::SeqCst) >= 1);
    assert!(
        !cache
            .stored_entries
            .lock()
            .expect("stored entries lock should not be poisoned")
            .is_empty()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_switch_during_cache_store_preserves_new_catalog_for_next_turn() -> Result<()> {
    let server = wiremock::MockServer::start().await;
    let home = Arc::new(TempDir::new()?);
    let initial_auth = GenexAuth::from_api_key("test-api-key");
    let auth = CredentialManager::from_auth_for_testing_with_home(
        initial_auth.clone(),
        home.path().to_path_buf(),
    );
    let mut provider_info = ModelProviderInfo::create_responses_provider(None);
    provider_info.base_url = Some(format!("{}/v1", server.uri()));
    provider_info.supports_websockets = false;
    let cache = Arc::new(TestModelsCache {
        entry: None,
        load_error: false,
        stored_entries: Mutex::new(Vec::new()),
        store_gate: Mutex::new(None),
    });
    let provider = create_model_provider(provider_info.clone(), Some(auth.clone()));
    let manager = build_models_manager_with_cache(
        cache.clone(),
        Arc::new(ProviderModelCatalogSource::new(provider.clone())),
        provider.credential_manager(),
        /*config_model_catalog*/ None,
    );
    let mut old_model = remote_model("legacy-model");
    old_model.default_reasoning_summary = ReasoningSummary::Concise;
    let initial = responses::mount_models_once(
        &server,
        ModelCatalog {
            models: vec![old_model.clone()],
        },
    )
    .await;
    let test = test_genex()
        .with_home(home.clone())
        .with_auth(initial_auth.clone())
        .with_model("legacy-model")
        .with_models_manager(manager.clone())
        .with_config(move |config| {
            config.model_provider = provider_info;
            config.model_reasoning_summary = None;
        })
        .build_with_auto_env(&server)
        .await?;
    assert_eq!(initial.requests().len(), 1);

    // Pause after the old catalog has been stored, before its refresh can publish it.
    let (store_entered, entered) = oneshot::channel();
    let (release_store, release) = oneshot::channel();
    *cache.store_gate.lock().unwrap() = Some(StoreGate {
        entered: store_entered,
        release,
    });
    let old_request = responses::mount_models_once(
        &server,
        ModelCatalog {
            models: vec![old_model],
        },
    )
    .await;
    let old_refresh = tokio::spawn({
        let manager = manager.clone();
        async move {
            manager
                .list_models(
                    RefreshStrategy::Online,
                    genex_core::test_support::default_http_client_factory(),
                )
                .await
        }
    });
    timeout(Duration::from_secs(10), entered).await??;

    std::fs::write(
        home.path().join("auth.json"),
        serde_json::to_vec(&serde_json::json!({
            "auth_mode": "apiKey", "PROVIDER_API_KEY": "second-account-token",
        }))?,
    )?;
    auth.reload().await;
    test.thread_manager.credential_manager().reload().await;

    let mut new_model = remote_model("legacy-model");
    new_model.default_reasoning_summary = ReasoningSummary::Detailed;
    let new_request = responses::mount_models_once(
        &server,
        ModelCatalog {
            models: vec![new_model.clone()],
        },
    )
    .await;
    manager
        .list_models(
            RefreshStrategy::Online,
            genex_core::test_support::default_http_client_factory(),
        )
        .await;
    assert_eq!(manager.get_remote_models().await, vec![new_model.clone()]);
    release_store.send(()).unwrap();
    timeout(Duration::from_secs(10), old_refresh).await??;
    assert_eq!(manager.get_remote_models().await, vec![new_model]);
    for (mock, token) in [
        (old_request, "test-api-key"),
        (new_request, "second-account-token"),
    ] {
        let requests = mock.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].headers["authorization"],
            format!("Bearer {token}")
        );
    }

    let response = mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("next-turn"),
            ev_completed("next-turn"),
        ]),
    )
    .await;
    test.submit_turn("use the new account's model defaults")
        .await?;
    let request = response.single_request();
    assert_eq!(
        request.header("authorization").as_deref(),
        Some("Bearer second-account-token")
    );
    assert_eq!(request.body_json()["reasoning"]["summary"], "detailed");
    Ok(())
}

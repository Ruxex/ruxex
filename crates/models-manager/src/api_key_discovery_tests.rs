//! API-key discovery rollout must gate network requests and cached catalog authority.

use super::*;
use pretty_assertions::assert_eq;

#[tokio::test]
async fn api_key_discovery_disabled_preserves_command_auth_discovery() {
    let models = vec![remote_model(
        "command-auth",
        "Command Auth",
        /*priority*/ 0,
    )];
    let endpoint = Arc::new(TestModelCatalogSource {
        has_command_auth: true,
        uses_managed_auth: false,
        responses: Mutex::new(vec![models.clone()].into()),
        etag: None,
        fetch_count: AtomicUsize::new(0),
        observed_proxy_policy: Mutex::new(None),
    });
    let manager = RemoteModelsManager::new_without_cache(
        endpoint.clone(),
        Some(CredentialManager::from_auth_for_testing(
            GenexAuth::from_api_key("test-key"),
        )),
    );
    assert_eq!(
        manager
            .raw_model_catalog(RefreshStrategy::Online, DEFAULT_HTTP_CLIENT_FACTORY)
            .await
            .models,
        models
    );
    assert_eq!(endpoint.fetch_count(), 1);
}

#[tokio::test]
async fn api_key_discovery_startup_flag_controls_fetches_and_cached_catalogs() {
    let home = tempdir().unwrap();
    let models = vec![remote_model("dynamic", "Dynamic", /*priority*/ 0)];
    let endpoint = TestModelCatalogSource::without_refresh(vec![models.clone()]);
    let auth = CredentialManager::from_auth_for_testing(GenexAuth::from_api_key("test-key"));
    let manager =
        RemoteModelsManager::new(home.path().into(), endpoint.clone(), Some(auth.clone()));

    for strategy in [
        RefreshStrategy::Online,
        RefreshStrategy::OnlineIfUncached,
        RefreshStrategy::Offline,
    ] {
        assert_eq!(
            manager
                .raw_model_catalog(strategy, DEFAULT_HTTP_CLIENT_FACTORY)
                .await
                .models,
            Vec::new()
        );
    }
    assert_eq!(endpoint.fetch_count(), 0);

    let manager =
        RemoteModelsManager::new(home.path().into(), endpoint.clone(), Some(auth.clone()));
    manager.set_api_key_model_discovery_enabled(/*enabled*/ true);
    assert_eq!(
        manager
            .raw_model_catalog(RefreshStrategy::Online, DEFAULT_HTTP_CLIENT_FACTORY)
            .await
            .models,
        models
    );
    assert_eq!(endpoint.fetch_count(), 1);

    // A new session must honor its startup flag even when a matching disk cache exists.
    for enabled in [false, true] {
        let restarted =
            RemoteModelsManager::new(home.path().into(), endpoint.clone(), Some(auth.clone()));
        restarted.set_api_key_model_discovery_enabled(enabled);
        let expected: &[ModelInfo] = if enabled { &models } else { &[] };
        for strategy in [RefreshStrategy::Offline, RefreshStrategy::OnlineIfUncached] {
            assert_eq!(
                &restarted
                    .raw_model_catalog(strategy, DEFAULT_HTTP_CLIENT_FACTORY)
                    .await
                    .models,
                expected
            );
        }
        assert_eq!(&restarted.try_get_remote_models().unwrap(), expected);
    }
    assert_eq!(endpoint.fetch_count(), 1);
}

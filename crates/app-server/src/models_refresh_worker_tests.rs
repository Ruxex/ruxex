use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Duration;

use ruxex_http_client::HttpClientFactory;
use ruxex_http_client::OutboundProxyPolicy;
use ruxex_models_manager::manager::ModelCatalogFuture;
use ruxex_models_manager::manager::ModelCatalogSnapshot;
use ruxex_models_manager::manager::ModelCatalogSource;
use ruxex_models_manager::manager::RemoteModelsManager;
use ruxex_models_manager::manager::SharedModelsManager;
use ruxex_protocol::error::RuxexErr;
use ruxex_protocol::error::Result as CoreResult;
use ruxex_protocol::model_types::ModelCatalog;
use pretty_assertions::assert_eq;
use tempfile::tempdir;
use tokio::sync::Notify;

use super::*;

#[derive(Debug)]
struct TestModelsEndpoint {
    fetch_count: AtomicUsize,
    fetched: Notify,
    release_second_fetch: Notify,
}

impl TestModelsEndpoint {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            fetch_count: AtomicUsize::new(0),
            fetched: Notify::new(),
            release_second_fetch: Notify::new(),
        })
    }

    async fn wait_for_fetch_count(&self, expected: usize) {
        tokio::time::timeout(Duration::from_secs(1), async {
            while self.fetch_count.load(Ordering::SeqCst) < expected {
                self.fetched.notified().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("expected {expected} model fetches"));
    }
}

impl ModelCatalogSource for TestModelsEndpoint {
    fn identity(&self) -> Option<String> {
        Some("test-provider".to_string())
    }

    fn has_command_auth(&self) -> bool {
        true
    }

    fn uses_managed_auth(&self) -> ModelCatalogFuture<'_, bool> {
        Box::pin(async { false })
    }

    fn list_models<'a>(
        &'a self,
        _client_version: &'a str,
        _http_client_factory: HttpClientFactory,
    ) -> ModelCatalogFuture<'a, CoreResult<ModelCatalogSnapshot>> {
        Box::pin(async move {
            let fetch_index = self.fetch_count.fetch_add(1, Ordering::SeqCst);
            self.fetched.notify_one();
            if fetch_index == 0 {
                return Err(RuxexErr::Io(std::io::Error::other("test failure")));
            }
            if fetch_index == 1 {
                self.release_second_fetch.notified().await;
            }
            Ok(ModelCatalogSnapshot {
                catalog: ModelCatalog { models: Vec::new() },
                etag: None,
                identity: self.identity().expect("test endpoint identity"),
            })
        })
    }
}

#[tokio::test]
async fn refreshes_immediately_periodically_and_stops_when_dropped() {
    let ruxex_home = tempdir().expect("temp dir");
    let endpoint = TestModelsEndpoint::new();
    let models_manager: SharedModelsManager = Arc::new(RemoteModelsManager::new(
        ruxex_home.path().to_path_buf(),
        endpoint.clone(),
    ));
    let worker = spawn_with_interval(
        &models_manager,
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        Duration::from_millis(10),
    );

    endpoint.wait_for_fetch_count(/*expected*/ 2).await;
    drop(worker);
    endpoint.release_second_fetch.notify_one();
    tokio::time::sleep(Duration::from_millis(30)).await;

    assert_eq!(endpoint.fetch_count.load(Ordering::SeqCst), 2);
}

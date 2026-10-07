use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use genex_api::ModelsClient;
use genex_api::RequestTelemetry;
use genex_api::ReqwestTransport;
use genex_api::TransportError;
use genex_api::auth_header_telemetry;
use genex_api::map_api_error;
use genex_http_client::ClientRouteClass;
use genex_http_client::HttpClientFactory;
use genex_login::AuthEnvTelemetry;
use genex_login::AuthManager;
use genex_login::GenexAuth;
use genex_login::collect_auth_env_telemetry;
use genex_login::default_client::create_client_for_route_async;
use genex_model_provider_info::ModelProviderInfo;
use genex_models_manager::manager::ModelCatalogFuture;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_models_manager::manager::ModelCatalogSource;
use genex_otel::TelemetryAuthMode;
use genex_protocol::error::GenexErr;
use genex_protocol::error::Result as CoreResult;
use genex_protocol::model_types::ModelCatalog;
use genex_response_debug_context::extract_response_debug_context;
use genex_response_debug_context::telemetry_transport_error_message;
use http::HeaderMap;
use tokio::time::timeout;

use crate::auth::resolve_provider_auth;

const MODELS_REFRESH_TIMEOUT: Duration = Duration::from_secs(5);
const MODELS_ENDPOINT: &str = "/models";

#[derive(Debug, Clone, Copy)]
pub(crate) enum ModelsEndpointAuthMode {
    ConfiguredProvider,
    MyProvider,
}

/// Provider-owned provider-compatible `/models` endpoint.
#[derive(Debug)]
pub(crate) struct ModelsEndpoint {
    provider_info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
    transport_builder: Arc<dyn ModelsTransportBuilder>,
    auth_mode: ModelsEndpointAuthMode,
}

impl ModelsEndpoint {
    pub(crate) fn new(
        provider_info: ModelProviderInfo,
        auth_manager: Option<Arc<AuthManager>>,
    ) -> Self {
        Self::new_with_auth_mode(
            provider_info,
            auth_manager,
            ModelsEndpointAuthMode::ConfiguredProvider,
        )
    }

    pub(crate) fn new_with_auth_mode(
        provider_info: ModelProviderInfo,
        auth_manager: Option<Arc<AuthManager>>,
        auth_mode: ModelsEndpointAuthMode,
    ) -> Self {
        Self {
            provider_info,
            auth_manager,
            transport_builder: Arc::new(RouteAwareModelsTransportBuilder),
            auth_mode,
        }
    }

    async fn auth(&self) -> Option<GenexAuth> {
        match self.auth_manager.as_ref() {
            Some(auth_manager) => auth_manager.auth().await,
            None => None,
        }
    }

    pub(crate) async fn list_models(
        &self,
        client_version: &str,
        http_client_factory: HttpClientFactory,
    ) -> CoreResult<ModelCatalogSnapshot> {
        let _timer =
            genex_otel::start_global_timer("genex.remote_models.fetch_update.duration_ms", &[]);
        let auth = self.auth().await;
        let identity = crate::models_identity::identity(&self.provider_info, auth.as_ref())?;
        let auth_mode = auth.as_ref().map(GenexAuth::auth_mode);
        let api_provider = self.provider_info.to_api_provider(auth_mode)?;
        let api_auth = match self.auth_mode {
            ModelsEndpointAuthMode::ConfiguredProvider => {
                resolve_provider_auth(auth.as_ref(), &self.provider_info)?
            }
            ModelsEndpointAuthMode::MyProvider => {
                crate::my_provider::my_provider_auth_provider(auth.as_ref())
            }
        };
        let request_url =
            ModelsClient::<ReqwestTransport>::request_url(&api_provider, client_version);
        let auth_telemetry = auth_header_telemetry(api_auth.as_ref());
        let request_telemetry: Arc<dyn RequestTelemetry> = Arc::new(ModelsRequestTelemetry {
            auth_mode: auth_mode.map(|mode| TelemetryAuthMode::from(mode).to_string()),
            auth_header_attached: auth_telemetry.attached,
            auth_header_name: auth_telemetry.name,
            auth_env: self.auth_env(),
        });
        let (models, etag) = timeout(MODELS_REFRESH_TIMEOUT, async {
            let transport = self
                .transport_builder
                .build(http_client_factory, request_url.clone())
                .await?;
            let client = ModelsClient::new(transport, api_provider, api_auth)
                .with_telemetry(Some(request_telemetry));
            client
                .list_models(request_url, HeaderMap::new())
                .await
                .map_err(map_api_error)
        })
        .await
        .map_err(|_| GenexErr::RequestTimeout)??;
        Ok(ModelCatalogSnapshot {
            catalog: ModelCatalog { models },
            identity,
            etag,
        })
    }

    fn auth_env(&self) -> AuthEnvTelemetry {
        let api_key_env_enabled = self
            .auth_manager
            .as_ref()
            .is_some_and(|auth_manager| auth_manager.api_key_env_enabled());
        collect_auth_env_telemetry(&self.provider_info, api_key_env_enabled)
    }
}

impl ModelCatalogSource for ModelsEndpoint {
    fn supports_api_key_models(&self) -> bool {
        self.provider_info.env_key.is_some()
            || matches!(self.auth_mode, ModelsEndpointAuthMode::MyProvider)
    }

    fn identity(&self) -> Option<String> {
        let auth = self
            .auth_manager
            .as_ref()
            .and_then(|manager| manager.auth_cached());
        crate::models_identity::identity(&self.provider_info, auth.as_ref()).ok()
    }

    fn has_command_auth(&self) -> bool {
        self.provider_info.has_command_auth()
    }

    fn uses_managed_auth(&self) -> ModelCatalogFuture<'_, bool> {
        Box::pin(async { false })
    }

    fn supports_remote_models(&self) -> bool {
        true
    }

    fn list_models<'a>(
        &'a self,
        client_version: &'a str,
        http_client_factory: HttpClientFactory,
    ) -> ModelCatalogFuture<'a, CoreResult<ModelCatalogSnapshot>> {
        Box::pin(ModelsEndpoint::list_models(
            self,
            client_version,
            http_client_factory,
        ))
    }
}

type ModelsTransportFuture<'a> =
    Pin<Box<dyn Future<Output = std::io::Result<ReqwestTransport>> + Send + 'a>>;

/// Builds the concrete transport selected for one models request.
///
/// Implementations must honor the supplied request-time client factory and exact request URL.
trait ModelsTransportBuilder: fmt::Debug + Send + Sync {
    fn build(
        &self,
        http_client_factory: HttpClientFactory,
        request_url: String,
    ) -> ModelsTransportFuture<'_>;
}

#[derive(Debug)]
struct RouteAwareModelsTransportBuilder;

impl ModelsTransportBuilder for RouteAwareModelsTransportBuilder {
    fn build(
        &self,
        http_client_factory: HttpClientFactory,
        request_url: String,
    ) -> ModelsTransportFuture<'_> {
        Box::pin(async move {
            create_client_for_route_async(http_client_factory, request_url, ClientRouteClass::Api)
                .await
                .map(ReqwestTransport::from_http_client)
        })
    }
}

#[derive(Clone)]
struct ModelsRequestTelemetry {
    auth_mode: Option<String>,
    auth_header_attached: bool,
    auth_header_name: Option<&'static str>,
    auth_env: AuthEnvTelemetry,
}

impl RequestTelemetry for ModelsRequestTelemetry {
    fn on_request(
        &self,
        attempt: u64,
        status: Option<http::StatusCode>,
        error: Option<&TransportError>,
        duration: Duration,
    ) {
        let success = status.is_some_and(|code| code.is_success()) && error.is_none();
        let error_message = error.map(telemetry_transport_error_message);
        let response_debug = error
            .map(extract_response_debug_context)
            .unwrap_or_default();
        let status = status.map(|status| status.as_u16());
        tracing::event!(
            target: "genex_otel.log_only",
            tracing::Level::INFO,
            event.name = "genex.api_request",
            duration_ms = %duration.as_millis(),
            http.response.status_code = status,
            success = success,
            error.message = error_message.as_deref(),
            attempt = attempt,
            endpoint = MODELS_ENDPOINT,
            auth.header_attached = self.auth_header_attached,
            auth.header_name = self.auth_header_name,
            auth.env_genex_api_key_present = self.auth_env.api_key_env_present,
            auth.env_genex_api_key_enabled = self.auth_env.api_key_env_enabled,
            auth.env_provider_key_name = self.auth_env.provider_env_key_name.as_deref(),
            auth.env_provider_key_present = self.auth_env.provider_env_key_present,
            auth.request_id = response_debug.request_id.as_deref(),
            auth.cf_ray = response_debug.cf_ray.as_deref(),
            auth.mode = self.auth_mode.as_deref(),
        );
        tracing::event!(
            target: "genex_otel.trace_safe",
            tracing::Level::INFO,
            event.name = "genex.api_request",
            duration_ms = %duration.as_millis(),
            http.response.status_code = status,
            success = success,
            error.message = error_message.as_deref(),
            attempt = attempt,
            endpoint = MODELS_ENDPOINT,
            auth.header_attached = self.auth_header_attached,
            auth.header_name = self.auth_header_name,
            auth.env_genex_api_key_present = self.auth_env.api_key_env_present,
            auth.env_genex_api_key_enabled = self.auth_env.api_key_env_enabled,
            auth.env_provider_key_name = self.auth_env.provider_env_key_name.as_deref(),
            auth.env_provider_key_present = self.auth_env.provider_env_key_present,
            auth.request_id = response_debug.request_id.as_deref(),
            auth.cf_ray = response_debug.cf_ray.as_deref(),
            auth.mode = self.auth_mode.as_deref(),
        );
    }
}

#[cfg(test)]
#[path = "models_endpoint_timeout_tests.rs"]
mod timeout_tests;

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;
    use std::sync::Mutex;

    use super::*;
    use genex_http_client::OutboundProxyPolicy;
    use genex_login::default_client::RESIDENCY_HEADER_NAME;
    use genex_login::default_client::ResidencyRequirement;
    use genex_login::default_client::create_client;
    use genex_login::default_client::set_default_client_residency_requirement;
    use genex_protocol::config_types::ModelProviderAuthInfo;
    use genex_protocol::model_types::ModelCatalog;
    use pretty_assertions::assert_eq;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::header;
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::matchers::query_param;

    #[derive(Debug)]
    struct RecordingTransportBuilder {
        observed_request: Arc<Mutex<Option<(OutboundProxyPolicy, String)>>>,
    }

    impl ModelsTransportBuilder for RecordingTransportBuilder {
        fn build(
            &self,
            http_client_factory: HttpClientFactory,
            request_url: String,
        ) -> ModelsTransportFuture<'_> {
            let observed_request = Arc::clone(&self.observed_request);
            Box::pin(async move {
                *observed_request
                    .lock()
                    .expect("observed request lock should not be poisoned") =
                    Some((http_client_factory.outbound_proxy_policy(), request_url));
                Ok(ReqwestTransport::from_http_client(create_client()))
            })
        }
    }

    fn provider_info_with_command_auth() -> ModelProviderInfo {
        ModelProviderInfo {
            auth: Some(ModelProviderAuthInfo {
                command: "print-token".to_string(),
                args: Vec::new(),
                timeout_ms: NonZeroU64::new(5_000).expect("timeout should be non-zero"),
                refresh_interval_ms: 300_000,
                cwd: std::env::current_dir()
                    .expect("current dir should be available")
                    .try_into()
                    .expect("current dir should be absolute"),
            }),
            requires_auth: false,
            ..ModelProviderInfo::create_responses_provider(/*base_url*/ None)
        }
    }

    #[test]
    fn command_auth_provider_reports_command_auth_without_cached_auth() {
        let endpoint = ModelsEndpoint::new(
            provider_info_with_command_auth(),
            /*auth_manager*/ None,
        );

        assert!(endpoint.has_command_auth());
    }

    #[test]
    fn provider_without_command_auth_reports_no_command_auth() {
        let endpoint = ModelsEndpoint::new(
            ModelProviderInfo::create_responses_provider(/*base_url*/ None),
            /*auth_manager*/ None,
        );

        assert!(!endpoint.has_command_auth());
    }

    #[tokio::test]
    async fn model_request_uses_request_time_proxy_policy_and_exact_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .and(query_param("client_version", "0.0.0"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ModelCatalog { models: Vec::new() }),
            )
            .expect(1)
            .mount(&server)
            .await;

        let observed_request = Arc::new(Mutex::new(None));
        let endpoint = ModelsEndpoint {
            provider_info: ModelProviderInfo::create_responses_provider(Some(server.uri())),
            auth_manager: None,
            transport_builder: Arc::new(RecordingTransportBuilder {
                observed_request: Arc::clone(&observed_request),
            }),
            auth_mode: ModelsEndpointAuthMode::ConfiguredProvider,
        };

        endpoint
            .list_models(
                "0.0.0",
                HttpClientFactory::new(OutboundProxyPolicy::RespectSystemProxy),
            )
            .await
            .expect("models request should succeed");

        assert_eq!(
            *observed_request
                .lock()
                .expect("observed request lock should not be poisoned"),
            Some((
                OutboundProxyPolicy::RespectSystemProxy,
                format!("{}/models?client_version=0.0.0", server.uri()),
            ))
        );
    }

    #[tokio::test]
    async fn model_discovery_enforces_managed_residency_over_provider_headers() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/models"))
            .and(header(RESIDENCY_HEADER_NAME, "us"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(ModelCatalog { models: Vec::new() }),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut provider_info = ModelProviderInfo::create_responses_provider(Some(server.uri()));
        provider_info.http_headers = Some(std::collections::HashMap::from([(
            RESIDENCY_HEADER_NAME.to_string(),
            "eu".into(),
        )]));
        let endpoint = ModelsEndpoint {
            provider_info,
            auth_manager: None,
            transport_builder: Arc::new(RecordingTransportBuilder {
                observed_request: Arc::new(Mutex::new(None)),
            }),
            auth_mode: ModelsEndpointAuthMode::ConfiguredProvider,
        };

        set_default_client_residency_requirement(Some(ResidencyRequirement::Us));
        endpoint
            .list_models(
                "0.0.0",
                HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            )
            .await
            .expect("managed residency model discovery should succeed");
        set_default_client_residency_requirement(/*enforce_residency*/ None);

        assert_eq!(
            endpoint
                .provider_info
                .http_headers
                .as_ref()
                .and_then(|headers| headers.get(RESIDENCY_HEADER_NAME)),
            Some(&"eu".into())
        );
    }

    #[derive(Debug)]
    struct RotatingAuth(std::sync::atomic::AtomicUsize);

    impl genex_login::ExternalAuth for RotatingAuth {
        fn resolve(&self) -> genex_login::ExternalAuthFuture<'_, GenexAuth> {
            Box::pin(async move {
                let generation = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(GenexAuth::from_api_key(&format!("token-{generation}")))
            })
        }

        fn refresh(
            &self,
            _context: genex_login::ExternalAuthRefreshContext,
        ) -> genex_login::ExternalAuthFuture<'_, GenexAuth> {
            self.resolve()
        }
    }

    #[tokio::test]
    async fn command_auth_refresh_fetches_a_catalog_for_the_current_credentials() {
        use genex_models_manager::manager::ModelsManager;
        use genex_models_manager::manager::RefreshStrategy;
        use genex_models_manager::manager::RemoteModelsManager;

        let server = MockServer::start().await;
        let auth = AuthManager::from_auth_for_testing(GenexAuth::from_api_key("initial"));
        auth.set_external_auth(Arc::new(RotatingAuth(std::sync::atomic::AtomicUsize::new(
            0,
        ))))
        .await
        .unwrap();
        let model = genex_protocol::model_types::ModelInfo {
            used_fallback_model_metadata: false,
            ..genex_models_manager::model_info::model_info_from_slug("command-auth-model")
        };
        Mock::given(method("GET"))
            .and(path("/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ModelCatalog {
                models: vec![model.clone()],
            }))
            .expect(2)
            .mount(&server)
            .await;
        let mut provider = provider_info_with_command_auth();
        provider.base_url = Some(server.uri());
        // Keep this test independent of the residency override exercised in parallel.
        provider.http_headers = Some(std::collections::HashMap::from([(
            RESIDENCY_HEADER_NAME.to_string(),
            "us".into(),
        )]));
        let manager = RemoteModelsManager::new_without_cache(
            Arc::new(ModelsEndpoint::new(provider, Some(auth.clone()))),
            Some(auth.clone()),
        );
        let catalog = manager
            .raw_model_catalog(
                RefreshStrategy::OnlineIfUncached,
                HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
            )
            .await;
        assert_eq!(
            catalog
                .models
                .iter()
                .find(|candidate| candidate.slug == model.slug),
            Some(&model)
        );
        auth.auth().await;
        assert_eq!(manager.get_remote_models().await, Vec::new());
        assert_eq!(manager.try_get_remote_models().unwrap(), Vec::new());
        assert_eq!(
            manager
                .raw_model_catalog(
                    RefreshStrategy::Offline,
                    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
                )
                .await,
            ModelCatalog { models: Vec::new() }
        );
        assert_eq!(
            manager
                .raw_model_catalog(
                    RefreshStrategy::OnlineIfUncached,
                    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
                )
                .await,
            catalog
        );
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| request.headers["authorization"].to_str().unwrap())
                .collect::<Vec<_>>(),
            vec!["Bearer token-1", "Bearer token-3"]
        );
    }
}

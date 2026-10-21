use std::sync::Arc;

use ruxex::provider::auth::CredentialManager;
use ruxex::provider::config::{ModelProviderInfo, WireApi};
use ruxex::provider::protocol::{
    ModelCatalog, ModelVerification, ProviderAccount, ProviderAuthStatus, RateLimitSnapshot,
    ReasoningEffort, ReasoningSummary, ResponseItem, ResponseUsageMetadata, SessionSource,
    TokenUsage,
};
use ruxex::provider::transport::{
    ClientRouteClass, HttpClientFactory, HttpTransport, ReqwestTransport,
};
use ruxex::provider::{ModelProvider, ModelProviderFactory, ProviderInit};

use ruxex::api::{
    ApiError, AuthProvider, ChatClient, ChatRequest, ChatRequestBuilder, ChatRequestInput,
    ChatUsageDecoder, Compression, ModelsClient, Provider, RequestTelemetry,
    ReqwestTransport as ApiReqwestTransport, ResponseEvent, ResponseStream, ResponsesApiRequest,
    ResponsesClient, RetryConfig, SearchClient, SearchRequest, SharedAuthProvider,
    StandardChatCompletionApi,
};
use ruxex::arg0::{Arg0DispatchPaths, Arg0PathEntryGuard, arg0_dispatch};
use ruxex::catalog::{
    ModelCatalogAuthContext, ModelCatalogFuture, ModelCatalogSnapshot, ModelCatalogSource,
    ModelsManager, ModelsManagerFuture, ProviderModelCatalog, ProviderModelCatalogAuth,
    ProviderModelCatalogSource, RefreshStrategy, SharedModelsManager,
};
use ruxex::client::{
    ClientRedirectPolicy, Originator, create_client, create_client_for_route,
    create_client_for_route_async, create_client_without_request_logging, default_headers,
    get_user_agent, originator,
};
use ruxex::home::{
    DEFAULT_RUXEX_HOME_DIR_NAME, DEFAULT_RUXEX_HOME_MARKER_FILE, ProductIdentity,
    RuxexHomeUserInstructionsProvider, find_product_home, find_ruxex_home,
};

fn accepts_provider<P: ModelProvider>() {}

fn accepts_provider_factory<P: ModelProviderFactory>() {}

fn accepts_http_transport<T: HttpTransport>() {}

#[test]
fn provider_facade_exposes_the_provider_author_surface() {
    accepts_provider::<TestProvider>();
    accepts_provider_factory::<TestProvider>();

    let _ = ModelProviderInfo {
        wire_api: WireApi::Responses,
        ..ModelProviderInfo::default()
    };
    let _: fn(ProviderInit) = accepts_provider_init;

    let _: Option<Arc<CredentialManager>> = None;
    let _: Option<HttpClientFactory> = None;
    let _: Option<ClientRouteClass> = None;
    accepts_http_transport::<ReqwestTransport>();

    let _: Option<ResponseItem> = None;
    let _: Option<ModelCatalog> = None;
    let _: Option<ProviderAccount> = None;
    let _: Option<ProviderAuthStatus> = None;
    let _: Option<RateLimitSnapshot> = None;
    let _: Option<ReasoningEffort> = None;
    let _: Option<ReasoningSummary> = None;
    let _: Option<ResponseUsageMetadata> = None;
    let _: Option<SessionSource> = None;
    let _: Option<TokenUsage> = None;
    let _: Option<ModelVerification> = None;
}

#[test]
fn facade_exposes_host_integration_and_catalog_surfaces() {
    let _: Option<Arg0DispatchPaths> = None;
    let _: Option<Arg0PathEntryGuard> = None;
    let _ = arg0_dispatch;

    let _: Option<Originator> = None;
    let _: Option<ClientRedirectPolicy> = None;
    let _ = (
        create_client,
        create_client_for_route,
        create_client_for_route_async,
        create_client_without_request_logging,
        default_headers,
        get_user_agent,
        originator,
    );

    let _: Option<ProductIdentity> = None;
    let _: Option<RuxexHomeUserInstructionsProvider> = None;
    let _: &str = DEFAULT_RUXEX_HOME_DIR_NAME;
    let _: &str = DEFAULT_RUXEX_HOME_MARKER_FILE;
    let _ = (find_product_home, find_ruxex_home);

    let _: Option<ModelCatalogAuthContext> = None;
    let _: Option<ModelCatalogFuture<'static, ()>> = None;
    let _: Option<ModelCatalogSnapshot> = None;
    let _: Option<&dyn ModelCatalogSource> = None;
    let _: Option<ModelsManagerFuture<'static, ()>> = None;
    let _: Option<&dyn ModelsManager> = None;
    let _: Option<ProviderModelCatalog> = None;
    let _: Option<ProviderModelCatalogAuth> = None;
    let _: Option<ProviderModelCatalogSource> = None;
    let _: Option<RefreshStrategy> = None;
    let _: Option<SharedModelsManager> = None;
}

#[test]
fn facade_exposes_api_client_and_protocol_surfaces() {
    let _: Option<ApiError> = None;
    let _: Option<ChatRequest> = None;
    let _: Option<ChatRequestBuilder<'static>> = None;
    let _: Option<ChatRequestInput<'static>> = None;
    let _: Option<&dyn ChatUsageDecoder> = None;
    let _: Option<Compression> = None;
    let _: Option<ChatClient<ApiReqwestTransport>> = None;
    let _: Option<Provider> = None;
    let _: Option<ResponsesApiRequest> = None;
    let _: Option<ResponsesClient<ApiReqwestTransport>> = None;
    let _: Option<ModelsClient<ApiReqwestTransport>> = None;
    let _: Option<SearchClient<ApiReqwestTransport>> = None;
    let _: Option<ResponseEvent> = None;
    let _: Option<ResponseStream> = None;
    let _: Option<RetryConfig> = None;
    let _: Option<SearchRequest> = None;
    let _: Option<SharedAuthProvider> = None;
    let _: Option<StandardChatCompletionApi> = None;
    let _: Option<&dyn AuthProvider> = None;
    let _: Option<&dyn RequestTelemetry> = None;
}

fn accepts_provider_init(_: ProviderInit) {}

#[derive(Debug)]
struct TestProvider;

impl ModelProvider for TestProvider {
    fn descriptor(&self) -> ruxex::provider::contract::ProviderDescriptor {
        ruxex::provider::contract::ProviderDescriptor::new("test", "1")
    }

    fn capabilities(&self) -> ruxex::provider::contract::ProviderCapabilities {
        ruxex::provider::contract::ProviderCapabilities::default()
    }

    fn chat(
        &self,
        _request: ruxex::provider::contract::ChatRequest,
    ) -> ruxex::provider::contract::ProviderFuture<
        '_,
        ruxex::provider::contract::ProviderResult<ruxex::provider::contract::ProviderChatStream>,
    > {
        Box::pin(async { Err(ruxex::provider::contract::ProviderError::Unsupported) })
    }

    fn list_models(
        &self,
        _request: ruxex::provider::contract::ListModelsRequest,
    ) -> ruxex::provider::contract::ProviderFuture<
        '_,
        ruxex::provider::contract::ProviderResult<ruxex::provider::contract::ProviderModelCatalog>,
    > {
        Box::pin(async { Err(ruxex::provider::contract::ProviderError::Unsupported) })
    }

    fn auth_state(&self) -> ruxex::provider::contract::ProviderAuthState {
        ruxex::provider::contract::ProviderAuthState {
            status: ProviderAuthStatus::Unauthenticated,
            account_id: None,
            display_name: None,
            managed_auth: false,
            revision: None,
        }
    }

    fn request_snapshot(
        &self,
    ) -> ruxex::provider::contract::ProviderFuture<
        '_,
        ruxex::provider::contract::ProviderResult<
            ruxex::provider::contract::ProviderRequestSnapshot,
        >,
    > {
        Box::pin(async { Err(ruxex::provider::contract::ProviderError::Unsupported) })
    }

    fn auth_fields(&self) -> Vec<ruxex::provider::contract::ProviderAuthField> {
        Vec::new()
    }

    fn account_state(
        &self,
    ) -> ruxex::provider::contract::ProviderResult<ruxex::provider::contract::ProviderAccountState>
    {
        Err(ruxex::provider::contract::ProviderError::Unsupported)
    }

    fn login(
        &self,
        _credentials: std::collections::BTreeMap<String, String>,
    ) -> ruxex::provider::contract::ProviderFuture<'_, ruxex::provider::contract::ProviderResult<()>>
    {
        Box::pin(async { Err(ruxex::provider::contract::ProviderError::Unsupported) })
    }

    fn logout(
        &self,
    ) -> ruxex::provider::contract::ProviderFuture<'_, ruxex::provider::contract::ProviderResult<()>>
    {
        Box::pin(async { Err(ruxex::provider::contract::ProviderError::Unsupported) })
    }

    fn delete(
        &self,
    ) -> ruxex::provider::contract::ProviderFuture<
        '_,
        ruxex::provider::contract::ProviderResult<bool>,
    > {
        Box::pin(async { Err(ruxex::provider::contract::ProviderError::Unsupported) })
    }

    fn usage(
        &self,
        _request: ruxex::provider::contract::UsageRequest,
    ) -> ruxex::provider::contract::ProviderFuture<
        '_,
        ruxex::provider::contract::ProviderResult<ruxex::provider::contract::ProviderUsage>,
    > {
        Box::pin(async { Err(ruxex::provider::contract::ProviderError::Unsupported) })
    }
}

impl ModelProviderFactory for TestProvider {
    fn provider_descriptor() -> ruxex::provider::contract::ProviderDescriptor {
        ruxex::provider::contract::ProviderDescriptor::new("test", "1")
    }

    fn new(_: ProviderInit) -> Self {
        Self
    }
}

mod auth;
mod bearer_auth_provider;
mod contract;
mod factory;
mod home_marker;
mod model_catalog_source;
mod models_endpoint;
mod models_identity;
pub mod test_support;

/// Effective destination and credentials that permit reusing a Responses socket.
#[derive(Debug, PartialEq, Eq)]
pub struct ResponsesConnectionKey {
    base_url: String,
    auth_revision: Option<u64>,
}

impl ResponsesConnectionKey {
    pub fn new(provider: &ruxex_api::Provider, auth_revision: Option<u64>) -> Self {
        Self {
            base_url: provider.base_url.clone(),
            auth_revision,
        }
    }
}

pub use auth::auth_provider_from_auth;
pub use auth::auth_provider_from_credential_manager;
#[doc(hidden)]
pub use auth::cached_provider_auth;
#[doc(hidden)]
pub use auth::credential_manager_for_provider;
#[doc(hidden)]
pub use auth::provider_auth;
#[doc(hidden)]
pub use auth::resolve_provider_auth;
pub use auth::resolve_provider_auth_for_provider;
pub use auth::unauthenticated_auth_provider;
pub use bearer_auth_provider::BearerAuthProvider;
pub use bearer_auth_provider::BearerAuthProvider as CoreAuthProvider;
pub use contract::CURRENT_PROVIDER_PROTOCOL_VERSION;
pub use contract::ChatRequest;
pub use contract::ChatTransportOptions;
pub use contract::ListModelsRequest;
pub use contract::ModelProvider;
pub use contract::ProviderAccountState;
pub use contract::ProviderAuthEnvironment;
pub use contract::ProviderAuthField;
pub use contract::ProviderAuthRecoveryMessages;
pub use contract::ProviderAuthState;
pub use contract::ProviderBackendRequest;
pub use contract::ProviderBackendResponse;
pub use contract::ProviderCapabilities;
pub use contract::ProviderChatEvent;
pub use contract::ProviderChatStream;
pub use contract::ProviderConnectionKey;
pub use contract::ProviderCredentialSnapshot;
pub use contract::ProviderDescriptor;
pub use contract::ProviderError;
pub use contract::ProviderFuture;
pub use contract::ProviderModelCatalog;
pub use contract::ProviderModelCatalogAuth;
pub use contract::ProviderReasoning;
pub use contract::ProviderReasoningContext;
pub use contract::ProviderReasoningSummaryDelivery;
pub use contract::ProviderRequestAuth;
pub use contract::ProviderRequestCompression;
pub use contract::ProviderRequestFormat;
pub use contract::ProviderRequestSnapshot;
pub use contract::ProviderResult;
pub use contract::ProviderSafetyBuffering;
pub use contract::ProviderSearchRequest;
pub use contract::ProviderSearchResponse;
pub use contract::ProviderStreamConnectRequest;
pub use contract::ProviderStreamContinuation;
pub use contract::ProviderStreamEvent;
pub use contract::ProviderStreamOptions;
pub use contract::ProviderStreamOutcome;
pub use contract::ProviderStreamRequest;
pub use contract::ProviderStreamSession;
pub use contract::ProviderStreamTelemetry;
pub use contract::ProviderTextControls;
pub use contract::ProviderTextFormat;
pub use contract::ProviderTextFormatType;
pub use contract::ProviderUnauthorizedRecovery;
pub use contract::ProviderUsage;
pub use contract::ProviderUsageResult;
#[doc(hidden)]
pub use contract::ProviderWebsocketConnectRequest;
#[doc(hidden)]
pub use contract::ProviderWebsocketConnection;
#[doc(hidden)]
pub use contract::ProviderWebsocketEvent;
#[doc(hidden)]
pub use contract::ProviderWebsocketPayloadRequest;
#[doc(hidden)]
pub use contract::ProviderWebsocketRequest;
#[doc(hidden)]
pub use contract::ProviderWebsocketTelemetry;
#[doc(hidden)]
pub use contract::ProviderWireApi;
pub use contract::RemoteCompactionSupport;
pub use contract::SharedModelProvider;
pub use contract::SharedProviderCredentialSnapshot;
pub use contract::SharedProviderStreamSession;
#[doc(hidden)]
pub use contract::SharedProviderWebsocketConnection;
pub use contract::UsageRequest;
pub use contract::build_provider_text_controls;
#[doc(hidden)]
pub use contract::headers_to_http;
pub use factory::ModelProviderFactory;
pub use factory::ProviderInit;
pub use home_marker::PROVIDER_HOME_FORMAT_VERSION;
pub use home_marker::PROVIDER_HOME_MARKER_FILE;
pub use home_marker::ProviderHomeError;
pub use home_marker::ProviderHomeMarker;
pub use home_marker::mount_provider_home;
pub use model_catalog_source::ProviderModelCatalogSource;
#[doc(hidden)]
pub use models_endpoint::ModelsEndpoint;
pub use ruxex_protocol::account::ProviderAccount;
#[doc(hidden)]
pub mod provider_models_identity {
    pub use crate::models_identity::identity;
}

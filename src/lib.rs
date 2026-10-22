//! Public entry point for building provider-neutral Ruxex applications.
//!
//! The implementation remains split across the workspace crates, while this
//! crate presents the main host integration points from one dependency.

#[cfg(feature = "app-server")]
pub use ruxex_app_server::runtime::AppServerRuntimeBuilder;
#[cfg(feature = "cli")]
pub use ruxex_cli::ProviderRuntimeFactory;
#[cfg(feature = "cli")]
pub use ruxex_cli::run;
#[cfg(feature = "cli")]
pub use ruxex_cli::run_with_provider_factory;
#[cfg(feature = "cli")]
pub use ruxex_cli::run_with_provider_factory_and_identity;
#[cfg(feature = "cli")]
pub use ruxex_cli::run_with_provider_factory_and_identity_and_resolvers;
#[cfg(feature = "cli")]
pub use ruxex_cli::run_with_provider_factory_and_resolvers;
pub use ruxex_model_provider::ModelProvider;
pub use ruxex_model_provider::ModelProviderFactory;
pub use ruxex_model_provider::ProviderInit;
pub use ruxex_model_provider::SharedModelProvider;
#[cfg(feature = "tui")]
pub use ruxex_tui::TuiBrandProfile;
pub use ruxex_utils_home_dir::ProductIdentity;

/// Arg0 dispatch and helper executable setup APIs.
pub mod arg0 {
    pub use ruxex_arg0::*;
}

/// Provider-facing HTTP API clients, request types, and protocol values.
pub mod api {
    pub use ruxex_api::*;
}

/// Model catalog contracts and manager-facing catalog APIs.
pub mod catalog {
    pub use ruxex_model_provider::ProviderModelCatalog;
    pub use ruxex_model_provider::ProviderModelCatalogAuth;
    pub use ruxex_model_provider::ProviderModelCatalogSource;
    pub use ruxex_models_manager::manager::ModelCatalogAuthContext;
    pub use ruxex_models_manager::manager::ModelCatalogFuture;
    pub use ruxex_models_manager::manager::ModelCatalogSnapshot;
    pub use ruxex_models_manager::manager::ModelCatalogSource;
    pub use ruxex_models_manager::manager::ModelsManager;
    pub use ruxex_models_manager::manager::ModelsManagerFuture;
    pub use ruxex_models_manager::manager::RefreshStrategy;
    pub use ruxex_models_manager::manager::SharedModelsManager;
    pub use ruxex_models_manager::model_info::model_info_template_from_slug;
}

/// Ruxex HTTP client construction and request metadata APIs.
pub mod client {
    pub use ruxex_login::default_client::*;
}

/// Ruxex home-directory and global user-instructions APIs.
pub mod home {
    pub use ruxex_home::*;
    pub use ruxex_utils_home_dir::DEFAULT_RUXEX_HOME_DIR_NAME;
    pub use ruxex_utils_home_dir::DEFAULT_RUXEX_HOME_MARKER_FILE;
    pub use ruxex_utils_home_dir::ProductIdentity;
    pub use ruxex_utils_home_dir::find_product_home;
    pub use ruxex_utils_home_dir::find_ruxex_home;
}

/// Provider implementation APIs grouped by responsibility.
pub mod provider {
    /// Client-side typed credential discovery contracts.
    pub mod credentials {
        pub use ruxex_provider_auth::*;
    }

    /// Provider-neutral contract types implemented by provider adapters.
    pub mod contract {
        pub use ruxex_model_provider::*;
    }

    pub use contract::*;

    /// Provider configuration types passed through [`super::contract::ProviderInit`].
    pub mod config {
        pub use ruxex_model_provider_info::ModelProviderInfo;
        pub use ruxex_model_provider_info::WireApi;
        pub use ruxex_protocol::config_types::ModelProviderAuthInfo;
    }

    /// Host-managed authentication types available to a provider adapter.
    pub mod auth {
        pub use ruxex_login::CredentialManager;
        pub use ruxex_login::ProviderAuth;
        pub use ruxex_login::RuxexAuth;
    }

    /// Ruxex HTTP routing and transport types for provider-owned requests.
    pub mod transport {
        pub use ruxex_http_client::ClientRouteClass;
        pub use ruxex_http_client::HttpClientFactory;
        pub use ruxex_http_client::HttpTransport;
        pub use ruxex_http_client::OutboundProxyPolicy;
        pub use ruxex_http_client::ReqwestTransport;
    }

    /// Provider-neutral protocol values used by the contract request and event types.
    pub mod protocol {
        pub use ruxex_protocol::ResponseUsageMetadata;
        pub use ruxex_protocol::account::ProviderAccount;
        pub use ruxex_protocol::account::ProviderAuthStatus;
        pub use ruxex_protocol::config_types::ReasoningSummary;
        pub use ruxex_protocol::config_types::Verbosity;
        pub use ruxex_protocol::model_types::ModelCatalog;
        pub use ruxex_protocol::model_types::ModelInfo;
        pub use ruxex_protocol::model_types::ModelVisibility;
        pub use ruxex_protocol::model_types::ReasoningEffort;
        pub use ruxex_protocol::models::ResponseItem;
        pub use ruxex_protocol::protocol::ModelVerification;
        pub use ruxex_protocol::protocol::RateLimitSnapshot;
        pub use ruxex_protocol::protocol::SessionSource;
        pub use ruxex_protocol::protocol::TokenUsage;
    }
}

/// Advanced App Server APIs.
#[cfg(feature = "app-server")]
pub mod app_server {
    pub use ruxex_app_server::*;
}

/// CLI and provider-owned host lifecycle APIs.
#[cfg(feature = "cli")]
pub mod cli {
    pub use ruxex_cli::*;
}

/// Provider contracts and construction types.
pub mod model_provider {
    pub use ruxex_model_provider::*;
}

/// TUI widgets, themes, and branding APIs.
#[cfg(feature = "tui")]
pub mod tui {
    pub use ruxex_tui::*;
}

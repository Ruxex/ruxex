//! Public entry point for building provider-neutral Ruxex applications.
//!
//! The implementation remains split across the workspace crates, while this
//! crate presents the main host integration points from one dependency.

pub use ruxex_app_server::runtime::AppServerRuntimeBuilder;
pub use ruxex_cli::ProviderRuntimeFactory;
pub use ruxex_cli::run;
pub use ruxex_cli::run_with_provider_factory;
pub use ruxex_cli::run_with_provider_factory_and_identity;
pub use ruxex_model_provider::ModelProvider;
pub use ruxex_model_provider::ModelProviderFactory;
pub use ruxex_model_provider::ProviderInit;
pub use ruxex_model_provider::SharedModelProvider;
pub use ruxex_tui::TuiBrandProfile;
pub use ruxex_utils_home_dir::ProductIdentity;

/// Provider implementation APIs grouped by responsibility.
pub mod provider {
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
        pub use ruxex_protocol::model_types::ReasoningEffort;
        pub use ruxex_protocol::models::ResponseItem;
        pub use ruxex_protocol::protocol::ModelVerification;
        pub use ruxex_protocol::protocol::RateLimitSnapshot;
        pub use ruxex_protocol::protocol::SessionSource;
        pub use ruxex_protocol::protocol::TokenUsage;
    }
}

/// Advanced App Server APIs.
pub mod app_server {
    pub use ruxex_app_server::*;
}

/// CLI and provider-owned host lifecycle APIs.
pub mod cli {
    pub use ruxex_cli::*;
}

/// Provider contracts and construction types.
pub mod model_provider {
    pub use ruxex_model_provider::*;
}

/// TUI widgets, themes, and branding APIs.
pub mod tui {
    pub use ruxex_tui::*;
}

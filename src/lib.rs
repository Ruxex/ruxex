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

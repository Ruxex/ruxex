//! Curated facade for embedded desktop clients.
//!
//! This surface contains the App Server client, daemon, protocol, and host
//! integration types needed by desktop applications. It intentionally does
//! not depend on the interactive CLI or TUI profile.

pub mod client {
    pub use ruxex_app_server_client::AppServerClient;
    pub use ruxex_app_server_client::AppServerEvent;
    pub use ruxex_app_server_client::AppServerRequestHandle;
    pub use ruxex_app_server_client::AppServerRuntime;
    pub use ruxex_app_server_client::AppServerRuntimeBuilder;
    pub use ruxex_app_server_client::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY;
    pub use ruxex_app_server_client::EnvironmentManager;
    pub use ruxex_app_server_client::ExecServerRuntimePaths;
    pub use ruxex_app_server_client::InProcessAppServerClient;
    pub use ruxex_app_server_client::InProcessClientStartArgs;
    pub use ruxex_app_server_client::InProcessServerEvent;
    pub use ruxex_app_server_client::RemoteAppServerClient;
    pub use ruxex_app_server_client::RemoteAppServerConnectArgs;
    pub use ruxex_app_server_client::RemoteAppServerEndpoint;
    pub use ruxex_app_server_client::StateDbHandle;
    pub use ruxex_app_server_client::app_server_control_socket_path;
    pub use ruxex_app_server_client::provider_init_from_config_with_descriptor;
}

pub mod config {
    pub use ruxex_config::LoaderOverrides;
    pub use ruxex_utils_cli::CliConfigOverrides;
}

pub mod core {
    pub use ruxex_core::config::{Config, ConfigOverrides};
    pub use ruxex_core::init_state_db;

    pub mod config {
        pub use ruxex_core::config::{Config, ConfigOverrides};
    }
}

pub mod daemon {
    pub use ruxex_app_server_daemon::BackendKind;
    pub use ruxex_app_server_daemon::BootstrapOptions;
    pub use ruxex_app_server_daemon::BootstrapOutput;
    pub use ruxex_app_server_daemon::BootstrapStatus;
    pub use ruxex_app_server_daemon::HOST_APP_SERVER_DAEMON_MODE_ARG;
    pub use ruxex_app_server_daemon::LifecycleCommand;
    pub use ruxex_app_server_daemon::LifecycleOutput;
    pub use ruxex_app_server_daemon::LifecycleStatus;
    pub use ruxex_app_server_daemon::bootstrap;
    pub use ruxex_app_server_daemon::bootstrap_from_package;
    pub use ruxex_app_server_daemon::bootstrap_from_package_with_host_args;
    pub use ruxex_app_server_daemon::probe_app_server_version;
    pub use ruxex_app_server_daemon::run;
    pub use ruxex_app_server_daemon::run_with_host_executable;
}

pub mod host {
    use ruxex_app_server::{
        AppServerExit, AppServerRuntime, AppServerRuntimeOptions, AppServerTransport,
        AppServerWebsocketAuthSettings, ProviderDescriptor, ProviderInit,
        app_server_control_socket_path, run_main_with_transport_options_and_provider_factory,
    };
    use ruxex_arg0::Arg0DispatchPaths;
    use ruxex_config::LoaderOverrides;
    use ruxex_protocol::protocol::SessionSource;
    use ruxex_utils_cli::CliConfigOverrides;
    use ruxex_utils_home_dir::{ProductIdentity, find_product_home};

    pub type ProviderRuntimeFactory = (fn(ProviderInit) -> AppServerRuntime, ProviderDescriptor);

    /// Runs a provider-owned App Server daemon for an embedded desktop host.
    ///
    /// The transport and lifecycle behavior matches the legacy CLI helper,
    /// while keeping the desktop profile independent from the TUI crate.
    pub async fn run_host_app_server_daemon_with_provider_factory_and_identity(
        arg0_paths: Arg0DispatchPaths,
        cli_config_overrides: CliConfigOverrides,
        loader_overrides: LoaderOverrides,
        strict_config: bool,
        provider_factory: ProviderRuntimeFactory,
        product_identity: ProductIdentity,
    ) -> std::io::Result<AppServerExit> {
        let ruxex_home = find_product_home(&product_identity)?;
        let socket_path = app_server_control_socket_path(&ruxex_home)?;
        run_main_with_transport_options_and_provider_factory(
            arg0_paths,
            cli_config_overrides,
            loader_overrides,
            strict_config,
            false,
            AppServerTransport::UnixSocket { socket_path },
            SessionSource::Cli,
            AppServerWebsocketAuthSettings::default(),
            AppServerRuntimeOptions {
                product_identity,
                managed_daemon: true,
                ..Default::default()
            },
            provider_factory,
        )
        .await
    }

    /// Runs a provider-owned daemon using the default Ruxex product identity.
    pub async fn run_host_app_server_daemon_with_provider_factory(
        arg0_paths: Arg0DispatchPaths,
        cli_config_overrides: CliConfigOverrides,
        loader_overrides: LoaderOverrides,
        strict_config: bool,
        provider_factory: ProviderRuntimeFactory,
    ) -> std::io::Result<AppServerExit> {
        run_host_app_server_daemon_with_provider_factory_and_identity(
            arg0_paths,
            cli_config_overrides,
            loader_overrides,
            strict_config,
            provider_factory,
            ProductIdentity::ruxex(),
        )
        .await
    }
}

pub mod protocol {
    pub use ruxex_app_server_protocol::ClientRequest;
    pub use ruxex_app_server_protocol::ClientResponse;
    pub use ruxex_app_server_protocol::ConfigWarningNotification;
    pub use ruxex_app_server_protocol::JSONRPCErrorError;
    pub use ruxex_app_server_protocol::RequestId;
    pub use ruxex_app_server_protocol::Result;
    pub use ruxex_app_server_protocol::Result as JsonRpcResult;
}

pub mod providers {
    pub use ruxex_providers::ResponsesModelProvider;
}

pub mod utils {
    pub mod absolute_path {
        pub use ruxex_utils_absolute_path::AbsolutePathBuf;
        pub use ruxex_utils_absolute_path::canonicalize_existing_preserving_symlinks;
        pub use ruxex_utils_absolute_path::canonicalize_preserving_symlinks;
    }
}

pub use ruxex_app_server::AppServerExit;
pub use ruxex_app_server::AppServerRuntimeOptions;
pub use ruxex_app_server::AppServerTransport;
pub use ruxex_app_server::AppServerWebsocketAuthSettings;
pub use ruxex_app_server::ProviderDescriptor;
pub use ruxex_app_server::ProviderInit;
pub use ruxex_app_server::ProviderRuntimeFactory;
pub use ruxex_app_server::app_server_control_socket_path;
pub use ruxex_utils_home_dir::ProductIdentity;

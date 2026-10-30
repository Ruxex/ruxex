use ruxex::desktop::{
    client::{
        AppServerClient, AppServerEvent, AppServerRequestHandle, AppServerRuntime,
        AppServerRuntimeBuilder, InProcessAppServerClient, InProcessClientStartArgs,
        RemoteAppServerClient, RemoteAppServerConnectArgs, RemoteAppServerEndpoint,
        provider_init_from_config_with_descriptor,
    },
    config::{CliConfigOverrides, LoaderOverrides},
    core::{Config, ConfigOverrides, config as core_config, init_state_db},
    daemon::{LifecycleCommand, LifecycleOutput},
    protocol::{
        ClientRequest, ClientResponse, ConfigWarningNotification, JsonRpcResult, RequestId,
        Result as ProtocolResult,
    },
    providers::ResponsesModelProvider,
    utils::absolute_path::AbsolutePathBuf,
};

#[test]
fn desktop_facade_exposes_embedded_host_surface() {
    let _: Option<AppServerClient> = None;
    let _: Option<AppServerEvent> = None;
    let _: Option<AppServerRequestHandle> = None;
    let _: Option<AppServerRuntime> = None;
    let _: Option<AppServerRuntimeBuilder> = None;
    let _: Option<InProcessAppServerClient> = None;
    let _: Option<InProcessClientStartArgs> = None;
    let _: Option<RemoteAppServerClient> = None;
    let _: Option<RemoteAppServerConnectArgs> = None;
    let _: Option<RemoteAppServerEndpoint> = None;
    let _ = provider_init_from_config_with_descriptor;

    let _: Option<CliConfigOverrides> = None;
    let _: Option<LoaderOverrides> = None;
    let _: Option<Config> = None;
    let _: Option<ConfigOverrides> = None;
    let _: Option<core_config::Config> = None;
    let _: Option<core_config::ConfigOverrides> = None;
    let _ = init_state_db;
    let _: Option<LifecycleCommand> = None;
    let _: Option<LifecycleOutput> = None;
    let _: Option<ClientRequest> = None;
    let _: Option<ClientResponse> = None;
    let _: Option<ConfigWarningNotification> = None;
    let _: Option<RequestId> = None;
    let _: Option<JsonRpcResult> = None;
    let _: Option<ProtocolResult> = None;
    let _: Option<ResponsesModelProvider> = None;
    let _: Option<AbsolutePathBuf> = None;

    let _ = ruxex::desktop::app_server_control_socket_path;
    let _host_helper =
        ruxex::desktop::host::run_host_app_server_daemon_with_provider_factory_and_identity;
}

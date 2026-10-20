use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use ruxex_app_server_client::AppServerEvent;
use ruxex_app_server_client::AppServerRequestHandle;
use ruxex_app_server_client::RemoteAppServerClient;
use ruxex_app_server_client::RemoteAppServerConnectArgs;
use ruxex_app_server_client::RemoteAppServerEndpoint;
use ruxex_app_server_protocol::ClientRequest;
use ruxex_app_server_protocol::ClientResponse;
use ruxex_app_server_protocol::JSONRPCErrorError;
use ruxex_app_server_protocol::RequestId;
use ruxex_app_server_protocol::Result as JsonRpcResult;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_home_dir::ProductIdentity;
use serde::Serialize;
use serde_json::Value;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri::State;

#[derive(Default)]
struct AppState {
    initialization: tauri::async_runtime::Mutex<()>,
    request_handle: tauri::async_runtime::Mutex<Option<AppServerRequestHandle>>,
    status: tauri::async_runtime::Mutex<Option<AppServerStatus>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppServerStatus {
    socket_path: String,
    app_server_version: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum DesktopAppServerErrorKind {
    Lifecycle,
    Initialization,
    Connection,
    Transport,
    Server,
    Decode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopAppServerError {
    kind: DesktopAppServerErrorKind,
    message: String,
}

type CommandResult<T> = Result<T, DesktopAppServerError>;

fn command_error(
    kind: DesktopAppServerErrorKind,
    message: impl Into<String>,
) -> DesktopAppServerError {
    DesktopAppServerError {
        kind,
        message: message.into(),
    }
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
enum DesktopAppServerEvent {
    Lagged { skipped: usize },
    ServerNotification(Value),
    ServerRequest(Value),
    Disconnected { message: String },
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .setup(|_app| {
            let home = desktop_ruxex_home().map_err(std::io::Error::other)?;

            // The setup hook runs before Tauri exposes commands or starts the
            // app-server child, so these process-wide values are safe to set
            // once here and are inherited by the managed daemon.
            unsafe {
                std::env::set_var("RUXEX_HOME", &home);
                std::env::set_var("RUXEX_SQLITE_HOME", &home);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ensure_app_server,
            app_server_request,
            app_server_resolve_server_request,
            app_server_reject_server_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running Ruxex Desktop");
}

#[cfg(test)]
mod tests {
    use super::DESKTOP_EXPERIMENTAL_API;
    use super::DesktopAppServerErrorKind;
    use super::command_error;
    use super::desktop_product_identity;
    use super::desktop_ruxex_home_from_user_home;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn desktop_home_is_isolated_under_user_home() {
        assert_eq!(
            desktop_ruxex_home_from_user_home(Path::new("/home/user")),
            Path::new("/home/user/.ruxex_desktop"),
        );
    }

    #[test]
    fn desktop_product_identity_is_separate_from_tui_branding() {
        let identity = desktop_product_identity();

        assert_eq!(identity.product_name(), "Ruxex Desktop");
        assert_eq!(identity.product_slug(), "ruxex-desktop");
        assert_eq!(identity.home_dir_name(), ".ruxex_desktop");
    }

    #[test]
    fn desktop_connects_with_experimental_protocol_capability() {
        assert!(DESKTOP_EXPERIMENTAL_API);
    }

    #[test]
    fn app_server_error_categories_are_structured() {
        let errors = [
            command_error(DesktopAppServerErrorKind::Initialization, "bad handshake"),
            command_error(DesktopAppServerErrorKind::Transport, "socket closed"),
            command_error(DesktopAppServerErrorKind::Server, "request rejected"),
        ];
        let kinds = errors
            .into_iter()
            .map(|error| serde_json::to_value(error).unwrap()["kind"].clone())
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            [json!("initialization"), json!("transport"), json!("server")]
        );

        let transport_error = command_error(DesktopAppServerErrorKind::Transport, "socket closed");
        assert_eq!(
            serde_json::to_value(transport_error).unwrap(),
            json!({
                "kind": "transport",
                "message": "socket closed",
            })
        );
    }
}

const DESKTOP_EXPERIMENTAL_API: bool = true;

fn desktop_product_identity() -> ProductIdentity {
    ProductIdentity::new(
        "Ruxex Desktop",
        "ruxex-desktop",
        ".ruxex_desktop",
        "ruxex-home.json",
    )
    .expect("desktop product identity is static and valid")
}

fn desktop_ruxex_home_from_user_home(user_home: &Path) -> PathBuf {
    desktop_product_identity()
        .resolve_home_from(user_home)
        .expect("desktop user home should be absolute")
        .into_path_buf()
}

fn desktop_ruxex_home() -> Result<PathBuf, String> {
    let user_home =
        dirs::home_dir().ok_or_else(|| "failed to resolve user home directory".to_string())?;
    let home = desktop_ruxex_home_from_user_home(&user_home);
    fs::create_dir_all(&home)
        .map_err(|error| format!("failed to create Desktop Ruxex home: {error}"))?;
    Ok(home)
}

async fn connect_app_server(
    socket_path: AbsolutePathBuf,
) -> Result<RemoteAppServerClient, std::io::Error> {
    RemoteAppServerClient::connect(RemoteAppServerConnectArgs {
        endpoint: RemoteAppServerEndpoint::UnixSocket { socket_path },
        client_name: desktop_product_identity().product_name().to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: DESKTOP_EXPERIMENTAL_API,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: 128,
    })
    .await
}

#[tauri::command]
async fn ensure_app_server(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<AppServerStatus> {
    let _initialization = state.initialization.lock().await;
    if let Some(status) = state.status.lock().await.clone() {
        return Ok(status);
    }

    let home = desktop_ruxex_home()
        .map_err(|error| command_error(DesktopAppServerErrorKind::Lifecycle, error))?;
    let socket_path =
        ruxex_app_server_client::app_server_control_socket_path(&home).map_err(|error| {
            command_error(DesktopAppServerErrorKind::Initialization, error.to_string())
        })?;
    let (client, bootstrapped_version) = match connect_app_server(socket_path.clone()).await {
        Ok(client) => (client, None),
        Err(_) => {
            let package_source = runtime_package_source(&app)
                .map_err(|error| command_error(DesktopAppServerErrorKind::Lifecycle, error))?;
            let bootstrap = ruxex_app_server_daemon::bootstrap_from_package_with_host_args(
                &package_source,
                &[
                    OsString::from(ruxex_app_server_daemon::HOST_APP_SERVER_DAEMON_MODE_ARG),
                    OsString::from("--managed-daemon"),
                ],
            )
            .await
            .map_err(|error| {
                command_error(DesktopAppServerErrorKind::Lifecycle, format!("{error:#}"))
            })?;
            let socket_path = AbsolutePathBuf::from_absolute_path(bootstrap.socket_path.clone())
                .map_err(|error| {
                    command_error(DesktopAppServerErrorKind::Initialization, error.to_string())
                })?;
            let client = connect_app_server(socket_path).await.map_err(|error| {
                command_error(DesktopAppServerErrorKind::Initialization, error.to_string())
            })?;
            (client, Some(bootstrap.app_server_version))
        }
    };

    let status = AppServerStatus {
        socket_path: socket_path.display().to_string(),
        app_server_version: client
            .server_version()
            .map(str::to_owned)
            .or(bootstrapped_version)
            .unwrap_or_else(|| "unknown".to_string()),
    };
    let request_handle = client.request_handle();
    *state.request_handle.lock().await = Some(AppServerRequestHandle::Remote(request_handle));
    *state.status.lock().await = Some(status.clone());

    let event_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut client = client;
        while let Some(event) = client.next_event().await {
            let payload = match event {
                AppServerEvent::Lagged { skipped } => DesktopAppServerEvent::Lagged { skipped },
                AppServerEvent::ServerNotification(notification) => {
                    match serde_json::to_value(notification) {
                        Ok(notification) => DesktopAppServerEvent::ServerNotification(notification),
                        Err(error) => DesktopAppServerEvent::Disconnected {
                            message: format!("failed to serialize server notification: {error}"),
                        },
                    }
                }
                AppServerEvent::ServerRequest(request) => match serde_json::to_value(request) {
                    Ok(request) => DesktopAppServerEvent::ServerRequest(request),
                    Err(error) => DesktopAppServerEvent::Disconnected {
                        message: format!("failed to serialize server request: {error}"),
                    },
                },
                AppServerEvent::Disconnected { message } => {
                    DesktopAppServerEvent::Disconnected { message }
                }
            };
            if event_app.emit("app-server-event", payload).is_err() {
                break;
            }
        }
    });

    Ok(status)
}

#[tauri::command]
async fn app_server_request(
    state: State<'_, AppState>,
    request: ClientRequest,
) -> CommandResult<ClientResponse> {
    let request_handle = request_handle(&state).await?;
    let method = request.method_name();
    let request_id = request.id().clone();
    let result = request_handle
        .request(request)
        .await
        .map_err(|error| {
            command_error(
                DesktopAppServerErrorKind::Transport,
                format!("{method} transport error: {error}"),
            )
        })?
        .map_err(|error| {
            command_error(
                DesktopAppServerErrorKind::Server,
                format!("{method} failed: {} (code {})", error.message, error.code),
            )
        })?;

    let response = serde_json::json!({
        "method": method,
        "id": request_id,
        "response": result,
    });
    serde_json::from_value(response).map_err(|error| {
        command_error(
            DesktopAppServerErrorKind::Decode,
            format!("{method} response decode error: {error}"),
        )
    })
}

#[tauri::command]
async fn app_server_resolve_server_request(
    state: State<'_, AppState>,
    request_id: RequestId,
    result: JsonRpcResult,
) -> CommandResult<()> {
    request_handle(&state)
        .await?
        .resolve_server_request(request_id, result)
        .await
        .map_err(|error| command_error(DesktopAppServerErrorKind::Transport, error.to_string()))?;
    Ok(())
}

#[tauri::command]
async fn app_server_reject_server_request(
    state: State<'_, AppState>,
    request_id: RequestId,
    error: JSONRPCErrorError,
) -> CommandResult<()> {
    request_handle(&state)
        .await?
        .reject_server_request(request_id, error)
        .await
        .map_err(|error| command_error(DesktopAppServerErrorKind::Transport, error.to_string()))?;
    Ok(())
}

async fn request_handle(state: &State<'_, AppState>) -> CommandResult<AppServerRequestHandle> {
    state.request_handle.lock().await.clone().ok_or_else(|| {
        command_error(
            DesktopAppServerErrorKind::Connection,
            "app-server is not connected",
        )
    })
}

fn runtime_package_source(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_package = app
        .path()
        .resource_dir()
        .map_err(|error| format!("failed to resolve application resources: {error}"))?
        .join("ruxex-package");
    if resource_package.join("ruxex-package.json").is_file() {
        return Ok(resource_package);
    }

    let development_package = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("ruxex-package");
    if development_package.join("ruxex-package.json").is_file() {
        return Ok(development_package);
    }

    Err(format!(
        "bundled Ruxex runtime package not found at {} or {}",
        resource_package.display(),
        development_package.display()
    ))
}

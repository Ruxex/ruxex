use std::fs;
use std::path::Path;
use std::path::PathBuf;

use genex_app_server_client::AppServerEvent;
use genex_app_server_client::AppServerRequestHandle;
use genex_app_server_client::RemoteAppServerClient;
use genex_app_server_client::RemoteAppServerConnectArgs;
use genex_app_server_client::RemoteAppServerEndpoint;
use genex_app_server_protocol::ClientRequest;
use genex_app_server_protocol::ClientResponse;
use genex_app_server_protocol::JSONRPCErrorError;
use genex_app_server_protocol::RequestId;
use genex_app_server_protocol::Result as JsonRpcResult;
use genex_utils_absolute_path::AbsolutePathBuf;
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopAppServerError {
    kind: &'static str,
    message: String,
}

type CommandResult<T> = Result<T, DesktopAppServerError>;

fn command_error(kind: &'static str, message: impl Into<String>) -> DesktopAppServerError {
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
            let home = desktop_genex_home().map_err(std::io::Error::other)?;

            // The setup hook runs before Tauri exposes commands or starts the
            // app-server child, so these process-wide values are safe to set
            // once here and are inherited by the managed daemon.
            unsafe {
                std::env::set_var("GENEX_HOME", &home);
                std::env::set_var("GENEX_SQLITE_HOME", &home);
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
        .expect("error while running Genex Desktop");
}

#[cfg(test)]
mod tests {
    use super::desktop_genex_home_from_user_home;
    use std::path::Path;

    #[test]
    fn desktop_home_is_isolated_under_user_home() {
        assert_eq!(
            desktop_genex_home_from_user_home(Path::new("/home/user")),
            Path::new("/home/user/.genex_desktop"),
        );
    }
}

const DESKTOP_GENEX_HOME_DIR: &str = ".genex_desktop";

fn desktop_genex_home_from_user_home(user_home: &Path) -> PathBuf {
    user_home.join(DESKTOP_GENEX_HOME_DIR)
}

fn desktop_genex_home() -> Result<PathBuf, String> {
    let user_home =
        dirs::home_dir().ok_or_else(|| "failed to resolve user home directory".to_string())?;
    let home = desktop_genex_home_from_user_home(&user_home);
    fs::create_dir_all(&home)
        .map_err(|error| format!("failed to create Desktop Genex home: {error}"))?;
    Ok(home)
}

async fn connect_app_server(
    socket_path: AbsolutePathBuf,
) -> Result<RemoteAppServerClient, std::io::Error> {
    RemoteAppServerClient::connect(RemoteAppServerConnectArgs {
        endpoint: RemoteAppServerEndpoint::UnixSocket { socket_path },
        client_name: "genex-desktop".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: false,
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

    let home = desktop_genex_home().map_err(|error| command_error("lifecycle", error))?;
    let socket_path = genex_app_server_client::app_server_control_socket_path(&home)
        .map_err(|error| command_error("initialization", error.to_string()))?;
    let (client, bootstrapped_version) = match connect_app_server(socket_path.clone()).await {
        Ok(client) => (client, None),
        Err(_) => {
            let package_source =
                runtime_package_source(&app).map_err(|error| command_error("lifecycle", error))?;
            let bootstrap = genex_app_server_daemon::bootstrap_from_package(&package_source)
                .await
                .map_err(|error| command_error("lifecycle", format!("{error:#}")))?;
            let socket_path = AbsolutePathBuf::from_absolute_path(bootstrap.socket_path.clone())
                .map_err(|error| command_error("initialization", error.to_string()))?;
            let client = connect_app_server(socket_path)
                .await
                .map_err(|error| command_error("initialization", error.to_string()))?;
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
        .map_err(|error| command_error("transport", format!("{method} transport error: {error}")))?
        .map_err(|error| {
            command_error(
                "server",
                format!("{method} failed: {} (code {})", error.message, error.code),
            )
        })?;

    let response = serde_json::json!({
        "method": method,
        "id": request_id,
        "response": result,
    });
    serde_json::from_value(response).map_err(|error| {
        command_error("decode", format!("{method} response decode error: {error}"))
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
        .map_err(|error| command_error("transport", error.to_string()))?;
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
        .map_err(|error| command_error("transport", error.to_string()))?;
    Ok(())
}

async fn request_handle(state: &State<'_, AppState>) -> CommandResult<AppServerRequestHandle> {
    state
        .request_handle
        .lock()
        .await
        .clone()
        .ok_or_else(|| command_error("connection", "app-server is not connected"))
}

fn runtime_package_source(app: &AppHandle) -> Result<PathBuf, String> {
    let resource_package = app
        .path()
        .resource_dir()
        .map_err(|error| format!("failed to resolve application resources: {error}"))?
        .join("genex-package");
    if resource_package.join("genex-package.json").is_file() {
        return Ok(resource_package);
    }

    let development_package = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("genex-package");
    if development_package.join("genex-package.json").is_file() {
        return Ok(development_package);
    }

    Err(format!(
        "bundled Genex runtime package not found at {} or {}",
        resource_package.display(),
        development_package.display()
    ))
}

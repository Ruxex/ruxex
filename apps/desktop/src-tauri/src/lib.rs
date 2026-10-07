use std::path::PathBuf;

use genex_app_server_client::AppServerEvent;
use genex_app_server_client::AppServerRequestHandle;
use genex_app_server_client::RemoteAppServerClient;
use genex_app_server_client::RemoteAppServerConnectArgs;
use genex_app_server_client::RemoteAppServerEndpoint;
use genex_app_server_daemon::BootstrapOptions;
use genex_app_server_protocol::ClientRequest;
use genex_app_server_protocol::ClientResponse;
use genex_utils_absolute_path::AbsolutePathBuf;
use serde::Serialize;
use serde_json::Value;
use tauri::AppHandle;
use tauri::Manager;
use tauri::State;

#[derive(Default)]
struct AppState {
    request_handle: tauri::async_runtime::Mutex<Option<AppServerRequestHandle>>,
    status: tauri::async_runtime::Mutex<Option<AppServerStatus>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppServerStatus {
    socket_path: String,
    app_server_version: String,
}

#[derive(Serialize)]
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
        .invoke_handler(tauri::generate_handler![
            ensure_app_server,
            app_server_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running Genex Desktop");
}

#[tauri::command]
async fn ensure_app_server(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppServerStatus, String> {
    if let Some(status) = state.status.lock().await.clone() {
        return Ok(status);
    }

    let package_source = runtime_package_source(&app)?;
    let bootstrap =
        genex_app_server_daemon::bootstrap_from_package(&package_source, BootstrapOptions {})
            .await
            .map_err(|error| format!("failed to bootstrap app-server daemon: {error:#}"))?;
    let socket_path = AbsolutePathBuf::from_absolute_path(bootstrap.socket_path.clone())
        .map_err(|error| format!("invalid app-server socket path: {error}"))?;
    let client = RemoteAppServerClient::connect(RemoteAppServerConnectArgs {
        endpoint: RemoteAppServerEndpoint::UnixSocket { socket_path },
        client_name: "genex-desktop".to_string(),
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        experimental_api: false,
        opt_out_notification_methods: Vec::new(),
        channel_capacity: 128,
    })
    .await
    .map_err(|error| format!("failed to connect to app-server: {error}"))?;

    let status = AppServerStatus {
        socket_path: bootstrap.socket_path.display().to_string(),
        app_server_version: bootstrap.app_server_version,
    };
    let request_handle = client.request_handle();
    *state.request_handle.lock().await = Some(request_handle);
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
) -> Result<ClientResponse, String> {
    let request_handle = state
        .request_handle
        .lock()
        .await
        .clone()
        .ok_or_else(|| "app-server is not connected".to_string())?;
    let method = request.method_name();
    let request_id = request.id().clone();
    let result = request_handle
        .request(request)
        .await
        .map_err(|error| format!("{method} transport error: {error}"))?
        .map_err(|error| format!("{method} failed: {} (code {})", error.message, error.code))?;

    let response = serde_json::json!({
        "method": method,
        "id": request_id,
        "response": result,
    });
    serde_json::from_value(response)
        .map_err(|error| format!("{method} response decode error: {error}"))
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

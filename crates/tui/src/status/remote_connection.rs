use crate::AppServerTarget;
use crate::RemoteAppServerEndpoint;
use sha2::Digest;
use sha2::Sha256;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemoteConnectionStatus {
    pub(crate) address: String,
    pub(crate) version: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServerVersionNotice {
    pub(crate) message: String,
    pub(crate) offer_update: bool,
}

pub(crate) fn remote_connection_status_value(
    app_server_target: &AppServerTarget,
    server_version: Option<&str>,
) -> Option<RemoteConnectionStatus> {
    let endpoint = match app_server_target {
        AppServerTarget::Embedded => return None,
        AppServerTarget::LocalDaemon { endpoint, .. } | AppServerTarget::Remote { endpoint } => {
            endpoint
        }
    };
    let address = match endpoint {
        RemoteAppServerEndpoint::WebSocket { websocket_url, .. } => {
            sanitized_websocket_url(websocket_url)
                .map(|url| url.to_string())
                .unwrap_or_else(|| "<invalid websocket URL>".to_string())
        }
        RemoteAppServerEndpoint::UnixSocket { socket_path } => {
            format!("unix://{}", socket_path.display())
        }
    };
    let version = server_version
        .map(|version| format!("v{version}"))
        .unwrap_or_else(|| "unknown".to_string());
    Some(RemoteConnectionStatus { address, version })
}

pub(crate) fn server_version_notice(client: &str, server: Option<&str>) -> Option<String> {
    let server = server?;
    crate::update_versions::is_official_server_older(client, server).then(|| {
        format!(
            "A background Genex service is running v{server}, older than your Genex CLI v{client}."
        )
    })
}

pub(crate) fn server_version_notice_for_tui(
    settings: &genex_config::types::Tui,
    client: &str,
    server: Option<&str>,
) -> Option<String> {
    if settings.show_server_version_notice {
        server_version_notice(client, server)
    } else {
        None
    }
}

fn hash_identity_part(hasher: &mut Sha256, tag: &[u8], value: &[u8]) {
    hasher.update(tag);
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value);
}

pub(crate) fn server_version_notice_key(
    target: &AppServerTarget,
    server_home: Option<&str>,
    client: &str,
    server: Option<&str>,
) -> Option<String> {
    let server = server?;
    server_version_notice(client, Some(server))?;
    let mut hasher = Sha256::new();
    let endpoint = match target {
        AppServerTarget::Embedded => None,
        AppServerTarget::LocalDaemon { endpoint, .. } | AppServerTarget::Remote { endpoint } => {
            Some(endpoint)
        }
    };
    if let Some(server_home) = server_home {
        hash_identity_part(&mut hasher, b"server-home:", server_home.as_bytes());
    }
    match endpoint {
        Some(RemoteAppServerEndpoint::UnixSocket { socket_path }) => {
            hash_identity_part(
                &mut hasher,
                b"unix:",
                socket_path.as_path().as_os_str().as_encoded_bytes(),
            );
        }
        Some(RemoteAppServerEndpoint::WebSocket { websocket_url, .. }) => {
            if let Ok(mut url) = Url::parse(websocket_url) {
                let routing = url
                    .query_pairs()
                    .filter(|(name, _)| {
                        matches!(
                            name.to_ascii_lowercase().as_str(),
                            "workspace"
                                | "project"
                                | "tenant"
                                | "environment"
                                | "namespace"
                                | "organization"
                                | "org"
                                | "account"
                                | "route"
                                | "instance"
                                | "region"
                                | "deployment"
                                | "cluster"
                        )
                    })
                    .map(|(name, value)| (name.into_owned(), value.into_owned()))
                    .collect::<Vec<_>>();
                let _ = url.set_username("");
                let _ = url.set_password(None);
                url.set_query(None);
                url.set_fragment(None);
                if !routing.is_empty() {
                    url.query_pairs_mut().extend_pairs(routing);
                }
                hash_identity_part(&mut hasher, b"websocket:", url.as_str().as_bytes());
            }
        }
        None => hash_identity_part(&mut hasher, b"embedded:", b""),
    }
    Some(format!("{:x}:{client}-{server}", hasher.finalize()))
}

pub(crate) fn pending_server_version_notice(
    settings: &genex_config::types::Tui,
    target: &AppServerTarget,
    server_home: Option<&str>,
    client: &str,
    server: Option<&str>,
    last_shown: Option<&str>,
) -> Option<(ServerVersionNotice, String)> {
    let message = server_version_notice_for_tui(settings, client, server)?;
    let notice = ServerVersionNotice {
        message,
        offer_update: matches!(target, AppServerTarget::LocalDaemon { .. }),
    };
    let key = server_version_notice_key(target, server_home, client, server)?;
    (last_shown != Some(key.as_str())).then_some((notice, key))
}

pub(crate) fn sanitized_websocket_url(raw: &str) -> Option<Url> {
    let mut url = Url::parse(raw).ok()?;
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    Some(url)
}

use crate::plugin_bundle_archive::PluginBundlePackError;
use crate::plugin_bundle_archive::pack_plugin_bundle_tar_gz;
use genex_http_client::RouteAwareClientPool;
use http::Method;
use http::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

const MAX_PLUGIN_BUNDLE_BYTES: usize = 50 * 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_ERROR_BODY_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone)]
pub struct PluginPublishRequest {
    pub registry_url: String,
    pub plugin_path: PathBuf,
    pub plugin_id: Option<String>,
    pub auth_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginPublishOutcome {
    pub plugin_id: String,
    pub version: Option<String>,
    pub plugin_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum PluginPublishError {
    #[error("invalid registry URL '{url}': {reason}")]
    InvalidRegistryUrl { url: String, reason: String },

    #[error("invalid plugin path '{path}': {reason}")]
    InvalidPluginPath { path: PathBuf, reason: String },

    #[error("plugin archive would be {bytes} bytes, exceeding maximum size of {max_bytes} bytes")]
    ArchiveTooLarge { bytes: usize, max_bytes: usize },

    #[error("failed to archive plugin bundle at '{path}': {source}")]
    Archive {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("request to '{url}' failed: {source}")]
    Request {
        url: String,
        #[source]
        source: genex_http_client::RouteAwareRequestError,
    },

    #[error("unexpected response from '{url}': HTTP {status}: {body}")]
    UnexpectedStatus {
        url: String,
        status: StatusCode,
        body: String,
    },

    #[error("failed to decode response from '{url}': {source}")]
    Decode {
        url: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("registry response from '{url}' did not include an ETag")]
    MissingEtag { url: String },
}

#[derive(Debug, Serialize)]
struct UploadReservationRequest<'a> {
    filename: &'a str,
    mime_type: &'a str,
    size_bytes: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugin_id: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
struct UploadReservationResponse {
    file_id: String,
    upload_url: String,
    #[serde(default)]
    etag: Option<String>,
    #[serde(default)]
    upload_headers: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct FinalizeUploadRequest {
    file_id: String,
    etag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugin_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FinalizeUploadResponse {
    plugin_id: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    plugin_url: Option<String>,
}

pub async fn publish_plugin_bundle(
    client: &RouteAwareClientPool,
    request: PluginPublishRequest,
) -> Result<PluginPublishOutcome, PluginPublishError> {
    let registry_url = normalize_registry_url(&request.registry_url)?;
    let archive_filename = archive_filename(&request.plugin_path)?;
    let archive = pack_plugin_bundle_tar_gz(&request.plugin_path, MAX_PLUGIN_BUNDLE_BYTES)
        .map_err(|error| map_pack_error(&request.plugin_path, error))?;

    let reservation_url = format!("{registry_url}/v1/plugins/uploads");
    let reservation_request = UploadReservationRequest {
        filename: &archive_filename,
        mime_type: "application/gzip",
        size_bytes: archive.len(),
        plugin_id: request.plugin_id.as_deref(),
    };
    let reservation: UploadReservationResponse = send_json(
        client,
        Method::POST,
        &reservation_url,
        request.auth_token.as_deref(),
        Some(&reservation_request),
    )
    .await?;

    let upload_url = reservation.upload_url.clone();
    let mut upload_request = client
        .request(Method::PUT, &upload_url)
        .timeout(REQUEST_TIMEOUT)
        .header("Content-Type", "application/gzip")
        .body(archive);
    for (name, value) in reservation.upload_headers {
        upload_request = upload_request.header(name, value);
    }
    let upload_response =
        upload_request
            .send()
            .await
            .map_err(|source| PluginPublishError::Request {
                url: upload_url.clone(),
                source,
            })?;
    let upload_status = upload_response.status();
    let upload_etag = upload_response
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    if !upload_status.is_success() {
        let body = upload_response.text().await.unwrap_or_default();
        return Err(PluginPublishError::UnexpectedStatus {
            url: upload_url,
            status: upload_status,
            body: truncate_body(body),
        });
    }

    let etag = reservation
        .etag
        .or(upload_etag)
        .ok_or_else(|| PluginPublishError::MissingEtag {
            url: reservation_url.clone(),
        })?;

    let finalize_url = format!("{registry_url}/v1/plugins");
    let finalized: FinalizeUploadResponse = send_json(
        client,
        Method::POST,
        &finalize_url,
        request.auth_token.as_deref(),
        Some(&FinalizeUploadRequest {
            file_id: reservation.file_id,
            etag,
            plugin_id: request.plugin_id,
        }),
    )
    .await?;

    Ok(PluginPublishOutcome {
        plugin_id: finalized.plugin_id,
        version: finalized.version,
        plugin_url: finalized.plugin_url,
    })
}

fn normalize_registry_url(registry_url: &str) -> Result<String, PluginPublishError> {
    let url = Url::parse(registry_url.trim()).map_err(|error| {
        PluginPublishError::InvalidRegistryUrl {
            url: registry_url.to_string(),
            reason: error.to_string(),
        }
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(PluginPublishError::InvalidRegistryUrl {
            url: registry_url.to_string(),
            reason: "registry URL must use http or https and include a host".to_string(),
        });
    }
    Ok(registry_url.trim_end_matches('/').to_string())
}

fn archive_filename(plugin_path: &Path) -> Result<String, PluginPublishError> {
    let plugin_name = plugin_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PluginPublishError::InvalidPluginPath {
            path: plugin_path.to_path_buf(),
            reason: "plugin path must end in a valid UTF-8 directory name".to_string(),
        })?;
    Ok(format!("{plugin_name}.tar.gz"))
}

fn map_pack_error(path: &Path, error: PluginBundlePackError) -> PluginPublishError {
    match error {
        PluginBundlePackError::InvalidPluginPath { path, reason } => {
            PluginPublishError::InvalidPluginPath { path, reason }
        }
        PluginBundlePackError::ArchiveTooLarge { bytes, max_bytes } => {
            PluginPublishError::ArchiveTooLarge { bytes, max_bytes }
        }
        PluginBundlePackError::Io { source } => PluginPublishError::Archive {
            path: path.to_path_buf(),
            source,
        },
    }
}

async fn send_json<T, B>(
    client: &RouteAwareClientPool,
    method: Method,
    url: &str,
    auth_token: Option<&str>,
    body: Option<&B>,
) -> Result<T, PluginPublishError>
where
    T: for<'de> Deserialize<'de>,
    B: ?Sized + Serialize,
{
    let mut request = client.request(method, url).timeout(REQUEST_TIMEOUT);
    if let Some(auth_token) = auth_token {
        request = request.header("Authorization", format!("Bearer {auth_token}"));
    }
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request
        .send()
        .await
        .map_err(|source| PluginPublishError::Request {
            url: url.to_string(),
            source,
        })?;
    let status = response.status();
    let bytes = response
        .bytes()
        .await
        .map_err(|source| PluginPublishError::Request {
            url: url.to_string(),
            source: genex_http_client::RouteAwareRequestError::Request(source),
        })?;
    if !status.is_success() {
        return Err(PluginPublishError::UnexpectedStatus {
            url: url.to_string(),
            status,
            body: truncate_body(String::from_utf8_lossy(&bytes).into_owned()),
        });
    }
    serde_json::from_slice(&bytes).map_err(|source| PluginPublishError::Decode {
        url: url.to_string(),
        source,
    })
}

fn truncate_body(mut body: String) -> String {
    if body.len() > MAX_ERROR_BODY_BYTES {
        body.truncate(MAX_ERROR_BODY_BYTES);
        body.push('…');
    }
    body
}

#[cfg(test)]
mod tests {
    use super::super::test_support::test_http_client_factory;
    use genex_http_client::ClientRouteClass;
    use genex_http_client::RouteAwareClientPool;
    use serde_json::Value;
    use serde_json::json;
    use std::fs;
    use std::sync::Arc;
    use tempfile::TempDir;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;
    use wiremock::matchers::method;
    use wiremock::matchers::path;

    #[tokio::test]
    async fn publishes_a_plugin_bundle_through_a_generic_registry() {
        let server = MockServer::start().await;
        let plugin = plugin_dir();

        Mock::given(method("POST"))
            .and(path("/v1/plugins/uploads"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "file_id": "file-123",
                "upload_url": format!("{}/upload/file-123", server.uri())
            })))
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/upload/file-123"))
            .respond_with(ResponseTemplate::new(201).insert_header("etag", "etag-123"))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/plugins"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "plugin_id": "demo",
                "version": "0.1.0",
                "plugin_url": format!("{}/plugins/demo", server.uri())
            })))
            .mount(&server)
            .await;

        let client = Arc::new(RouteAwareClientPool::new(
            test_http_client_factory(),
            ClientRouteClass::Other,
        ));
        let outcome = super::publish_plugin_bundle(
            &client,
            super::PluginPublishRequest {
                registry_url: server.uri(),
                plugin_path: plugin.path().to_path_buf(),
                plugin_id: Some("demo".to_string()),
                auth_token: Some("token-123".to_string()),
            },
        )
        .await
        .expect("publish plugin bundle");

        assert_eq!(outcome.plugin_id, "demo");
        assert_eq!(outcome.version.as_deref(), Some("0.1.0"));
        assert_eq!(
            outcome.plugin_url.as_deref(),
            Some(format!("{}/plugins/demo", server.uri()).as_str())
        );

        let requests = server.received_requests().await.expect("received requests");
        let reservation = requests
            .iter()
            .find(|request| request.url.path() == "/v1/plugins/uploads")
            .expect("upload reservation request");
        let reservation_body: Value =
            serde_json::from_slice(&reservation.body).expect("reservation JSON");
        assert_eq!(
            reservation_body["filename"],
            format!(
                "{}.tar.gz",
                plugin
                    .path()
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap()
            )
        );
        assert_eq!(reservation_body["mime_type"], "application/gzip");
        assert_eq!(reservation_body["plugin_id"], "demo");
        assert!(reservation_body["size_bytes"].as_u64().is_some());
        assert_eq!(
            reservation
                .headers
                .get("authorization")
                .and_then(|value| value.to_str().ok()),
            Some("Bearer token-123")
        );

        let upload = requests
            .iter()
            .find(|request| request.url.path() == "/upload/file-123")
            .expect("bundle upload request");
        assert!(!upload.body.is_empty());
        assert_eq!(
            upload
                .headers
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("application/gzip")
        );

        let finalize = requests
            .iter()
            .find(|request| request.url.path() == "/v1/plugins")
            .expect("finalize request");
        let finalize_body: Value = serde_json::from_slice(&finalize.body).expect("finalize JSON");
        assert_eq!(
            finalize_body,
            json!({
                "file_id": "file-123",
                "etag": "etag-123",
                "plugin_id": "demo"
            })
        );
    }

    fn plugin_dir() -> TempDir {
        let dir = tempfile::tempdir().expect("create plugin directory");
        let manifest = dir.path().join(".genex-plugin/plugin.json");
        fs::create_dir_all(manifest.parent().expect("manifest parent"))
            .expect("create manifest directory");
        fs::write(manifest, r#"{"name":"demo","description":"demo plugin"}"#)
            .expect("write manifest");
        dir
    }
}

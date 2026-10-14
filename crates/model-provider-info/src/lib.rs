//! Resolution and definitions for the model provider mounted by Ruxex.
//!
//! Providers can be defined in two places:
//!   1. Built-in defaults compiled into the binary so Ruxex works out-of-the-box.
//!   2. A single user-defined entry inside `~/.ruxex/config.toml` under the
//!      `model_provider` key.
//!
//! API provider construction applies the process-wide managed residency policy also
//! used by default HTTP headers.

use ruxex_api::Provider as ApiProvider;
use ruxex_api::RetryConfig as ApiRetryConfig;
use ruxex_protocol::auth::AuthMode;
use ruxex_protocol::config_types::ModelProviderAuthInfo;
use ruxex_protocol::error::EnvVarError;
use ruxex_protocol::error::RuxexErr;
use ruxex_protocol::error::Result as RuxexResult;
use ruxex_utils_redacted_string::RedactedString;
use http::HeaderMap;
use http::header::HeaderName;
use http::header::HeaderValue;
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt;
use std::sync::PoisonError;
use std::sync::RwLock;
use std::time::Duration;

pub const RESIDENCY_HEADER_NAME: &str = "x-codex-residency";

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResidencyRequirement {
    Us,
}

static REQUIREMENTS_RESIDENCY: RwLock<Option<ResidencyRequirement>> = RwLock::new(None);

/// Sets the process-wide residency requirement loaded from managed configuration.
pub fn set_managed_residency_requirement(enforce_residency: Option<ResidencyRequirement>) {
    // Recover the stored policy if the lock is poisoned rather than silently disabling it.
    *REQUIREMENTS_RESIDENCY
        .write()
        .unwrap_or_else(PoisonError::into_inner) = enforce_residency;
}

/// Returns the current process-wide managed residency requirement.
pub fn read_managed_residency_requirement() -> Option<ResidencyRequirement> {
    *REQUIREMENTS_RESIDENCY
        .read()
        .unwrap_or_else(PoisonError::into_inner)
}

const DEFAULT_STREAM_IDLE_TIMEOUT_MS: u64 = 300_000;
const DEFAULT_STREAM_MAX_RETRIES: u64 = 5;
const DEFAULT_REQUEST_MAX_RETRIES: u64 = 4;
pub const DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS: u64 = 15_000;
/// Hard cap for user-configured `stream_max_retries`.
const MAX_STREAM_MAX_RETRIES: u64 = 100;
/// Hard cap for user-configured `request_max_retries`.
const MAX_REQUEST_MAX_RETRIES: u64 = 100;

pub const CLOUDFLARE_PROVIDER_ID: &str = "cloudflare";
pub const CLOUDFLARE_PROVIDER_NAME: &str = "Cloudflare Workers AI";
pub const CLOUDFLARE_API_TOKEN_ENV_VAR: &str = "CLOUDFLARE_API_TOKEN";
pub const CLOUDFLARE_ACCOUNT_ID_ENV_VAR: &str = "CLOUDFLARE_ACCOUNT_ID";

/// A credential value requested by a provider during interactive setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAuthField {
    pub id: String,
    pub label: String,
    pub secret: bool,
}

impl ProviderAuthField {
    pub fn new(id: impl Into<String>, label: impl Into<String>, secret: bool) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            secret,
        }
    }
}
/// Wire protocol that the provider speaks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WireApi {
    /// The Chat Completions API exposed by provider at `/v1/chat/completions`.
    Chat,
    /// The Responses API exposed by provider at `/v1/responses`.
    #[default]
    Responses,
}

impl fmt::Display for WireApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Chat => "chat",
            Self::Responses => "responses",
        };
        f.write_str(value)
    }
}

impl<'de> Deserialize<'de> for WireApi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "chat" => Ok(Self::Chat),
            "responses" => Ok(Self::Responses),
            _ => Err(serde::de::Error::unknown_variant(
                &value,
                &["chat", "responses"],
            )),
        }
    }
}

/// Serializable representation of a provider definition.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ModelProviderInfo {
    /// Stable identifier used for provider-scoped credentials and diagnostics.
    pub id: Option<String>,
    /// Friendly display name.
    #[serde(default)]
    pub name: String,
    /// Base URL for the provider's provider-compatible API.
    pub base_url: Option<String>,
    /// Environment variable that stores the user's API key for this provider.
    pub env_key: Option<String>,

    /// Optional instructions to help the user get a valid value for the
    /// variable and set it.
    pub env_key_instructions: Option<String>,
    /// Value to use with `Authorization: Bearer <token>` header. Use of this
    /// config is discouraged in favor of `env_key` for security reasons, but
    /// this may be necessary when using this programmatically.
    pub experimental_bearer_token: Option<RedactedString>,
    /// Command-backed bearer-token configuration for this provider.
    pub auth: Option<ModelProviderAuthInfo>,
    /// Which wire protocol this provider expects.
    #[serde(default)]
    pub wire_api: WireApi,
    /// Optional query parameters to append to the base URL.
    pub query_params: Option<HashMap<String, RedactedString>>,
    /// Additional HTTP headers to include in requests to this provider where
    /// the (key, value) pairs are the header name and value.
    pub http_headers: Option<HashMap<String, RedactedString>>,
    /// Optional HTTP headers to include in requests to this provider where the
    /// (key, value) pairs are the header name and _environment variable_ whose
    /// value should be used. If the environment variable is not set, or the
    /// value is empty, the header will not be included in the request.
    pub env_http_headers: Option<HashMap<String, String>>,
    /// Maximum number of times to retry a failed HTTP request to this provider.
    pub request_max_retries: Option<u64>,
    /// Number of times to retry reconnecting a dropped streaming response before failing.
    pub stream_max_retries: Option<u64>,
    /// Idle timeout (in milliseconds) to wait for activity on a streaming response before treating
    /// the connection as lost.
    pub stream_idle_timeout_ms: Option<u64>,
    /// Maximum time (in milliseconds) to wait for a websocket connection attempt before treating
    /// it as failed.
    pub websocket_connect_timeout_ms: Option<u64>,
    /// Whether this provider supports the Responses API WebSocket transport.
    #[serde(default)]
    pub supports_websockets: bool,
    /// Whether this provider supports the standalone web-search endpoint.
    #[serde(default)]
    pub supports_standalone_web_search: bool,
}

/// The single provider configuration selected for one Ruxex runtime.
///
/// Configuration files may still use the historical provider map while they
/// are being migrated, but this is the boundary passed to runtime code. A
/// runtime must never receive the map itself.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedModelProvider {
    pub id: String,
    pub info: ModelProviderInfo,
}

impl ModelProviderInfo {
    pub fn validate(&self) -> std::result::Result<(), String> {
        let Some(auth) = self.auth.as_ref() else {
            return Ok(());
        };

        if auth.command.trim().is_empty() {
            return Err("provider auth.command must not be empty".to_string());
        }

        let mut conflicts = Vec::new();
        if self.env_key.is_some() {
            conflicts.push("env_key");
        }
        if self.experimental_bearer_token.is_some() {
            conflicts.push("experimental_bearer_token");
        }
        if conflicts.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "provider auth cannot be combined with {}",
                conflicts.join(", ")
            ))
        }
    }

    fn build_header_map(&self) -> RuxexResult<HeaderMap> {
        let capacity = self.http_headers.as_ref().map_or(0, HashMap::len)
            + self.env_http_headers.as_ref().map_or(0, HashMap::len);
        let mut headers = HeaderMap::with_capacity(capacity);
        if let Some(extra) = &self.http_headers {
            for (k, v) in extra {
                if let (Ok(name), Ok(value)) =
                    (HeaderName::try_from(k), HeaderValue::try_from(v.as_str()))
                {
                    headers.insert(name, value);
                }
            }
        }

        if let Some(env_headers) = &self.env_http_headers {
            for (header, env_var) in env_headers {
                if let Ok(val) = std::env::var(env_var)
                    && !val.trim().is_empty()
                    && let (Ok(name), Ok(value)) =
                        (HeaderName::try_from(header), HeaderValue::try_from(val))
                {
                    headers.insert(name, value);
                }
            }
        }

        Ok(headers)
    }

    /// Builds an API provider with managed residency taking precedence over configured headers.
    pub fn to_api_provider(&self, _auth_mode: Option<AuthMode>) -> RuxexResult<ApiProvider> {
        let default_base_url = "https://example.com/v1";
        let base_url = self
            .base_url
            .clone()
            .unwrap_or_else(|| default_base_url.to_string());

        let mut headers = self.build_header_map()?;
        if let Some(requirement) = read_managed_residency_requirement() {
            let value = match requirement {
                ResidencyRequirement::Us => HeaderValue::from_static("us"),
            };
            headers.insert(RESIDENCY_HEADER_NAME, value);
        }
        let retry = ApiRetryConfig {
            max_attempts: self.request_max_retries(),
            base_delay: Duration::from_millis(200),
            retry_429: false,
            retry_5xx: true,
            retry_transport: true,
        };

        Ok(ApiProvider {
            name: self.name.clone(),
            base_url,
            query_params: self.query_params.clone().map(|params| {
                params
                    .into_iter()
                    .map(|(name, value)| (name, value.into_inner()))
                    .collect()
            }),
            headers,
            retry,
            stream_idle_timeout: self.stream_idle_timeout(),
        })
    }

    /// If `env_key` is Some, returns the API key for this provider if present
    /// (and non-empty) in the environment. If `env_key` is required but
    /// cannot be found, returns an error.
    pub fn api_key(&self) -> RuxexResult<Option<String>> {
        match &self.env_key {
            Some(env_key) => {
                let api_key = std::env::var(env_key)
                    .ok()
                    .filter(|v| !v.trim().is_empty())
                    .ok_or_else(|| {
                        RuxexErr::EnvVar(EnvVarError {
                            var: env_key.clone(),
                            instructions: self.env_key_instructions.clone(),
                        })
                    })?;
                Ok(Some(api_key))
            }
            None => Ok(None),
        }
    }

    /// Effective maximum number of request retries for this provider.
    pub fn request_max_retries(&self) -> u64 {
        self.request_max_retries
            .unwrap_or(DEFAULT_REQUEST_MAX_RETRIES)
            .min(MAX_REQUEST_MAX_RETRIES)
    }

    /// Effective maximum number of stream reconnection attempts for this provider.
    pub fn stream_max_retries(&self) -> u64 {
        self.stream_max_retries
            .unwrap_or(DEFAULT_STREAM_MAX_RETRIES)
            .min(MAX_STREAM_MAX_RETRIES)
    }

    /// Effective idle timeout for streaming responses.
    pub fn stream_idle_timeout(&self) -> Duration {
        self.stream_idle_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_STREAM_IDLE_TIMEOUT_MS))
    }

    /// Effective timeout for websocket connect attempts.
    pub fn websocket_connect_timeout(&self) -> Duration {
        self.websocket_connect_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(DEFAULT_WEBSOCKET_CONNECT_TIMEOUT_MS))
    }

    /// Constructs an explicitly configured Responses provider.
    ///
    /// This helper does not register the provider in the built-in catalog.
    /// Applications that need it must opt in through their provider
    /// configuration instead.
    pub fn create_responses_provider(base_url: Option<String>) -> ModelProviderInfo {
        ModelProviderInfo {
            name: "Responses provider".into(),
            base_url,
            wire_api: WireApi::Responses,
            http_headers: Some(
                [("version".to_string(), env!("CARGO_PKG_VERSION").into())]
                    .into_iter()
                    .collect(),
            ),
            env_http_headers: None,
            supports_websockets: true,
            ..ModelProviderInfo::default()
        }
    }

    pub fn create_cloudflare_provider() -> ModelProviderInfo {
        ModelProviderInfo {
            name: CLOUDFLARE_PROVIDER_NAME.to_string(),
            env_key: Some(CLOUDFLARE_API_TOKEN_ENV_VAR.to_string()),
            env_key_instructions: Some(
                "Set CLOUDFLARE_API_TOKEN to a Cloudflare API token with Workers AI access."
                    .to_string(),
            ),
            wire_api: WireApi::Chat,
            ..ModelProviderInfo::default()
        }
    }

    /// Returns whether this provider uses the Responses wire protocol.
    pub fn is_responses_provider(&self) -> bool {
        self.wire_api == WireApi::Responses
    }

    pub fn has_command_auth(&self) -> bool {
        self.auth.is_some()
    }
}

/// Resolve configuration input to the one provider mounted by a runtime.
///
/// The built-in provider is available as the default. User configuration may
/// replace it with one explicit provider definition, but there is no provider
/// registry or active-provider selector at this boundary.
pub fn resolve_model_provider(
    configured_provider: Option<ModelProviderInfo>,
) -> Result<ResolvedModelProvider, String> {
    let provider_info =
        configured_provider.unwrap_or_else(ModelProviderInfo::create_cloudflare_provider);
    if provider_info.name.trim().is_empty() {
        return Err("model_provider.name must not be empty".to_string());
    }
    provider_info
        .validate()
        .map_err(|message| format!("model_provider: {message}"))?;
    let provider_id = provider_info
        .id
        .clone()
        .unwrap_or_else(|| provider_info.name.clone());
    if provider_id.trim().is_empty() {
        return Err("model_provider.id must not be empty".to_string());
    }

    let mut provider_info = provider_info;
    provider_info.id = Some(provider_id.clone());

    Ok(ResolvedModelProvider {
        id: provider_id,
        info: provider_info,
    })
}

pub fn create_responses_provider_with_base_url(
    base_url: &str,
    wire_api: WireApi,
) -> ModelProviderInfo {
    ModelProviderInfo {
        id: None,
        name: "custom-responses".into(),
        base_url: Some(base_url.into()),
        env_key: None,
        env_key_instructions: None,
        experimental_bearer_token: None,
        auth: None,
        wire_api,
        query_params: None,
        http_headers: None,
        env_http_headers: None,
        request_max_retries: None,
        stream_max_retries: None,
        stream_idle_timeout_ms: None,
        websocket_connect_timeout_ms: None,
        supports_websockets: false,
        supports_standalone_web_search: false,
    }
}

#[cfg(test)]
#[path = "model_provider_info_tests.rs"]
mod tests;

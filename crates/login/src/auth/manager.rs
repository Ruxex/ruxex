use std::env;
use std::fmt::Debug;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::sync::Semaphore;
use tokio::sync::watch;
use tracing::instrument;

use genex_protocol::auth::AuthMode;
use genex_protocol::config_types::ForcedLoginMethod;
use genex_protocol::config_types::ModelProviderAuthInfo;

use super::change_state::AuthChangeState;
use super::change_state::same_owner;
use super::external_bearer::BearerTokenRefresher;
use crate::auth::AuthHeaders;
pub use crate::auth::storage::AuthDotJson;
pub use crate::auth::storage::AuthKeyringBackendKind;
pub use crate::auth::storage::ProviderAuth;
use crate::auth::storage::create_auth_storage;
use crate::outbound_proxy::AuthRouteConfig;
use genex_config::ManagedAuthPolicy;
use genex_config::types::AuthCredentialsStoreMode;
use genex_http_client::HttpClientFactory;
use genex_http_client::OutboundProxyPolicy;
use genex_protocol::auth::RefreshTokenFailedError;
use genex_protocol::auth::RefreshTokenFailedReason;
use thiserror::Error;

/// Authentication mechanism used by the current user.
#[derive(Debug, Clone)]
pub enum GenexAuth {
    ApiKey(ApiKeyAuth),
    Headers(AuthHeaders),
}

impl PartialEq for GenexAuth {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Headers(a), Self::Headers(b)) => a == b,
            _ => self.api_auth_mode() == other.api_auth_mode(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    api_key: String,
}

const REFRESH_TOKEN_UNKNOWN_MESSAGE: &str = "Authentication refresh is unavailable.";
const REFRESH_TOKEN_ACCOUNT_MISMATCH_MESSAGE: &str =
    "Authentication account changed; refresh is unavailable.";

#[derive(Debug, Error)]
pub enum RefreshTokenError {
    #[error("{0}")]
    Permanent(#[from] RefreshTokenFailedError),
    #[error(transparent)]
    Transient(#[from] std::io::Error),
}

/// Error returned when constructing an [`AuthManager`] from resolved configuration.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct AuthManagerInitializationError(AuthManagerInitializationErrorSource);

#[derive(Debug, Error)]
enum AuthManagerInitializationErrorSource {
    #[error(transparent)]
    InitialAuth(RefreshTokenError),
}

impl From<RefreshTokenError> for AuthManagerInitializationError {
    fn from(error: RefreshTokenError) -> Self {
        Self(AuthManagerInitializationErrorSource::InitialAuth(error))
    }
}

impl From<AuthManagerInitializationError> for std::io::Error {
    fn from(error: AuthManagerInitializationError) -> Self {
        Self::other(error)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalAuthRefreshReason {
    Unauthorized,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalAuthRefreshContext {
    pub reason: ExternalAuthRefreshReason,
    pub previous_account_id: Option<String>,
}

/// Pluggable auth provider used by `AuthManager` for externally managed auth flows.
///
/// Implementations own the current auth value and any source-specific refresh mechanism.
pub trait ExternalAuth: Send + Sync {
    /// Returns the provider's current auth value.
    fn resolve(&self) -> ExternalAuthFuture<'_, GenexAuth>;

    /// Refreshes auth and makes the returned value current for future `resolve()` calls.
    fn refresh(&self, context: ExternalAuthRefreshContext) -> ExternalAuthFuture<'_, GenexAuth>;

    /// Maps a provider error into the retry policy used by external-auth reload and recovery.
    fn classify_error(&self, error: std::io::Error) -> RefreshTokenError {
        RefreshTokenError::Transient(error)
    }
}

pub type ExternalAuthFuture<'a, T> = Pin<Box<dyn Future<Output = std::io::Result<T>> + Send + 'a>>;

impl RefreshTokenError {
    pub fn failed_reason(&self) -> Option<RefreshTokenFailedReason> {
        match self {
            Self::Permanent(error) => Some(error.reason),
            Self::Transient(_) => None,
        }
    }
}

impl From<RefreshTokenError> for std::io::Error {
    fn from(err: RefreshTokenError) -> Self {
        match err {
            RefreshTokenError::Permanent(failed) => std::io::Error::other(failed),
            RefreshTokenError::Transient(inner) => inner,
        }
    }
}

impl GenexAuth {
    async fn from_auth_dot_json(
        _genex_home: &Path,
        auth_dot_json: AuthDotJson,
        _auth_credentials_store_mode: AuthCredentialsStoreMode,
        _managed_auth_base_url: Option<&str>,
        _keyring_backend_kind: AuthKeyringBackendKind,
        _auth_route_config: &AuthRouteConfig,
    ) -> std::io::Result<Self> {
        let Some(credentials) = auth_dot_json.active_auth() else {
            return Err(std::io::Error::other("stored auth has no active provider"));
        };
        let auth_mode = credentials.resolved_mode();
        if auth_mode == AuthMode::ApiKey {
            let Some(api_key) = credentials.api_key.as_deref() else {
                return Err(std::io::Error::other("API key auth is missing a key."));
            };
            return Ok(Self::from_api_key(api_key));
        }
        if auth_mode == AuthMode::Headers {
            return Err(std::io::Error::other(
                "externally provided auth cannot be loaded from auth storage.",
            ));
        }

        match auth_mode {
            AuthMode::ApiKey => unreachable!("api key mode is handled above"),
            AuthMode::Headers => {
                unreachable!("externally provided auth is never loaded from auth storage")
            }
        }
    }

    pub async fn from_auth_storage(
        genex_home: &Path,
        auth_credentials_store_mode: AuthCredentialsStoreMode,
        managed_auth_base_url: Option<&str>,
        keyring_backend_kind: AuthKeyringBackendKind,
        auth_route_config: &AuthRouteConfig,
    ) -> std::io::Result<Option<Self>> {
        load_auth(
            genex_home,
            /*enable_api_key_env*/ false,
            auth_credentials_store_mode,
            /*allowed_login_methods*/ None,
            /*forced_managed_workspace_id*/ None,
            managed_auth_base_url,
            keyring_backend_kind,
            auth_route_config,
        )
        .await
    }

    /// Returns the effective backend auth mode.
    ///
    pub fn auth_mode(&self) -> AuthMode {
        match self {
            Self::ApiKey(_) => AuthMode::ApiKey,
            Self::Headers(_) => AuthMode::Headers,
        }
    }

    /// Returns the precise kind of credentials backing this authentication.
    pub fn api_auth_mode(&self) -> AuthMode {
        match self {
            Self::ApiKey(_) => AuthMode::ApiKey,
            Self::Headers(_) => AuthMode::Headers,
        }
    }

    pub fn is_api_key_auth(&self) -> bool {
        self.auth_mode() == AuthMode::ApiKey
    }

    pub fn uses_managed_auth(&self) -> bool {
        matches!(self, Self::Headers(_))
    }

    fn supports_unauthorized_recovery(&self) -> bool {
        matches!(self, Self::Headers(_))
    }

    /// Returns `None` if `auth_mode() != AuthMode::ApiKey`.
    pub fn api_key(&self) -> Option<&str> {
        match self {
            Self::ApiKey(auth) => Some(auth.api_key.as_str()),
            Self::Headers(_) => None,
        }
    }

    /// Returns the token string used for bearer authentication.
    pub fn get_token(&self) -> Result<String, std::io::Error> {
        match self {
            Self::ApiKey(auth) => Ok(auth.api_key.clone()),
            Self::Headers(_) => Err(std::io::Error::other(
                "header auth does not expose a bearer token",
            )),
        }
    }

    /// Returns an optional provider-supplied identity for managed auth.
    pub fn get_account_id(&self) -> Option<String> {
        match self {
            Self::Headers(headers) => headers
                .account_id()
                .filter(|account_id| !account_id.is_empty() && account_id.trim() == *account_id)
                .map(ToOwned::to_owned),
            _ => None,
        }
    }

    pub fn from_api_key(api_key: &str) -> Self {
        Self::ApiKey(ApiKeyAuth {
            api_key: api_key.to_owned(),
        })
    }
}

pub const GENEX_API_KEY_ENV_VAR: &str = "GENEX_API_KEY";

pub fn read_api_key_from_env() -> Option<String> {
    read_non_empty_env_var(GENEX_API_KEY_ENV_VAR)
}

fn read_non_empty_env_var(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Delete the auth.json file inside `genex_home` if it exists. Returns `Ok(true)`
/// if a file was removed, `Ok(false)` if no auth file was present.
pub fn logout(
    genex_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<bool> {
    let storage = create_auth_storage(
        genex_home.to_path_buf(),
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    storage.delete()
}

/// Writes an `auth.json` that contains only the API key.
pub fn login_with_api_key(
    genex_home: &Path,
    api_key: &str,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<()> {
    let auth_dot_json = AuthDotJson::legacy(Some(AuthMode::ApiKey), Some(api_key.to_string()));
    save_auth(
        genex_home,
        &auth_dot_json,
        auth_credentials_store_mode,
        keyring_backend_kind,
    )
}

/// Persist the provided auth payload using the specified backend.
pub fn save_auth(
    genex_home: &Path,
    auth: &AuthDotJson,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<()> {
    let storage = create_auth_storage(
        genex_home.to_path_buf(),
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    storage.save(auth)
}

/// Load the raw stored auth payload without applying environment overrides.
///
/// Returns `None` when no credentials are stored. Prefer `AuthManager` for
/// ordinary production reads; this helper is for tests and write-side
/// maintenance that must inspect the exact payload in storage.
pub fn load_auth_dot_json(
    genex_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<Option<AuthDotJson>> {
    let storage = create_auth_storage(
        genex_home.to_path_buf(),
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    storage.load()
}

/// Load credentials for one model provider without exposing other providers' payloads.
pub fn load_provider_auth(
    genex_home: &Path,
    provider_id: &str,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<Option<ProviderAuth>> {
    validate_provider_id(provider_id)?;
    let storage = create_auth_storage(
        genex_home.to_path_buf(),
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    Ok(storage
        .load()?
        .and_then(|auth| auth.provider_auth(provider_id).cloned()))
}

/// Persist credentials for one model provider while retaining every other provider record.
pub fn save_provider_auth(
    genex_home: &Path,
    provider_id: &str,
    provider_auth: &ProviderAuth,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<()> {
    validate_provider_id(provider_id)?;
    let storage = create_auth_storage(
        genex_home.to_path_buf(),
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    let current = storage.load()?.unwrap_or_default();
    let mut auth = current.into_provider_storage();
    auth.providers
        .insert(provider_id.to_string(), provider_auth.clone());
    storage.save(&auth)
}

/// Delete credentials for one model provider without deleting other providers.
pub fn delete_provider_auth(
    genex_home: &Path,
    provider_id: &str,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<bool> {
    validate_provider_id(provider_id)?;
    let storage = create_auth_storage(
        genex_home.to_path_buf(),
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    let Some(current) = storage.load()? else {
        return Ok(false);
    };
    if current.providers.contains_key(provider_id) {
        let mut auth = current.into_provider_storage();
        let removed = auth.providers.remove(provider_id).is_some();
        if auth.active_provider.as_deref() == Some(provider_id) {
            auth.active_provider = None;
        }
        if auth.providers.is_empty() && auth.active_provider.is_none() {
            storage.delete()?;
        } else {
            storage.save(&auth)?;
        }
        return Ok(removed);
    }
    if current.has_legacy_auth() {
        storage.delete()?;
        return Ok(true);
    }
    Ok(false)
}

/// List provider IDs with persisted provider-scoped credentials.
pub fn list_provider_ids(
    genex_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<Vec<String>> {
    let storage = create_auth_storage(
        genex_home.to_path_buf(),
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    Ok(storage
        .load()?
        .map(|auth| auth.providers.into_keys().collect())
        .unwrap_or_default())
}

fn validate_provider_id(provider_id: &str) -> std::io::Result<()> {
    if provider_id.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "provider ID must not be empty",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthConfig {
    pub genex_home: PathBuf,
    pub auth_credentials_store_mode: AuthCredentialsStoreMode,
    pub keyring_backend_kind: AuthKeyringBackendKind,
    pub forced_login_method: Option<ForcedLoginMethod>,
    pub managed_auth_base_url: Option<String>,
    pub forced_managed_workspace_id: Option<Vec<String>>,
    pub managed_auth_policy: ManagedAuthPolicy,
    pub auth_route_config: AuthRouteConfig,
}

impl AuthConfig {
    pub fn is_login_method_allowed(&self, method: ForcedLoginMethod) -> bool {
        self.managed_auth_policy.allows_login_method(
            method,
            self.forced_login_method,
            self.forced_managed_workspace_id.as_deref(),
        )
    }

    pub fn effective_managed_workspaces(&self) -> Option<Vec<String>> {
        self.managed_auth_policy
            .effective_managed_workspaces(self.forced_managed_workspace_id.as_deref())
    }

    pub fn validate(&self) -> std::io::Result<()> {
        if self.is_login_method_allowed(ForcedLoginMethod::Api)
            || self.is_login_method_allowed(ForcedLoginMethod::Managed)
        {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "authentication requirements do not permit any usable login method",
            ))
        }
    }

    pub fn allows_auth(&self, auth: &GenexAuth) -> bool {
        let allowed_login_methods = self.allowed_login_methods();
        let workspaces = self.effective_managed_workspaces();
        validate_auth_restrictions(Some(&allowed_login_methods), workspaces.as_deref(), auth)
            .is_ok()
    }

    pub async fn load_auth(&self, enable_api_key_env: bool) -> std::io::Result<Option<GenexAuth>> {
        let allowed_login_methods = self.allowed_login_methods();
        let workspaces = self.effective_managed_workspaces();
        let auth = load_auth(
            &self.genex_home,
            enable_api_key_env,
            self.auth_credentials_store_mode,
            Some(&allowed_login_methods),
            workspaces.as_deref(),
            self.managed_auth_base_url.as_deref(),
            self.keyring_backend_kind,
            &self.auth_route_config,
        )
        .await?;
        Ok(auth.filter(|auth| self.allows_auth(auth)))
    }

    fn allowed_login_methods(&self) -> Vec<ForcedLoginMethod> {
        self.managed_auth_policy.allowed_login_methods(
            self.forced_login_method,
            self.forced_managed_workspace_id.as_deref(),
        )
    }
}

fn auth_mode_is_allowed(
    allowed_login_methods: Option<&[ForcedLoginMethod]>,
    mode: AuthMode,
) -> bool {
    let method = if mode.uses_managed_auth() {
        ForcedLoginMethod::Managed
    } else {
        ForcedLoginMethod::Api
    };
    allowed_login_methods.is_none_or(|allowed| allowed.contains(&method))
}

fn validate_auth_restrictions(
    allowed_login_methods: Option<&[ForcedLoginMethod]>,
    expected_workspaces: Option<&[String]>,
    auth: &GenexAuth,
) -> Result<(), String> {
    if !auth_mode_is_allowed(allowed_login_methods, auth.auth_mode()) {
        return Err(match allowed_login_methods {
            Some(methods) if methods.contains(&ForcedLoginMethod::Api) => {
                "API key login is required".to_string()
            }
            Some(_) => "Managed authentication is required".to_string(),
            None => unreachable!("unrestricted login methods accept every auth mode"),
        });
    }

    let Some(expected_workspaces) = expected_workspaces else {
        return Ok(());
    };
    if matches!(auth, GenexAuth::ApiKey(_)) {
        return Ok(());
    }

    let actual_workspace = auth.get_account_id();
    if actual_workspace
        .as_ref()
        .is_some_and(|workspace| expected_workspaces.contains(workspace))
    {
        Ok(())
    } else {
        let actual = actual_workspace.unwrap_or_else(|| "unknown".to_string());
        Err(format!(
            "Login is restricted to workspace(s) {}, but current credentials belong to {actual}",
            expected_workspaces.join(", ")
        ))
    }
}

/// Enforces configured login restrictions using auth-owned HTTP settings.
pub async fn enforce_login_restrictions(config: &AuthConfig) -> std::io::Result<()> {
    // Managed-only restrictions are enforced by AuthManager.
    if config.forced_login_method.is_none() && config.forced_managed_workspace_id.is_none() {
        return Ok(());
    }

    let Some(auth) = load_auth(
        &config.genex_home,
        /*enable_api_key_env*/ true,
        config.auth_credentials_store_mode,
        /*allowed_login_methods*/ None,
        /*forced_managed_workspace_id*/ None,
        config.managed_auth_base_url.as_deref(),
        config.keyring_backend_kind,
        &config.auth_route_config,
    )
    .await?
    else {
        return Ok(());
    };

    if let Some(required_method) = config.forced_login_method {
        let method_violation = match (required_method, auth.auth_mode()) {
            (ForcedLoginMethod::Api, AuthMode::ApiKey) => None,
            (ForcedLoginMethod::Managed, AuthMode::Headers) => None,
            (ForcedLoginMethod::Api, AuthMode::Headers) => Some(
                "API key login is required, but managed authentication is currently being used. Logging out."
                    .to_string(),
            ),
            (ForcedLoginMethod::Managed, AuthMode::ApiKey) => Some(
                "Managed authentication is required, but an API key is currently being used. Logging out."
                    .to_string(),
            ),
        };

        if let Some(message) = method_violation {
            return logout_with_message(
                &config.genex_home,
                message,
                config.auth_credentials_store_mode,
                config.keyring_backend_kind,
            );
        }
    }

    if let Some(expected_account_ids) = config.forced_managed_workspace_id.as_deref() {
        let workspace_id = match &auth {
            GenexAuth::ApiKey(_) => {
                return Ok(());
            }
            GenexAuth::Headers(_) => auth.get_account_id(),
        };

        // workspace is the external identifier for account id.
        let workspace_id = workspace_id.as_deref();
        if !workspace_id.is_some_and(|actual| {
            expected_account_ids
                .iter()
                .any(|expected| expected == actual)
        }) {
            let expected_workspaces = expected_account_ids.join(", ");
            let message = match workspace_id {
                Some(actual) => format!(
                    "Login is restricted to workspace(s) {expected_workspaces}, but current credentials belong to {actual}. Logging out."
                ),
                None => format!(
                    "Login is restricted to workspace(s) {expected_workspaces}, but current credentials lack a workspace identifier. Logging out."
                ),
            };
            return logout_with_message(
                &config.genex_home,
                message,
                config.auth_credentials_store_mode,
                config.keyring_backend_kind,
            );
        }
    }

    Ok(())
}

fn logout_with_message(
    genex_home: &Path,
    message: String,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<()> {
    // External auth tokens live in the ephemeral store, but persistent auth may still exist
    // from earlier logins. Clear both so a forced logout truly removes all active auth.
    let removal_result = logout_all_stores(
        genex_home,
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    let error_message = match removal_result {
        Ok(_) => message,
        Err(err) => format!("{message}. Failed to remove auth.json: {err}"),
    };
    Err(std::io::Error::other(error_message))
}

fn logout_all_stores(
    genex_home: &Path,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> std::io::Result<bool> {
    if auth_credentials_store_mode == AuthCredentialsStoreMode::Ephemeral {
        return logout(
            genex_home,
            AuthCredentialsStoreMode::Ephemeral,
            AuthKeyringBackendKind::default(),
        );
    }
    let removed_ephemeral = logout(
        genex_home,
        AuthCredentialsStoreMode::Ephemeral,
        AuthKeyringBackendKind::default(),
    )?;
    let removed_managed = logout(
        genex_home,
        auth_credentials_store_mode,
        keyring_backend_kind,
    )?;
    Ok(removed_ephemeral || removed_managed)
}

#[allow(clippy::too_many_arguments)]
async fn load_auth(
    genex_home: &Path,
    enable_api_key_env: bool,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    allowed_login_methods: Option<&[ForcedLoginMethod]>,
    _forced_managed_workspace_id: Option<&[String]>,
    managed_auth_base_url: Option<&str>,
    keyring_backend_kind: AuthKeyringBackendKind,
    auth_route_config: &AuthRouteConfig,
) -> std::io::Result<Option<GenexAuth>> {
    // API key via env var takes precedence over any other auth method.
    if enable_api_key_env
        && auth_mode_is_allowed(allowed_login_methods, AuthMode::ApiKey)
        && let Some(api_key) = read_api_key_from_env()
    {
        return Ok(Some(GenexAuth::from_api_key(api_key.as_str())));
    }

    // Ephemeral credentials take precedence over persisted credentials.
    let ephemeral_storage = create_auth_storage(
        genex_home.to_path_buf(),
        AuthCredentialsStoreMode::Ephemeral,
        AuthKeyringBackendKind::default(),
    );
    if let Some(auth_dot_json) = ephemeral_storage.load()?
        && auth_dot_json.has_supported_credentials()
        && auth_mode_is_allowed(allowed_login_methods, auth_dot_json.resolved_mode())
    {
        let auth = GenexAuth::from_auth_dot_json(
            genex_home,
            auth_dot_json,
            AuthCredentialsStoreMode::Ephemeral,
            managed_auth_base_url,
            keyring_backend_kind,
            auth_route_config,
        )
        .await?;
        return Ok(Some(auth));
    }

    // If the caller explicitly requested ephemeral auth, there is no persisted fallback.
    if auth_credentials_store_mode == AuthCredentialsStoreMode::Ephemeral {
        return Ok(None);
    }

    // Fall back to the configured persistent store (file/keyring/auto).
    let storage = create_auth_storage(
        genex_home.to_path_buf(),
        auth_credentials_store_mode,
        keyring_backend_kind,
    );
    let auth_dot_json = match storage.load()? {
        Some(auth) => auth,
        None => return Ok(None),
    };
    if !auth_dot_json.has_supported_credentials() {
        return Ok(None);
    }
    if !auth_mode_is_allowed(allowed_login_methods, auth_dot_json.resolved_mode()) {
        return Ok(None);
    }
    let auth = GenexAuth::from_auth_dot_json(
        genex_home,
        auth_dot_json,
        auth_credentials_store_mode,
        managed_auth_base_url,
        keyring_backend_kind,
        auth_route_config,
    )
    .await?;
    Ok(Some(auth))
}

impl AuthDotJson {
    fn has_supported_credentials(&self) -> bool {
        self.active_auth()
            .is_some_and(ProviderAuth::has_supported_credentials)
    }

    pub(super) fn resolved_mode(&self) -> AuthMode {
        self.active_auth()
            .map(ProviderAuth::resolved_mode)
            .unwrap_or(AuthMode::ApiKey)
    }
}

/// Internal cached auth state.
#[derive(Clone)]
struct CachedAuth {
    auth: Option<GenexAuth>,
    /// Permanent refresh failure cached for the current auth snapshot so
    /// later refresh attempts for the same credentials fail fast without network.
    permanent_refresh_failure: Option<AuthScopedRefreshFailure>,
}

#[derive(Clone)]
struct AuthScopedRefreshFailure {
    auth: GenexAuth,
    error: RefreshTokenFailedError,
}

impl Debug for CachedAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedAuth")
            .field(
                "auth_mode",
                &self.auth.as_ref().map(GenexAuth::api_auth_mode),
            )
            .field(
                "permanent_refresh_failure",
                &self
                    .permanent_refresh_failure
                    .as_ref()
                    .map(|failure| failure.error.reason),
            )
            .finish()
    }
}

enum UnauthorizedRecoveryStep {
    Reload,
    RefreshToken,
    ExternalRefresh,
    Done,
}

enum ReloadOutcome {
    /// Reload was performed and the cached auth changed
    ReloadedChanged,
    /// Reload was performed and the cached auth remained the same
    ReloadedNoChange,
    /// Reload was skipped (missing or mismatched account id)
    Skipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnauthorizedRecoveryMode {
    Managed,
    External,
}

// UnauthorizedRecovery is a state machine that handles an attempt to refresh the authentication when requests
// to API fail with 401 status code.
// The client calls next() every time it encounters a 401 error, one time per retry.
// For API key based authentication, we don't do anything and let the error bubble to the user.
//
// For external auth sources, UnauthorizedRecovery retries once by asking the
// configured provider to refresh and caching the returned auth through the same
// path used by other auth sources.
pub struct UnauthorizedRecovery {
    manager: Arc<AuthManager>,
    step: UnauthorizedRecoveryStep,
    expected_account_id: Option<String>,
    mode: UnauthorizedRecoveryMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnauthorizedRecoveryStepResult {
    auth_state_changed: Option<bool>,
}

impl UnauthorizedRecoveryStepResult {
    pub fn auth_state_changed(&self) -> Option<bool> {
        self.auth_state_changed
    }
}

impl UnauthorizedRecovery {
    fn new(manager: Arc<AuthManager>) -> Self {
        let cached_auth = manager.auth_cached();
        let expected_account_id = cached_auth.as_ref().and_then(GenexAuth::get_account_id);
        let mode = if manager.has_external_auth() {
            UnauthorizedRecoveryMode::External
        } else {
            UnauthorizedRecoveryMode::Managed
        };
        let step = match mode {
            UnauthorizedRecoveryMode::Managed => UnauthorizedRecoveryStep::Reload,
            UnauthorizedRecoveryMode::External => UnauthorizedRecoveryStep::ExternalRefresh,
        };
        Self {
            manager,
            step,
            expected_account_id,
            mode,
        }
    }

    pub fn has_next(&self) -> bool {
        if self.manager.has_refreshable_external_auth() {
            return !matches!(self.step, UnauthorizedRecoveryStep::Done);
        }

        if !self
            .manager
            .auth_cached()
            .as_ref()
            .is_some_and(GenexAuth::supports_unauthorized_recovery)
        {
            return false;
        }

        if self.mode == UnauthorizedRecoveryMode::External && !self.manager.has_external_auth() {
            return false;
        }

        !matches!(self.step, UnauthorizedRecoveryStep::Done)
    }

    pub fn unavailable_reason(&self) -> &'static str {
        if self.manager.has_refreshable_external_auth() {
            return if matches!(self.step, UnauthorizedRecoveryStep::Done) {
                "recovery_exhausted"
            } else {
                "ready"
            };
        }

        if !self
            .manager
            .auth_cached()
            .as_ref()
            .is_some_and(GenexAuth::supports_unauthorized_recovery)
        {
            return "not_managed_auth";
        }

        if self.mode == UnauthorizedRecoveryMode::External && !self.manager.has_external_auth() {
            return "no_external_auth";
        }

        if matches!(self.step, UnauthorizedRecoveryStep::Done) {
            return "recovery_exhausted";
        }

        "ready"
    }

    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            UnauthorizedRecoveryMode::Managed => "managed",
            UnauthorizedRecoveryMode::External => "external",
        }
    }

    pub fn step_name(&self) -> &'static str {
        match self.step {
            UnauthorizedRecoveryStep::Reload => "reload",
            UnauthorizedRecoveryStep::RefreshToken => "refresh_token",
            UnauthorizedRecoveryStep::ExternalRefresh => "external_refresh",
            UnauthorizedRecoveryStep::Done => "done",
        }
    }

    pub async fn next(&mut self) -> Result<UnauthorizedRecoveryStepResult, RefreshTokenError> {
        if !self.has_next() {
            return Err(RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                RefreshTokenFailedReason::Other,
                "No more recovery steps available.",
            )));
        }

        match self.step {
            UnauthorizedRecoveryStep::Reload => {
                match self
                    .manager
                    .reload_if_account_id_matches(self.expected_account_id.as_deref())
                    .await
                {
                    ReloadOutcome::ReloadedChanged => {
                        self.step = UnauthorizedRecoveryStep::RefreshToken;
                        return Ok(UnauthorizedRecoveryStepResult {
                            auth_state_changed: Some(true),
                        });
                    }
                    ReloadOutcome::ReloadedNoChange => {
                        self.step = UnauthorizedRecoveryStep::RefreshToken;
                        return Ok(UnauthorizedRecoveryStepResult {
                            auth_state_changed: Some(false),
                        });
                    }
                    ReloadOutcome::Skipped => {
                        self.step = UnauthorizedRecoveryStep::Done;
                        return Err(RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                            RefreshTokenFailedReason::Other,
                            REFRESH_TOKEN_ACCOUNT_MISMATCH_MESSAGE.to_string(),
                        )));
                    }
                }
            }
            UnauthorizedRecoveryStep::RefreshToken => {
                self.manager.refresh_token_from_authority().await?;
                self.step = UnauthorizedRecoveryStep::Done;
                return Ok(UnauthorizedRecoveryStepResult {
                    auth_state_changed: Some(true),
                });
            }
            UnauthorizedRecoveryStep::ExternalRefresh => {
                self.manager.refresh_token_from_authority().await?;
                self.step = UnauthorizedRecoveryStep::Done;
                return Ok(UnauthorizedRecoveryStepResult {
                    auth_state_changed: Some(true),
                });
            }
            UnauthorizedRecoveryStep::Done => {}
        }
        Ok(UnauthorizedRecoveryStepResult {
            auth_state_changed: None,
        })
    }
}

/// Central manager providing a single source of truth for auth.json derived
/// authentication data. It loads once (or on preference change) and then
/// hands out cloned `GenexAuth` values so the rest of the program has a
/// consistent snapshot.
///
/// External modifications to `auth.json` will NOT be observed until
/// `reload()` is called explicitly. This matches the design goal of avoiding
/// different parts of the program seeing inconsistent auth data mid‑run.
pub struct AuthManager {
    genex_home: PathBuf,
    inner: RwLock<CachedAuth>,
    auth_change_tx: watch::Sender<u64>,
    auth_change_state_tx: watch::Sender<AuthChangeState>,
    enable_api_key_env: bool,
    auth_credentials_store_mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
    forced_login_method: Option<ForcedLoginMethod>,
    forced_managed_workspace_id: RwLock<Option<Vec<String>>>,
    managed_auth_policy: ManagedAuthPolicy,
    managed_auth_base_url: Option<String>,
    refresh_lock: Semaphore,
    external_auth: RwLock<Option<Arc<dyn ExternalAuth>>>,
    auth_route_config: AuthRouteConfig,
}

/// Configuration view required to construct a shared [`AuthManager`].
///
/// Implementations should return the auth-related config values for the
/// already-resolved runtime configuration. The primary implementation is
/// `genex_core::config::Config`, but this trait keeps `genex-login` independent
/// from `genex-core`.
pub trait AuthManagerConfig {
    /// Returns the Genex home directory used for auth storage.
    fn genex_home(&self) -> PathBuf;

    /// Returns the CLI auth credential storage mode for auth loading.
    fn cli_auth_credentials_store_mode(&self) -> AuthCredentialsStoreMode;

    /// Returns the backend to use when CLI auth keyring storage is selected.
    fn auth_keyring_backend_kind(&self) -> AuthKeyringBackendKind;

    /// Returns the resolved login-method restriction, if any.
    fn forced_login_method(&self) -> Option<ForcedLoginMethod>;

    /// Returns the workspace IDs that managed auth should be restricted to, if any.
    fn forced_managed_workspace_id(&self) -> Option<Vec<String>>;

    /// Returns administrator-managed authentication restrictions.
    fn managed_auth_policy(&self) -> ManagedAuthPolicy;

    /// Returns the base URL used for host-managed authorization.
    fn managed_auth_base_url(&self) -> String;

    /// Returns route-selection settings for auth-owned clients.
    fn auth_route_config(&self) -> AuthRouteConfig;
}

impl Debug for AuthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthManager")
            .field("genex_home", &self.genex_home)
            .field("inner", &self.inner)
            .field("enable_api_key_env", &self.enable_api_key_env)
            .field(
                "auth_credentials_store_mode",
                &self.auth_credentials_store_mode,
            )
            .field("keyring_backend_kind", &self.keyring_backend_kind)
            .field("forced_login_method", &self.forced_login_method)
            .field(
                "forced_managed_workspace_id",
                &self.forced_managed_workspace_id,
            )
            .field("managed_auth_policy", &self.managed_auth_policy)
            .field("managed_auth_base_url", &self.managed_auth_base_url)
            .field("auth_route_config", &self.auth_route_config)
            .field("has_external_auth", &self.has_external_auth())
            .finish_non_exhaustive()
    }
}

impl AuthManager {
    /// Create a new manager loading the initial auth using the provided
    /// preferred auth method. Errors loading auth are swallowed; `auth()` will
    /// simply return `None` in that case so callers can treat it as an
    /// unauthenticated state.
    pub async fn new(
        genex_home: PathBuf,
        enable_api_key_env: bool,
        auth_credentials_store_mode: AuthCredentialsStoreMode,
        forced_managed_workspace_id: Option<Vec<String>>,
        managed_auth_base_url: Option<String>,
        keyring_backend_kind: AuthKeyringBackendKind,
        auth_route_config: AuthRouteConfig,
    ) -> Self {
        Self::new_from_auth_config(
            AuthConfig {
                genex_home,
                auth_credentials_store_mode,
                keyring_backend_kind,
                forced_login_method: None,
                managed_auth_base_url,
                forced_managed_workspace_id,
                managed_auth_policy: ManagedAuthPolicy::default(),
                auth_route_config,
            },
            enable_api_key_env,
        )
        .await
    }

    async fn new_from_auth_config(auth_config: AuthConfig, enable_api_key_env: bool) -> Self {
        let managed_auth = auth_config
            .load_auth(enable_api_key_env)
            .await
            .ok()
            .flatten();
        let AuthConfig {
            genex_home,
            auth_credentials_store_mode,
            keyring_backend_kind,
            forced_login_method,
            managed_auth_base_url,
            forced_managed_workspace_id,
            managed_auth_policy,
            auth_route_config,
        } = auth_config;
        let (auth_change_tx, _auth_change_rx) = watch::channel(0);
        Self {
            genex_home,
            inner: RwLock::new(CachedAuth {
                auth: managed_auth,
                permanent_refresh_failure: None,
            }),
            auth_change_tx,
            auth_change_state_tx: watch::channel(AuthChangeState::default()).0,
            enable_api_key_env,
            auth_credentials_store_mode,
            keyring_backend_kind,
            forced_login_method,
            forced_managed_workspace_id: RwLock::new(forced_managed_workspace_id),
            managed_auth_policy,
            managed_auth_base_url,
            refresh_lock: Semaphore::new(/*permits*/ 1),
            external_auth: RwLock::new(None),
            auth_route_config,
        }
    }

    /// Create an AuthManager with a specific GenexAuth, for testing only.
    pub fn from_auth_for_testing(auth: GenexAuth) -> Arc<Self> {
        Self::from_optional_auth_for_testing(Some(auth))
    }

    /// Create an AuthManager with an optional GenexAuth, for testing only.
    pub(crate) fn from_optional_auth_for_testing(auth: Option<GenexAuth>) -> Arc<Self> {
        let cached = CachedAuth {
            auth,
            permanent_refresh_failure: None,
        };
        let (auth_change_tx, _auth_change_rx) = watch::channel(0);

        Arc::new(Self {
            genex_home: PathBuf::from("non-existent"),
            inner: RwLock::new(cached),
            auth_change_tx,
            auth_change_state_tx: watch::channel(AuthChangeState::default()).0,
            enable_api_key_env: false,
            auth_credentials_store_mode: AuthCredentialsStoreMode::File,
            keyring_backend_kind: AuthKeyringBackendKind::default(),
            forced_login_method: None,
            forced_managed_workspace_id: RwLock::new(None),
            managed_auth_policy: ManagedAuthPolicy::default(),
            managed_auth_base_url: None,
            refresh_lock: Semaphore::new(/*permits*/ 1),
            external_auth: RwLock::new(None),
            auth_route_config: crate::test_support::transport_default_auth_route_config(),
        })
    }

    /// Create an AuthManager with a specific GenexAuth and genex home, for testing only.
    pub fn from_auth_for_testing_with_home(auth: GenexAuth, genex_home: PathBuf) -> Arc<Self> {
        let cached = CachedAuth {
            auth: Some(auth),
            permanent_refresh_failure: None,
        };
        let (auth_change_tx, _auth_change_rx) = watch::channel(0);
        Arc::new(Self {
            genex_home,
            inner: RwLock::new(cached),
            auth_change_tx,
            auth_change_state_tx: watch::channel(AuthChangeState::default()).0,
            enable_api_key_env: false,
            auth_credentials_store_mode: AuthCredentialsStoreMode::File,
            keyring_backend_kind: AuthKeyringBackendKind::default(),
            forced_login_method: None,
            forced_managed_workspace_id: RwLock::new(None),
            managed_auth_policy: ManagedAuthPolicy::default(),
            managed_auth_base_url: None,
            refresh_lock: Semaphore::new(/*permits*/ 1),
            external_auth: RwLock::new(None),
            auth_route_config: crate::test_support::transport_default_auth_route_config(),
        })
    }

    pub fn external_bearer_only(config: ModelProviderAuthInfo) -> Arc<Self> {
        let (auth_change_tx, _auth_change_rx) = watch::channel(0);
        Arc::new(Self {
            genex_home: PathBuf::from("non-existent"),
            inner: RwLock::new(CachedAuth {
                auth: None,
                permanent_refresh_failure: None,
            }),
            auth_change_tx,
            auth_change_state_tx: watch::channel(AuthChangeState::default()).0,
            enable_api_key_env: false,
            auth_credentials_store_mode: AuthCredentialsStoreMode::File,
            keyring_backend_kind: AuthKeyringBackendKind::default(),
            forced_login_method: None,
            forced_managed_workspace_id: RwLock::new(None),
            managed_auth_policy: ManagedAuthPolicy::default(),
            managed_auth_base_url: None,
            refresh_lock: Semaphore::new(/*permits*/ 1),
            external_auth: RwLock::new(Some(Arc::new(BearerTokenRefresher::new(config)))),
            // External bearer auth refreshes by running the provider's command and never makes
            // auth-owned HTTP requests, so this route is intentionally inert.
            auth_route_config: AuthRouteConfig::from_http_client_factory(HttpClientFactory::new(
                OutboundProxyPolicy::ReqwestDefault,
            )),
        })
    }

    /// Current cached auth (clone) without attempting a refresh.
    pub fn auth_cached(&self) -> Option<GenexAuth> {
        self.inner
            .read()
            .ok()
            .and_then(|cached| cached.auth.clone())
    }

    /// Subscribes to cached auth changes that can affect request recovery.
    pub fn auth_change_receiver(&self) -> watch::Receiver<u64> {
        self.auth_change_tx.subscribe()
    }

    /// Subscribes to credential and owner revisions published together, including when changes coalesce.
    pub fn auth_change_state_receiver(&self) -> watch::Receiver<AuthChangeState> {
        self.auth_change_state_tx.subscribe()
    }

    pub fn refresh_failure_for_auth(&self, auth: &GenexAuth) -> Option<RefreshTokenFailedError> {
        self.inner.read().ok().and_then(|cached| {
            cached
                .permanent_refresh_failure
                .as_ref()
                .filter(|failure| Self::auths_equal_for_refresh(Some(auth), Some(&failure.auth)))
                .map(|failure| failure.error.clone())
        })
    }

    /// Current cached auth (clone). May be `None` if not logged in or load failed.
    /// Resolve current authentication, refreshing configured external credentials first.
    #[instrument(level = "trace", skip_all)]
    pub async fn auth(&self) -> Option<GenexAuth> {
        if self.has_external_auth() {
            self.reload().await;
            return self.auth_cached();
        }

        self.auth_cached()
    }

    /// Reloads auth from the active source. Returns whether the auth value changed.
    pub async fn reload(&self) -> bool {
        tracing::info!("Reloading auth");
        let new_auth = self.load_auth().await;
        self.set_cached_auth(new_auth)
    }

    async fn reload_if_account_id_matches(
        &self,
        expected_account_id: Option<&str>,
    ) -> ReloadOutcome {
        let expected_account_id = match expected_account_id {
            Some(account_id) => account_id,
            None => {
                tracing::info!("Skipping auth reload because no account id is available.");
                return ReloadOutcome::Skipped;
            }
        };

        let new_auth = self.load_auth().await;
        let new_account_id = new_auth.as_ref().and_then(GenexAuth::get_account_id);

        if new_account_id.as_deref() != Some(expected_account_id) {
            let found_account_id = new_account_id.as_deref().unwrap_or("unknown");
            tracing::info!(
                "Skipping auth reload due to account id mismatch (expected: {expected_account_id}, found: {found_account_id})"
            );
            return ReloadOutcome::Skipped;
        }

        tracing::info!("Reloading auth for account {expected_account_id}");
        let cached_before_reload = self.auth_cached();
        let auth_changed =
            !Self::auths_equal_for_refresh(cached_before_reload.as_ref(), new_auth.as_ref());
        self.set_cached_auth(new_auth);
        if auth_changed {
            ReloadOutcome::ReloadedChanged
        } else {
            ReloadOutcome::ReloadedNoChange
        }
    }

    fn auths_equal_for_refresh(a: Option<&GenexAuth>, b: Option<&GenexAuth>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => match (a.api_auth_mode(), b.api_auth_mode()) {
                (AuthMode::ApiKey, AuthMode::ApiKey) => a.api_key() == b.api_key(),
                (AuthMode::Headers, AuthMode::Headers) => a == b,
                _ => false,
            },
            _ => false,
        }
    }

    fn auths_equal(a: Option<&GenexAuth>, b: Option<&GenexAuth>) -> bool {
        match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }

    /// Records a permanent refresh failure only if the failed refresh was
    /// attempted against the auth snapshot that is still cached.
    fn record_permanent_refresh_failure_if_unchanged(
        &self,
        attempted_auth: &GenexAuth,
        error: &RefreshTokenFailedError,
    ) {
        if let Ok(mut guard) = self.inner.write() {
            let current_auth_matches =
                Self::auths_equal_for_refresh(Some(attempted_auth), guard.auth.as_ref());
            if current_auth_matches {
                guard.permanent_refresh_failure = Some(AuthScopedRefreshFailure {
                    auth: attempted_auth.clone(),
                    error: error.clone(),
                });
            }
        }
    }

    async fn load_auth(&self) -> Option<GenexAuth> {
        if let Some(external_auth) = self.external_auth_provider() {
            let cached_auth = self.auth_cached();
            if cached_auth
                .as_ref()
                .is_some_and(|auth| self.refresh_failure_for_auth(auth).is_some())
            {
                return cached_auth;
            }
            return match self.resolve_external_auth(external_auth.as_ref()).await {
                Ok(auth) => Some(auth),
                Err(err) => {
                    tracing::error!("Failed to resolve external auth: {err}");
                    match err {
                        RefreshTokenError::Permanent(error) => {
                            if let Some(auth) = cached_auth.as_ref() {
                                self.record_permanent_refresh_failure_if_unchanged(auth, &error);
                            }
                            cached_auth
                        }
                        RefreshTokenError::Transient(_) => None,
                    }
                }
            };
        }

        let allowed_login_methods = self.allowed_login_methods();
        let effective_managed_workspaces = self.effective_managed_workspaces();
        load_auth(
            &self.genex_home,
            self.enable_api_key_env,
            self.auth_credentials_store_mode,
            Some(&allowed_login_methods),
            effective_managed_workspaces.as_deref(),
            self.managed_auth_base_url.as_deref(),
            self.keyring_backend_kind,
            &self.auth_route_config,
        )
        .await
        .ok()
        .flatten()
        .filter(|auth| {
            validate_auth_restrictions(
                Some(&allowed_login_methods),
                effective_managed_workspaces.as_deref(),
                auth,
            )
            .is_ok()
        })
    }

    fn set_cached_auth(&self, new_auth: Option<GenexAuth>) -> bool {
        if let Ok(mut guard) = self.inner.write() {
            let previous = guard.auth.as_ref();
            let changed = !AuthManager::auths_equal(previous, new_auth.as_ref());
            let auth_changed_for_refresh =
                !Self::auths_equal_for_refresh(previous, new_auth.as_ref());
            let owner_changed =
                auth_changed_for_refresh && !same_owner(previous, new_auth.as_ref());
            if auth_changed_for_refresh {
                guard.permanent_refresh_failure = None;
            }
            tracing::info!("Reloaded auth, changed: {changed}");
            guard.auth = new_auth;
            if auth_changed_for_refresh {
                self.auth_change_state_tx.send_modify(|state| {
                    state.generation += 1;
                    if owner_changed {
                        state.owner_generation += 1;
                    }
                });
                self.auth_change_tx.send_modify(|revision| *revision += 1);
            }
            changed
        } else {
            false
        }
    }

    pub async fn set_external_auth(
        &self,
        external_auth: Arc<dyn ExternalAuth>,
    ) -> Result<(), RefreshTokenError> {
        self.install_external_auth(external_auth).await
    }

    async fn install_external_auth(
        &self,
        external_auth: Arc<dyn ExternalAuth>,
    ) -> Result<(), RefreshTokenError> {
        let auth = self.resolve_external_auth(external_auth.as_ref()).await?;
        let mut external_auth_slot = self.external_auth.write().map_err(|_| {
            RefreshTokenError::Transient(std::io::Error::other("external auth lock is poisoned"))
        })?;
        *external_auth_slot = Some(external_auth);
        drop(external_auth_slot);
        if let Ok(mut guard) = self.inner.write() {
            guard.permanent_refresh_failure = None;
        }
        self.commit_external_auth(auth)
    }

    pub fn clear_external_auth(&self) {
        if let Ok(mut external_auth) = self.external_auth.write()
            && external_auth.take().is_some()
        {
            self.set_cached_auth(/*new_auth*/ None);
        }
    }

    pub fn set_forced_managed_workspace_id(&self, workspace_id: Option<Vec<String>>) {
        if let Ok(mut guard) = self.forced_managed_workspace_id.write()
            && *guard != workspace_id
        {
            *guard = workspace_id;
        }
    }

    pub fn forced_managed_workspace_id(&self) -> Option<Vec<String>> {
        self.forced_managed_workspace_id
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    pub fn effective_managed_workspaces(&self) -> Option<Vec<String>> {
        self.managed_auth_policy
            .effective_managed_workspaces(self.forced_managed_workspace_id().as_deref())
    }

    pub fn is_login_method_allowed(&self, method: ForcedLoginMethod) -> bool {
        self.managed_auth_policy.allows_login_method(
            method,
            self.forced_login_method,
            self.forced_managed_workspace_id().as_deref(),
        )
    }

    /// Returns the login methods permitted by the current effective authentication policy.
    pub fn allowed_login_methods(&self) -> Vec<ForcedLoginMethod> {
        self.managed_auth_policy.allowed_login_methods(
            self.forced_login_method,
            self.forced_managed_workspace_id().as_deref(),
        )
    }

    pub fn has_external_auth(&self) -> bool {
        self.external_auth_provider().is_some()
    }

    pub fn api_key_env_enabled(&self) -> bool {
        self.enable_api_key_env
    }

    /// Convenience constructor returning an `Arc` wrapper.
    pub async fn shared(
        genex_home: PathBuf,
        enable_api_key_env: bool,
        auth_credentials_store_mode: AuthCredentialsStoreMode,
        forced_managed_workspace_id: Option<Vec<String>>,
        managed_auth_base_url: Option<String>,
        keyring_backend_kind: AuthKeyringBackendKind,
        auth_route_config: AuthRouteConfig,
    ) -> Arc<Self> {
        Arc::new(
            Self::new(
                genex_home,
                enable_api_key_env,
                auth_credentials_store_mode,
                forced_managed_workspace_id,
                managed_auth_base_url,
                keyring_backend_kind,
                auth_route_config,
            )
            .await,
        )
    }

    /// Builds a shared manager from the resolved authentication configuration.
    pub async fn shared_from_config(
        config: &impl AuthManagerConfig,
        enable_api_key_env: bool,
    ) -> Result<Arc<Self>, AuthManagerInitializationError> {
        Self::shared_from_auth_config(auth_config_from(config), enable_api_key_env).await
    }

    pub async fn shared_from_auth_config(
        auth_config: AuthConfig,
        enable_api_key_env: bool,
    ) -> Result<Arc<Self>, AuthManagerInitializationError> {
        Ok(Arc::new(
            Self::new_from_auth_config(auth_config, enable_api_key_env).await,
        ))
    }

    pub fn unauthorized_recovery(self: &Arc<Self>) -> UnauthorizedRecovery {
        UnauthorizedRecovery::new(Arc::clone(self))
    }

    fn external_auth_provider(&self) -> Option<Arc<dyn ExternalAuth>> {
        self.external_auth
            .read()
            .ok()
            .and_then(|external_auth| external_auth.clone())
    }

    fn has_refreshable_external_auth(&self) -> bool {
        self.has_external_auth()
            && self
                .auth_cached()
                .as_ref()
                .is_none_or(|auth| auth.is_api_key_auth() || auth.supports_unauthorized_recovery())
    }

    async fn resolve_external_auth(
        &self,
        external_auth: &dyn ExternalAuth,
    ) -> Result<GenexAuth, RefreshTokenError> {
        let auth = external_auth
            .resolve()
            .await
            .map_err(|error| external_auth.classify_error(error))?;
        self.validate_external_auth(&auth, external_auth)?;
        Ok(auth)
    }

    /// Attempt to refresh the token by first performing a guarded reload from
    /// the active auth source. If the loaded token differs from the cached token,
    /// we can assume that the source already refreshed it. Otherwise, ask the
    /// token authority to refresh.
    pub async fn refresh_token(&self) -> Result<(), RefreshTokenError> {
        let _refresh_guard = self.refresh_lock.acquire().await.map_err(|_| {
            RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                RefreshTokenFailedReason::Other,
                REFRESH_TOKEN_UNKNOWN_MESSAGE.to_string(),
            ))
        })?;
        let auth_before_reload = self.auth_cached();
        if auth_before_reload
            .as_ref()
            .is_some_and(GenexAuth::is_api_key_auth)
        {
            return Ok(());
        }
        let expected_account_id = auth_before_reload
            .as_ref()
            .and_then(GenexAuth::get_account_id);

        match self
            .reload_if_account_id_matches(expected_account_id.as_deref())
            .await
        {
            ReloadOutcome::ReloadedChanged => {
                tracing::info!("Skipping token refresh because auth changed after guarded reload.");
                Ok(())
            }
            ReloadOutcome::ReloadedNoChange => self.refresh_token_from_authority_impl().await,
            ReloadOutcome::Skipped => {
                Err(RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                    RefreshTokenFailedReason::Other,
                    REFRESH_TOKEN_ACCOUNT_MISMATCH_MESSAGE.to_string(),
                )))
            }
        }
    }

    /// Attempt to refresh the current auth token from the authority that issued
    /// it and update the shared cache. If the token refresh fails, returns the
    /// error to the caller.
    pub async fn refresh_token_from_authority(&self) -> Result<(), RefreshTokenError> {
        let _refresh_guard = self.refresh_lock.acquire().await.map_err(|_| {
            RefreshTokenError::Permanent(RefreshTokenFailedError::new(
                RefreshTokenFailedReason::Other,
                REFRESH_TOKEN_UNKNOWN_MESSAGE.to_string(),
            ))
        })?;
        self.refresh_token_from_authority_impl().await
    }

    async fn refresh_token_from_authority_impl(&self) -> Result<(), RefreshTokenError> {
        tracing::info!("Refreshing token");

        let attempted_auth = self.auth_cached();
        if let Some(error) = attempted_auth
            .as_ref()
            .and_then(|auth| self.refresh_failure_for_auth(auth))
        {
            return Err(RefreshTokenError::Permanent(error));
        }

        let result = if self.has_external_auth() {
            self.refresh_external_auth(ExternalAuthRefreshReason::Unauthorized)
                .await
        } else {
            match attempted_auth.as_ref() {
                Some(GenexAuth::ApiKey(_) | GenexAuth::Headers(_)) | None => Ok(()),
            }
        };
        if let Some(attempted_auth) = attempted_auth.as_ref()
            && let Err(RefreshTokenError::Permanent(error)) = &result
        {
            self.record_permanent_refresh_failure_if_unchanged(attempted_auth, error);
        }
        result
    }

    /// Log out by deleting the on‑disk auth.json (if present). Returns Ok(true)
    /// if a file was removed, Ok(false) if no auth file existed. On success,
    /// reloads the in‑memory auth cache so callers immediately observe the
    /// unauthenticated state.
    pub async fn logout(&self) -> std::io::Result<bool> {
        self.ensure_logout_allowed()?;
        let removed = logout_all_stores(
            &self.genex_home,
            self.auth_credentials_store_mode,
            self.keyring_backend_kind,
        )?;
        // Always reload to clear any cached auth (even if file absent).
        self.clear_external_auth();
        self.reload().await;
        Ok(removed)
    }

    fn ensure_logout_allowed(&self) -> std::io::Result<()> {
        Ok(())
    }

    /// Returns the precise kind of credentials backing the current authentication.
    pub fn get_api_auth_mode(&self) -> Option<AuthMode> {
        self.auth_cached().as_ref().map(GenexAuth::api_auth_mode)
    }

    /// Returns the effective backend auth mode for the current authentication.
    pub fn auth_mode(&self) -> Option<AuthMode> {
        self.auth_cached().as_ref().map(GenexAuth::auth_mode)
    }

    pub fn current_auth_uses_managed_auth(&self) -> bool {
        self.get_api_auth_mode()
            .is_some_and(AuthMode::uses_managed_auth)
    }

    async fn refresh_external_auth(
        &self,
        reason: ExternalAuthRefreshReason,
    ) -> Result<(), RefreshTokenError> {
        let Some(external_auth) = self.external_auth_provider() else {
            return Err(RefreshTokenError::Transient(std::io::Error::other(
                "external auth is not configured",
            )));
        };
        let previous_account_id = self
            .auth_cached()
            .as_ref()
            .and_then(GenexAuth::get_account_id);
        let context = ExternalAuthRefreshContext {
            reason,
            previous_account_id,
        };

        let refreshed = external_auth
            .refresh(context)
            .await
            .map_err(|error| external_auth.classify_error(error))?;
        self.validate_external_auth(&refreshed, external_auth.as_ref())?;
        self.commit_external_auth(refreshed)?;
        Ok(())
    }

    fn commit_external_auth(&self, auth: GenexAuth) -> Result<(), RefreshTokenError> {
        self.set_cached_auth(Some(auth));
        Ok(())
    }

    fn validate_external_auth(
        &self,
        auth: &GenexAuth,
        external_auth: &dyn ExternalAuth,
    ) -> Result<(), RefreshTokenError> {
        let allowed_login_methods = self.allowed_login_methods();
        validate_auth_restrictions(
            Some(&allowed_login_methods),
            self.effective_managed_workspaces().as_deref(),
            auth,
        )
        .map_err(|error| external_auth.classify_error(std::io::Error::other(error)))
    }
}

fn auth_config_from(config: &impl AuthManagerConfig) -> AuthConfig {
    AuthConfig {
        genex_home: config.genex_home(),
        auth_credentials_store_mode: config.cli_auth_credentials_store_mode(),
        keyring_backend_kind: config.auth_keyring_backend_kind(),
        forced_login_method: config.forced_login_method(),
        managed_auth_base_url: (!config.managed_auth_base_url().is_empty())
            .then(|| config.managed_auth_base_url()),
        forced_managed_workspace_id: config.forced_managed_workspace_id(),
        managed_auth_policy: config.managed_auth_policy(),
        auth_route_config: config.auth_route_config(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_store() -> (
        tempfile::TempDir,
        AuthCredentialsStoreMode,
        AuthKeyringBackendKind,
    ) {
        (
            tempfile::tempdir().expect("temporary auth home"),
            AuthCredentialsStoreMode::File,
            AuthKeyringBackendKind::default(),
        )
    }

    #[test]
    fn provider_credentials_are_isolated_and_deletable() {
        let (home, mode, keyring) = file_store();
        save_provider_auth(
            home.path(),
            "provider-a",
            &ProviderAuth::api_key("a-secret"),
            mode,
            keyring,
        )
        .expect("save provider A");
        save_provider_auth(
            home.path(),
            "provider-b",
            &ProviderAuth::api_key("b-secret"),
            mode,
            keyring,
        )
        .expect("save provider B");

        assert_eq!(
            load_provider_auth(home.path(), "provider-a", mode, keyring)
                .expect("load provider A")
                .and_then(|auth| auth.api_key),
            Some("a-secret".to_string())
        );
        let provider_a = load_provider_auth(home.path(), "provider-a", mode, keyring)
            .expect("load provider A")
            .expect("provider A exists");
        assert!(
            !serde_json::to_string(&provider_a)
                .expect("serialize provider A")
                .contains("b-secret")
        );
        assert_eq!(
            list_provider_ids(home.path(), mode, keyring).expect("list providers"),
            vec!["provider-a".to_string(), "provider-b".to_string()]
        );

        assert!(
            delete_provider_auth(home.path(), "provider-a", mode, keyring)
                .expect("delete provider A")
        );
        assert!(
            load_provider_auth(home.path(), "provider-a", mode, keyring)
                .expect("load deleted provider A")
                .is_none()
        );
        assert_eq!(
            load_provider_auth(home.path(), "provider-b", mode, keyring)
                .expect("load provider B")
                .and_then(|auth| auth.api_key),
            Some("b-secret".to_string())
        );
    }

    #[test]
    fn legacy_single_provider_auth_loads_and_migrates_on_provider_write() {
        let (home, mode, keyring) = file_store();
        let legacy = serde_json::json!({
            "auth_mode": "apiKey",
            "api_key": "legacy-secret"
        });
        std::fs::write(home.path().join("auth.json"), legacy.to_string())
            .expect("write legacy auth");

        let auth = load_provider_auth(home.path(), "provider-a", mode, keyring)
            .expect("load legacy provider auth")
            .expect("legacy auth exists");
        assert_eq!(auth.api_key.as_deref(), Some("legacy-secret"));

        save_provider_auth(home.path(), "provider-a", &auth, mode, keyring)
            .expect("migrate legacy auth");
        let serialized =
            std::fs::read_to_string(home.path().join("auth.json")).expect("read migrated auth");
        assert!(serialized.contains("providers"));
        assert!(!serialized.contains("\"auth_mode\""));
        assert_eq!(
            load_auth_dot_json(home.path(), mode, keyring)
                .expect("load migrated auth")
                .expect("migrated auth")
                .providers
                .keys()
                .cloned()
                .collect::<Vec<_>>(),
            vec!["provider-a".to_string()]
        );
    }

    #[test]
    fn unknown_auth_mode_is_not_usable_credentials() {
        let auth: AuthDotJson = serde_json::from_value(serde_json::json!({
            "providers": {
                "provider-a": {
                    "auth_mode": "removedMode",
                    "api_key": "secret"
                }
            }
        }))
        .expect("unknown auth mode should remain parseable");

        assert!(!auth.has_supported_credentials());
        let serialized = serde_json::to_string(&auth).expect("serialize unknown auth mode");
        assert!(serialized.contains("removedMode"));
        let round_trip: AuthDotJson =
            serde_json::from_str(&serialized).expect("deserialize unknown auth mode");
        assert!(!round_trip.has_supported_credentials());
    }

    #[tokio::test]
    async fn token_only_auth_is_not_loaded_as_genex_auth() {
        let auth: AuthDotJson = serde_json::from_value(serde_json::json!({
            "auth_mode": "legacy",
            "tokens": {"access_token": "old-token"},
            "last_refresh": "2026-09-30T00:00:00Z"
        }))
        .expect("legacy token fields should remain parseable as unknown fields");

        let result = GenexAuth::from_auth_dot_json(
            Path::new("/tmp/genex-test"),
            auth,
            AuthCredentialsStoreMode::File,
            None,
            AuthKeyringBackendKind::default(),
            &crate::test_support::transport_default_auth_route_config(),
        )
        .await;

        assert!(result.is_err(), "token-only auth must not be accepted");
    }
}

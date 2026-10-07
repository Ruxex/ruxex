use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeStruct;
use sha2::Digest;
use sha2::Sha256;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tracing::warn;

use genex_config::types::AuthCredentialsStoreMode;
pub use genex_config::types::AuthKeyringBackendKind;
use genex_keyring_store::DefaultKeyringStore;
use genex_keyring_store::KeyringStore;
use genex_protocol::auth::AuthMode;
use genex_secrets::LocalSecretsNamespace;
use genex_secrets::SecretName;
use genex_secrets::SecretScope;
use genex_secrets::SecretsBackendKind;
use genex_secrets::SecretsManager;
use once_cell::sync::Lazy;

/// A provider-scoped credential payload persisted in `auth.json`.
#[derive(Clone, PartialEq, Eq)]
pub struct ProviderAuth {
    pub auth_mode: Option<AuthMode>,

    pub api_key: Option<String>,

    invalid_auth_mode: bool,
    raw_auth_mode: Option<String>,
}

impl Serialize for ProviderAuth {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Wire<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            auth_mode: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            api_key: Option<&'a str>,
        }

        let auth_mode = if self.invalid_auth_mode {
            self.raw_auth_mode.as_deref()
        } else {
            self.auth_mode.map(|mode| match mode {
                AuthMode::ApiKey => "apiKey",
                AuthMode::Headers => "headers",
            })
        };
        Wire {
            auth_mode,
            api_key: self.api_key.as_deref(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProviderAuth {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(default)]
            auth_mode: Option<String>,
            #[serde(default)]
            api_key: Option<String>,
        }

        let wire = Wire::deserialize(deserializer)?;
        let (auth_mode, invalid_auth_mode, raw_auth_mode) = parse_auth_mode(wire.auth_mode);
        Ok(Self {
            auth_mode,
            api_key: wire.api_key,
            invalid_auth_mode,
            raw_auth_mode,
        })
    }
}

impl std::fmt::Debug for ProviderAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderAuth")
            .field("auth_mode", &self.auth_mode)
            .field("has_api_key", &self.api_key.is_some())
            .finish()
    }
}

impl ProviderAuth {
    pub fn api_key(api_key: impl Into<String>) -> Self {
        Self {
            auth_mode: Some(AuthMode::ApiKey),
            api_key: Some(api_key.into()),
            invalid_auth_mode: false,
            raw_auth_mode: None,
        }
    }

    pub fn has_supported_credentials(&self) -> bool {
        !self.invalid_auth_mode
            && self.resolved_mode() == AuthMode::ApiKey
            && self.api_key.is_some()
    }

    pub fn resolved_mode(&self) -> AuthMode {
        if let Some(mode) = self.auth_mode {
            return mode;
        }
        AuthMode::ApiKey
    }

    fn serialized_auth_mode(&self) -> Option<&str> {
        if self.invalid_auth_mode {
            return self.raw_auth_mode.as_deref();
        }
        self.auth_mode.map(|mode| match mode {
            AuthMode::ApiKey => "apiKey",
            AuthMode::Headers => "headers",
        })
    }
}

/// Expected structure for $GENEX_HOME/auth.json.
///
/// The canonical representation contains provider-keyed records. The private
/// legacy field is populated only when reading the former single-provider
/// shape, allowing existing installations to be read until a provider-scoped
/// write migrates them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthDotJson {
    pub providers: BTreeMap<String, ProviderAuth>,
    pub active_provider: Option<String>,
    legacy: Option<ProviderAuth>,
}

impl AuthDotJson {
    pub fn for_provider(provider_id: impl Into<String>, auth: ProviderAuth) -> Self {
        Self {
            providers: BTreeMap::from([(provider_id.into(), auth)]),
            active_provider: None,
            legacy: None,
        }
    }

    pub fn legacy(auth_mode: Option<AuthMode>, api_key: Option<String>) -> Self {
        Self {
            providers: BTreeMap::new(),
            active_provider: None,
            legacy: Some(ProviderAuth {
                auth_mode,
                api_key,
                invalid_auth_mode: false,
                raw_auth_mode: None,
            }),
        }
    }

    pub fn active_provider_auth(&self) -> Option<&ProviderAuth> {
        if let Some(provider_id) = self.active_provider.as_deref() {
            return self.providers.get(provider_id);
        }
        if self.providers.len() == 1 {
            return self.providers.values().next();
        }
        self.legacy.as_ref()
    }

    pub fn active_provider_id(&self) -> Option<&str> {
        self.active_provider.as_deref()
    }

    pub(crate) fn active_auth(&self) -> Option<&ProviderAuth> {
        self.active_provider_auth()
    }

    pub(crate) fn provider_auth(&self, provider_id: &str) -> Option<&ProviderAuth> {
        self.providers.get(provider_id).or(self.legacy.as_ref())
    }

    pub(crate) fn has_legacy_auth(&self) -> bool {
        self.legacy.is_some()
    }

    pub(crate) fn into_provider_storage(self) -> Self {
        Self {
            providers: self.providers,
            active_provider: self.active_provider,
            legacy: None,
        }
    }
}

impl Default for AuthDotJson {
    fn default() -> Self {
        Self {
            providers: BTreeMap::new(),
            active_provider: None,
            legacy: None,
        }
    }
}

impl Serialize for AuthDotJson {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.providers.is_empty()
            && self.active_provider.is_none()
            && let Some(legacy) = &self.legacy
        {
            let mut state = serializer.serialize_struct("AuthDotJson", 2)?;
            state.serialize_field("auth_mode", &legacy.serialized_auth_mode())?;
            state.serialize_field("api_key", &legacy.api_key)?;
            return state.end();
        }

        let mut state = serializer.serialize_struct("AuthDotJson", 2)?;
        state.serialize_field("providers", &self.providers)?;
        if let Some(active_provider) = &self.active_provider {
            state.serialize_field("active_provider", active_provider)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for AuthDotJson {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(default)]
            providers: BTreeMap<String, ProviderAuth>,
            #[serde(default)]
            active_provider: Option<String>,
            #[serde(default)]
            auth_mode: Option<String>,
            #[serde(default)]
            api_key: Option<String>,
        }

        let wire = Wire::deserialize(deserializer)?;
        let (auth_mode, invalid_auth_mode, raw_auth_mode) = parse_auth_mode(wire.auth_mode);
        let legacy = if wire.providers.is_empty() && wire.active_provider.is_none() {
            (auth_mode.is_some() || wire.api_key.is_some() || invalid_auth_mode).then_some(
                ProviderAuth {
                    auth_mode,
                    api_key: wire.api_key,
                    invalid_auth_mode,
                    raw_auth_mode,
                },
            )
        } else {
            None
        };
        Ok(Self {
            providers: wire.providers,
            active_provider: wire.active_provider,
            legacy,
        })
    }
}

/// Ignore authentication modes removed from the persisted format so old
/// token-only auth.json files are treated as unauthenticated state, while
/// remembering that an unrecognized mode must not become valid API-key auth.
fn parse_auth_mode(value: Option<String>) -> (Option<AuthMode>, bool, Option<String>) {
    match value {
        Some(value) => match value.as_str() {
            "apiKey" | "apikey" => (Some(AuthMode::ApiKey), false, None),
            "headers" => (Some(AuthMode::Headers), false, None),
            _ => (None, true, Some(value)),
        },
        None => (None, false, None),
    }
}

pub(super) fn get_auth_file(genex_home: &Path) -> PathBuf {
    genex_home.join("auth.json")
}

pub(super) fn delete_file_if_exists(genex_home: &Path) -> std::io::Result<bool> {
    let auth_file = get_auth_file(genex_home);
    match std::fs::remove_file(&auth_file) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

pub(super) trait AuthStorageBackend: Debug + Send + Sync {
    fn load(&self) -> std::io::Result<Option<AuthDotJson>>;
    fn save(&self, auth: &AuthDotJson) -> std::io::Result<()>;
    fn delete(&self) -> std::io::Result<bool>;
}

#[derive(Clone, Debug)]
pub(super) struct FileAuthStorage {
    genex_home: PathBuf,
}

impl FileAuthStorage {
    pub(super) fn new(genex_home: PathBuf) -> Self {
        Self { genex_home }
    }

    /// Attempt to read and parse the `auth.json` file in the given `GENEX_HOME` directory.
    /// Returns the full AuthDotJson structure.
    pub(super) fn try_read_auth_json(&self, auth_file: &Path) -> std::io::Result<AuthDotJson> {
        let mut file = File::open(auth_file)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let auth_dot_json: AuthDotJson = serde_json::from_str(&contents)?;

        Ok(auth_dot_json)
    }
}

impl AuthStorageBackend for FileAuthStorage {
    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        let auth_file = get_auth_file(&self.genex_home);
        let auth_dot_json = match self.try_read_auth_json(&auth_file) {
            Ok(auth) => auth,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };
        Ok(Some(auth_dot_json))
    }

    fn save(&self, auth_dot_json: &AuthDotJson) -> std::io::Result<()> {
        let auth_file = get_auth_file(&self.genex_home);

        if let Some(parent) = auth_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json_data = serde_json::to_string_pretty(auth_dot_json)?;
        let mut options = OpenOptions::new();
        options.truncate(true).write(true).create(true);
        #[cfg(unix)]
        {
            options.mode(0o600);
        }
        let mut file = options.open(auth_file)?;
        file.write_all(json_data.as_bytes())?;
        file.flush()?;
        Ok(())
    }

    fn delete(&self) -> std::io::Result<bool> {
        delete_file_if_exists(&self.genex_home)
    }
}

static GENEX_AUTH_SECRET_NAME: Lazy<SecretName> =
    Lazy::new(|| match SecretName::new("GENEX_AUTH") {
        Ok(name) => name,
        Err(err) => unreachable!("GENEX_AUTH should be a valid secret name: {err}"),
    });
const KEYRING_SERVICE: &str = "Genex Auth";

// turns genex_home path into a stable, short key string
fn compute_store_key(genex_home: &Path) -> std::io::Result<String> {
    let canonical = genex_home
        .canonicalize()
        .unwrap_or_else(|_| genex_home.to_path_buf());
    let path_str = canonical.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(path_str.as_bytes());
    let digest = hasher.finalize();
    let hex = format!("{digest:x}");
    let truncated = hex.get(..16).unwrap_or(&hex);
    Ok(format!("cli|{truncated}"))
}

#[derive(Clone, Debug)]
struct DirectKeyringAuthStorage {
    genex_home: PathBuf,
    keyring_store: Arc<dyn KeyringStore>,
}

impl DirectKeyringAuthStorage {
    fn new(genex_home: PathBuf, keyring_store: Arc<dyn KeyringStore>) -> Self {
        Self {
            genex_home,
            keyring_store,
        }
    }

    fn load_from_keyring(&self, key: &str) -> std::io::Result<Option<AuthDotJson>> {
        match self.keyring_store.load(KEYRING_SERVICE, key) {
            Ok(Some(serialized)) => serde_json::from_str(&serialized).map(Some).map_err(|err| {
                std::io::Error::other(format!(
                    "failed to deserialize CLI auth from keyring: {err}"
                ))
            }),
            Ok(None) => Ok(None),
            Err(error) => Err(std::io::Error::other(format!(
                "failed to load CLI auth from keyring: {}",
                error.message()
            ))),
        }
    }

    fn save_to_keyring(&self, key: &str, value: &str) -> std::io::Result<()> {
        match self.keyring_store.save(KEYRING_SERVICE, key, value) {
            Ok(()) => Ok(()),
            Err(error) => {
                let message = format!(
                    "failed to write stored credentials to keyring: {}",
                    error.message()
                );
                warn!("{message}");
                Err(std::io::Error::other(message))
            }
        }
    }
}

impl AuthStorageBackend for DirectKeyringAuthStorage {
    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        let key = compute_store_key(&self.genex_home)?;
        self.load_from_keyring(&key)
    }

    fn save(&self, auth: &AuthDotJson) -> std::io::Result<()> {
        let key = compute_store_key(&self.genex_home)?;
        // Simpler error mapping per style: prefer method reference over closure
        let serialized = serde_json::to_string(auth).map_err(std::io::Error::other)?;
        self.save_to_keyring(&key, &serialized)?;
        if let Err(err) = delete_file_if_exists(&self.genex_home) {
            warn!("failed to remove CLI auth fallback file: {err}");
        }
        Ok(())
    }

    fn delete(&self) -> std::io::Result<bool> {
        let key = compute_store_key(&self.genex_home)?;
        let keyring_removed = self
            .keyring_store
            .delete(KEYRING_SERVICE, &key)
            .map_err(|err| {
                std::io::Error::other(format!("failed to delete auth from keyring: {err}"))
            })?;
        let file_removed = delete_file_if_exists(&self.genex_home)?;
        Ok(keyring_removed || file_removed)
    }
}

#[derive(Clone)]
struct SecretsKeyringAuthStorage {
    genex_home: PathBuf,
    direct_storage: DirectKeyringAuthStorage,
    secrets_manager: SecretsManager,
}

impl Debug for SecretsKeyringAuthStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretsKeyringAuthStorage")
            .field("genex_home", &self.genex_home)
            .finish_non_exhaustive()
    }
}

impl SecretsKeyringAuthStorage {
    fn new(genex_home: PathBuf, keyring_store: Arc<dyn KeyringStore>) -> Self {
        let direct_storage =
            DirectKeyringAuthStorage::new(genex_home.clone(), Arc::clone(&keyring_store));
        let secrets_manager = SecretsManager::new_with_keyring_store_and_namespace(
            genex_home.clone(),
            SecretsBackendKind::Local,
            keyring_store,
            LocalSecretsNamespace::GenexAuth,
        );
        Self {
            genex_home,
            direct_storage,
            secrets_manager,
        }
    }
}

impl AuthStorageBackend for SecretsKeyringAuthStorage {
    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        match self
            .secrets_manager
            .get(&SecretScope::Global, &GENEX_AUTH_SECRET_NAME)
            .map_err(|err| {
                std::io::Error::other(format!(
                    "failed to load CLI auth from encrypted auth storage: {err}"
                ))
            })? {
            Some(serialized) => serde_json::from_str(&serialized).map(Some).map_err(|err| {
                std::io::Error::other(format!(
                    "failed to deserialize CLI auth from encrypted auth storage: {err}"
                ))
            }),
            None => Ok(None),
        }
    }

    fn save(&self, auth: &AuthDotJson) -> std::io::Result<()> {
        let serialized = serde_json::to_string(auth).map_err(std::io::Error::other)?;
        self.secrets_manager
            .set(&SecretScope::Global, &GENEX_AUTH_SECRET_NAME, &serialized)
            .map_err(|err| {
                let message =
                    format!("failed to write stored credentials to encrypted auth storage: {err}");
                warn!("{message}");
                std::io::Error::other(message)
            })?;
        if let Err(err) = delete_file_if_exists(&self.genex_home) {
            warn!("failed to remove CLI auth fallback file: {err}");
        }
        Ok(())
    }

    fn delete(&self) -> std::io::Result<bool> {
        let keyring_removed = self
            .secrets_manager
            .delete(&SecretScope::Global, &GENEX_AUTH_SECRET_NAME)
            .map_err(|err| {
                std::io::Error::other(format!(
                    "failed to delete auth from encrypted auth storage: {err}"
                ))
            })?;
        let file_removed = delete_file_if_exists(&self.genex_home)?;
        let direct_removed = self.direct_storage.delete()?;
        Ok(keyring_removed || file_removed || direct_removed)
    }
}

#[derive(Clone, Debug)]
struct AutoAuthStorage {
    keyring_storage: Arc<dyn AuthStorageBackend>,
    file_storage: Arc<FileAuthStorage>,
}

impl AutoAuthStorage {
    fn new(
        genex_home: PathBuf,
        keyring_store: Arc<dyn KeyringStore>,
        keyring_backend_kind: AuthKeyringBackendKind,
    ) -> Self {
        Self {
            keyring_storage: create_keyring_auth_storage(
                genex_home.clone(),
                keyring_store,
                keyring_backend_kind,
            ),
            file_storage: Arc::new(FileAuthStorage::new(genex_home)),
        }
    }
}

impl AuthStorageBackend for AutoAuthStorage {
    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        match self.keyring_storage.load() {
            Ok(Some(auth)) => Ok(Some(auth)),
            Ok(None) => self.file_storage.load(),
            Err(err) => {
                warn!("failed to load CLI auth from keyring, falling back to file storage: {err}");
                self.file_storage.load()
            }
        }
    }

    fn save(&self, auth: &AuthDotJson) -> std::io::Result<()> {
        match self.keyring_storage.save(auth) {
            Ok(()) => Ok(()),
            Err(err) => {
                warn!("failed to save auth to keyring, falling back to file storage: {err}");
                self.file_storage.save(auth)
            }
        }
    }

    fn delete(&self) -> std::io::Result<bool> {
        // Keyring storage will delete from disk as well
        self.keyring_storage.delete()
    }
}

// A global in-memory store for mapping genex_home -> AuthDotJson.
static EPHEMERAL_AUTH_STORE: Lazy<Mutex<HashMap<String, AuthDotJson>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug)]
struct EphemeralAuthStorage {
    genex_home: PathBuf,
}

impl EphemeralAuthStorage {
    fn new(genex_home: PathBuf) -> Self {
        Self { genex_home }
    }

    fn with_store<F, T>(&self, action: F) -> std::io::Result<T>
    where
        F: FnOnce(&mut HashMap<String, AuthDotJson>, String) -> std::io::Result<T>,
    {
        let key = compute_store_key(&self.genex_home)?;
        let mut store = EPHEMERAL_AUTH_STORE
            .lock()
            .map_err(|_| std::io::Error::other("failed to lock ephemeral auth storage"))?;
        action(&mut store, key)
    }
}

impl AuthStorageBackend for EphemeralAuthStorage {
    fn load(&self) -> std::io::Result<Option<AuthDotJson>> {
        self.with_store(|store, key| Ok(store.get(&key).cloned()))
    }

    fn save(&self, auth: &AuthDotJson) -> std::io::Result<()> {
        self.with_store(|store, key| {
            store.insert(key, auth.clone());
            Ok(())
        })
    }

    fn delete(&self) -> std::io::Result<bool> {
        self.with_store(|store, key| Ok(store.remove(&key).is_some()))
    }
}

pub(super) fn create_auth_storage(
    genex_home: PathBuf,
    mode: AuthCredentialsStoreMode,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> Arc<dyn AuthStorageBackend> {
    let keyring_store: Arc<dyn KeyringStore> = Arc::new(DefaultKeyringStore);
    create_auth_storage_with_store(genex_home, mode, keyring_store, keyring_backend_kind)
}

fn create_auth_storage_with_store(
    genex_home: PathBuf,
    mode: AuthCredentialsStoreMode,
    keyring_store: Arc<dyn KeyringStore>,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> Arc<dyn AuthStorageBackend> {
    match mode {
        AuthCredentialsStoreMode::File => Arc::new(FileAuthStorage::new(genex_home)),
        AuthCredentialsStoreMode::Keyring => {
            create_keyring_auth_storage(genex_home, keyring_store, keyring_backend_kind)
        }
        AuthCredentialsStoreMode::Auto => Arc::new(AutoAuthStorage::new(
            genex_home,
            keyring_store,
            keyring_backend_kind,
        )),
        AuthCredentialsStoreMode::Ephemeral => Arc::new(EphemeralAuthStorage::new(genex_home)),
    }
}

fn create_keyring_auth_storage(
    genex_home: PathBuf,
    keyring_store: Arc<dyn KeyringStore>,
    keyring_backend_kind: AuthKeyringBackendKind,
) -> Arc<dyn AuthStorageBackend> {
    match keyring_backend_kind {
        AuthKeyringBackendKind::Direct => {
            Arc::new(DirectKeyringAuthStorage::new(genex_home, keyring_store))
        }
        AuthKeyringBackendKind::Secrets => {
            Arc::new(SecretsKeyringAuthStorage::new(genex_home, keyring_store))
        }
    }
}

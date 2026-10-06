use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use arc_swap::ArcSwapOption;
use genex_http_client::HttpClientFactory;
use genex_http_client::OutboundProxyPolicy;
use genex_protocol::capabilities::SelectedCapabilityRoot;

use crate::CapabilityRootsDiscoverParams;
use crate::CapabilityRootsDiscoverResponse;
use crate::EnvironmentConfigReadParams;
use crate::EnvironmentConfigReadResponse;
use crate::ExecServerError;
use crate::ExecServerRuntimePaths;
use crate::ExecutorFileSystem;
use crate::HttpClient;
use crate::client::LazyRemoteExecServerClient;
use crate::client::http_client::RouteAwareHttpClient;
use crate::client_api::ExecServerTransportParams;
use crate::environment_bootstrap::PreparedEnvironmentManager;
use crate::environment_bootstrap::PreparedEnvironmentSource;
use crate::environment_config::read_environment_config;
use crate::environment_provider::DefaultEnvironmentProvider;
use crate::environment_provider::EnvironmentDefault;
use crate::environment_provider::EnvironmentProvider;
use crate::environment_provider::EnvironmentProviderSnapshot;
use crate::environment_toml::environment_provider_from_genex_home;
use crate::local_file_system::LocalFileSystem;
use crate::local_process::LocalProcess;
use crate::process::ExecBackend;
use crate::protocol::EnvironmentInfo;
use crate::remote_file_system::RemoteFileSystem;
use crate::remote_process::RemoteProcess;
use tokio::sync::watch;
use tokio_util::task::AbortOnDropHandle;
use tracing::Instrument;
use tracing::instrument::WithSubscriber;

/// The current connection state for one concrete environment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvironmentConnectionState {
    /// An initialized exec-server connection is available.
    Connected,
    /// No initialized exec-server connection is currently available.
    Disconnected,
}

/// Owns the execution/filesystem environments available to the Genex runtime.
///
/// `EnvironmentManager` is a shared registry for concrete local and stdio
/// environments. Filesystem and execution backends share the resulting
/// startup and reconnect as needed.
#[derive(Debug)]
pub struct EnvironmentManager {
    default_environment: Option<String>,
    pub(super) environments: RwLock<HashMap<String, Arc<Environment>>>,
    local_environment: Option<Arc<Environment>>,
    http_client_factory: HttpClientFactory,
}

/// Information supplied by the environment owner when an environment is ready.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EnvironmentReadyInfo {
    /// Ordered capability roots selected for this environment.
    pub selected_capability_roots: Vec<SelectedCapabilityRoot>,
}

/// Maximum capability roots accepted from environment ready information.
pub const MAX_SELECTED_CAPABILITY_ROOTS: usize = 256;

pub const LOCAL_ENVIRONMENT_ID: &str = "local";
/// Non-mutating connection status observed by an environment owner.
///
/// Computing this status never starts, waits for, or reconnects an exec-server
/// transport. Already-ready remote environments may receive a fail-fast probe
/// over their existing connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvironmentObservedStatus {
    /// A local environment, or a remote environment whose existing connection answered a probe.
    Ready,
    /// The configured environment has no ready connection and no observed connection failure.
    ///
    /// This includes lazy transports that have never been started and initial startup that has
    /// not finished. Computing status does not start the environment or wait for startup.
    Pending,
    /// A connection attempt, prior connection, or fail-fast status probe observed a failure.
    ///
    /// This does not promise that the failure is terminal: later normal environment use may
    /// recover the connection. Computing status itself does not trigger recovery.
    Disconnected {
        /// Human-readable reason recorded by the failed connection attempt or probe.
        error: String,
    },
}

impl EnvironmentManager {
    /// Builds a test-only manager without configured sandbox helper paths.
    pub fn default_for_tests() -> Self {
        Self {
            default_environment: Some(LOCAL_ENVIRONMENT_ID.to_string()),
            environments: RwLock::new(HashMap::from([(
                LOCAL_ENVIRONMENT_ID.to_string(),
                Arc::new(Environment::default_for_tests()),
            )])),
            local_environment: Some(Arc::new(Environment::default_for_tests())),
            // Test-only construction has no application config from which to resolve proxy policy.
            http_client_factory: HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        }
    }

    /// Builds a manager with no configured execution environments.
    pub fn without_environments(http_client_factory: HttpClientFactory) -> Self {
        Self {
            default_environment: None,
            environments: RwLock::new(HashMap::new()),
            local_environment: None,
            http_client_factory,
        }
    }

    /// Builds a test-only manager from a raw exec-server URL value.
    pub async fn create_for_tests(
        exec_server_url: Option<String>,
        local_runtime_paths: Option<ExecServerRuntimePaths>,
    ) -> Self {
        let provider = DefaultEnvironmentProvider::new(exec_server_url);
        match Self::from_snapshot(
            provider.snapshot_inner(),
            local_runtime_paths,
            // Test-only construction has no application config from which to resolve proxy policy.
            HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        ) {
            Ok(manager) => manager,
            Err(err) => panic!("default provider should create valid environments: {err}"),
        }
    }

    /// Discovers configured environments without starting stdio connections.
    pub async fn prepare_from_genex_home(
        genex_home: impl AsRef<std::path::Path>,
    ) -> Result<PreparedEnvironmentManager, ExecServerError> {
        let provider = environment_provider_from_genex_home(genex_home.as_ref())?;
        let source = PreparedEnvironmentSource::Snapshot(provider.snapshot().await?);
        Ok(PreparedEnvironmentManager { source })
    }

    /// Builds a manager from `GENEX_HOME` with an explicit outbound HTTP policy.
    pub async fn from_genex_home(
        genex_home: impl AsRef<std::path::Path>,
        local_runtime_paths: Option<ExecServerRuntimePaths>,
        http_client_factory: HttpClientFactory,
    ) -> Result<Self, ExecServerError> {
        Self::prepare_from_genex_home(genex_home)
            .await?
            .build(local_runtime_paths, http_client_factory)
    }

    /// Discovers environment-variable environments without starting connections.
    pub async fn prepare_from_env() -> Result<PreparedEnvironmentManager, ExecServerError> {
        let provider = DefaultEnvironmentProvider::from_env();
        let source = PreparedEnvironmentSource::Snapshot(provider.snapshot().await?);
        Ok(PreparedEnvironmentManager { source })
    }

    /// Builds a manager from environment variables with an explicit outbound HTTP policy.
    pub async fn from_env(
        local_runtime_paths: Option<ExecServerRuntimePaths>,
        http_client_factory: HttpClientFactory,
    ) -> Result<Self, ExecServerError> {
        Self::prepare_from_env()
            .await?
            .build(local_runtime_paths, http_client_factory)
    }

    /// Builds a test-only manager that keeps the provider default while also
    /// allowing tests to select the local environment explicitly.
    pub async fn create_for_tests_with_local(
        exec_server_url: Option<String>,
        local_runtime_paths: ExecServerRuntimePaths,
    ) -> Self {
        let mut snapshot = DefaultEnvironmentProvider::new(exec_server_url).snapshot_inner();
        snapshot.include_local = true;
        match Self::from_snapshot(
            snapshot,
            Some(local_runtime_paths),
            // Test-only construction has no application config from which to resolve proxy policy.
            HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        ) {
            Ok(manager) => manager,
            Err(err) => panic!("test provider with local should create valid environments: {err}"),
        }
    }

    pub(crate) fn from_snapshot(
        snapshot: EnvironmentProviderSnapshot,
        local_runtime_paths: Option<ExecServerRuntimePaths>,
        http_client_factory: HttpClientFactory,
    ) -> Result<Self, ExecServerError> {
        let EnvironmentProviderSnapshot {
            environments,
            default,
            include_local,
        } = snapshot;
        let mut environment_map =
            HashMap::with_capacity(environments.len() + usize::from(include_local));
        let local_environment = if include_local {
            let local_runtime_paths = local_runtime_paths.clone().ok_or_else(|| {
                ExecServerError::Protocol(
                    "local environment requires configured runtime paths".to_string(),
                )
            })?;
            let local_environment = Arc::new(Environment::local(
                local_runtime_paths,
                http_client_factory.clone(),
            ));
            environment_map.insert(
                LOCAL_ENVIRONMENT_ID.to_string(),
                Arc::clone(&local_environment),
            );
            Some(local_environment)
        } else {
            None
        };
        for (id, transport) in environments {
            if id.is_empty() {
                return Err(ExecServerError::Protocol(
                    "environment id cannot be empty".to_string(),
                ));
            }
            if id == LOCAL_ENVIRONMENT_ID {
                return Err(ExecServerError::Protocol(format!(
                    "environment id `{LOCAL_ENVIRONMENT_ID}` is reserved for EnvironmentManager"
                )));
            }
            let environment = Environment::remote_with_transport(
                transport,
                /*local_runtime_paths*/ None,
                http_client_factory.clone(),
            );
            if environment_map
                .insert(id.clone(), Arc::new(environment))
                .is_some()
            {
                return Err(ExecServerError::Protocol(format!(
                    "environment id `{id}` is duplicated"
                )));
            }
        }
        let default_environment = match default {
            EnvironmentDefault::Disabled => None,
            EnvironmentDefault::EnvironmentId(environment_id) => {
                if !environment_map.contains_key(&environment_id) {
                    return Err(ExecServerError::Protocol(format!(
                        "default environment `{environment_id}` is not configured"
                    )));
                }
                Some(environment_id)
            }
        };
        // The snapshot is valid; start connecting its remote environments in the background.
        for environment in environment_map.values() {
            environment.start_connecting();
        }
        Ok(Self {
            default_environment,
            environments: RwLock::new(environment_map),
            local_environment,
            http_client_factory,
        })
    }

    /// Returns the default environment instance.
    pub fn default_environment(&self) -> Option<Arc<Environment>> {
        self.default_environment
            .as_deref()
            .and_then(|environment_id| self.get_environment(environment_id))
    }

    /// Returns the id of the default environment.
    pub fn default_environment_id(&self) -> Option<&str> {
        self.default_environment.as_deref()
    }

    /// Returns the ordered environment ids used for new thread startup.
    pub fn default_environment_ids(&self) -> Vec<String> {
        let Some(default_environment_id) = self.default_environment.as_ref() else {
            return Vec::new();
        };
        let environments = self
            .environments
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut environment_ids = Vec::with_capacity(environments.len());
        environment_ids.push(default_environment_id.clone());
        environment_ids.extend(
            environments
                .keys()
                .filter(|environment_id| *environment_id != default_environment_id)
                .cloned(),
        );
        environment_ids
    }

    /// Returns the local environment instance when one is configured.
    pub fn try_local_environment(&self) -> Option<Arc<Environment>> {
        self.local_environment.as_ref().map(Arc::clone)
    }

    /// Returns a named environment instance.
    pub fn get_environment(&self, environment_id: &str) -> Option<Arc<Environment>> {
        self.environments
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(environment_id)
            .cloned()
    }

    /// Returns the outbound HTTP policy carried by this manager.
    pub fn http_client_factory(&self) -> &HttpClientFactory {
        &self.http_client_factory
    }

    /// Returns the current status of one named environment when it is configured.
    pub async fn get_environment_status(
        &self,
        environment_id: &str,
    ) -> Option<EnvironmentObservedStatus> {
        let environment = self.get_environment(environment_id)?;
        Some(environment.status().await)
    }
}

/// Concrete execution/filesystem environment selected for a session.
///
/// This bundles the selected backend metadata together with the local runtime
/// paths used by filesystem helpers.
#[derive(Clone)]
pub struct Environment {
    remote_client: Option<LazyRemoteExecServerClient>,
    ready_info: Arc<ArcSwapOption<EnvironmentReadyInfo>>,
    // Dropping the environment stops unfinished background startup work.
    startup_task: Arc<Mutex<Option<AbortOnDropHandle<()>>>>,
    exec_backend: Arc<dyn ExecBackend>,
    filesystem: Arc<dyn ExecutorFileSystem>,
    http_client: Arc<dyn HttpClient>,
    local_runtime_paths: Option<ExecServerRuntimePaths>,
}

impl Environment {
    /// Builds a test-only local environment without configured sandbox helper paths.
    pub fn default_for_tests() -> Self {
        Self {
            remote_client: None,
            ready_info: Arc::new(ArcSwapOption::empty()),
            startup_task: Arc::new(Mutex::new(None)),
            exec_backend: Arc::new(LocalProcess::default()),
            filesystem: Arc::new(LocalFileSystem::unsandboxed()),
            // Test-only construction has no application config from which to resolve proxy policy.
            http_client: Arc::new(RouteAwareHttpClient::new(HttpClientFactory::new(
                OutboundProxyPolicy::ReqwestDefault,
            ))),
            local_runtime_paths: None,
        }
    }
}

impl std::fmt::Debug for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Environment").finish_non_exhaustive()
    }
}

impl Environment {
    /// Builds an environment using the caller's effective outbound HTTP policy.
    pub fn create(
        exec_server_url: Option<String>,
        local_runtime_paths: ExecServerRuntimePaths,
        http_client_factory: HttpClientFactory,
    ) -> Result<Self, ExecServerError> {
        Self::create_inner(
            exec_server_url,
            Some(local_runtime_paths),
            http_client_factory,
        )
    }

    /// Builds a test-only environment without configured sandbox helper paths.
    pub fn create_for_tests(exec_server_url: Option<String>) -> Result<Self, ExecServerError> {
        Self::create_inner(
            exec_server_url,
            /*local_runtime_paths*/ None,
            HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
        )
    }

    /// Builds a local environment. The legacy URL argument is retained only
    /// for source compatibility with test-only callers and is ignored.
    fn create_inner(
        _exec_server_url: Option<String>,
        local_runtime_paths: Option<ExecServerRuntimePaths>,
        http_client_factory: HttpClientFactory,
    ) -> Result<Self, ExecServerError> {
        Ok(match local_runtime_paths {
            Some(local_runtime_paths) => Self::local(local_runtime_paths, http_client_factory),
            None => Self::default_for_tests(),
        })
    }

    pub(crate) fn local(
        local_runtime_paths: ExecServerRuntimePaths,
        http_client_factory: HttpClientFactory,
    ) -> Self {
        Self {
            remote_client: None,
            ready_info: Arc::new(ArcSwapOption::empty()),
            startup_task: Arc::new(Mutex::new(None)),
            exec_backend: Arc::new(LocalProcess::with_local_runtime_paths(
                local_runtime_paths.clone(),
            )),
            filesystem: Arc::new(LocalFileSystem::with_runtime_paths(
                local_runtime_paths.clone(),
            )),
            http_client: Arc::new(RouteAwareHttpClient::new(http_client_factory)),
            local_runtime_paths: Some(local_runtime_paths),
        }
    }

    pub(crate) fn remote_with_transport(
        remote_transport: ExecServerTransportParams,
        local_runtime_paths: Option<ExecServerRuntimePaths>,
        http_client_factory: HttpClientFactory,
    ) -> Self {
        let client = LazyRemoteExecServerClient::new(remote_transport, http_client_factory);
        Self::remote_with_client(client, local_runtime_paths)
    }

    pub(crate) fn remote_with_client(
        client: LazyRemoteExecServerClient,
        local_runtime_paths: Option<ExecServerRuntimePaths>,
    ) -> Self {
        let exec_backend: Arc<dyn ExecBackend> = Arc::new(RemoteProcess::new(client.clone()));
        let filesystem: Arc<dyn ExecutorFileSystem> =
            Arc::new(RemoteFileSystem::new(client.clone()));

        Self {
            remote_client: Some(client.clone()),
            ready_info: Arc::new(ArcSwapOption::empty()),
            startup_task: Arc::new(Mutex::new(None)),
            exec_backend,
            filesystem,
            http_client: Arc::new(client),
            local_runtime_paths,
        }
    }

    pub fn is_remote(&self) -> bool {
        self.remote_client.is_some()
    }

    /// Returns a snapshot of the last accepted Ready report.
    ///
    /// `None` means no Ready report has been accepted, including for ordinary environments.
    /// A report with no capability roots is distinct from `None`. The snapshot does not change
    /// when later reports arrive and does not indicate whether the connection is healthy.
    pub fn last_ready_info(&self) -> Option<Arc<EnvironmentReadyInfo>> {
        self.ready_info.load_full()
    }

    /// Returns the capability roots most recently reported for this environment.
    pub fn selected_capability_roots(&self) -> Vec<SelectedCapabilityRoot> {
        self.ready_info
            .load()
            .as_ref()
            .map_or_else(Vec::new, |ready_info| {
                ready_info.selected_capability_roots.clone()
            })
    }

    /// Subscribes to the current connection state for this remote environment.
    pub fn subscribe_connection_state(
        &self,
    ) -> Option<watch::Receiver<EnvironmentConnectionState>> {
        self.remote_client
            .as_ref()
            .map(LazyRemoteExecServerClient::subscribe_connection_state)
    }

    pub fn local_runtime_paths(&self) -> Option<&ExecServerRuntimePaths> {
        self.local_runtime_paths.as_ref()
    }

    /// Returns environment information from the selected execution/filesystem environment.
    /// Remote metadata is cached for the current client's lifetime.
    #[tracing::instrument(
        name = "exec_server.environment.info",
        skip_all,
        fields(remote = self.is_remote())
    )]
    pub async fn info(&self) -> Result<EnvironmentInfo, ExecServerError> {
        match &self.remote_client {
            Some(client) => client.environment_info().await,
            None => Ok(EnvironmentInfo::local()),
        }
    }

    /// Fetches uncached metadata, connecting or waiting for recovery as needed.
    // TODO: Remove after app-server migrates off of force_environment_info.
    #[tracing::instrument(
        name = "exec_server.environment.force_info",
        skip_all,
        fields(remote = self.is_remote())
    )]
    pub async fn force_info(&self) -> Result<EnvironmentInfo, ExecServerError> {
        match &self.remote_client {
            Some(client) => client.get().await?.force_environment_info().await,
            None => Ok(EnvironmentInfo::local()),
        }
    }

    /// Reads selected executor-local configuration fields for this environment.
    pub async fn read_environment_config(
        &self,
        params: EnvironmentConfigReadParams,
    ) -> Result<EnvironmentConfigReadResponse, ExecServerError> {
        match &self.remote_client {
            Some(client) => client.get().await?.read_environment_config(params).await,
            None => read_environment_config(self.filesystem.as_ref(), params)
                .await
                .map_err(|error| ExecServerError::Protocol(error.to_string())),
        }
    }

    /// Discovers plugin and skill manifests through the environment's high-level discovery API.
    pub async fn discover_capability_roots(
        &self,
        params: CapabilityRootsDiscoverParams,
    ) -> Result<CapabilityRootsDiscoverResponse, ExecServerError> {
        match &self.remote_client {
            Some(client) => {
                let mut connection_state = client.subscribe_connection_state();
                let client = client.get().await?;
                let discover = || async {
                    if params.roots.iter().any(|root| {
                        root.sandbox
                            .as_ref()
                            .is_some_and(crate::FileSystemSandboxContext::should_read_from_sandbox)
                    }) && !client
                        .environment_info()
                        .await?
                        .capabilities
                        .capability_discovery_sandbox
                    {
                        return Err(ExecServerError::Protocol(
                            "exec-server does not support sandboxed capability discovery"
                                .to_string(),
                        ));
                    }
                    client.discover_capability_roots(params.clone()).await
                };
                match discover().await {
                    Err(error) if crate::client::is_retryable_recovery_error(&error) => {
                        tracing::warn!(%error, "replaying capability discovery after executor recovery");
                        let recovered =
                            tokio::time::timeout(std::time::Duration::from_secs(8), async {
                                while self.readiness_result().is_none_or(|result| result.is_err()) {
                                    if connection_state.changed().await.is_err() {
                                        return false;
                                    }
                                }
                                true
                            })
                            .await
                            .unwrap_or(false);
                        if recovered {
                            discover().await
                        } else {
                            Err(error)
                        }
                    }
                    response => response,
                }
            }
            None => crate::discover_capability_roots(self.filesystem.as_ref(), params)
                .await
                .map_err(|error| ExecServerError::Protocol(error.to_string())),
        }
    }

    /// Starts connecting a remote environment without waiting for it.
    /// Requires an active Tokio runtime when background startup is supported.
    pub fn start_connecting(&self) {
        let Some(client) = &self.remote_client else {
            return;
        };
        let mut startup_task = self
            .startup_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if startup_task.is_none() {
            *startup_task = client.start_connecting();
        }
    }

    /// Starts the initial connection after an environment is actually selected for use.
    pub(crate) fn start_connecting_for_use(environment: &Arc<Self>) {
        let Some(client) = &environment.remote_client else {
            return;
        };
        let mut startup_task = environment
            .startup_task
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if startup_task.is_none() {
            let client = client.clone();
            *startup_task = Some(AbortOnDropHandle::new(tokio::spawn(
                async move {
                    if let Err(error) = client.wait_until_ready().await {
                        tracing::debug!(%error, "exec-server environment startup failed");
                    }
                }
                .in_current_span()
                .with_current_subscriber(),
            )));
        }
    }

    /// Returns whether startup has completed, including a first connection made by refresh.
    pub fn startup_finished(&self) -> bool {
        self.remote_client
            .as_ref()
            .is_none_or(LazyRemoteExecServerClient::startup_finished)
    }

    /// Waits for initial startup, retrying a previous transient failure when possible.
    #[tracing::instrument(
        name = "exec_server.environment.wait_until_ready",
        skip_all,
        fields(remote = self.is_remote())
    )]
    pub async fn wait_until_ready(&self) -> Result<(), ExecServerError> {
        match &self.remote_client {
            Some(client) => client.wait_until_ready().await,
            None => Ok(()),
        }
    }

    /// Returns whether the environment can serve a request without waiting or reconnecting.
    pub(crate) fn readiness_result(&self) -> Option<Result<(), ExecServerError>> {
        match &self.remote_client {
            Some(client) => client.readiness_result(),
            None => Some(Ok(())),
        }
    }

    /// Returns the environment's status without starting or recovering it.
    ///
    /// Local environments are always ready. Remote environments with an
    /// already-ready cached connection receive a fail-fast `environment/status`
    /// probe; other remote states are returned from cached connection state
    /// without waiting for startup or recovery.
    pub async fn status(&self) -> EnvironmentObservedStatus {
        match &self.remote_client {
            Some(client) => client.status().await,
            None => EnvironmentObservedStatus::Ready,
        }
    }

    pub fn get_exec_backend(&self) -> Arc<dyn ExecBackend> {
        Arc::clone(&self.exec_backend)
    }

    pub fn get_http_client(&self) -> Arc<dyn HttpClient> {
        Arc::clone(&self.http_client)
    }

    pub fn get_filesystem(&self) -> Arc<dyn ExecutorFileSystem> {
        Arc::clone(&self.filesystem)
    }

    /// Returns a filesystem view that fails instead of starting or waiting for a connection.
    pub fn get_filesystem_without_reconnect(&self) -> Arc<dyn ExecutorFileSystem> {
        match &self.remote_client {
            Some(client) => Arc::new(RemoteFileSystem::new(client.fail_fast())),
            None => Arc::clone(&self.filesystem),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use super::Environment;
    use super::EnvironmentManager;
    use super::EnvironmentObservedStatus;
    use super::LOCAL_ENVIRONMENT_ID;
    use crate::ExecServerRuntimePaths;
    use crate::ProcessId;
    use crate::client_api::ExecServerTransportParams;
    use crate::client_api::StdioExecServerCommand;
    use crate::environment_provider::EnvironmentDefault;
    use crate::environment_provider::EnvironmentProviderSnapshot;
    use genex_http_client::HttpClientFactory;
    use genex_http_client::OutboundProxyPolicy;
    use genex_utils_path_uri::PathUri;
    use pretty_assertions::assert_eq;

    fn legacy_http_client_factory() -> HttpClientFactory {
        HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault)
    }

    fn test_runtime_paths() -> ExecServerRuntimePaths {
        ExecServerRuntimePaths::new(
            std::env::current_exe().expect("current exe"),
            /*genex_linux_sandbox_exe*/ None,
        )
        .expect("runtime paths")
    }

    fn assert_local_environment_unavailable(manager: &EnvironmentManager) {
        assert!(manager.try_local_environment().is_none());
    }

    #[test]
    fn local_environment_info_includes_current_directory() {
        let info = super::EnvironmentInfo::local();

        assert_eq!(
            info.cwd,
            Some(
                PathUri::from_host_native_path(std::env::current_dir().expect("current directory"))
                    .expect("cwd URI")
            )
        );
        assert_eq!(
            info.temp_dir,
            PathUri::from_host_native_path(std::env::temp_dir()).ok()
        );
    }

    #[tokio::test]
    async fn create_local_environment_does_not_connect() {
        let environment = Environment::create(
            /*exec_server_url*/ None,
            test_runtime_paths(),
            legacy_http_client_factory(),
        )
        .expect("create environment");

        assert!(!environment.is_remote());
        assert!(environment.info().await.is_ok());
    }

    #[tokio::test]
    async fn environment_manager_normalizes_empty_url() {
        let manager =
            EnvironmentManager::create_for_tests(Some(String::new()), Some(test_runtime_paths()))
                .await;

        let environment = manager.default_environment().expect("default environment");
        assert_eq!(manager.default_environment_id(), Some(LOCAL_ENVIRONMENT_ID));
        assert!(Arc::ptr_eq(
            &environment,
            &manager
                .get_environment(LOCAL_ENVIRONMENT_ID)
                .expect("local environment")
        ));
        assert!(Arc::ptr_eq(
            &environment,
            &manager.try_local_environment().expect("local environment")
        ));
        assert!(manager.try_local_environment().is_some());
        assert!(!environment.is_remote());
    }

    #[tokio::test]
    async fn disabled_environment_manager_has_no_default_or_local_environment() {
        let manager = EnvironmentManager::without_environments(HttpClientFactory::new(
            OutboundProxyPolicy::RespectSystemProxy,
        ));

        assert!(manager.default_environment().is_none());
        assert_eq!(manager.default_environment_id(), None);
        assert_local_environment_unavailable(&manager);
        assert!(manager.get_environment(LOCAL_ENVIRONMENT_ID).is_none());
        assert_eq!(
            manager.http_client_factory().outbound_proxy_policy(),
            OutboundProxyPolicy::RespectSystemProxy
        );
    }

    async fn environment_manager_default_environment_caches_environment() {
        let manager = EnvironmentManager::default_for_tests();

        let first = manager.default_environment().expect("default environment");
        let second = manager.default_environment().expect("default environment");

        assert!(Arc::ptr_eq(&first, &second));
        assert!(Arc::ptr_eq(
            &first.get_filesystem(),
            &second.get_filesystem()
        ));
    }

    async fn environment_manager_includes_local_for_default_provider_without_url() {
        let manager = EnvironmentManager::create_for_tests(
            /*exec_server_url*/ None,
            Some(test_runtime_paths()),
        )
        .await;

        let environment = manager.default_environment().expect("default environment");
        assert_eq!(manager.default_environment_id(), Some(LOCAL_ENVIRONMENT_ID));
        assert!(Arc::ptr_eq(
            &environment,
            &manager
                .get_environment(LOCAL_ENVIRONMENT_ID)
                .expect("local environment")
        ));
        assert!(Arc::ptr_eq(
            &environment,
            &manager.try_local_environment().expect("local environment")
        ));
        assert!(!environment.is_remote());
    }

    #[tokio::test]
    async fn environment_manager_carries_local_runtime_paths() {
        let runtime_paths = test_runtime_paths();
        let manager = EnvironmentManager::create_for_tests(
            /*exec_server_url*/ None,
            Some(runtime_paths.clone()),
        )
        .await;

        let environment = manager.try_local_environment().expect("local environment");

        assert_eq!(environment.local_runtime_paths(), Some(&runtime_paths));
        let manager = EnvironmentManager::create_for_tests(
            /*exec_server_url*/ None,
            Some(
                environment
                    .local_runtime_paths()
                    .expect("local runtime paths")
                    .clone(),
            ),
        )
        .await;
        let environment = manager.try_local_environment().expect("local environment");
        assert_eq!(environment.local_runtime_paths(), Some(&runtime_paths));
    }

    #[tokio::test]
    async fn environment_manager_snapshot_without_local_environment_disables_local_default() {
        let mut snapshot = EnvironmentProviderSnapshot {
            environments: Vec::new(),
            default: EnvironmentDefault::EnvironmentId(LOCAL_ENVIRONMENT_ID.to_string()),
            include_local: true,
        };
        snapshot.include_local = false;
        snapshot.default = EnvironmentDefault::Disabled;
        let manager = EnvironmentManager::from_snapshot(
            snapshot,
            /*local_runtime_paths*/ None,
            legacy_http_client_factory(),
        )
        .expect("environment manager");

        assert!(manager.default_environment().is_none());
        assert_eq!(manager.default_environment_id(), None);
        assert!(manager.get_environment(LOCAL_ENVIRONMENT_ID).is_none());
        assert_local_environment_unavailable(&manager);
    }

    #[tokio::test]
    async fn get_environment_returns_none_for_unknown_id() {
        let manager = EnvironmentManager::default_for_tests();

        assert!(manager.get_environment("does-not-exist").is_none());
    }

    #[tokio::test]
    async fn environment_status_keeps_stdio_environment_pending() {
        let environment = Environment::remote_with_transport(
            ExecServerTransportParams::StdioCommand {
                command: StdioExecServerCommand {
                    program: "genex-missing-exec-server-for-test".to_string(),
                    args: Vec::new(),
                    env: HashMap::new(),
                    cwd: None,
                },
                initialize_timeout: Duration::from_secs(1),
            },
            /*local_runtime_paths*/ None,
            legacy_http_client_factory(),
        );

        assert_eq!(
            environment.status().await,
            EnvironmentObservedStatus::Pending
        );
        assert!(!environment.startup_finished());
    }

    #[tokio::test]
    async fn environment_manager_leaves_stdio_environment_lazy() {
        let transport = ExecServerTransportParams::StdioCommand {
            command: StdioExecServerCommand {
                program: "genex-missing-exec-server-for-test".to_string(),
                args: Vec::new(),
                env: HashMap::new(),
                cwd: None,
            },
            initialize_timeout: Duration::from_secs(1),
        };
        let manager = EnvironmentManager::from_snapshot(
            EnvironmentProviderSnapshot {
                environments: vec![("stdio".to_string(), transport)],
                default: EnvironmentDefault::Disabled,
                include_local: false,
            },
            /*local_runtime_paths*/ None,
            legacy_http_client_factory(),
        )
        .expect("environment manager");
        let environment = manager.get_environment("stdio").expect("stdio environment");

        assert!(!environment.startup_finished());
        assert!(environment.wait_until_ready().await.is_err());
        assert!(environment.startup_finished());
    }

    #[tokio::test]
    async fn selected_capability_inspection_keeps_stdio_environment_lazy() {
        use genex_protocol::capabilities::CapabilityRootLocation;
        use genex_protocol::capabilities::SelectedCapabilityRoot;

        let transport = ExecServerTransportParams::StdioCommand {
            command: StdioExecServerCommand {
                program: "genex-missing-exec-server-for-test".to_string(),
                args: Vec::new(),
                env: HashMap::new(),
                cwd: None,
            },
            initialize_timeout: Duration::from_secs(1),
        };
        let manager = EnvironmentManager::from_snapshot(
            EnvironmentProviderSnapshot {
                environments: vec![("stdio".to_string(), transport)],
                default: EnvironmentDefault::Disabled,
                include_local: false,
            },
            /*local_runtime_paths*/ None,
            legacy_http_client_factory(),
        )
        .expect("environment manager");
        let environment = manager.get_environment("stdio").expect("stdio environment");
        let selected_root = SelectedCapabilityRoot {
            id: "demo@1".to_string(),
            location: CapabilityRootLocation::Environment {
                environment_id: "stdio".to_string(),
                path: PathUri::parse("file:///plugins/demo").expect("plugin path URI"),
            },
        };

        let status =
            manager.inspect_selected_capability_roots(std::slice::from_ref(&selected_root));
        assert!(status.ready_roots.is_empty());
        assert_eq!(status.warnings, Vec::<String>::new());
        assert!(!environment.startup_finished());

        let missing_root = SelectedCapabilityRoot {
            id: "missing@1".to_string(),
            location: CapabilityRootLocation::Environment {
                environment_id: "missing".to_string(),
                path: PathUri::parse("file:///plugins/missing").expect("missing plugin path URI"),
            },
        };
        let status = manager.inspect_selected_capability_roots(&[missing_root]);
        assert!(status.ready_roots.is_empty());
        assert_eq!(
            status.warnings,
            vec![
                "selected capability root `missing@1` references unavailable environment `missing`"
                    .to_string()
            ]
        );

        assert!(environment.wait_until_ready().await.is_err());

        let status = manager.inspect_selected_capability_roots(&[selected_root]);
        assert!(status.ready_roots.is_empty());
        assert_eq!(status.warnings.len(), 1);
        assert!(status.warnings[0].contains("environment `stdio` is unavailable"));
    }

    #[tokio::test]
    async fn default_environment_has_ready_local_executor() {
        let environment = Environment::default_for_tests();

        let response = environment
            .get_exec_backend()
            .start(crate::ExecParams {
                metadata: Default::default(),
                process_id: ProcessId::from("default-env-proc"),
                argv: vec!["true".to_string()],
                cwd: PathUri::from_host_native_path(
                    std::env::current_dir().expect("read current dir"),
                )
                .expect("cwd URI"),
                shell_snapshot: None,
                env_policy: None,
                env: Default::default(),
                tty: false,
                pipe_stdin: false,
                arg0: None,
                sandbox: None,
                enforce_managed_network: false,
                managed_network: None,
                network_proxy: None,
            })
            .await
            .expect("start process");

        assert_eq!(response.process.process_id().as_str(), "default-env-proc");
    }

    #[tokio::test]
    async fn local_environment_passes_runtime_paths_to_exec_backend() {
        let environment = Environment::local(test_runtime_paths(), legacy_http_client_factory());
        let uri = "file://server/share/checkout";
        let sandbox_cwd = PathUri::parse(uri).expect("non-native sandbox cwd URI");
        let source = sandbox_cwd
            .to_abs_path()
            .expect_err("sandbox cwd should not be native to this host");
        let sandbox = crate::FileSystemSandboxContext::from_permission_profile(
            genex_protocol::models::PermissionProfile::workspace_write(),
            sandbox_cwd.clone(),
        );

        let result = environment
            .get_exec_backend()
            .start(crate::ExecParams {
                metadata: Default::default(),
                process_id: ProcessId::from("local-sandbox-proc"),
                argv: vec!["true".to_string()],
                cwd: PathUri::from_host_native_path(
                    std::env::current_dir().expect("read current dir"),
                )
                .expect("cwd URI"),
                shell_snapshot: None,
                env_policy: None,
                env: Default::default(),
                tty: false,
                pipe_stdin: false,
                arg0: None,
                sandbox: Some(sandbox),
                enforce_managed_network: false,
                managed_network: None,
                network_proxy: None,
            })
            .await;
        let Err(err) = result else {
            panic!("sandbox cwd should be rejected after resolving runtime paths");
        };

        assert_eq!(
            err.to_string(),
            format!(
                "exec-server rejected request (-32602): sandbox cwd URI `{sandbox_cwd}` is not valid on this exec-server host: {source}"
            )
        );
    }

    #[tokio::test]
    async fn test_environment_rejects_sandboxed_filesystem_without_runtime_paths() {
        let environment = Environment::default_for_tests();
        let path = genex_utils_absolute_path::AbsolutePathBuf::from_absolute_path(
            std::env::current_exe().expect("current exe").as_path(),
        )
        .expect("absolute current exe");
        let path = genex_utils_path_uri::PathUri::from_abs_path(&path);
        let sandbox = crate::FileSystemSandboxContext::from_permission_profile(
            genex_protocol::models::PermissionProfile::from_runtime_permissions(
                &genex_protocol::permissions::FileSystemSandboxPolicy::restricted(Vec::new()),
                genex_protocol::permissions::NetworkSandboxPolicy::Restricted,
            ),
            PathUri::from_host_native_path(std::env::current_dir().expect("read current dir"))
                .expect("cwd URI"),
        );

        let err = environment
            .get_filesystem()
            .read_file(&path, Default::default(), Some(&sandbox))
            .await
            .expect_err("sandboxed read should require runtime paths");

        assert_eq!(
            err.to_string(),
            "sandboxed filesystem operations require configured runtime paths"
        );
    }
}

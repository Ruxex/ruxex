use codex_http_client::HttpClientFactory;

use crate::EnvironmentManager;
use crate::ExecServerError;
use crate::ExecServerRuntimePaths;
use crate::environment_provider::EnvironmentProviderSnapshot;

#[derive(Debug)]
pub(crate) enum PreparedEnvironmentSource {
    Snapshot(EnvironmentProviderSnapshot),
}

/// Holds discovered execution environments before their HTTP policy is resolved.
///
/// Preparing environments does not start remote connections. Callers can inspect
/// the default environment to choose config-loading behavior and then build the
/// manager with the effective outbound HTTP policy.
#[derive(Debug)]
pub struct PreparedEnvironmentManager {
    pub(crate) source: PreparedEnvironmentSource,
}

impl PreparedEnvironmentManager {
    /// Returns whether the discovered default environment is remote.
    pub fn default_environment_is_remote(&self) -> bool {
        match &self.source {
            PreparedEnvironmentSource::Snapshot(_) => false,
        }
    }

    /// Builds the manager and starts remote connections using the supplied policy.
    pub fn build(
        self,
        local_runtime_paths: Option<ExecServerRuntimePaths>,
        http_client_factory: HttpClientFactory,
    ) -> Result<EnvironmentManager, ExecServerError> {
        let PreparedEnvironmentSource::Snapshot(snapshot) = self.source;
        EnvironmentManager::from_snapshot(snapshot, local_runtime_paths, http_client_factory)
    }
}

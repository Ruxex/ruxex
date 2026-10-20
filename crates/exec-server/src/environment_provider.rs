use std::future::Future;
use std::pin::Pin;

use crate::ExecServerError;
use crate::client_api::ExecServerTransportParams;
use crate::environment::LOCAL_ENVIRONMENT_ID;

/// Provides the execution environments configured for startup.
pub trait EnvironmentProvider: Send + Sync {
    fn snapshot(&self) -> EnvironmentProviderFuture<'_>;
}

pub type EnvironmentProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<EnvironmentProviderSnapshot, ExecServerError>> + Send + 'a>>;

#[derive(Clone)]
pub struct EnvironmentProviderSnapshot {
    pub(crate) environments: Vec<(String, ExecServerTransportParams)>,
    pub default: EnvironmentDefault,
    pub include_local: bool,
}

impl std::fmt::Debug for EnvironmentProviderSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let environment_ids: Vec<_> = self.environments.iter().map(|(id, _)| id).collect();
        f.debug_struct("EnvironmentProviderSnapshot")
            .field("environments", &environment_ids)
            .field("default", &self.default)
            .field("include_local", &self.include_local)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EnvironmentDefault {
    Disabled,
    EnvironmentId(String),
}

/// Default provider for installations without an `environments.toml` file.
///
/// The provider-neutral default is the local environment. Remote exec-server
/// URLs are no longer read from process environment variables.
#[derive(Clone, Debug, Default)]
pub struct DefaultEnvironmentProvider;

impl DefaultEnvironmentProvider {
    pub fn new(_ignored_legacy_value: Option<String>) -> Self {
        Self
    }

    pub fn from_env() -> Self {
        Self
    }

    pub(crate) fn snapshot_inner(&self) -> EnvironmentProviderSnapshot {
        EnvironmentProviderSnapshot {
            environments: Vec::new(),
            default: EnvironmentDefault::EnvironmentId(LOCAL_ENVIRONMENT_ID.to_string()),
            include_local: true,
        }
    }
}

impl EnvironmentProvider for DefaultEnvironmentProvider {
    fn snapshot(&self) -> EnvironmentProviderFuture<'_> {
        Box::pin(async { Ok(self.snapshot_inner()) })
    }
}

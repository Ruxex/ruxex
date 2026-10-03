use genex_state::SqliteConfig;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

pub trait RolloutConfigView {
    fn genex_home(&self) -> &Path;
    fn sqlite_config(&self) -> &SqliteConfig;
    fn cwd(&self) -> &Path;
    fn model_provider_id(&self) -> &str;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RolloutConfig {
    pub genex_home: PathBuf,
    pub sqlite: SqliteConfig,
    pub cwd: PathBuf,
    pub model_provider_id: String,
}

pub type Config = RolloutConfig;

impl RolloutConfig {
    pub fn from_view(view: &impl RolloutConfigView) -> Self {
        Self {
            genex_home: view.genex_home().to_path_buf(),
            sqlite: view.sqlite_config().clone(),
            cwd: view.cwd().to_path_buf(),
            model_provider_id: view.model_provider_id().to_string(),
        }
    }
}

impl RolloutConfigView for RolloutConfig {
    fn genex_home(&self) -> &Path {
        self.genex_home.as_path()
    }

    fn sqlite_config(&self) -> &SqliteConfig {
        &self.sqlite
    }

    fn cwd(&self) -> &Path {
        self.cwd.as_path()
    }

    fn model_provider_id(&self) -> &str {
        self.model_provider_id.as_str()
    }
}

impl<T: RolloutConfigView + ?Sized> RolloutConfigView for &T {
    fn genex_home(&self) -> &Path {
        (*self).genex_home()
    }

    fn sqlite_config(&self) -> &SqliteConfig {
        (*self).sqlite_config()
    }

    fn cwd(&self) -> &Path {
        (*self).cwd()
    }

    fn model_provider_id(&self) -> &str {
        (*self).model_provider_id()
    }
}

impl<T: RolloutConfigView + ?Sized> RolloutConfigView for Arc<T> {
    fn genex_home(&self) -> &Path {
        self.as_ref().genex_home()
    }

    fn sqlite_config(&self) -> &SqliteConfig {
        self.as_ref().sqlite_config()
    }

    fn cwd(&self) -> &Path {
        self.as_ref().cwd()
    }

    fn model_provider_id(&self) -> &str {
        self.as_ref().model_provider_id()
    }
}

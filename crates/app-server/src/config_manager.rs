use ruxex_arg0::Arg0DispatchPaths;
use ruxex_config::ConfigLayerStack;
use ruxex_config::LoaderOverrides;
use ruxex_config::ThreadConfigLoader;
use ruxex_config::loader::load_config_layers_state;
use ruxex_core::config::Config;
use ruxex_core::config::ConfigBuilder;
use ruxex_core::config::ConfigOverrides;
use ruxex_exec_server::LOCAL_FS;
use ruxex_features::feature_for_key;
use ruxex_login::default_client::set_default_client_residency_requirement;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_json_to_toml::json_to_toml;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::RwLock;
use toml::Value as TomlValue;
use tracing::instrument;
use tracing::warn;

/// Shared app-server entry point for loading effective Ruxex configuration.
#[derive(Clone)]
pub(crate) struct ConfigManager {
    ruxex_home: PathBuf,
    cli_overrides: Arc<RwLock<Vec<(String, TomlValue)>>>,
    runtime_feature_enablement: Arc<RwLock<BTreeMap<String, bool>>>,
    loader_overrides: LoaderOverrides,
    strict_config: bool,
    arg0_paths: Arg0DispatchPaths,
    thread_config_loader: Arc<dyn ThreadConfigLoader>,
}

impl ConfigManager {
    pub(crate) fn new(
        ruxex_home: PathBuf,
        cli_overrides: Vec<(String, TomlValue)>,
        loader_overrides: LoaderOverrides,
        strict_config: bool,
        arg0_paths: Arg0DispatchPaths,
        thread_config_loader: Arc<dyn ThreadConfigLoader>,
    ) -> Self {
        Self {
            ruxex_home,
            cli_overrides: Arc::new(RwLock::new(cli_overrides)),
            runtime_feature_enablement: Arc::new(RwLock::new(BTreeMap::new())),
            loader_overrides,
            strict_config,
            arg0_paths,
            thread_config_loader,
        }
    }

    pub(crate) fn ruxex_home(&self) -> &Path {
        self.ruxex_home.as_path()
    }

    pub(crate) fn user_config_path(&self) -> std::io::Result<AbsolutePathBuf> {
        self.loader_overrides.user_config_path(self.ruxex_home())
    }

    pub(crate) fn current_cli_overrides(&self) -> Vec<(String, TomlValue)> {
        self.cli_overrides
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub(crate) fn extend_runtime_feature_enablement<I>(&self, enablement: I) -> Result<(), ()>
    where
        I: IntoIterator<Item = (String, bool)>,
    {
        let mut runtime_feature_enablement =
            self.runtime_feature_enablement.write().map_err(|_| ())?;
        runtime_feature_enablement.extend(enablement);
        Ok(())
    }

    pub(crate) async fn sync_default_client_residency_requirement(&self) {
        match self.load_latest_config(/*fallback_cwd*/ None).await {
            Ok(config) => {
                set_default_client_residency_requirement(config.enforce_residency.value());
            }
            Err(err) => warn!(
                error = %err,
                "failed to sync default client residency requirement after auth refresh"
            ),
        }
    }

    pub(crate) async fn load_latest_config(
        &self,
        fallback_cwd: Option<PathBuf>,
    ) -> std::io::Result<Config> {
        self.load_with_cli_overrides(
            &self.current_cli_overrides(),
            /*request_overrides*/ None,
            ConfigOverrides::default(),
            fallback_cwd,
        )
        .await
    }

    /// Loads system, user, and runtime settings without discovering a project
    /// from the app-server process's working directory.
    pub(crate) async fn load_non_project_config(&self) -> std::io::Result<Config> {
        let mut manager = self.clone();
        manager.loader_overrides.ignore_project_config = true;
        manager.load_latest_config(/*fallback_cwd*/ None).await
    }

    pub(crate) async fn load_latest_config_with_session_layers(
        &self,
        session_layers: &ConfigLayerStack,
        cwd: &Path,
    ) -> std::io::Result<Config> {
        let refreshed_config = self.load_latest_config(Some(cwd.to_path_buf())).await?;
        let mut config = Config::rebuild_with_session_layers(
            session_layers,
            cwd.to_path_buf(),
            &refreshed_config,
        )
        .await?;
        self.apply_runtime_feature_enablement(&mut config);
        self.apply_arg0_paths(&mut config);
        Ok(config)
    }

    pub(crate) async fn load_default_config(&self) -> std::io::Result<Config> {
        let mut loader_overrides = self.loader_overrides.clone();
        loader_overrides.ignore_user_config = true;
        let mut config = ConfigBuilder::default()
            .ruxex_home(self.ruxex_home.clone())
            .cli_overrides(self.current_cli_overrides())
            .loader_overrides(loader_overrides)
            .fallback_cwd(Some(self.ruxex_home.clone()))
            .build()
            .await?;
        self.apply_runtime_feature_enablement(&mut config);
        self.apply_arg0_paths(&mut config);
        Ok(config)
    }

    pub(crate) async fn load_with_overrides(
        &self,
        request_overrides: Option<HashMap<String, serde_json::Value>>,
        typesafe_overrides: ConfigOverrides,
    ) -> std::io::Result<Config> {
        self.load_with_cli_overrides(
            &self.current_cli_overrides(),
            request_overrides,
            typesafe_overrides,
            /*fallback_cwd*/ None,
        )
        .await
    }

    pub(crate) async fn load_for_cwd(
        &self,
        request_overrides: Option<HashMap<String, serde_json::Value>>,
        typesafe_overrides: ConfigOverrides,
        cwd: Option<PathBuf>,
    ) -> std::io::Result<Config> {
        self.load_with_cli_overrides(
            &self.current_cli_overrides(),
            request_overrides,
            typesafe_overrides,
            cwd,
        )
        .await
    }

    /// Reload sources using the task's session flags before materializing config.
    pub(crate) async fn load_permission_config_for_thread(
        &self,
        thread_config: &Config,
        cwd: AbsolutePathBuf,
        permission_profile: String,
    ) -> std::io::Result<Config> {
        let mut session_flags = TomlValue::Table(Default::default());
        for layer in thread_config.config_layer_stack.layers_low_to_high() {
            if matches!(layer.name, ruxex_config::ConfigLayerSource::SessionFlags) {
                ruxex_config::merge_toml_values(&mut session_flags, &layer.config);
            }
        }
        let overrides = session_flags
            .as_table()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "session flags must be a table",
                )
            })?
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>();
        self.load_with_cli_overrides(
            &overrides,
            /*request_overrides*/ None,
            ConfigOverrides {
                cwd: Some(cwd.to_path_buf()),
                default_permissions: Some(permission_profile),
                ruxex_linux_sandbox_exe: self.arg0_paths.ruxex_linux_sandbox_exe.clone(),
                ..Default::default()
            },
            Some(cwd.to_path_buf()),
        )
        .await
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) async fn load_with_cli_overrides(
        &self,
        cli_overrides: &[(String, TomlValue)],
        request_overrides: Option<HashMap<String, serde_json::Value>>,
        mut typesafe_overrides: ConfigOverrides,
        fallback_cwd: Option<PathBuf>,
    ) -> std::io::Result<Config> {
        let mut request_overrides = request_overrides.unwrap_or_default();
        if let Some(value) = request_overrides.remove("bypass_hook_trust") {
            typesafe_overrides.bypass_hook_trust = Some(value.as_bool().ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "`bypass_hook_trust` override must be a boolean",
                )
            })?);
        }
        let merged_cli_overrides = cli_overrides
            .iter()
            .cloned()
            .chain(
                request_overrides
                    .into_iter()
                    .map(|(key, value)| (key, json_to_toml(value))),
            )
            .collect::<Vec<_>>();
        let mut config = ruxex_core::config::ConfigBuilder::default()
            .ruxex_home(self.ruxex_home.clone())
            .cli_overrides(merged_cli_overrides)
            .loader_overrides(self.loader_overrides.clone())
            .strict_config(self.strict_config)
            .harness_overrides(typesafe_overrides)
            .fallback_cwd(fallback_cwd)
            .thread_config_loader(Arc::clone(&self.thread_config_loader))
            .build()
            .await?;
        self.apply_runtime_feature_enablement(&mut config);
        self.apply_arg0_paths(&mut config);
        Ok(config)
    }

    pub(crate) async fn load_config_layers_for_cwd(
        &self,
        cwd: AbsolutePathBuf,
    ) -> std::io::Result<ConfigLayerStack> {
        self.load_config_layers(Some(cwd)).await
    }

    pub(crate) async fn load_config_layers(
        &self,
        cwd: Option<AbsolutePathBuf>,
    ) -> std::io::Result<ConfigLayerStack> {
        load_config_layers_state(
            LOCAL_FS.as_ref(),
            &self.ruxex_home,
            cwd,
            &self.current_cli_overrides(),
            ruxex_config::ConfigLoadOptions {
                loader_overrides: self.loader_overrides.clone(),
                strict_config: self.strict_config,
            },
            self.thread_config_loader.as_ref(),
        )
        .await
    }

    fn apply_runtime_feature_enablement(&self, config: &mut Config) {
        apply_runtime_feature_enablement(config, &self.current_runtime_feature_enablement());
    }

    fn current_runtime_feature_enablement(&self) -> BTreeMap<String, bool> {
        self.runtime_feature_enablement
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    fn apply_arg0_paths(&self, config: &mut Config) {
        config.ruxex_self_exe = self.arg0_paths.ruxex_self_exe.clone();
        config.ruxex_linux_sandbox_exe = self.arg0_paths.ruxex_linux_sandbox_exe.clone();
    }
}

pub(crate) fn protected_feature_keys(config_layer_stack: &ConfigLayerStack) -> BTreeSet<String> {
    let mut protected_features = config_layer_stack
        .effective_config()
        .get("features")
        .and_then(toml::Value::as_table)
        .map(|features| features.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();

    if let Some(feature_requirements) = config_layer_stack
        .requirements_toml()
        .feature_requirements
        .as_ref()
    {
        protected_features.extend(feature_requirements.entries.keys().cloned());
    }

    protected_features
}

pub(crate) fn apply_runtime_feature_enablement(
    config: &mut Config,
    runtime_feature_enablement: &BTreeMap<String, bool>,
) {
    let protected_features = protected_feature_keys(&config.config_layer_stack);
    for (name, enabled) in runtime_feature_enablement {
        if protected_features.contains(name) {
            continue;
        }
        let Some(feature) = feature_for_key(name) else {
            continue;
        };
        if let Err(err) = config.features.set_enabled(feature, *enabled) {
            warn!(
                feature = name,
                error = %err,
                "failed to apply runtime feature enablement"
            );
        }
    }
}

#[path = "marketplace_context.rs"]
mod marketplace_context;
pub use marketplace_context::PluginMarketplaceContext;
pub use marketplace_context::PluginMarketplaceScope;

use super::LoadedPlugin;
use super::PluginLoadOutcome;
use crate::PluginGitMode;
use crate::installed_marketplaces::installed_marketplace_roots_from_layer_stack;
use crate::loaded_cache_metrics;
use crate::loaded_cache_metrics::RequestOutcome;
use crate::loader::load_plugin_hooks;
use crate::loader::load_plugin_mcp_servers_from_manifest_with_format;
use crate::loader::load_plugin_skill_inventory;
use crate::loader::load_plugins_from_layer_stack;
use crate::loader::log_plugin_load_errors;
use crate::loader::materialize_marketplace_plugin_source;
#[cfg(test)]
use crate::loader::plugin_capability_summary_from_root;
use crate::loader::refresh_non_curated_plugin_cache_detailed;
use crate::loader::refresh_non_curated_plugin_cache_force_reinstall_detailed;
use crate::manifest::PluginManifestFormat;
use crate::manifest::PluginManifestInterface;
use crate::manifest::load_plugin_manifest;
use crate::manifest::load_plugin_manifest_with_format;
use crate::marketplace::MarketplaceError;
use crate::marketplace::MarketplaceInterface;
use crate::marketplace::MarketplaceListError;
use crate::marketplace::MarketplaceListOutcome;
use crate::marketplace::MarketplacePluginAuthPolicy;
use crate::marketplace::MarketplacePluginManifestFallback;
use crate::marketplace::MarketplacePluginPolicy;
use crate::marketplace::MarketplacePluginSource;
use crate::marketplace::ResolvedMarketplacePlugin;
use crate::marketplace::find_installable_marketplace_plugin;
use crate::marketplace::find_marketplace_plugin;
use crate::marketplace::home_dir;
use crate::marketplace::list_marketplaces_with_home;
use crate::marketplace::plugin_interface_with_marketplace_category;
use crate::marketplace_policy::MarketplacePolicy;
use crate::marketplace_policy::configured_plugins_from_stack;
use crate::marketplace_upgrade::ConfigLayerReload;
use crate::marketplace_upgrade::ConfiguredMarketplaceUpgradeError;
use crate::marketplace_upgrade::ConfiguredMarketplaceUpgradeOutcome;
use crate::marketplace_upgrade::upgrade_configured_git_marketplaces_with_mode;
use crate::skill_snapshots::new_plugin_skill_snapshots;
use crate::store::PluginInstallResult as StorePluginInstallResult;
use crate::store::PluginStore;
use crate::store::PluginStoreError;
use crate::store::error_context_sub_error_type;
use ruxex_config::ConfigLayerStack;
use ruxex_config::SkillConfigRules;
use ruxex_config::clear_user_plugin;
use ruxex_config::set_user_plugin_enabled;
use ruxex_config::skill_config_rules_from_stack;
use ruxex_config::types::PluginConfig;
use ruxex_credential_broker::SharedCredentialBroker;
use ruxex_hooks::plugin_hook_declarations;
use ruxex_plugin::PluginCapabilitySummary;
use ruxex_plugin::PluginId;
use ruxex_plugin::PluginIdError;
use ruxex_plugin::prompt_safe_plugin_description;
use ruxex_protocol::protocol::HookEventName;
use ruxex_protocol::protocol::Product;
use ruxex_skills::SkillMetadata;
use ruxex_skills::SkillRootLoader;
use ruxex_skills::SkillRootSnapshots;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_plugins::PluginIdentity;
use ruxex_utils_plugins::PluginSkillRoot;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::time::Instant;
use tokio::sync::Semaphore;
use tokio::sync::watch;
use tracing::instrument;
use tracing::warn;

const LOADED_PLUGINS_CACHE_CAPACITY: usize = 8;

type EffectivePluginsChangedCallback = Arc<dyn Fn(EffectivePluginsChange) + Send + Sync + 'static>;

#[derive(Debug, Clone)]
pub struct PluginsConfigInput {
    pub config_layer_stack: ConfigLayerStack,
    pub plugins_enabled: bool,
}

impl PluginsConfigInput {
    pub fn new(config_layer_stack: ConfigLayerStack, plugins_enabled: bool) -> Self {
        Self {
            config_layer_stack,
            plugins_enabled,
        }
    }
}

/// Effective-plugin changes that downstream composition layers may act on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EffectivePluginsChange {}

#[derive(Clone, PartialEq, Eq)]
struct NonCuratedCacheRefreshRequest {
    roots: Vec<AbsolutePathBuf>,
    configured_plugin_keys: Vec<String>,
    configured_plugin_sources: Vec<NonCuratedPluginSource>,
    mode: NonCuratedCacheRefreshMode,
    git_mode: PluginGitMode,
}

#[derive(Clone, PartialEq, Eq)]
struct NonCuratedPluginSource {
    marketplace_path: AbsolutePathBuf,
    plugin_key: String,
    source: MarketplacePluginSource,
    local_version: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NonCuratedCacheRefreshMode {
    IfVersionChanged,
    ForceReinstall,
}

#[derive(Default)]
struct NonCuratedCacheRefreshState {
    requested: Option<NonCuratedCacheRefreshRequest>,
    last_refreshed: Option<NonCuratedCacheRefreshRequest>,
    in_flight: bool,
}

#[derive(Clone, Copy, Default)]
struct NonCuratedCacheRefreshCompletion {
    sequence: u64,
    changed_sequence: u64,
}

#[derive(Default)]
struct ConfiguredMarketplaceUpgradeState {
    in_flight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInstallRequest {
    pub plugin_name: String,
    pub marketplace_path: AbsolutePathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginReadRequest {
    pub plugin_name: String,
    pub marketplace_path: AbsolutePathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInstallOutcome {
    pub plugin_id: PluginId,
    pub plugin_version: String,
    pub installed_path: AbsolutePathBuf,
    pub auth_policy: MarketplacePluginAuthPolicy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluginReadOutcome {
    pub marketplace_name: String,
    pub marketplace_path: Option<AbsolutePathBuf>,
    pub plugin: PluginDetail,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluginDetail {
    pub id: String,
    pub name: String,
    pub local_version: Option<String>,
    pub description: Option<String>,
    pub source: MarketplacePluginSource,
    pub policy: MarketplacePluginPolicy,
    pub interface: Option<PluginManifestInterface>,
    pub keywords: Vec<String>,
    pub installed: bool,
    pub enabled: bool,
    pub skills: Vec<SkillMetadata>,
    pub disabled_skill_paths: HashSet<AbsolutePathBuf>,
    pub onboarding_skill: Option<AbsolutePathBuf>,
    pub hooks: Vec<PluginHookSummary>,
    pub mcp_server_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PluginHookSummary {
    pub key: String,
    pub event_name: HookEventName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredMarketplace {
    pub name: String,
    pub path: AbsolutePathBuf,
    pub interface: Option<MarketplaceInterface>,
    pub plugins: Vec<ConfiguredMarketplacePlugin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredMarketplacePlugin {
    pub id: String,
    pub name: String,
    pub local_version: Option<String>,
    pub installed_version: Option<String>,
    pub source: MarketplacePluginSource,
    pub policy: MarketplacePluginPolicy,
    pub interface: Option<PluginManifestInterface>,
    pub keywords: Vec<String>,
    pub manifest_fallback: Option<MarketplacePluginManifestFallback>,
    pub installed: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfiguredMarketplaceListOutcome {
    pub marketplaces: Vec<ConfiguredMarketplace>,
    pub errors: Vec<MarketplaceListError>,
}

#[derive(Default)]
struct ConfiguredPluginStates {
    installed: HashSet<String>,
    enabled: HashSet<String>,
}

impl From<PluginDetail> for PluginCapabilitySummary {
    fn from(value: PluginDetail) -> Self {
        let has_skills = value.skills.iter().any(|skill| {
            !value
                .disabled_skill_paths
                .contains(&skill.path_to_skills_md)
        });
        Self {
            config_name: value.id,
            display_name: value.name,
            plugin_namespace: None,
            description: prompt_safe_plugin_description(value.description.as_deref()),
            has_skills,
            mcp_server_names: value.mcp_server_names,
        }
    }
}

pub struct PluginsManager {
    ruxex_home: PathBuf,
    store: PluginStore,
    configured_marketplace_upgrade_state: RwLock<ConfiguredMarketplaceUpgradeState>,
    non_curated_cache_refresh_state: RwLock<NonCuratedCacheRefreshState>,
    non_curated_cache_refresh_completion: watch::Sender<NonCuratedCacheRefreshCompletion>,
    // Loaded capabilities vary by effective configuration.
    loaded_plugins_cache: Mutex<LoadedPluginsCache>,
    loaded_plugins_load_semaphore: Semaphore,
    skill_root_loader: Arc<dyn SkillRootLoader<PluginSkillRoot>>,
    restriction_product: Option<Product>,
    credential_broker: SharedCredentialBroker,
}

#[derive(Clone)]
struct LoadedPluginsCacheEntry {
    key: PluginLoadCacheKey,
    plugins: Vec<LoadedPlugin>,
    plugin_skill_snapshots: SkillRootSnapshots<PluginSkillRoot>,
}

#[derive(Default)]
struct LoadedPluginsCache {
    generation: u64,
    // Most recently used first.
    entries: VecDeque<LoadedPluginsCacheEntry>,
}

impl LoadedPluginsCache {
    fn get(
        &mut self,
        key: &PluginLoadCacheKey,
        store: &PluginStore,
    ) -> Option<&LoadedPluginsCacheEntry> {
        let index = self.entries.iter().position(|entry| &entry.key == key)?;
        let entry = self.entries.remove(index)?;
        // Another process can replace installed versions without invalidating this manager.
        // Keep the parsed skills paired with the installation whose paths they advertise.
        if entry.plugins.iter().any(|plugin| {
            let Ok(plugin_id) = PluginId::parse(&plugin.config_name) else {
                return false;
            };
            let installed_root = store
                .active_plugin_root(&plugin_id)
                .unwrap_or_else(|| store.plugin_base_root(&plugin_id));
            installed_root != plugin.root
        }) {
            return None;
        }
        self.entries.push_front(entry);
        self.entries.front()
    }
}

#[derive(Clone, PartialEq, Eq)]
struct PluginLoadCacheKey {
    configured_plugins: HashMap<String, PluginConfig>,
    skill_config_rules: SkillConfigRules,
}

impl PluginLoadCacheKey {
    fn from_config(config: &PluginsConfigInput, ruxex_home: &Path) -> Self {
        Self {
            configured_plugins: configured_plugins_from_stack(
                &config.config_layer_stack,
                ruxex_home,
            ),
            skill_config_rules: skill_config_rules_from_stack(&config.config_layer_stack),
        }
    }
}

impl PluginsManager {
    pub fn new(
        ruxex_home: PathBuf,
        credential_broker: SharedCredentialBroker,
        skill_root_loader: Arc<dyn SkillRootLoader<PluginSkillRoot>>,
    ) -> Self {
        Self::new_with_options(
            ruxex_home,
            Some(Product::Codex),
            credential_broker,
            skill_root_loader,
        )
    }

    pub fn new_with_options(
        ruxex_home: PathBuf,
        restriction_product: Option<Product>,
        credential_broker: SharedCredentialBroker,
        skill_root_loader: Arc<dyn SkillRootLoader<PluginSkillRoot>>,
    ) -> Self {
        // Product restrictions are enforced at marketplace admission time for a given RUXEX_HOME:
        // listing, install, and curated refresh all consult this restriction context before new
        // plugins enter local config or cache. After admission, runtime plugin loading trusts the
        // contents of that RUXEX_HOME and does not re-filter configured plugins by product, so
        // already-admitted plugins may continue exposing MCP servers/tools from shared local state.
        //
        // This assumes a single RUXEX_HOME is only used by one product.
        Self {
            ruxex_home: ruxex_home.clone(),
            store: PluginStore::new(ruxex_home),
            configured_marketplace_upgrade_state: RwLock::new(
                ConfiguredMarketplaceUpgradeState::default(),
            ),
            non_curated_cache_refresh_state: RwLock::new(NonCuratedCacheRefreshState::default()),
            non_curated_cache_refresh_completion: watch::channel(
                NonCuratedCacheRefreshCompletion::default(),
            )
            .0,
            loaded_plugins_cache: Mutex::new(LoadedPluginsCache::default()),
            loaded_plugins_load_semaphore: Semaphore::new(/*permits*/ 1),
            skill_root_loader,
            restriction_product,
            credential_broker,
        }
    }

    /// Resolves a credential only for the explicitly supplied Plugin or Marketplace owner.
    pub async fn resolve_credential(
        &self,
        request: ruxex_credential_broker::CredentialRequest,
    ) -> ruxex_credential_broker::CredentialResult {
        self.credential_broker.resolve(request).await
    }

    fn restriction_product_matches(&self, products: Option<&[Product]>) -> bool {
        match products {
            None => true,
            Some([]) => false,
            Some(products) => self
                .restriction_product
                .is_some_and(|product| product.matches_product_restriction(products)),
        }
    }

    /// Returns skill snapshots parsed while loading the matching plugin cache entry.
    pub fn plugin_skill_snapshots_for_config(
        &self,
        config: &PluginsConfigInput,
    ) -> Option<SkillRootSnapshots<PluginSkillRoot>> {
        if !config.plugins_enabled {
            return None;
        }
        let key = PluginLoadCacheKey::from_config(config, self.ruxex_home.as_path());
        self.loaded_plugins_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key, &self.store)
            .map(|cached| cached.plugin_skill_snapshots.clone())
    }

    #[instrument(
        name = "plugins_for_config",
        level = "info",
        skip_all,
        fields(
            otel.name = "plugins_for_config",
            plugins_enabled = config.plugins_enabled
        )
    )]
    pub async fn plugins_for_config(&self, config: &PluginsConfigInput) -> PluginLoadOutcome {
        if !config.plugins_enabled {
            return PluginLoadOutcome::default();
        }
        let mut cache_outcome = RequestOutcome::Hit;
        let cache_key = PluginLoadCacheKey::from_config(config, self.ruxex_home.as_path());
        if let Some(plugins) = self.cached_loaded_plugins(&cache_key) {
            cache_outcome.record();
            return PluginLoadOutcome::from_plugins(plugins);
        }

        if matches!(cache_outcome, RequestOutcome::Hit) {
            cache_outcome = RequestOutcome::HitAfterWait;
        }
        let wait_started = Instant::now();
        let load_permit = self.loaded_plugins_load_semaphore.acquire().await;
        loaded_cache_metrics::record_duration(
            loaded_cache_metrics::WAIT_DURATION,
            wait_started.elapsed(),
        );
        let Ok(_load_permit) = load_permit else {
            warn!("plugin load semaphore closed");
            return PluginLoadOutcome::default();
        };
        if let Some(plugins) = self.cached_loaded_plugins(&cache_key) {
            cache_outcome.record();
            return PluginLoadOutcome::from_plugins(plugins);
        }

        let cache_generation = self.loaded_plugins_cache_generation();
        let plugin_skill_snapshots = new_plugin_skill_snapshots();
        cache_outcome = RequestOutcome::Load;
        let load_started = Instant::now();
        let plugins = load_plugins_from_layer_stack(
            &config.config_layer_stack,
            &self.store,
            Some(&plugin_skill_snapshots),
            self.restriction_product,
            self.skill_root_loader.as_ref(),
        )
        .await;
        loaded_cache_metrics::record_duration(
            loaded_cache_metrics::LOAD_DURATION,
            load_started.elapsed(),
        );
        log_plugin_load_errors(&plugins);
        self.cache_loaded_plugins_if_current(
            cache_generation,
            cache_key,
            plugins.clone(),
            plugin_skill_snapshots,
        );
        cache_outcome.record();
        PluginLoadOutcome::from_plugins(plugins)
    }

    pub fn clear_cache(&self) {
        self.clear_loaded_plugins_cache();
    }

    fn clear_loaded_plugins_cache(&self) {
        let mut cache = match self.loaded_plugins_cache.lock() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        cache.generation = cache.generation.wrapping_add(1);
        cache.entries.clear();
        drop(cache);
        loaded_cache_metrics::record_event("clear");
    }

    fn clear_caches_after_marketplace_source_refresh(
        &self,
        installed_plugin_cache_refreshed: bool,
        on_effective_plugins_changed: Option<&EffectivePluginsChangedCallback>,
    ) {
        if installed_plugin_cache_refreshed {
            self.clear_cache();
            if let Some(on_effective_plugins_changed) = on_effective_plugins_changed {
                on_effective_plugins_changed(EffectivePluginsChange::default());
            }
        }
    }

    fn cached_loaded_plugins(&self, key: &PluginLoadCacheKey) -> Option<Vec<LoadedPlugin>> {
        self.loaded_plugins_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(key, &self.store)
            .map(|cached| cached.plugins.clone())
    }

    fn loaded_plugins_cache_generation(&self) -> u64 {
        match self.loaded_plugins_cache.lock() {
            Ok(cache) => cache.generation,
            Err(err) => err.into_inner().generation,
        }
    }

    fn cache_loaded_plugins_if_current(
        &self,
        generation: u64,
        key: PluginLoadCacheKey,
        plugins: Vec<LoadedPlugin>,
        plugin_skill_snapshots: SkillRootSnapshots<PluginSkillRoot>,
    ) {
        let mut cache = match self.loaded_plugins_cache.lock() {
            Ok(cache) => cache,
            Err(err) => err.into_inner(),
        };
        if cache.generation != generation {
            return;
        }
        cache.entries.retain(|entry| entry.key != key);
        cache.entries.push_front(LoadedPluginsCacheEntry {
            key,
            plugins,
            plugin_skill_snapshots,
        });
        let evicted = cache.entries.len() > LOADED_PLUGINS_CACHE_CAPACITY;
        cache.entries.truncate(LOADED_PLUGINS_CACHE_CAPACITY);
        drop(cache);
        if evicted {
            loaded_cache_metrics::record_event("capacity_eviction");
        }
    }

    pub async fn install_plugin(
        &self,
        config_layer_stack: &ConfigLayerStack,
        request: PluginInstallRequest,
    ) -> Result<PluginInstallOutcome, PluginInstallError> {
        let resolved = self.resolve_installable_plugin(config_layer_stack, &request)?;
        let plugin_id = resolved.plugin_id.clone();
        match self.install_resolved_plugin(resolved).await {
            Ok(outcome) => Ok(outcome),
            Err(err) => {
                self.track_plugin_install_failed(
                    &plugin_id,
                    plugin_install_error_type(&err),
                    err.sub_error_type(),
                    err.to_string(),
                );
                Err(err)
            }
        }
    }

    fn resolve_installable_plugin(
        &self,
        config_layer_stack: &ConfigLayerStack,
        request: &PluginInstallRequest,
    ) -> Result<ResolvedMarketplacePlugin, PluginInstallError> {
        let resolved = match find_installable_marketplace_plugin(
            &request.marketplace_path,
            &request.plugin_name,
            self.restriction_product,
        ) {
            Ok(resolved) => resolved,
            Err(err) => {
                self.track_plugin_install_resolution_failed(&err);
                return Err(err.into());
            }
        };
        if let Err(message) =
            MarketplacePolicy::from_requirements(config_layer_stack.requirements())
                .validate_install(
                    config_layer_stack,
                    self.ruxex_home.as_path(),
                    &request.marketplace_path,
                    &resolved.plugin_id.marketplace_name,
                )
        {
            let err = MarketplaceError::InvalidMarketplaceFile {
                path: request.marketplace_path.to_path_buf(),
                message,
            };
            self.track_plugin_install_resolution_failed(&err);
            return Err(err.into());
        }
        Ok(resolved)
    }

    fn track_plugin_install_resolution_failed(&self, err: &MarketplaceError) {
        let sub_error_type = marketplace_error_sub_error_type(err);
        let plugin_id = match err {
            MarketplaceError::PluginNotFound {
                plugin_name,
                marketplace_name,
            }
            | MarketplaceError::PluginNotAvailable {
                plugin_name,
                marketplace_name,
            } => PluginId::new(plugin_name.clone(), marketplace_name.clone()).ok(),
            MarketplaceError::Io { .. }
            | MarketplaceError::MarketplaceNotFound { .. }
            | MarketplaceError::InvalidMarketplaceFile { .. }
            | MarketplaceError::PluginsDisabled
            | MarketplaceError::InvalidPlugin(_) => None,
        };
        if let Some(plugin_id) = plugin_id {
            self.track_plugin_install_failed(
                &plugin_id,
                marketplace_error_type(err),
                sub_error_type,
                err.to_string(),
            );
        } else {
            tracing::warn!(
                error_type = %marketplace_error_type(err),
                sub_error_type = sub_error_type.as_deref(),
                error = %err,
                "plugin install failed while resolving marketplace plugin"
            );
        }
    }

    fn track_plugin_install_failed(
        &self,
        plugin_id: &PluginId,
        error_type: &'static str,
        sub_error_type: Option<String>,
        error_message: String,
    ) {
        tracing::warn!(
            plugin_id = %plugin_id.as_key(),
            error_type = %error_type,
            sub_error_type = sub_error_type.as_deref(),
            error = %error_message,
            "plugin install failed"
        );
    }

    async fn install_resolved_plugin(
        &self,
        resolved: ResolvedMarketplacePlugin,
    ) -> Result<PluginInstallOutcome, PluginInstallError> {
        let auth_policy = resolved.policy.authentication;
        let store = self.store.clone();
        let ruxex_home = self.ruxex_home.clone();
        let manifest_fallback_contents = resolved
            .manifest_fallback
            .contents_if_has_metadata()
            .map(str::to_string);
        let result: StorePluginInstallResult = tokio::task::spawn_blocking(move || {
            let materialized =
                materialize_marketplace_plugin_source(ruxex_home.as_path(), &resolved.source)
                    .map_err(PluginStoreError::Invalid)?;
            let source_path = materialized.path;
            match manifest_fallback_contents.as_deref() {
                Some(manifest_contents) => store.install_with_fallback_manifest(
                    source_path,
                    resolved.plugin_id,
                    manifest_contents,
                ),
                None => store.install(source_path, resolved.plugin_id),
            }
        })
        .await
        .map_err(PluginInstallError::join)??;

        set_user_plugin_enabled(
            &self.ruxex_home,
            result.plugin_id.as_key(),
            /*enabled*/ true,
        )
        .await
        .map_err(anyhow::Error::from)?;

        Ok(PluginInstallOutcome {
            plugin_id: result.plugin_id,
            plugin_version: result.plugin_version,
            installed_path: result.installed_path,
            auth_policy,
        })
    }

    pub async fn uninstall_plugin(&self, plugin_id: String) -> Result<(), PluginUninstallError> {
        let plugin_id = PluginId::parse(&plugin_id)?;
        self.uninstall_plugin_id(plugin_id).await
    }

    async fn uninstall_plugin_id(&self, plugin_id: PluginId) -> Result<(), PluginUninstallError> {
        let store = self.store.clone();
        let plugin_id_for_store = plugin_id.clone();
        tokio::task::spawn_blocking(move || store.uninstall(&plugin_id_for_store))
            .await
            .map_err(PluginUninstallError::join)??;

        clear_user_plugin(&self.ruxex_home, plugin_id.as_key())
            .await
            .map_err(anyhow::Error::from)?;

        Ok(())
    }

    pub fn list_marketplaces_for_context(
        &self,
        context: &PluginMarketplaceContext,
    ) -> Result<ConfiguredMarketplaceListOutcome, MarketplaceError> {
        context.list_marketplaces(self)
    }

    pub fn list_marketplaces_for_config(
        &self,
        config: &PluginsConfigInput,
        additional_roots: &[AbsolutePathBuf],
    ) -> Result<ConfiguredMarketplaceListOutcome, MarketplaceError> {
        if !config.plugins_enabled {
            return Ok(ConfiguredMarketplaceListOutcome::default());
        }

        let plugin_states = self.configured_plugin_states(config);
        self.list_marketplaces_for_config_with_states(config, additional_roots, &plugin_states)
    }

    fn list_marketplaces_for_config_with_states(
        &self,
        config: &PluginsConfigInput,
        additional_roots: &[AbsolutePathBuf],
        plugin_states: &ConfiguredPluginStates,
    ) -> Result<ConfiguredMarketplaceListOutcome, MarketplaceError> {
        let marketplace_roots = self.marketplace_roots(config, additional_roots);
        let marketplace_outcome = self.list_marketplaces_with_policy(config, &marketplace_roots)?;
        let mut seen_plugin_keys = HashSet::new();
        let marketplaces = marketplace_outcome
            .marketplaces
            .into_iter()
            .filter_map(|marketplace| {
                let marketplace_name = marketplace.name.clone();
                let plugins = marketplace
                    .plugins
                    .into_iter()
                    .filter_map(|plugin| {
                        let plugin_key = format!("{}@{marketplace_name}", plugin.name);
                        if !seen_plugin_keys.insert(plugin_key.clone()) {
                            return None;
                        }
                        if !self.restriction_product_matches(plugin.policy.products.as_deref()) {
                            return None;
                        }
                        let plugin_id =
                            PluginId::new(plugin.name.clone(), marketplace_name.clone()).ok();
                        let installed = plugin_states.installed.contains(&plugin_key);
                        let installed_version = installed.then_some(()).and_then(|_| {
                            plugin_id
                                .as_ref()
                                .and_then(|plugin_id| self.store.active_plugin_version(plugin_id))
                        });
                        let enabled = plugin_states.enabled.contains(&plugin_key);
                        let mut interface = plugin.interface;
                        let mut local_version = plugin.local_version;
                        let manifest_fallback = plugin.manifest_fallback.clone();
                        if installed
                            && plugin.source.is_install_materialized()
                            && let Some(plugin_id) = plugin_id.as_ref()
                            && let Some(plugin_root) = self.store.active_plugin_root(plugin_id)
                            && let Some(manifest) = load_plugin_manifest(plugin_root.as_path())
                        {
                            local_version = manifest.version.clone();
                            let marketplace_category = interface
                                .as_ref()
                                .and_then(|interface| interface.category.clone());
                            interface = plugin_interface_with_marketplace_category(
                                manifest.interface,
                                marketplace_category,
                            );
                        }

                        Some(ConfiguredMarketplacePlugin {
                            // Enabled state is keyed by `<plugin>@<marketplace>`, so duplicate
                            // plugin entries from duplicate marketplace files intentionally
                            // resolve to the first discovered source.
                            id: plugin_key,
                            installed_version,
                            installed,
                            enabled,
                            name: plugin.name,
                            local_version,
                            source: plugin.source,
                            policy: plugin.policy,
                            keywords: plugin.keywords,
                            interface,
                            manifest_fallback,
                        })
                    })
                    .collect::<Vec<_>>();

                (!plugins.is_empty()).then_some(ConfiguredMarketplace {
                    name: marketplace.name,
                    path: marketplace.path,
                    interface: marketplace.interface,
                    plugins,
                })
            })
            .collect();

        Ok(ConfiguredMarketplaceListOutcome {
            marketplaces,
            errors: marketplace_outcome.errors,
        })
    }

    pub fn discover_marketplaces_for_config(
        &self,
        config: &PluginsConfigInput,
        additional_roots: &[AbsolutePathBuf],
    ) -> Result<MarketplaceListOutcome, MarketplaceError> {
        if !config.plugins_enabled {
            return Ok(MarketplaceListOutcome::default());
        }

        let marketplace_roots = self.marketplace_roots(config, additional_roots);
        self.list_marketplaces_with_policy(config, &marketplace_roots)
    }

    pub async fn read_plugin_for_config(
        &self,
        config: &PluginsConfigInput,
        request: &PluginReadRequest,
    ) -> Result<PluginReadOutcome, MarketplaceError> {
        if !config.plugins_enabled {
            return Err(MarketplaceError::PluginsDisabled);
        }

        let plugin = find_marketplace_plugin(&request.marketplace_path, &request.plugin_name)?;
        MarketplacePolicy::from_requirements(config.config_layer_stack.requirements())
            .validate_install(
                &config.config_layer_stack,
                self.ruxex_home.as_path(),
                &request.marketplace_path,
                &plugin.plugin_id.marketplace_name,
            )
            .map_err(|message| MarketplaceError::InvalidMarketplaceFile {
                path: request.marketplace_path.to_path_buf(),
                message,
            })?;
        if !self.restriction_product_matches(plugin.policy.products.as_deref()) {
            return Err(MarketplaceError::PluginNotFound {
                plugin_name: plugin.plugin_id.plugin_name,
                marketplace_name: plugin.plugin_id.marketplace_name,
            });
        }

        let marketplace_name = plugin.plugin_id.marketplace_name.clone();
        let plugin_key = plugin.plugin_id.as_key();
        let manifest_fallback = plugin
            .manifest_fallback
            .contents_if_has_metadata()
            .map(|_| plugin.manifest_fallback.clone());
        let plugin_states = self.configured_plugin_states(config);
        let installed = plugin_states.installed.contains(&plugin_key);
        let installed_version = if installed {
            self.store.active_plugin_version(&plugin.plugin_id)
        } else {
            None
        };
        let plugin = self
            .read_plugin_detail_for_marketplace_plugin(
                config,
                &marketplace_name,
                ConfiguredMarketplacePlugin {
                    id: plugin_key.clone(),
                    name: plugin.plugin_id.plugin_name,
                    local_version: plugin
                        .manifest
                        .as_ref()
                        .and_then(|manifest| manifest.version.clone()),
                    installed_version,
                    source: plugin.source,
                    policy: plugin.policy,
                    interface: plugin.interface,
                    keywords: plugin
                        .manifest
                        .as_ref()
                        .map(|manifest| manifest.keywords.clone())
                        .unwrap_or_default(),
                    manifest_fallback,
                    installed,
                    enabled: plugin_states.enabled.contains(&plugin_key),
                },
            )
            .await?;

        Ok(PluginReadOutcome {
            marketplace_name,
            marketplace_path: Some(request.marketplace_path.clone()),
            plugin,
        })
    }

    #[instrument(level = "trace", skip_all)]
    pub async fn read_plugin_detail_for_marketplace_plugin(
        &self,
        config: &PluginsConfigInput,
        marketplace_name: &str,
        plugin: ConfiguredMarketplacePlugin,
    ) -> Result<PluginDetail, MarketplaceError> {
        if !self.restriction_product_matches(plugin.policy.products.as_deref()) {
            return Err(MarketplaceError::PluginNotFound {
                plugin_name: plugin.name,
                marketplace_name: marketplace_name.to_string(),
            });
        }

        let plugin_id =
            PluginId::new(plugin.name.clone(), marketplace_name.to_string()).map_err(|err| {
                match err {
                    PluginIdError::Invalid(message) => MarketplaceError::InvalidPlugin(message),
                }
            })?;
        let plugin_key = plugin_id.as_key();
        let source_path = if plugin.source.is_install_materialized() && plugin.installed {
            self.store.active_plugin_root(&plugin_id).ok_or_else(|| {
                MarketplaceError::InvalidPlugin(format!(
                    "installed plugin cache entry is missing for {plugin_key}"
                ))
            })?
        } else {
            let ruxex_home = self.ruxex_home.clone();
            let source = plugin.source.clone();
            let materialized = tokio::task::spawn_blocking(move || {
                materialize_marketplace_plugin_source(ruxex_home.as_path(), &source)
            })
            .await
            .map_err(|err| {
                MarketplaceError::InvalidPlugin(format!(
                    "failed to materialize plugin source: {err}"
                ))
            })?
            .map_err(MarketplaceError::InvalidPlugin)?;
            materialized.path.clone()
        };
        if !source_path.as_path().is_dir() {
            return Err(MarketplaceError::InvalidPlugin(
                "path does not exist or is not a directory".to_string(),
            ));
        }
        let loaded_manifest =
            if ruxex_utils_plugins::find_plugin_manifest_path(source_path.as_path()).is_some() {
                load_plugin_manifest_with_format(source_path.as_path())
            } else {
                plugin
                    .manifest_fallback
                    .as_ref()
                    .and_then(|fallback| fallback.parse_for_plugin_root(source_path.as_path()))
                    .map(|manifest| crate::manifest::LoadedPluginManifest {
                        manifest,
                        format: PluginManifestFormat::Legacy,
                    })
            }
            .ok_or_else(|| {
                MarketplaceError::InvalidPlugin("missing or invalid plugin.json".to_string())
            })?;
        let manifest_format = loaded_manifest.format;
        let manifest = loaded_manifest.manifest;
        let description = manifest.description.clone();
        let marketplace_category = plugin
            .interface
            .as_ref()
            .and_then(|interface| interface.category.clone());
        let interface = plugin_interface_with_marketplace_category(
            manifest.interface.clone(),
            marketplace_category,
        );
        let plugin_identity = PluginIdentity {
            plugin_id: plugin_id.as_key(),
            remote_plugin_id: None,
        };
        let skill_config_rules = skill_config_rules_from_stack(&config.config_layer_stack);
        let resolved_skills = load_plugin_skill_inventory(
            &source_path,
            &plugin_identity,
            &manifest,
            manifest_format,
            self.restriction_product,
            /*plugin_skill_snapshots*/ None,
            self.skill_root_loader.as_ref(),
        )
        .await
        .resolve(&skill_config_rules);
        let plugin_data_root = self.store.plugin_data_root(&plugin_id);
        let (hook_sources, _hook_load_warnings) = if manifest_format == PluginManifestFormat::Legacy
        {
            load_plugin_hooks(&source_path, &plugin_id, &plugin_data_root, &manifest.paths)
        } else {
            (Vec::new(), Vec::new())
        };
        let hooks = plugin_hook_declarations(&hook_sources)
            .into_iter()
            .map(|hook| PluginHookSummary {
                key: hook.key,
                event_name: hook.event_name,
            })
            .collect();
        let mcp_data_root = (manifest_format == PluginManifestFormat::AgentPlugin)
            .then(|| self.store.mcp_data_root(&plugin_id, manifest_format));
        let mcp_servers = load_plugin_mcp_servers_from_manifest_with_format(
            source_path.as_path(),
            &manifest.paths,
            /*plugin_policy*/ None,
            mcp_data_root.as_deref(),
            manifest_format,
        )
        .await;
        let mut mcp_server_names = mcp_servers.into_keys().collect::<Vec<_>>();
        mcp_server_names.sort_unstable();
        mcp_server_names.dedup();

        Ok(PluginDetail {
            id: plugin.id,
            name: plugin.name,
            local_version: manifest.version.clone(),
            description,
            source: plugin.source,
            policy: plugin.policy,
            interface,
            keywords: manifest.keywords,
            installed: plugin.installed,
            enabled: plugin.enabled,
            skills: resolved_skills.skills,
            disabled_skill_paths: resolved_skills.disabled_skill_paths,
            onboarding_skill: manifest.paths.onboarding_skill,
            hooks,
            mcp_server_names,
        })
    }

    pub fn maybe_start_plugin_startup_tasks_for_config(
        self: &Arc<Self>,
        config: &PluginsConfigInput,
        reload_config: ConfigLayerReload,
        on_effective_plugins_changed: Option<EffectivePluginsChangedCallback>,
    ) {
        if config.plugins_enabled {
            let should_spawn_marketplace_auto_upgrade = {
                let mut state = match self.configured_marketplace_upgrade_state.write() {
                    Ok(state) => state,
                    Err(err) => err.into_inner(),
                };
                if state.in_flight {
                    false
                } else {
                    state.in_flight = true;
                    true
                }
            };
            if should_spawn_marketplace_auto_upgrade {
                let manager = Arc::clone(self);
                let config = config.clone();
                let on_effective_plugins_changed = on_effective_plugins_changed.clone();
                let runtime = tokio::runtime::Handle::current();
                if let Err(err) = std::thread::Builder::new()
                    .name("plugins-marketplace-auto-upgrade".to_string())
                    .spawn(move || {
                        let outcome = manager.upgrade_configured_marketplaces_for_config_with_mode(
                            &config,
                            /*marketplace_name*/ None,
                            PluginGitMode::Automatic,
                            &reload_config,
                        );
                        match outcome {
                            Ok(outcome) => {
                                if !outcome.upgraded_roots.is_empty()
                                    && let Some(on_effective_plugins_changed) =
                                        on_effective_plugins_changed
                                {
                                    runtime.spawn(async move {
                                        on_effective_plugins_changed(
                                            EffectivePluginsChange::default(),
                                        );
                                    });
                                }
                                for error in outcome.errors {
                                    warn!(
                                        marketplace = error.marketplace_name,
                                        error = %error.message,
                                        "failed to auto-upgrade configured marketplace"
                                    );
                                }
                            }
                            Err(err) => {
                                warn!("failed to auto-upgrade configured marketplaces: {err}");
                            }
                        }

                        let mut state = match manager.configured_marketplace_upgrade_state.write() {
                            Ok(state) => state,
                            Err(err) => err.into_inner(),
                        };
                        state.in_flight = false;
                    })
                {
                    let mut state = match self.configured_marketplace_upgrade_state.write() {
                        Ok(state) => state,
                        Err(err) => err.into_inner(),
                    };
                    state.in_flight = false;
                    warn!("failed to start configured marketplace auto-upgrade task: {err}");
                }
            }
        }
    }

    pub fn upgrade_configured_marketplaces_for_config(
        &self,
        config: &PluginsConfigInput,
        marketplace_name: Option<&str>,
        reload_config: &ConfigLayerReload,
    ) -> Result<ConfiguredMarketplaceUpgradeOutcome, String> {
        self.upgrade_configured_marketplaces_for_config_with_mode(
            config,
            marketplace_name,
            PluginGitMode::Manual,
            reload_config,
        )
    }

    /// Carries the initiating Git trust policy through marketplace and plugin refreshes.
    fn upgrade_configured_marketplaces_for_config_with_mode(
        &self,
        config: &PluginsConfigInput,
        marketplace_name: Option<&str>,
        mode: PluginGitMode,
        reload_config: &ConfigLayerReload,
    ) -> Result<ConfiguredMarketplaceUpgradeOutcome, String> {
        let mut outcome = upgrade_configured_git_marketplaces_with_mode(
            self.ruxex_home.as_path(),
            &config.config_layer_stack,
            marketplace_name,
            mode,
            reload_config,
        );
        if let Some(marketplace_name) = marketplace_name
            && outcome.selected_marketplaces.is_empty()
        {
            return Err(format!(
                "marketplace `{marketplace_name}` is not configured as a Git marketplace"
            ));
        }
        if !outcome.upgraded_roots.is_empty() {
            let mut configured_plugin_keys = configured_plugins_from_stack(
                &config.config_layer_stack,
                self.ruxex_home.as_path(),
            )
            .into_keys()
            .collect::<Vec<_>>();
            configured_plugin_keys.sort_unstable();
            match refresh_non_curated_plugin_cache_force_reinstall_detailed(
                self.ruxex_home.as_path(),
                &outcome.upgraded_roots,
                &configured_plugin_keys,
                mode,
            ) {
                Ok(refresh_outcome) => {
                    self.clear_caches_after_marketplace_source_refresh(
                        refresh_outcome.cache_refreshed,
                        /*on_effective_plugins_changed*/ None,
                    );
                    outcome
                        .errors
                        .extend(refresh_outcome.errors.into_iter().map(|error| {
                            ConfiguredMarketplaceUpgradeError {
                                marketplace_name: error.marketplace_name,
                                message: error.message,
                            }
                        }));
                }
                Err(err) => {
                    self.clear_cache();
                    outcome.errors.push(ConfiguredMarketplaceUpgradeError {
                        marketplace_name: marketplace_name
                            .unwrap_or("all configured marketplaces")
                            .to_string(),
                        message: format!(
                            "failed to refresh installed plugin cache after marketplace upgrade: {err}"
                        ),
                    });
                }
            }
        }
        Ok(outcome)
    }

    pub fn maybe_start_non_curated_plugin_cache_refresh(
        self: &Arc<Self>,
        context: &PluginMarketplaceContext,
        marketplaces: &[ConfiguredMarketplace],
    ) {
        if let Some(request) = context.non_curated_cache_refresh_request(
            self,
            marketplaces,
            NonCuratedCacheRefreshMode::IfVersionChanged,
            PluginGitMode::Automatic,
        ) {
            self.schedule_non_curated_plugin_cache_refresh(request);
        }
    }

    fn schedule_non_curated_plugin_cache_refresh(
        self: &Arc<Self>,
        mut request: NonCuratedCacheRefreshRequest,
    ) {
        let should_spawn = {
            let mut state = match self.non_curated_cache_refresh_state.write() {
                Ok(state) => state,
                Err(err) => err.into_inner(),
            };
            if request.mode == NonCuratedCacheRefreshMode::IfVersionChanged
                && state.last_refreshed.as_ref().is_some_and(|last_refreshed| {
                    request.configured_plugin_sources.iter().any(|source| {
                        last_refreshed
                            .configured_plugin_sources
                            .iter()
                            .any(|previous_source| {
                                previous_source.plugin_key == source.plugin_key
                                    && previous_source.local_version == source.local_version
                                    && (previous_source.marketplace_path != source.marketplace_path
                                        || previous_source.source != source.source)
                            })
                    })
                })
            {
                request.mode = NonCuratedCacheRefreshMode::ForceReinstall;
            }
            if request.mode == NonCuratedCacheRefreshMode::IfVersionChanged
                && state.requested.as_ref().is_some_and(|requested| {
                    requested.mode == NonCuratedCacheRefreshMode::ForceReinstall
                        && requested.roots == request.roots
                })
            {
                request.mode = NonCuratedCacheRefreshMode::ForceReinstall;
            }
            // Reconcile each canonical plugin generation once before publishing its resource.
            if state.requested.as_ref().is_some_and(|requested| {
                requested == &request
                    || (requested.git_mode == PluginGitMode::Manual
                        && request.git_mode == PluginGitMode::Automatic
                        && requested.roots == request.roots
                        && requested.configured_plugin_keys == request.configured_plugin_keys
                        && requested.configured_plugin_sources == request.configured_plugin_sources
                        && requested.mode == request.mode)
            }) || (request.mode == NonCuratedCacheRefreshMode::IfVersionChanged
                && !state.in_flight
                && state.last_refreshed.as_ref().is_some_and(|last_refreshed| {
                    last_refreshed.roots == request.roots
                        && last_refreshed.configured_plugin_keys == request.configured_plugin_keys
                        && last_refreshed.configured_plugin_sources
                            == request.configured_plugin_sources
                        && (last_refreshed.git_mode == request.git_mode
                            || last_refreshed.git_mode == PluginGitMode::Manual)
                }))
            {
                return;
            }
            state.requested = Some(request);
            if state.in_flight {
                false
            } else {
                state.in_flight = true;
                true
            }
        };
        if !should_spawn {
            return;
        }

        let manager = Arc::clone(self);
        if let Err(err) = std::thread::Builder::new()
            .name("plugins-non-curated-cache-refresh".to_string())
            .spawn(move || manager.run_non_curated_plugin_cache_refresh_loop())
        {
            let mut state = match self.non_curated_cache_refresh_state.write() {
                Ok(state) => state,
                Err(err) => err.into_inner(),
            };
            state.in_flight = false;
            state.requested = None;
            self.non_curated_cache_refresh_completion
                .send_modify(|completion| {
                    completion.sequence = completion.sequence.wrapping_add(1);
                });
            warn!("failed to start non-curated plugin cache refresh task: {err}");
        }
    }

    fn run_non_curated_plugin_cache_refresh_loop(self: Arc<Self>) {
        loop {
            let request = {
                let state = match self.non_curated_cache_refresh_state.read() {
                    Ok(state) => state,
                    Err(err) => err.into_inner(),
                };
                state.requested.clone()
            };

            let Some(request) = request else {
                let mut state = match self.non_curated_cache_refresh_state.write() {
                    Ok(state) => state,
                    Err(err) => err.into_inner(),
                };
                state.in_flight = false;
                self.non_curated_cache_refresh_completion
                    .send_modify(|completion| {
                        completion.sequence = completion.sequence.wrapping_add(1);
                    });
                return;
            };

            let refresh_result = match request.mode {
                NonCuratedCacheRefreshMode::IfVersionChanged => {
                    refresh_non_curated_plugin_cache_detailed(
                        self.ruxex_home.as_path(),
                        &request.roots,
                        &request.configured_plugin_keys,
                        request.git_mode,
                    )
                }
                NonCuratedCacheRefreshMode::ForceReinstall => {
                    refresh_non_curated_plugin_cache_force_reinstall_detailed(
                        self.ruxex_home.as_path(),
                        &request.roots,
                        &request.configured_plugin_keys,
                        request.git_mode,
                    )
                }
            };
            let (refreshed, cache_changed) = match refresh_result {
                Ok(refresh_outcome) => {
                    if refresh_outcome.cache_refreshed {
                        self.clear_cache();
                    }
                    for error in &refresh_outcome.errors {
                        warn!(
                            marketplace = error.marketplace_name,
                            error = %error.message,
                            "failed to refresh configured plugin cache"
                        );
                    }
                    (
                        refresh_outcome.errors.is_empty(),
                        refresh_outcome.cache_refreshed,
                    )
                }
                Err(err) => {
                    self.clear_cache();
                    warn!("failed to refresh non-curated plugin cache: {err}");
                    (false, false)
                }
            };

            let mut state = match self.non_curated_cache_refresh_state.write() {
                Ok(state) => state,
                Err(err) => err.into_inner(),
            };
            if refreshed {
                state.last_refreshed = Some(request.clone());
            }
            let complete = state.requested.as_ref() == Some(&request);
            if complete {
                state.requested = None;
                state.in_flight = false;
            }
            self.non_curated_cache_refresh_completion
                .send_modify(|completion| {
                    completion.sequence = completion.sequence.wrapping_add(1);
                    if cache_changed {
                        completion.changed_sequence = completion.changed_sequence.wrapping_add(1);
                    }
                });
            if complete {
                return;
            }
        }
    }

    fn configured_plugin_states(&self, config: &PluginsConfigInput) -> ConfiguredPluginStates {
        let configured_plugins =
            configured_plugins_from_stack(&config.config_layer_stack, self.ruxex_home.as_path());
        let installed = configured_plugins
            .keys()
            .filter(|plugin_key| {
                PluginId::parse(plugin_key)
                    .ok()
                    .is_some_and(|plugin_id| self.store.is_installed(&plugin_id))
            })
            .cloned()
            .collect::<HashSet<_>>();
        let enabled = configured_plugins
            .into_iter()
            .filter_map(|(plugin_key, plugin)| plugin.enabled.then_some(plugin_key))
            .collect::<HashSet<_>>();
        ConfiguredPluginStates { installed, enabled }
    }

    fn marketplace_roots(
        &self,
        config: &PluginsConfigInput,
        additional_roots: &[AbsolutePathBuf],
    ) -> Vec<AbsolutePathBuf> {
        let mut roots = additional_roots.to_vec();
        roots.extend(installed_marketplace_roots_from_layer_stack(
            &config.config_layer_stack,
            self.ruxex_home.as_path(),
        ));
        roots.sort_unstable();
        roots.dedup();
        roots
    }

    fn list_marketplaces_with_policy(
        &self,
        config: &PluginsConfigInput,
        roots: &[AbsolutePathBuf],
    ) -> Result<MarketplaceListOutcome, MarketplaceError> {
        let mut outcome = list_marketplaces_with_home(roots, home_dir().as_deref())?;
        let policy = MarketplacePolicy::from_requirements(config.config_layer_stack.requirements());
        outcome.marketplaces.retain(|marketplace| {
            policy
                .validate_install(
                    &config.config_layer_stack,
                    self.ruxex_home.as_path(),
                    &marketplace.path,
                    &marketplace.name,
                )
                .is_ok()
        });
        Ok(outcome)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginInstallError {
    #[error("{0}")]
    Marketplace(#[from] MarketplaceError),

    #[error("{0}")]
    Store(#[from] PluginStoreError),

    #[error("{0}")]
    Config(#[from] anyhow::Error),

    #[error("failed to join plugin install task: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl PluginInstallError {
    fn join(source: tokio::task::JoinError) -> Self {
        Self::Join(source)
    }

    pub fn sub_error_type(&self) -> Option<String> {
        match self {
            Self::Marketplace(err) => marketplace_error_sub_error_type(err),
            Self::Store(err) => err.sub_error_type(),
            Self::Config(_) => Some("failed_to_enable_plugin".to_string()),
            Self::Join(_) => Some("plugin_install_task_failed".to_string()),
        }
    }
}

fn plugin_install_error_type(err: &PluginInstallError) -> &'static str {
    match err {
        PluginInstallError::Marketplace(err) => marketplace_error_type(err),
        PluginInstallError::Store(err) => plugin_store_error_type(err),
        PluginInstallError::Config(_) => "config",
        PluginInstallError::Join(_) => "join",
    }
}

fn marketplace_error_type(err: &MarketplaceError) -> &'static str {
    match err {
        MarketplaceError::Io { .. } => "marketplace_io",
        MarketplaceError::MarketplaceNotFound { .. } => "marketplace_not_found",
        MarketplaceError::InvalidMarketplaceFile { .. } => "invalid_marketplace_file",
        MarketplaceError::PluginNotFound { .. } => "plugin_not_found",
        MarketplaceError::PluginNotAvailable { .. } => "plugin_not_available",
        MarketplaceError::PluginsDisabled => "plugins_disabled",
        MarketplaceError::InvalidPlugin(_) => "invalid_plugin",
    }
}

fn marketplace_error_sub_error_type(err: &MarketplaceError) -> Option<String> {
    match err {
        MarketplaceError::Io { context, .. } => Some(error_context_sub_error_type(context)),
        MarketplaceError::MarketplaceNotFound { .. }
        | MarketplaceError::InvalidMarketplaceFile { .. }
        | MarketplaceError::PluginNotFound { .. }
        | MarketplaceError::PluginNotAvailable { .. }
        | MarketplaceError::PluginsDisabled
        | MarketplaceError::InvalidPlugin(_) => None,
    }
}

fn plugin_store_error_type(err: &PluginStoreError) -> &'static str {
    match err {
        PluginStoreError::Io { .. } => "store_io",
        PluginStoreError::Invalid(_) => "store_invalid",
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginUninstallError {
    #[error("{0}")]
    InvalidPluginId(#[from] PluginIdError),

    #[error("{0}")]
    Store(#[from] PluginStoreError),

    #[error("{0}")]
    Config(#[from] anyhow::Error),

    #[error("failed to join plugin uninstall task: {0}")]
    Join(#[from] tokio::task::JoinError),
}

impl PluginUninstallError {
    fn join(source: tokio::task::JoinError) -> Self {
        Self::Join(source)
    }
}

#[cfg(test)]
#[path = "manager_tests.rs"]
mod tests;

use std::path::Path;
use std::path::PathBuf;

use ruxex_config::ConfigLayerStack;
use ruxex_plugin::validate_plugin_segment;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use tracing::warn;

use crate::marketplace::find_marketplace_manifest_path;
use crate::marketplace_assets::MarketplaceOrigin;
use crate::marketplace_policy::policy_filtered_plugin_config;
use std::collections::HashMap;

pub const INSTALLED_MARKETPLACES_DIR: &str = ".tmp/marketplaces";

/// Returns configured marketplace origins keyed by their resolved checkout roots.
pub fn configured_marketplace_origins_from_layer_stack(
    config_layer_stack: &ConfigLayerStack,
    ruxex_home: &Path,
) -> HashMap<AbsolutePathBuf, MarketplaceOrigin> {
    let Some(effective_config) = policy_filtered_plugin_config(config_layer_stack, ruxex_home)
    else {
        return HashMap::new();
    };
    let Some(marketplaces) = effective_config
        .get("marketplaces")
        .and_then(toml::Value::as_table)
    else {
        return HashMap::new();
    };

    let default_install_root = marketplace_install_root(ruxex_home);
    marketplaces
        .iter()
        .filter_map(|(marketplace_name, marketplace)| {
            let root = resolve_configured_marketplace_root(
                marketplace_name,
                marketplace,
                &default_install_root,
            )?;
            let root = AbsolutePathBuf::try_from(root).ok()?;
            let origin = match marketplace.get("source_type").and_then(toml::Value::as_str) {
                Some("git") => MarketplaceOrigin::Git {
                    url: marketplace
                        .get("source")
                        .and_then(toml::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    ref_name: marketplace
                        .get("ref")
                        .and_then(toml::Value::as_str)
                        .map(str::to_string),
                    revision: marketplace
                        .get("last_revision")
                        .and_then(toml::Value::as_str)
                        .map(str::to_string),
                },
                Some("local") => MarketplaceOrigin::Local { path: root.clone() },
                _ => MarketplaceOrigin::Unknown,
            };
            Some((root, origin))
        })
        .collect()
}

pub fn marketplace_install_root(ruxex_home: &Path) -> PathBuf {
    ruxex_home.join(INSTALLED_MARKETPLACES_DIR)
}

pub fn installed_marketplace_roots_from_layer_stack(
    config_layer_stack: &ConfigLayerStack,
    ruxex_home: &Path,
) -> Vec<AbsolutePathBuf> {
    let Some(effective_config) = policy_filtered_plugin_config(config_layer_stack, ruxex_home)
    else {
        return Vec::new();
    };
    let Some(marketplaces_value) = effective_config.get("marketplaces") else {
        return Vec::new();
    };
    let Some(marketplaces) = marketplaces_value.as_table() else {
        warn!("invalid marketplaces config: expected table");
        return Vec::new();
    };
    let default_install_root = marketplace_install_root(ruxex_home);
    let mut roots = marketplaces
        .iter()
        .filter_map(|(marketplace_name, marketplace)| {
            if !marketplace.is_table() {
                warn!(
                    marketplace_name,
                    "ignoring invalid configured marketplace entry"
                );
                return None;
            }
            if let Err(err) = validate_plugin_segment(marketplace_name, "marketplace name") {
                warn!(
                    marketplace_name,
                    error = %err,
                    "ignoring invalid configured marketplace name"
                );
                return None;
            }
            let path = resolve_configured_marketplace_root(
                marketplace_name,
                marketplace,
                &default_install_root,
            )?;
            find_marketplace_manifest_path(&path).map(|_| path)
        })
        .filter_map(|path| AbsolutePathBuf::try_from(path).ok())
        .collect::<Vec<_>>();
    roots.sort_unstable_by(|left, right| left.as_path().cmp(right.as_path()));
    roots
}

pub fn resolve_configured_marketplace_root(
    marketplace_name: &str,
    marketplace: &toml::Value,
    default_install_root: &Path,
) -> Option<PathBuf> {
    match marketplace.get("source_type").and_then(toml::Value::as_str) {
        Some("local") => marketplace
            .get("source")
            .and_then(toml::Value::as_str)
            .filter(|source| !source.is_empty())
            .map(PathBuf::from),
        _ => Some(default_install_root.join(marketplace_name)),
    }
}

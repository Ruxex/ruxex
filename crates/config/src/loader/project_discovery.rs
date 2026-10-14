//! Shared configuration composition for project-root and trust discovery.

use super::layer_io::LoadedConfigLayers;
use super::resolve_relative_paths_in_config_toml;
use crate::merge_toml_values;
use std::io;
use std::path::Path;
use toml::Value as TomlValue;

/// Overlay the already-read managed sources at their normal precedence: file,
/// then MDM. Resolve paths as the final loader does, without changing the raw
/// layers or reading either source again.
pub(super) fn merge_managed_config_for_discovery(
    discovery_config: &mut TomlValue,
    loaded: &LoadedConfigLayers,
    ruxex_home: &Path,
) -> io::Result<()> {
    if let Some(config) = &loaded.managed_config_from_mdm {
        let resolved =
            resolve_relative_paths_in_config_toml(config.managed_config.clone(), ruxex_home)?;
        merge_toml_values(discovery_config, &resolved);
    }
    Ok(())
}

#[cfg(test)]
#[path = "managed_project_discovery_tests.rs"]
mod tests;

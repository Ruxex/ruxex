#[cfg(target_os = "macos")]
use super::macos::ManagedAdminConfigLayer;
#[cfg(target_os = "macos")]
use super::macos::load_managed_admin_config_layer;
use crate::config_toml::ConfigToml;
use crate::diagnostics::config_error_from_toml;
use crate::diagnostics::io_error_from_config_error;
use crate::state::LoaderOverrides;
use crate::strict_config::config_error_from_ignored_toml_value_fields;
use ruxex_file_system::ExecutorFileSystem;
use ruxex_utils_absolute_path::AbsolutePathBuf;
use ruxex_utils_absolute_path::AbsolutePathBufGuard;
use ruxex_utils_path_uri::PathUri;
use std::io;
use std::path::Path;
use toml::Value as TomlValue;

#[derive(Debug, Clone)]
pub(super) struct ManagedConfigFromMdm {
    pub managed_config: TomlValue,
    pub raw_toml: String,
}

#[derive(Debug, Clone)]
pub(super) struct LoadedConfigLayers {
    /// If present, data read from current forced managed preferences (macOS only).
    pub managed_config_from_mdm: Option<ManagedConfigFromMdm>,
    /// User-facing warnings discovered while loading managed configuration.
    pub startup_warnings: Vec<String>,
}

pub(super) async fn load_config_layers_internal(
    fs: &dyn ExecutorFileSystem,
    ruxex_home: &Path,
    overrides: LoaderOverrides,
    strict_config: bool,
) -> io::Result<LoadedConfigLayers> {
    let _ = (&fs, ruxex_home, &overrides, strict_config);
    #[cfg(target_os = "macos")]
    let LoaderOverrides {
        managed_preferences_base64,
        ..
    } = overrides;
    let startup_warnings = Vec::new();

    #[cfg(target_os = "macos")]
    let managed_preferences = load_managed_admin_config_layer(
        managed_preferences_base64.as_deref(),
        strict_config,
        ruxex_home,
    )
    .await?
    .map(map_managed_admin_layer);

    #[cfg(not(target_os = "macos"))]
    let managed_preferences = None;

    Ok(LoadedConfigLayers {
        managed_config_from_mdm: managed_preferences,
        startup_warnings,
    })
}

#[cfg(target_os = "macos")]
fn map_managed_admin_layer(layer: ManagedAdminConfigLayer) -> ManagedConfigFromMdm {
    let ManagedAdminConfigLayer { config, raw_toml } = layer;
    ManagedConfigFromMdm {
        managed_config: config,
        raw_toml,
    }
}

pub(super) async fn read_config_from_path(
    fs: &dyn ExecutorFileSystem,
    path: &AbsolutePathBuf,
    log_missing_as_info: bool,
    strict_config: bool,
) -> io::Result<Option<TomlValue>> {
    let path_uri = PathUri::from_abs_path(path);
    match fs
        .read_file_text(&path_uri, Default::default(), /*sandbox*/ None)
        .await
    {
        Ok(contents) => match toml::from_str::<TomlValue>(&contents) {
            Ok(value) => {
                if strict_config {
                    validate_config_toml_strictly(path, &contents, &value)?;
                }
                Ok(Some(value))
            }
            Err(err) => {
                tracing::error!("Failed to parse {}: {err}", path.as_path().display());
                let config_error = config_error_from_toml(path.as_path(), &contents, err.clone());
                Err(io_error_from_config_error(
                    io::ErrorKind::InvalidData,
                    config_error,
                    Some(err),
                ))
            }
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            if log_missing_as_info {
                tracing::info!("{} not found, using defaults", path.as_path().display());
            } else {
                tracing::debug!("{} not found", path.as_path().display());
            }
            Ok(None)
        }
        Err(err) => {
            tracing::error!("Failed to read {}: {err}", path.as_path().display());
            Err(err)
        }
    }
}

fn validate_config_toml_strictly(
    path: &AbsolutePathBuf,
    contents: &str,
    value: &TomlValue,
) -> io::Result<()> {
    let Some(base_dir) = path.as_path().parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Config file {} has no parent directory", path.display()),
        ));
    };
    let _guard = AbsolutePathBufGuard::new(base_dir);
    if let Some(config_error) = config_error_from_ignored_toml_value_fields::<ConfigToml>(
        path.as_path(),
        contents,
        value.clone(),
    ) {
        return Err(io_error_from_config_error(
            io::ErrorKind::InvalidData,
            config_error,
            /*source*/ None,
        ));
    }

    Ok(())
}

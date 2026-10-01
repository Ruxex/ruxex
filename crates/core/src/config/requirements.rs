use genex_config::ConfigRequirements;
use genex_config::Sourced;
use genex_config::config_toml::ConfigToml;
use genex_login::default_client::RESIDENCY_HEADER_NAME;
use genex_utils_absolute_path::AbsolutePathBuf;
use std::collections::HashMap;
use std::path::Path;

/// Applies managed requirements to regular config before final config construction.
///
/// Managed values replace their configured counterparts, and conflicts produce
/// source-aware startup warnings.
pub(super) fn apply_to_config(
    config: &mut ConfigToml,
    requirements: &ConfigRequirements,
    startup_warnings: &mut Vec<String>,
) {
    macro_rules! apply_exact {
        ($field:ident) => {
            apply_exact_requirement(
                stringify!($field),
                &mut config.$field,
                requirements.$field.as_ref(),
                startup_warnings,
            );
        };
    }

    apply_exact!(cli_auth_credentials_store);
    apply_exact!(managed_auth_base_url);
    apply_exact!(sqlite_home);
    apply_exact!(log_dir);
    apply_exact!(model_catalog_json);
    apply_exact!(model_provider);
    if let Some(providers) = &requirements.model_providers {
        config.model_providers.extend(providers.value.clone());
    }
    apply_exact!(check_for_update_on_startup);
    apply_exact!(allow_login_shell);
    if requirements.enforce_residency.value().is_some() {
        for (provider_name, provider) in &config.model_providers {
            let has_residency_header = provider
                .http_headers
                .iter()
                .flat_map(HashMap::keys)
                .chain(provider.env_http_headers.iter().flat_map(HashMap::keys))
                .any(|name| name.eq_ignore_ascii_case(RESIDENCY_HEADER_NAME));

            if has_residency_header {
                let warning = format!(
                    "Ignoring `{RESIDENCY_HEADER_NAME}` in `model_providers.{provider_name}` because managed residency is required."
                );
                tracing::warn!(provider = provider_name, "{warning}");
                startup_warnings.push(warning);
            }
        }
    }
    if let Some(requirement) = requirements.windows_sandbox_private_desktop.as_ref() {
        apply_exact_requirement(
            "windows.sandbox_private_desktop",
            &mut config
                .windows
                .get_or_insert_default()
                .sandbox_private_desktop,
            Some(requirement),
            startup_warnings,
        );
    }
}

fn apply_exact_requirement<T>(
    field_name: &'static str,
    configured_value: &mut Option<T>,
    requirement: Option<&Sourced<T>>,
    startup_warnings: &mut Vec<String>,
) where
    T: Clone + PartialEq + std::fmt::Debug,
{
    let Some(Sourced { value, source }) = requirement else {
        return;
    };
    if configured_value
        .as_ref()
        .is_some_and(|configured| configured != value)
    {
        tracing::warn!(
            ?source,
            ?value,
            "configured value is overridden by an exact requirement for {field_name}"
        );
        startup_warnings.push(format!(
            "Configured value for `{field_name}` is overridden by the required value {value:?} from {source}."
        ));
    }
    *configured_value = Some(value.clone());
}

pub(super) fn push_sqlite_home_env_override_warning(
    configured_sqlite_home: Option<&AbsolutePathBuf>,
    sqlite_home_env: Option<&Path>,
    requirement: Option<&Sourced<AbsolutePathBuf>>,
    startup_warnings: &mut Vec<String>,
) {
    if configured_sqlite_home.is_some() {
        return;
    }
    let Some(sqlite_home_env) = sqlite_home_env else {
        return;
    };
    let Some(Sourced { value, source }) = requirement else {
        return;
    };
    if sqlite_home_env == value.as_path() {
        return;
    }

    tracing::warn!(
        ?source,
        ?value,
        "`CODEX_SQLITE_HOME` is overridden by an exact requirement for sqlite_home"
    );
    startup_warnings.push(format!(
        "Environment value for `$CODEX_SQLITE_HOME` is overridden by the required `sqlite_home` value {value:?} from {source}."
    ));
}

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
    apply_exact!(allow_login_shell);
    if requirements.enforce_residency.value().is_some() {
        if let Some(provider) = config.model_provider.as_ref() {
            let has_residency_header = provider
                .http_headers
                .iter()
                .flat_map(HashMap::keys)
                .chain(provider.env_http_headers.iter().flat_map(HashMap::keys))
                .any(|name| name.eq_ignore_ascii_case(RESIDENCY_HEADER_NAME));

            if has_residency_header {
                let warning = format!(
                    "Ignoring `{RESIDENCY_HEADER_NAME}` in `model_provider` because managed residency is required."
                );
                tracing::warn!(provider = %provider.name, "{warning}");
                startup_warnings.push(warning);
            }
        }
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
        "`GENEX_SQLITE_HOME` is overridden by an exact requirement for sqlite_home"
    );
    startup_warnings.push(format!(
        "Environment value for `$GENEX_SQLITE_HOME` is overridden by the required `sqlite_home` value {value:?} from {source}."
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use genex_config::ConfigRequirements;
    use genex_config::RequirementSource;
    use genex_config::Sourced;
    use genex_config::config_toml::ConfigToml;
    use genex_utils_absolute_path::AbsolutePathBuf;

    #[test]
    fn applies_exact_requirements_and_reports_conflicts() {
        let mut config = ConfigToml {
            allow_login_shell: Some(false),
            ..Default::default()
        };
        let requirements = ConfigRequirements {
            allow_login_shell: Some(Sourced::new(true, RequirementSource::Unknown)),
            ..Default::default()
        };
        let mut warnings = Vec::new();

        apply_to_config(&mut config, &requirements, &mut warnings);

        assert_eq!(config.allow_login_shell, Some(true));
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings
                .iter()
                .all(|warning| warning.contains("overridden"))
        );
    }

    #[test]
    fn sqlite_home_environment_warning_only_applies_to_conflicting_values() {
        let required = AbsolutePathBuf::try_from("/managed/sqlite").expect("absolute path");
        let env_path = AbsolutePathBuf::try_from("/environment/sqlite").expect("absolute path");
        let requirement = Sourced::new(required.clone(), RequirementSource::Unknown);
        let mut warnings = Vec::new();

        push_sqlite_home_env_override_warning(
            None,
            Some(env_path.as_path()),
            Some(&requirement),
            &mut warnings,
        );
        assert_eq!(warnings.len(), 1);

        push_sqlite_home_env_override_warning(
            Some(&required),
            Some(env_path.as_path()),
            Some(&requirement),
            &mut warnings,
        );
        push_sqlite_home_env_override_warning(
            None,
            Some(required.as_path()),
            Some(&requirement),
            &mut warnings,
        );
        push_sqlite_home_env_override_warning(None, None, Some(&requirement), &mut warnings);
        assert_eq!(warnings.len(), 1);
    }
}

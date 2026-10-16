use super::*;
use pretty_assertions::assert_eq;
use ruxex_config::ConfigLayerStack;
use ruxex_config::ConfigRequirements;
use ruxex_config::ConfigRequirementsToml;
use ruxex_config::FeatureRequirementsToml;
use ruxex_config::RequirementSource;
use ruxex_config::Sourced;
use ruxex_config::config_toml::ConfigToml;
use ruxex_config::config_toml::ForcedManagedWorkspaceIds;
use ruxex_config::types::AuthCredentialsStoreMode;
use ruxex_features::FeaturesToml;
use ruxex_protocol::config_types::ForcedLoginMethod;
use std::collections::BTreeMap;

#[test]
fn resolve_bootstrap_auth_keyring_backend_kind_uses_secret_auth_storage_feature()
-> std::io::Result<()> {
    let config_toml = ConfigToml {
        features: Some(FeaturesToml::from(BTreeMap::from([(
            "secret_auth_storage".to_string(),
            true,
        )]))),
        ..Default::default()
    };
    assert_eq!(
        resolve_bootstrap_auth_keyring_backend_kind(&config_toml_load_result(
            config_toml,
            /*feature_requirements*/ None,
        )?)?,
        AuthKeyringBackendKind::Secrets
    );

    let config_toml = ConfigToml {
        features: Some(FeaturesToml::from(BTreeMap::from([(
            "secret_auth_storage".to_string(),
            false,
        )]))),
        ..Default::default()
    };
    assert_eq!(
        resolve_bootstrap_auth_keyring_backend_kind(&config_toml_load_result(
            config_toml.clone(),
            /*feature_requirements*/ None,
        )?)?,
        AuthKeyringBackendKind::Direct
    );

    let requirements = Sourced::new(
        FeatureRequirementsToml {
            entries: BTreeMap::from([("secret_auth_storage".to_string(), true)]),
        },
        RequirementSource::Unknown,
    );
    assert_eq!(
        resolve_bootstrap_auth_keyring_backend_kind(&config_toml_load_result(
            config_toml,
            Some(requirements),
        )?)?,
        AuthKeyringBackendKind::Secrets
    );

    Ok(())
}

#[test]
fn managed_auth_restrictions_intersect_workspaces_and_fail_closed() {
    let config = ConfigToml {
        forced_login_method: None,
        forced_managed_workspace_id: Some(ForcedManagedWorkspaceIds::Multiple(vec![
            " denied ".to_string(),
            " allowed ".to_string(),
        ])),
        ..Default::default()
    };
    let mut requirements = ConfigRequirements {
        allowed_login_methods: Some(Sourced::new(
            vec![ForcedLoginMethod::Managed],
            RequirementSource::Unknown,
        )),
        allowed_managed_workspaces: Some(Sourced::new(
            vec!["allowed".to_string()],
            RequirementSource::Unknown,
        )),
        ..Default::default()
    };

    let bootstrap_config = ConfigTomlLoadResult {
        config_toml: config.clone(),
        config_layer_stack: ConfigLayerStack::new(
            Vec::new(),
            requirements.clone(),
            ConfigRequirementsToml::default(),
        )
        .expect("requirements should stack"),
    };
    let auth_config = bootstrap_auth_config(Path::new("ruxex-home"), &bootstrap_config)
        .expect("policy should resolve");
    assert_eq!(auth_config.forced_login_method, None);
    assert!(auth_config.is_login_method_allowed(ForcedLoginMethod::Managed));
    assert!(!auth_config.is_login_method_allowed(ForcedLoginMethod::Api));
    assert_eq!(
        auth_config.forced_managed_workspace_id,
        Some(vec!["denied".to_string(), "allowed".to_string()])
    );
    assert_eq!(
        auth_config.effective_managed_workspaces(),
        Some(vec!["allowed".to_string()])
    );

    requirements.allowed_managed_workspaces =
        Some(Sourced::new(Vec::new(), RequirementSource::Unknown));
    let bootstrap_config = ConfigTomlLoadResult {
        config_toml: config,
        config_layer_stack: ConfigLayerStack::new(
            Vec::new(),
            requirements,
            ConfigRequirementsToml::default(),
        )
        .expect("requirements should stack"),
    };
    assert_eq!(
        bootstrap_auth_config(Path::new("ruxex-home"), &bootstrap_config)
            .expect_err("managed-auth policy without an allowed workspace must fail")
            .kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

#[test]
fn bootstrap_auth_config_applies_managed_store_and_managed_auth_base_url() {
    let configured_store = AuthCredentialsStoreMode::File;
    let configured_url = "https://user.example/backend-api/";
    let managed_store = AuthCredentialsStoreMode::Keyring;
    let managed_url = "https://managed.example/backend-api/";
    let config_toml = ConfigToml {
        cli_auth_credentials_store: Some(configured_store),
        managed_auth_base_url: Some(configured_url.to_string()),
        ..Default::default()
    };
    let requirements = ConfigRequirements {
        cli_auth_credentials_store: Some(Sourced::new(managed_store, RequirementSource::Unknown)),
        managed_auth_base_url: Some(Sourced::new(
            managed_url.to_string(),
            RequirementSource::Unknown,
        )),
        ..Default::default()
    };
    let bootstrap_config = ConfigTomlLoadResult {
        config_toml,
        config_layer_stack: ConfigLayerStack::new(
            Vec::new(),
            requirements,
            ConfigRequirementsToml::default(),
        )
        .expect("requirements should stack"),
    };

    let auth_config = bootstrap_auth_config(Path::new("ruxex-home"), &bootstrap_config)
        .expect("managed authentication settings should resolve");

    assert_eq!(auth_config.auth_credentials_store_mode, managed_store);
    assert_eq!(
        auth_config.managed_auth_base_url.as_deref(),
        Some(managed_url)
    );
}

fn config_toml_load_result(
    config_toml: ConfigToml,
    feature_requirements: Option<Sourced<FeatureRequirementsToml>>,
) -> std::io::Result<ConfigTomlLoadResult> {
    let requirements = ConfigRequirements {
        feature_requirements,
        ..Default::default()
    };
    Ok(ConfigTomlLoadResult {
        config_toml,
        config_layer_stack: ConfigLayerStack::new(
            Vec::new(),
            requirements,
            ConfigRequirementsToml::default(),
        )?,
    })
}

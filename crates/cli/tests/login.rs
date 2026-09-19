use std::path::Path;

use anyhow::Result;
use codex_config::types::AuthCredentialsStoreMode;
use codex_login::AuthKeyringBackendKind;
use codex_login::CODEX_ACCESS_TOKEN_ENV_VAR;
use codex_login::login_with_bedrock_access_keys;
use codex_protocol::shell_environment::OPENAI_FEDERATION_RULE_ID_ENV_VAR;
use codex_protocol::shell_environment::OPENAI_IDENTITY_TOKEN_FILE_ENV_VAR;
use predicates::str::contains;
use pretty_assertions::assert_eq;
use tempfile::TempDir;

fn codex_command(codex_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("codex")?);
    cmd.env("CODEX_HOME", codex_home);
    Ok(cmd)
}

fn write_file_auth_config(codex_home: &Path) -> Result<()> {
    std::fs::write(
        codex_home.join("config.toml"),
        "cli_auth_credentials_store = \"file\"\n",
    )?;
    Ok(())
}

#[test]
fn login_status_reports_auth_storage_errors() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_file_auth_config(codex_home.path())?;
    std::fs::write(codex_home.path().join("auth.json"), "{invalid json")?;

    codex_command(codex_home.path())?
        .args(["login", "status"])
        .assert()
        .failure()
        .stderr(contains("Error checking login status:"));

    Ok(())
}

#[test]
fn login_status_validates_configured_workload_identity() -> Result<()> {
    let codex_home = TempDir::new()?;
    write_file_auth_config(codex_home.path())?;
    let missing_assertion = codex_home.path().join("missing-identity-token");

    codex_command(codex_home.path())?
        .env_remove(CODEX_ACCESS_TOKEN_ENV_VAR)
        .env(OPENAI_FEDERATION_RULE_ID_ENV_VAR, "rule-test")
        .env(OPENAI_IDENTITY_TOKEN_FILE_ENV_VAR, &missing_assertion)
        .args(["login", "status"])
        .assert()
        .failure()
        .stderr(contains("could not read workload identity assertion file"));

    Ok(())
}

#[test]
fn logout_clears_only_the_selected_bedrock_provider() -> Result<()> {
    for (model_provider_id, managed_bedrock_auth, model) in [
        ("amazon-bedrock", true, "openai.gpt-5.6-sol"),
        ("amazon-bedrock-runtime", true, "global.openai.gpt-5.6-sol"),
        ("openai", true, "gpt-5.6-sol"),
        ("amazon-bedrock", false, "gpt-5.6-sol"),
        ("amazon-bedrock-runtime", false, "us.openai.gpt-5.6-sol"),
        ("openai", false, "gpt-5.6-sol"),
    ] {
        let codex_home = TempDir::new()?;
        let config_path = codex_home.path().join("config.toml");
        std::fs::write(
            &config_path,
            format!(
                "cli_auth_credentials_store = \"file\"\n\
                 model_provider = \"{model_provider_id}\"\n\
                 model = \"{model}\"\n\
                 model_reasoning_effort = \"high\"\n\
                 [model_providers.amazon-bedrock]\n\
                 base_url = \"https://mantle.example.com/v1\"\n\
                 [model_providers.amazon-bedrock.aws]\n\
                 profile = \"mantle-profile\"\n\
                 region = \"us-west-2\"\n\
                 auth_refresh = {{ command = \"aws\", args = [\"sso\", \"login\"] }}\n\
                 [model_providers.amazon-bedrock-runtime]\n\
                 base_url = \"https://runtime.example.com/v1\"\n\
                 [model_providers.amazon-bedrock-runtime.aws]\n\
                 profile = \"runtime-profile\"\n\
                 region = \"us-east-1\"\n\
                 auth_refresh = {{ command = \"aws\", args = [\"login\"] }}\n"
            ),
        )?;
        if managed_bedrock_auth {
            login_with_bedrock_access_keys(
                codex_home.path(),
                "managed-access-key-id",
                "managed-secret-access-key",
                Some("managed-session-token"),
                AuthCredentialsStoreMode::File,
                AuthKeyringBackendKind::default(),
            )?;
        }
        let mut expected_config: toml::Value =
            toml::from_str(&std::fs::read_to_string(&config_path)?)?;
        if model_provider_id != "openai" {
            let expected_root = expected_config
                .as_table_mut()
                .expect("config should be a table");
            expected_root.remove("model_provider");
            expected_root.remove("model");
            expected_root["model_providers"][model_provider_id]
                .as_table_mut()
                .expect("selected Bedrock provider should be a table")
                .remove("aws");
        }
        let expected_message = if managed_bedrock_auth || model_provider_id != "openai" {
            "Successfully logged out"
        } else {
            "Not logged in"
        };

        codex_command(codex_home.path())?
            .env_remove(CODEX_ACCESS_TOKEN_ENV_VAR)
            .env("AWS_ACCESS_KEY_ID", "environment-access-key-id")
            .env("AWS_SECRET_ACCESS_KEY", "environment-secret-access-key")
            .args(["logout"])
            .assert()
            .success()
            .stderr(contains(expected_message));

        assert!(!codex_home.path().join("auth.json").exists());
        let actual_config: toml::Value = toml::from_str(&std::fs::read_to_string(&config_path)?)?;
        assert_eq!(actual_config, expected_config);
    }

    Ok(())
}

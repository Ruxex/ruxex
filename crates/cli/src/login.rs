//! CLI authentication status and logout commands.

use genex_core::config::Config;
use genex_login::CredentialManager;
use genex_model_provider::create_model_provider;
use genex_model_provider::provider_contract;
use genex_protocol::account::ProviderAuthStatus;
use genex_utils_cli::CliConfigOverrides;
use std::sync::Arc;

async fn load_cli_credential_manager(config: &Config) -> anyhow::Result<Arc<CredentialManager>> {
    Ok(CredentialManager::shared_from_config(config, /*enable_api_key_env*/ true).await?)
}

pub async fn run_login_status(cli_config_overrides: CliConfigOverrides) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let manager = match load_cli_credential_manager(&config).await {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Error checking login status: {err}");
            std::process::exit(1);
        }
    };
    let provider = provider_contract(create_model_provider(
        config.model_provider.clone(),
        Some(manager),
    ));
    let authenticated = provider.auth_state().status == ProviderAuthStatus::Authenticated;
    eprintln!(
        "{}: {}",
        config.model_provider_id,
        if authenticated {
            "authenticated"
        } else {
            "not authenticated"
        }
    );
    std::process::exit(if authenticated { 0 } else { 1 });
}

pub async fn run_logout(cli_config_overrides: CliConfigOverrides) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let manager = match load_cli_credential_manager(&config).await {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Error logging out: {err}");
            std::process::exit(1);
        }
    };
    let provider = provider_contract(create_model_provider(
        config.model_provider.clone(),
        Some(Arc::clone(&manager)),
    ));
    let was_authenticated = provider.auth_state().status == ProviderAuthStatus::Authenticated;
    match provider.logout().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Error logging out: {err}");
            std::process::exit(1);
        }
    }

    if was_authenticated {
        eprintln!("Successfully logged out");
    } else {
        eprintln!("Not logged in");
    }
    std::process::exit(0);
}

pub async fn run_login_delete(cli_config_overrides: CliConfigOverrides) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let manager = match load_cli_credential_manager(&config).await {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Error loading credentials: {err}");
            std::process::exit(1);
        }
    };
    let provider = provider_contract(create_model_provider(
        config.model_provider.clone(),
        Some(manager),
    ));
    match provider.delete().await {
        Ok(true) => eprintln!(
            "Deleted credentials for model provider `{}`",
            config.model_provider_id
        ),
        Ok(false) => eprintln!(
            "No stored credentials for model provider `{}`",
            config.model_provider_id
        ),
        Err(err) => {
            eprintln!("Error deleting provider credentials: {err}");
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

async fn load_config_or_exit(cli_config_overrides: CliConfigOverrides) -> Config {
    let cli_overrides = match cli_config_overrides.parse_overrides() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error parsing -c overrides: {e}");
            std::process::exit(1);
        }
    };

    match Config::load_with_cli_overrides(cli_overrides).await {
        Ok(config) => match config.auth_config().validate() {
            Ok(()) => config,
            Err(e) => {
                eprintln!("Error loading configuration: {e}");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Error loading configuration: {e}");
            std::process::exit(1);
        }
    }
}

fn safe_format_key(key: &str) -> String {
    if key.len() <= 13 {
        return "***".to_string();
    }
    let prefix = &key[..8];
    let suffix = &key[key.len() - 5..];
    format!("{prefix}***{suffix}")
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::safe_format_key;

    #[test]
    fn formats_long_key() {
        let key = "sk-proj-1234567890ABCDE";
        assert_eq!(safe_format_key(key), "sk-proj-***ABCDE");
    }

    #[test]
    fn short_key_returns_stars() {
        let key = "sk-proj-12345";
        assert_eq!(safe_format_key(key), "***");
    }
}

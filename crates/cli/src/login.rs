//! CLI authentication status and logout commands.

use genex_core::config::Config;
use genex_login::AuthManager;
use genex_login::list_provider_ids;
use genex_utils_cli::CliConfigOverrides;
use std::sync::Arc;

async fn load_cli_auth_manager(config: &Config) -> anyhow::Result<Arc<AuthManager>> {
    Ok(AuthManager::shared_from_config(config, /*enable_api_key_env*/ true).await?)
}

pub async fn run_login_status(cli_config_overrides: CliConfigOverrides) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let manager = match load_cli_auth_manager(&config).await {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Error checking login status: {err}");
            std::process::exit(1);
        }
    };
    let auth_config = config.auth_config();
    let stored_provider_ids = match list_provider_ids(
        &auth_config.genex_home,
        auth_config.auth_credentials_store_mode,
        auth_config.keyring_backend_kind,
    ) {
        Ok(provider_ids) => provider_ids,
        Err(err) => {
            eprintln!("Error checking stored provider credentials: {err}");
            std::process::exit(1);
        }
    };
    let active_provider_id = manager.active_provider_id();
    let mut has_active_provider = false;
    for (provider_id, provider) in &config.model_providers {
        let state = if active_provider_id.as_deref() == Some(provider_id) {
            has_active_provider = true;
            "active"
        } else if !provider.requires_auth {
            "available"
        } else if stored_provider_ids.iter().any(|id| id == provider_id) {
            "authenticated"
        } else {
            "not authenticated"
        };
        eprintln!("{provider_id}: {state}");
    }
    if !has_active_provider {
        eprintln!("No active model provider");
    }
    std::process::exit(if has_active_provider { 0 } else { 1 });
}

pub async fn run_logout(cli_config_overrides: CliConfigOverrides) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let manager = match load_cli_auth_manager(&config).await {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Error logging out: {err}");
            std::process::exit(1);
        }
    };
    let logged_out = match manager.logout().await {
        Ok(logged_out) => logged_out,
        Err(err) => {
            eprintln!("Error logging out: {err}");
            std::process::exit(1);
        }
    };

    if logged_out {
        eprintln!("Successfully logged out");
    } else {
        eprintln!("Not logged in");
    }
    std::process::exit(0);
}

pub async fn run_login_switch(cli_config_overrides: CliConfigOverrides, provider_id: String) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let manager = match load_cli_auth_manager(&config).await {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Error loading credentials: {err}");
            std::process::exit(1);
        }
    };
    match config.switch_model_provider(&manager, &provider_id).await {
        Ok(_) => eprintln!("Switched to model provider `{provider_id}`"),
        Err(err) => {
            eprintln!("Error switching model provider: {err}");
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

pub async fn run_login_delete(cli_config_overrides: CliConfigOverrides, provider_id: String) -> ! {
    let config = load_config_or_exit(cli_config_overrides).await;
    let manager = match load_cli_auth_manager(&config).await {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Error loading credentials: {err}");
            std::process::exit(1);
        }
    };
    match manager.delete(&provider_id).await {
        Ok(true) => eprintln!("Deleted credentials for model provider `{provider_id}`"),
        Ok(false) => eprintln!("No stored credentials for model provider `{provider_id}`"),
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

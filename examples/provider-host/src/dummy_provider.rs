use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use futures::stream;
use ruxex::provider::auth::{CredentialManager, ProviderAuth};
use ruxex::provider::protocol::{ModelCatalog, ModelInfo, ProviderAccount, ProviderAuthStatus};
use ruxex::provider::{
    ChatRequest, ListModelsRequest, ModelProvider, ModelProviderFactory, ProviderAccountState,
    ProviderAuthEnvironment, ProviderCapabilities, ProviderChatEvent, ProviderConnectionKey,
    ProviderCredentialSnapshot, ProviderDescriptor, ProviderError, ProviderFuture, ProviderInit,
    ProviderModelCatalog, ProviderRequestAuth, ProviderRequestSnapshot, ProviderResult,
    ProviderUsage, UsageRequest,
};
use ruxex_app_server::{AppServerRuntime, AppServerRuntimeBuilder};
use ruxex_arg0::Arg0DispatchPaths;
use ruxex_cli::APP_SERVER_DAEMON_MODE_ARG;
use ruxex_model_provider::CURRENT_PROVIDER_PROTOCOL_VERSION;

pub const DUMMY_PROVIDER_ID: &str = "dummy";
pub const FIXED_CHAT_ANSWER: &str = "これはダミーProviderの固定応答です。";
const DUMMY_HOME_ENV: &str = "RUXEX_DUMMY_HOME";

#[derive(Clone, Debug)]
pub struct DummyProvider {
    credential_manager: Arc<CredentialManager>,
}

#[derive(Debug)]
struct DummyCredentialSnapshot;

impl ProviderCredentialSnapshot for DummyCredentialSnapshot {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ModelProviderFactory for DummyProvider {
    fn provider_descriptor() -> ProviderDescriptor {
        ProviderDescriptor::new(DUMMY_PROVIDER_ID, CURRENT_PROVIDER_PROTOCOL_VERSION)
    }

    fn new(init: ProviderInit) -> Self {
        Self {
            credential_manager: init.credential_manager,
        }
    }
}

impl ModelProvider for DummyProvider {
    fn descriptor(&self) -> ProviderDescriptor {
        Self::provider_descriptor()
    }

    fn request_format(&self) -> ruxex::provider::ProviderRequestFormat {
        ruxex::provider::ProviderRequestFormat::Chat
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            api_key_model_discovery: true,
            web_search: false,
            external_web_access: false,
            namespace_tools: false,
            ..ProviderCapabilities::default()
        }
    }

    fn model_catalog_auth(&self) -> ruxex::provider::ProviderModelCatalogAuth {
        ruxex::provider::ProviderModelCatalogAuth {
            has_api_key_auth: true,
            ..Default::default()
        }
    }

    fn chat(
        &self,
        _request: ChatRequest,
    ) -> ProviderFuture<'_, ProviderResult<ruxex::provider::ProviderChatStream>> {
        Box::pin(async move {
            if !self.is_authenticated() {
                return Err(ProviderError::Unauthenticated);
            }
            Ok(fixed_chat_stream())
        })
    }

    fn list_models(
        &self,
        _request: ListModelsRequest,
    ) -> ProviderFuture<'_, ProviderResult<ProviderModelCatalog>> {
        Box::pin(async {
            Ok(ProviderModelCatalog {
                catalog: ModelCatalog {
                    models: fixed_models(),
                },
                identity: format!("{DUMMY_PROVIDER_ID}:fixed-v1"),
                etag: Some("dummy-fixed-v1".to_string()),
            })
        })
    }

    fn auth_state(&self) -> ruxex::provider::ProviderAuthState {
        let authenticated = self.is_authenticated();
        ruxex::provider::ProviderAuthState {
            status: if authenticated {
                ProviderAuthStatus::Authenticated
            } else {
                ProviderAuthStatus::Unauthenticated
            },
            account_id: authenticated.then(|| "dummy-account".to_string()),
            display_name: authenticated.then(|| "Dummy User".to_string()),
            managed_auth: false,
            revision: Some(*self.credential_manager.auth_change_receiver().borrow()),
        }
    }

    fn auth_environment(&self) -> ProviderAuthEnvironment {
        ProviderAuthEnvironment {
            host_api_key_env_present: std::env::var_os("RUXEX_API_KEY").is_some(),
            host_api_key_env_enabled: self.credential_manager.api_key_env_enabled(),
            ..ProviderAuthEnvironment::default()
        }
    }

    fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>> {
        Box::pin(async move {
            let revision = Some(*self.credential_manager.auth_change_receiver().borrow());
            Ok(ProviderRequestSnapshot {
                auth: ProviderRequestAuth {
                    authenticated: self.is_authenticated(),
                    managed_auth: false,
                    auth_header_attached: false,
                    auth_header_name: None,
                    revision,
                    owner_generation: None,
                },
                connection_key: ProviderConnectionKey {
                    provider_kind: DUMMY_PROVIDER_ID.to_string(),
                    auth_revision: revision,
                },
                credential_snapshot: Arc::new(DummyCredentialSnapshot),
            })
        })
    }

    fn account_state(&self) -> ProviderResult<ProviderAccountState> {
        let authenticated = self.is_authenticated();
        Ok(ProviderAccountState {
            provider_id: DUMMY_PROVIDER_ID.to_string(),
            account: authenticated.then_some(ProviderAccount::ApiKey),
            account_id: authenticated.then(|| "dummy-account".to_string()),
            display_name: authenticated.then(|| "Dummy User".to_string()),
            auth_status: if authenticated {
                ProviderAuthStatus::Authenticated
            } else {
                ProviderAuthStatus::Unauthenticated
            },
            usage: None,
        })
    }

    fn login(
        &self,
        mut credentials: BTreeMap<String, String>,
    ) -> ProviderFuture<'_, ProviderResult<()>> {
        let api_key = credentials.remove("api_key");
        Box::pin(async move {
            let api_key = validate_api_key(api_key.as_deref().unwrap_or_default())?;
            self.credential_manager
                .login(ProviderAuth::api_key(api_key))
                .await
                .map_err(|error| ProviderError::Provider(error.to_string()))
        })
    }

    fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>> {
        Box::pin(async move {
            self.credential_manager
                .logout()
                .await
                .map(|_| ())
                .map_err(|error| ProviderError::Provider(error.to_string()))
        })
    }

    fn delete(&self) -> ProviderFuture<'_, ProviderResult<bool>> {
        Box::pin(async move {
            self.credential_manager
                .delete()
                .await
                .map_err(|error| ProviderError::Provider(error.to_string()))
        })
    }

    fn usage(&self, _request: UsageRequest) -> ProviderFuture<'_, ProviderResult<ProviderUsage>> {
        Box::pin(async { Ok(ProviderUsage::Unsupported) })
    }
}

impl DummyProvider {
    fn is_authenticated(&self) -> bool {
        self.credential_manager
            .auth_cached()
            .is_some_and(|auth| auth.api_key().is_some())
    }
}

fn validate_api_key(api_key: &str) -> ProviderResult<String> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        Err(ProviderError::InvalidRequest(
            "api_key is required".to_string(),
        ))
    } else {
        Ok(api_key.to_string())
    }
}

fn fixed_models() -> Vec<ModelInfo> {
    ["A", "B", "C"]
        .into_iter()
        .map(ruxex_models_manager::model_info::model_info_template_from_slug)
        .collect()
}

fn fixed_chat_stream() -> ruxex::provider::ProviderChatStream {
    Box::pin(stream::iter([
        Ok(ProviderChatEvent::Created {
            response_id: Some("dummy-response".to_string()),
        }),
        Ok(ProviderChatEvent::ServerModel("A".to_string())),
        Ok(ProviderChatEvent::OutputTextDelta(
            FIXED_CHAT_ANSWER.to_string(),
        )),
        Ok(ProviderChatEvent::Completed {
            response_id: "dummy-response".to_string(),
            token_usage: None,
            usage_metadata: None,
            end_turn: Some(true),
        }),
    ]))
}

pub fn provider_factory() -> ruxex_cli::ProviderRuntimeFactory {
    fn runtime(init: ProviderInit) -> AppServerRuntime {
        AppServerRuntimeBuilder::build_from_provider_factory::<DummyProvider>(init)
    }

    (runtime, DummyProvider::provider_descriptor())
}

pub fn prepare_home() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os(DUMMY_HOME_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("ruxex-dummy-provider"));
    ruxex_model_provider::mount_provider_home(&home, &DummyProvider::provider_descriptor())
        .map_err(|error| anyhow::anyhow!(error))?;
    let config_path = home.join("config.toml");
    if !config_path.exists() {
        fs::write(
            &config_path,
            "approval_policy = \"never\"\nsandbox_mode = \"read-only\"\n",
        )?;
    }
    // SAFETY: this runs before the AppServer runtime is started and before the process creates
    // worker threads, so the process-wide home selection cannot race with another environment
    // mutation.
    unsafe { std::env::set_var("RUXEX_HOME", &home) };
    Ok(home)
}

pub async fn run(arg0_paths: Arg0DispatchPaths) -> anyhow::Result<()> {
    if std::env::args_os().any(|arg| arg == APP_SERVER_DAEMON_MODE_ARG) {
        let exit = ruxex_cli::run_host_app_server_daemon_with_provider_factory(
            arg0_paths,
            Default::default(),
            Default::default(),
            false,
            provider_factory(),
        )
        .await?;
        if exit == ruxex_app_server::AppServerExit::Forced {
            std::process::exit(0);
        }
    } else {
        ruxex_cli::run_with_provider_factory(arg0_paths, provider_factory(), None).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[test]
    fn exposes_three_fixed_models() {
        let models = fixed_models();

        assert_eq!(
            models
                .iter()
                .map(|model| model.slug.as_str())
                .collect::<Vec<_>>(),
            ["A", "B", "C"]
        );
    }

    #[test]
    fn returns_one_fixed_chat_answer() {
        let events = futures::executor::block_on(fixed_chat_stream().collect::<Vec<_>>());

        assert!(events.iter().any(|event| {
            matches!(
                event,
                Ok(ruxex::provider::ProviderChatEvent::OutputTextDelta(text))
                    if text == FIXED_CHAT_ANSWER
            )
        }));
        assert!(matches!(
            events.last(),
            Some(Ok(ruxex::provider::ProviderChatEvent::Completed { .. }))
        ));
    }

    #[test]
    fn rejects_blank_api_keys() {
        assert!(validate_api_key("   ").is_err());
        assert_eq!(validate_api_key("dummy-key").unwrap(), "dummy-key");
    }

    #[test]
    fn login_marks_the_provider_authenticated() {
        let home = tempfile::tempdir().expect("test home");
        let manager = Arc::new(futures::executor::block_on(CredentialManager::new(
            home.path().to_path_buf(),
            false,
            ruxex_login::AuthCredentialsStoreMode::Ephemeral,
            None,
            None,
            ruxex_login::AuthKeyringBackendKind::default(),
            ruxex_login::AuthRouteConfig::from_http_client_factory(
                ruxex::provider::transport::HttpClientFactory::new(
                    ruxex::provider::transport::OutboundProxyPolicy::ReqwestDefault,
                ),
            ),
        )));
        let provider = DummyProvider {
            credential_manager: manager,
        };
        let mut credentials = BTreeMap::new();
        credentials.insert("api_key".to_string(), "dummy-key".to_string());

        futures::executor::block_on(provider.login(credentials)).expect("login succeeds");

        assert!(provider.is_authenticated());
        assert_eq!(
            provider.auth_state().account_id.as_deref(),
            Some("dummy-account")
        );
    }
}

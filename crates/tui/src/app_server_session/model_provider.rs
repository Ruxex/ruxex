use super::AppServerSession;
use super::ThreadParamsMode;
use crate::legacy_core::config::Config;
use color_eyre::eyre::ContextCompat;
use color_eyre::eyre::Result;
use genex_app_server_protocol::AccountSwitchParams;
use genex_app_server_protocol::AccountSwitchResponse;
use genex_app_server_protocol::ClientRequest;
use genex_app_server_protocol::ModelProviderListParams;
use genex_app_server_protocol::ModelProviderListResponse;
use genex_app_server_protocol::ProviderAuthStatus;

impl AppServerSession {
    pub(crate) async fn switch_model_provider(
        &mut self,
        config: &mut Config,
        provider_id: &str,
    ) -> Result<Option<String>> {
        if config.model_provider_id == provider_id {
            return Ok(None);
        }

        let request_id = self.next_request_id();
        self.client
            .request_typed::<AccountSwitchResponse>(ClientRequest::AccountSwitch {
                request_id,
                params: AccountSwitchParams {
                    provider_id: provider_id.to_string(),
                },
            })
            .await
            .map_err(|err| {
                super::bootstrap_request_error("account/switch failed during TUI bootstrap", err)
            })?;

        if self.thread_params_mode == ThreadParamsMode::Embedded {
            let provider = config
                .model_providers
                .get(provider_id)
                .cloned()
                .wrap_err("authenticated model provider is not present in local TUI config")?;
            config.model_provider_id = provider_id.to_string();
            config.model_provider = provider;
        }

        Ok(Some(provider_id.to_string()))
    }

    pub(crate) async fn model_provider_list(&mut self) -> Result<ModelProviderListResponse> {
        let request_id = self.next_request_id();
        self.client
            .request_typed(ClientRequest::ModelProviderList {
                request_id,
                params: ModelProviderListParams {},
            })
            .await
            .map_err(|err| {
                super::bootstrap_request_error(
                    "modelProvider/list failed during TUI bootstrap",
                    err,
                )
            })
    }
}

pub(crate) fn authenticated_model_provider_id(
    response: &ModelProviderListResponse,
) -> Option<&str> {
    response
        .active_provider_id
        .as_deref()
        .and_then(|active_provider_id| {
            response.providers.iter().find(|provider| {
                provider.id == active_provider_id
                    && provider.auth_status == ProviderAuthStatus::Authenticated
            })
        })
        .or_else(|| {
            response
                .providers
                .iter()
                .find(|provider| provider.auth_status == ProviderAuthStatus::Authenticated)
        })
        .map(|provider| provider.id.as_str())
}

#[cfg(test)]
mod tests {
    use super::authenticated_model_provider_id;
    use genex_app_server_protocol::ModelProviderListEntry;
    use genex_app_server_protocol::ModelProviderListResponse;
    use genex_app_server_protocol::ProviderAuthStatus;
    use pretty_assertions::assert_eq;

    fn provider(id: &str, auth_status: ProviderAuthStatus) -> ModelProviderListEntry {
        ModelProviderListEntry {
            id: id.to_string(),
            name: id.to_string(),
            active: false,
            auth_status,
            auth_fields: Vec::new(),
        }
    }

    #[test]
    fn prefers_authenticated_active_provider() {
        let response = ModelProviderListResponse {
            active_provider_id: Some("provider_b".to_string()),
            providers: vec![
                provider("provider_a", ProviderAuthStatus::Authenticated),
                provider("provider_b", ProviderAuthStatus::Authenticated),
            ],
        };

        assert_eq!(
            authenticated_model_provider_id(&response),
            Some("provider_b")
        );
    }

    #[test]
    fn selects_first_authenticated_provider_when_active_is_unauthenticated() {
        let response = ModelProviderListResponse {
            active_provider_id: Some("provider_a".to_string()),
            providers: vec![
                provider("provider_a", ProviderAuthStatus::Unauthenticated),
                provider("provider_b", ProviderAuthStatus::Authenticated),
            ],
        };

        assert_eq!(
            authenticated_model_provider_id(&response),
            Some("provider_b")
        );
    }
}

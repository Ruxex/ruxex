use super::*;
use ruxex_app_server_protocol::Account;
use ruxex_app_server_protocol::AccountDeleteParams;
use ruxex_app_server_protocol::AccountDeleteResponse;
use ruxex_app_server_protocol::GetAccountResponse;

#[derive(Clone)]
pub(crate) struct AccountRequestProcessor {
    provider: ruxex_model_provider::SharedModelProvider,
    thread_manager: Arc<ThreadManager>,
    outgoing: Arc<OutgoingMessageSender>,
    config_manager: ConfigManager,
}

impl AccountRequestProcessor {
    pub(crate) fn new(
        provider: ruxex_model_provider::SharedModelProvider,
        thread_manager: Arc<ThreadManager>,
        outgoing: Arc<OutgoingMessageSender>,
        config_manager: ConfigManager,
    ) -> Self {
        let processor = Self {
            provider,
            thread_manager,
            outgoing,
            config_manager,
        };
        let startup = processor.clone();
        tokio::spawn(async move {
            let _ = startup
                .get_account_response(GetAccountParams {
                    refresh_token: false,
                })
                .await;
        });
        processor
    }

    pub(crate) async fn login_account(
        &self,
        request_id: ConnectionRequestId,
        params: LoginAccountParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.login_v2(request_id, params).await.map(|()| None)
    }

    pub(crate) async fn logout_account(
        &self,
        request_id: ConnectionRequestId,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.logout_v2(request_id).await.map(|()| None)
    }

    pub(crate) async fn delete_account(
        &self,
        request_id: ConnectionRequestId,
        _params: AccountDeleteParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let result = self
            .provider
            .delete()
            .await
            .map(|deleted| AccountDeleteResponse { deleted })
            .map_err(|err| internal_error(format!("failed to delete provider credentials: {err}")));
        self.outgoing.send_result(request_id, result).await;
        Ok(None)
    }

    pub(crate) async fn cancel_login_account(
        &self,
        params: CancelLoginAccountParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.cancel_login_response(params)
            .await
            .map(|response| Some(response.into()))
    }

    pub(crate) async fn get_account(
        &self,
        params: GetAccountParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.get_account_response(params)
            .await
            .map(|response| Some(response.into()))
    }

    async fn get_account_response(
        &self,
        params: GetAccountParams,
    ) -> Result<GetAccountResponse, JSONRPCErrorError> {
        let _ = params.refresh_token;
        let account_state = self
            .provider
            .account_state()
            .map_err(|err| invalid_request(err.to_string()))?;
        let usage = self
            .provider
            .usage(ruxex_model_provider::UsageRequest::default())
            .await
            .map_err(|err| invalid_request(err.to_string()))?;
        let usage = match usage {
            ruxex_model_provider::ProviderUsage::Unsupported => None,
            ruxex_model_provider::ProviderUsage::Available(snapshot) => {
                Some(ruxex_app_server_protocol::RateLimitSnapshot::from(snapshot))
            }
        };
        Ok(GetAccountResponse {
            provider_id: account_state.provider_id,
            account: account_state.account.map(Account::from),
            account_id: account_state.account_id,
            display_name: account_state.display_name,
            auth_status: account_state.auth_status,
            usage,
        })
    }

    pub(crate) async fn cancel_active_login(&self) {}

    pub(crate) fn clear_external_auth(&self) {}

    fn current_account_updated_notification(&self) -> AccountUpdatedNotification {
        AccountUpdatedNotification {
            auth_mode: None,
            plan_type: None,
        }
    }

    async fn maybe_refresh_plugin_caches_for_current_config(
        config_manager: &ConfigManager,
        thread_manager: &Arc<ThreadManager>,
    ) {
        thread_manager.plugins_manager().clear_cache();

        match config_manager
            .load_latest_config(/*fallback_cwd*/ None)
            .await
        {
            Ok(config) => {
                Self::spawn_effective_plugins_changed_task(
                    Arc::clone(thread_manager),
                    config_manager.clone(),
                );
                let refresh_thread_manager = Arc::clone(thread_manager);
                let refresh_config_manager = config_manager.clone();
                let on_effective_plugins_changed: Arc<
                    dyn Fn(ruxex_core_plugins::EffectivePluginsChange) + Send + Sync,
                > = Arc::new(move |_change| {
                    Self::spawn_effective_plugins_changed_task(
                        Arc::clone(&refresh_thread_manager),
                        refresh_config_manager.clone(),
                    );
                });
                let _ = (config, on_effective_plugins_changed);
            }
            Err(err) => {
                warn!("failed to reload config after account changed: {err}");
            }
        }
    }

    fn spawn_effective_plugins_changed_task(
        thread_manager: Arc<ThreadManager>,
        config_manager: ConfigManager,
    ) {
        tokio::spawn(async move {
            thread_manager.plugins_manager().clear_cache();
            thread_manager.skills_service().clear_cache();
            crate::mcp_refresh::reload_mcp_config_best_effort(&thread_manager, &config_manager)
                .await;
            thread_manager.invalidate_mcp_runtimes().await;
        });
    }

    async fn login_v2(
        &self,
        request_id: ConnectionRequestId,
        params: LoginAccountParams,
    ) -> Result<(), JSONRPCErrorError> {
        match params {
            LoginAccountParams::ApiKey {
                api_key,
                credentials,
            } => {
                self.login_api_key_v2(request_id, api_key, credentials)
                    .await;
            }
        }
        Ok(())
    }

    async fn login_api_key_common(
        &self,
        api_key: &str,
        credentials: &std::collections::BTreeMap<String, String>,
    ) -> std::result::Result<(), JSONRPCErrorError> {
        let mut provider_credentials = credentials.clone();
        provider_credentials.insert("api_key".to_string(), api_key.to_string());
        self.provider
            .login(provider_credentials)
            .await
            .map_err(|err| internal_error(format!("failed to save api key: {err}")))
    }

    async fn login_api_key_v2(
        &self,
        request_id: ConnectionRequestId,
        api_key: String,
        credentials: std::collections::BTreeMap<String, String>,
    ) {
        let result = self
            .login_api_key_common(&api_key, &credentials)
            .await
            .map(|()| LoginAccountResponse::ApiKey {});
        let logged_in = result.is_ok();
        self.outgoing.send_result(request_id, result).await;

        if logged_in {
            self.send_login_success_notifications(/*login_id*/ None)
                .await;
        }
    }

    async fn cancel_login_response(
        &self,
        _params: CancelLoginAccountParams,
    ) -> Result<CancelLoginAccountResponse, JSONRPCErrorError> {
        Ok(CancelLoginAccountResponse {
            status: CancelLoginAccountStatus::NotFound,
        })
    }

    async fn send_login_success_notifications(&self, login_id: Option<Uuid>) {
        self.send_account_login_notifications(AccountLoginCompletedNotification {
            login_id: login_id.map(|id| id.to_string()),
            success: true,
            error: None,
            onboarding_entrypoint: None,
        })
        .await;
    }

    async fn send_account_login_notifications(
        &self,
        mut payload: AccountLoginCompletedNotification,
    ) {
        if payload.success
            && let Err(error) = self
                .get_account_response(GetAccountParams {
                    refresh_token: false,
                })
                .await
        {
            payload.success = false;
            payload.error = Some(error.message);
        }
        if payload.success {
            Self::maybe_refresh_plugin_caches_for_current_config(
                &self.config_manager,
                &self.thread_manager,
            )
            .await;
        }

        let success = payload.success;
        self.outgoing
            .send_server_notification(ServerNotification::AccountLoginCompleted(payload))
            .await;

        if success {
            let notification = self.current_account_updated_notification();
            self.outgoing
                .send_server_notification(ServerNotification::AccountUpdated(notification))
                .await;
        }
    }

    async fn logout_common(&self) -> std::result::Result<Option<AuthMode>, JSONRPCErrorError> {
        match self.thread_manager.model_provider().logout().await {
            Ok(_) => {}
            Err(err) => {
                return Err(internal_error(format!("logout failed: {err}")));
            }
        }
        Self::maybe_refresh_plugin_caches_for_current_config(
            &self.config_manager,
            &self.thread_manager,
        )
        .await;

        // Reflect the current auth method after logout (likely None).
        Ok(None)
    }

    async fn logout_v2(&self, request_id: ConnectionRequestId) -> Result<(), JSONRPCErrorError> {
        let result = self.logout_common().await;
        let account_updated =
            result
                .as_ref()
                .ok()
                .cloned()
                .map(|auth_mode| AccountUpdatedNotification {
                    auth_mode,
                    plan_type: None,
                });
        self.outgoing
            .send_result(request_id, result.map(|_| LogoutAccountResponse {}))
            .await;

        if let Some(payload) = account_updated {
            self.outgoing
                .send_server_notification(ServerNotification::AccountUpdated(payload))
                .await;
        }
        Ok(())
    }
}

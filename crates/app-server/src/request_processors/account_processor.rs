use super::*;
use crate::auth_mode::auth_mode_to_api;
use crate::outgoing_message::AccountNotification;
use genex_app_server_protocol::Account;
use genex_app_server_protocol::AccountDeleteParams;
use genex_app_server_protocol::AccountDeleteResponse;
use genex_app_server_protocol::GetAccountResponse;
use genex_model_provider::ProviderLoginRequest;

enum RefreshTokenRequestOutcome {
    NotAttemptedOrSucceeded,
    FailedTransiently,
    FailedPermanently,
}

#[derive(Clone)]
pub(crate) struct AccountRequestProcessor {
    credential_manager: Arc<CredentialManager>,
    thread_manager: Arc<ThreadManager>,
    outgoing: Arc<OutgoingMessageSender>,
    config_manager: ConfigManager,
}

impl AccountRequestProcessor {
    pub(crate) fn new(
        credential_manager: Arc<CredentialManager>,
        thread_manager: Arc<ThreadManager>,
        outgoing: Arc<OutgoingMessageSender>,
        config_manager: ConfigManager,
    ) -> Self {
        let processor = Self {
            credential_manager,
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
        let provider = self.thread_manager.model_provider();
        let result = provider
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
        self.refresh_token_if_requested(params.refresh_token).await;
        let provider = self.thread_manager.model_provider();
        let account_state = provider
            .account_state()
            .map_err(|err| invalid_request(err.to_string()))?;
        let usage = provider
            .usage()
            .await
            .map_err(|err| invalid_request(err.to_string()))?;
        let usage = match usage {
            genex_model_provider::ProviderUsage::Unsupported => None,
            genex_model_provider::ProviderUsage::Available(snapshot) => {
                Some(genex_app_server_protocol::RateLimitSnapshot::from(snapshot))
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

    pub(crate) fn clear_external_auth(&self) {
        self.credential_manager.clear_external_auth();
    }

    fn current_account_updated_notification(&self) -> AccountUpdatedNotification {
        AccountUpdatedNotification {
            auth_mode: self
                .credential_manager
                .auth_cached()
                .as_ref()
                .map(GenexAuth::api_auth_mode)
                .map(auth_mode_to_api),
            plan_type: None,
        }
    }

    async fn maybe_refresh_plugin_caches_for_current_config(
        config_manager: &ConfigManager,
        thread_manager: &Arc<ThreadManager>,
        _auth: Option<GenexAuth>,
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
                    dyn Fn(genex_core_plugins::EffectivePluginsChange) + Send + Sync,
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
                self.login_api_key_v2(request_id, api_key, credentials).await;
            }
        }
        Ok(())
    }

    fn external_auth_active_error(&self) -> JSONRPCErrorError {
        invalid_request(
            "External auth is active. Use host-managed authentication to update it or account/logout to clear it.",
        )
    }

    async fn login_api_key_common(
        &self,
        api_key: &str,
        credentials: &std::collections::BTreeMap<String, String>,
    ) -> std::result::Result<(), JSONRPCErrorError> {
        if self.credential_manager.has_external_auth() {
            return Err(self.external_auth_active_error());
        }

        if !self
            .credential_manager
            .is_login_method_allowed(ForcedLoginMethod::Api)
        {
            return Err(invalid_request(
                "API key login is disabled. Use managed authentication instead.",
            ));
        }

        self.thread_manager
            .model_provider()
            .login(ProviderLoginRequest::new(api_key, credentials.clone()))
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
        let auth_changes = self.credential_manager.auth_change_state_receiver();
        let owner_generation = auth_changes.borrow().owner_generation;
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
        if payload.success && auth_changes.borrow().owner_generation == owner_generation {
            Self::maybe_refresh_plugin_caches_for_current_config(
                &self.config_manager,
                &self.thread_manager,
                self.credential_manager.auth_cached(),
            )
            .await;
        }

        let success = payload.success;
        self.outgoing
            .send_account_notification(
                /*connection_id*/ None,
                &auth_changes,
                owner_generation,
                AccountNotification::LoginCompleted(payload),
            )
            .await;

        if success {
            let notification = self.current_account_updated_notification();
            self.outgoing
                .send_account_notification(
                    /*connection_id*/ None,
                    &auth_changes,
                    owner_generation,
                    AccountNotification::Updated(notification),
                )
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
            self.credential_manager.auth_cached(),
        )
        .await;

        // Reflect the current auth method after logout (likely None).
        Ok(self
            .credential_manager
            .auth_cached()
            .as_ref()
            .map(GenexAuth::api_auth_mode)
            .map(auth_mode_to_api))
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

    async fn refresh_token_if_requested(&self, do_refresh: bool) -> RefreshTokenRequestOutcome {
        if self.credential_manager.has_external_auth() {
            return RefreshTokenRequestOutcome::NotAttemptedOrSucceeded;
        }
        if do_refresh && let Err(err) = self.credential_manager.refresh_token().await {
            let failed_reason = err.failed_reason();
            if failed_reason.is_none() {
                tracing::warn!("failed to refresh token while getting account: {err}");
                return RefreshTokenRequestOutcome::FailedTransiently;
            }
            return RefreshTokenRequestOutcome::FailedPermanently;
        }
        RefreshTokenRequestOutcome::NotAttemptedOrSucceeded
    }
}

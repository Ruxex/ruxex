use super::*;
use crate::app_info::app_info_to_api;
use codex_connectors::AppToolPolicyEvaluator;

mod installed;
mod read;

pub(crate) struct AppsRequestProcessor {
    auth_manager: Arc<AuthManager>,
    thread_manager: Arc<ThreadManager>,
    outgoing: Arc<OutgoingMessageSender>,
    config_manager: ConfigManager,
    shutdown_token: CancellationToken,
    _shutdown_drop_guard: DropGuard,
}

impl AppsRequestProcessor {
    pub(crate) fn new(
        auth_manager: Arc<AuthManager>,
        thread_manager: Arc<ThreadManager>,
        outgoing: Arc<OutgoingMessageSender>,
        config_manager: ConfigManager,
        shutdown_token: CancellationToken,
    ) -> Self {
        let shutdown_drop_guard = shutdown_token.clone().drop_guard();
        Self {
            auth_manager,
            thread_manager,
            outgoing,
            config_manager,
            shutdown_token,
            _shutdown_drop_guard: shutdown_drop_guard,
        }
    }

    pub(crate) async fn apps_list(
        &self,
        request_id: &ConnectionRequestId,
        params: AppsListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        self.apps_list_inner(request_id, params)
            .await
            .map(|response| response.map(Into::into))
    }

    async fn apps_list_inner(
        &self,
        request_id: &ConnectionRequestId,
        params: AppsListParams,
    ) -> Result<Option<AppsListResponse>, JSONRPCErrorError> {
        let installed_start = Instant::now();
        let reload = params.force_refetch;
        let thread = if let Some(thread_id) = params.thread_id.as_deref() {
            let (_, loaded_thread) = self.load_thread(thread_id).await?;
            Some(loaded_thread)
        } else {
            None
        };
        let fallback_cwd = match thread.as_ref() {
            Some(thread) => Some(thread.config_snapshot().await.cwd().to_path_buf()),
            None => None,
        };
        let mut config = self.load_latest_config(fallback_cwd).await?;

        if let Some(thread) = thread {
            let _ = config
                .features
                .set_enabled(Feature::Apps, thread.enabled(Feature::Apps));
        }

        let auth = self.auth_manager.auth().await;
        if !config
            .features
            .apps_enabled_for_auth(auth.as_ref().is_some_and(CodexAuth::uses_codex_backend))
        {
            let response = AppsListResponse {
                data: Vec::new(),
                next_cursor: None,
            };
            record_legacy_apps_installed_duration(installed_start, reload);
            return Ok(Some(response));
        }

        let request = request_id.clone();
        let outgoing = Arc::clone(&self.outgoing);
        let environment_manager = self.thread_manager.environment_manager();
        let mcp_manager = self.thread_manager.mcp_manager();
        let plugins_manager = self.thread_manager.plugins_manager();
        let shutdown_token = self.shutdown_token.child_token();
        tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_token.cancelled() => {}
                _ = Self::apps_list_task(
                    outgoing,
                    request,
                    params,
                    config,
                    environment_manager,
                    mcp_manager,
                    plugins_manager,
                    installed_start,
                ) => {}
            }
        });
        Ok(None)
    }

    pub(crate) fn shutdown(&self) {
        self.shutdown_token.cancel();
    }

    #[allow(clippy::too_many_arguments)]
    async fn apps_list_task(
        outgoing: Arc<OutgoingMessageSender>,
        request_id: ConnectionRequestId,
        params: AppsListParams,
        config: Config,
        environment_manager: Arc<EnvironmentManager>,
        mcp_manager: Arc<McpManager>,
        plugins_manager: Arc<PluginsManager>,
        installed_start: Instant,
    ) {
        let reload = params.force_refetch;
        let retry_params = params.clone();
        let retry_config = config.clone();
        let retry_environment_manager = Arc::clone(&environment_manager);
        let retry_mcp_manager = Arc::clone(&mcp_manager);
        let retry_plugins_manager = Arc::clone(&plugins_manager);
        let result = Self::apps_list_response(
            &outgoing,
            params,
            config,
            environment_manager,
            mcp_manager,
            plugins_manager,
        )
        .await;
        if result.is_ok() {
            record_legacy_apps_installed_duration(installed_start, reload);
        }
        let should_retry = result
            .as_ref()
            .is_ok_and(|(_, codex_apps_ready)| !codex_apps_ready);
        outgoing
            .send_result(request_id, result.map(|(response, _)| response))
            .await;

        if should_retry && !retry_params.force_refetch {
            let mut retry_params = retry_params;
            retry_params.force_refetch = true;
            if let Err(err) = Self::apps_list_response(
                &outgoing,
                retry_params,
                retry_config,
                retry_environment_manager,
                retry_mcp_manager,
                retry_plugins_manager,
            )
            .await
            {
                warn!("failed to refresh app list after codex-apps readiness retry: {err:?}");
            }
        }
    }

    async fn apps_list_response(
        outgoing: &Arc<OutgoingMessageSender>,
        params: AppsListParams,
        config: Config,
        environment_manager: Arc<EnvironmentManager>,
        mcp_manager: Arc<McpManager>,
        plugins_manager: Arc<PluginsManager>,
    ) -> Result<(AppsListResponse, bool), JSONRPCErrorError> {
        let AppsListParams {
            cursor,
            limit,
            thread_id: _,
            force_refetch,
        } = params;
        let start = match cursor {
            Some(cursor) => match cursor.parse::<usize>() {
                Ok(idx) => idx,
                Err(_) => return Err(invalid_request(format!("invalid cursor: {cursor}"))),
            },
            None => 0,
        };

        let loaded_plugins = plugins_manager
            .plugins_for_config(&config.plugins_config_input())
            .await;
        let connector_snapshot =
            codex_connectors::ConnectorSnapshot::from_plugin_capability_summaries(
                loaded_plugins.capability_summaries(),
            );
        let plugin_connectors = plugin_connectors_from_snapshot(&connector_snapshot);
        let app_policy = AppToolPolicyEvaluator::new(&config.config_layer_stack);
        let cached_accessible =
            connectors::list_cached_accessible_connectors_from_mcp_tools(&config)
                .await
                .unwrap_or_default();
        let cached = app_policy.apply_app_enabled_state(codex_connectors::merge::merge_connectors(
            plugin_connectors.clone(),
            cached_accessible,
        ));

        let status = connectors::list_accessible_connectors_from_mcp_tools_with_mcp_manager(
            &config,
            force_refetch,
            environment_manager,
            mcp_manager,
        )
        .await
        .map_err(|err| internal_error(format!("failed to load accessible apps: {err}")))?;
        let merged = app_policy.apply_app_enabled_state(codex_connectors::merge::merge_connectors(
            plugin_connectors,
            status.connectors,
        ));
        if cached != merged {
            send_app_list_updated_notification(outgoing, merged.clone()).await;
        }
        let response = paginate_apps(merged.as_slice(), start, limit)?;
        Ok((response, status.codex_apps_ready))
    }

    async fn load_thread(
        &self,
        thread_id: &str,
    ) -> Result<(ThreadId, Arc<CodexThread>), JSONRPCErrorError> {
        let thread_id = ThreadId::from_string(thread_id)
            .map_err(|err| invalid_request(format!("invalid thread id: {err}")))?;

        let thread = self
            .thread_manager
            .get_thread(thread_id)
            .await
            .map_err(|_| invalid_request(format!("thread not found: {thread_id}")))?;

        Ok((thread_id, thread))
    }

    async fn load_latest_config(
        &self,
        fallback_cwd: Option<PathBuf>,
    ) -> Result<Config, JSONRPCErrorError> {
        self.config_manager
            .load_latest_config(fallback_cwd)
            .await
            .map_err(|err| internal_error(format!("failed to reload config: {err}")))
    }

    async fn load_apps_config(&self, thread_id: Option<&str>) -> Result<Config, JSONRPCErrorError> {
        let Some(thread_id) = thread_id else {
            return self.load_latest_config(/*fallback_cwd*/ None).await;
        };
        let (_, thread) = self.load_thread(thread_id).await?;
        let thread_config = thread.config().await;
        self.config_manager
            .load_latest_config_with_session_layers(
                &thread_config.config_layer_stack,
                &thread_config.cwd,
            )
            .await
            .map_err(|err| internal_error(format!("failed to reload config: {err}")))
    }
}

// `app/list` is the legacy request-path baseline for the `app/installed` endpoint;
// `path=legacy` keeps it separate from the new snapshot-backed implementation in dashboards.
const APPS_INSTALLED_DURATION_METRIC: &str = "codex.apps.installed.duration_ms";

fn record_legacy_apps_installed_duration(started_at: Instant, reload: bool) {
    let reload = if reload { "true" } else { "false" };
    if let Some(metrics) = codex_otel::global() {
        let _ = metrics.record_duration(
            APPS_INSTALLED_DURATION_METRIC,
            started_at.elapsed(),
            &[("path", "legacy"), ("reload", reload)],
        );
    }
}
fn plugin_connectors_from_snapshot(snapshot: &codex_connectors::ConnectorSnapshot) -> Vec<AppInfo> {
    let mut connectors = codex_connectors::merge::merge_plugin_connectors(
        Vec::new(),
        snapshot.connector_ids().iter().map(|id| id.0.clone()),
    );
    for connector in &mut connectors {
        connector.install_url = None;
        connector.plugin_display_names = snapshot
            .plugin_display_names_for_connector_id(&connector.id)
            .to_vec();
    }
    connectors
}

fn paginate_apps(
    connectors: &[AppInfo],
    start: usize,
    limit: Option<u32>,
) -> Result<AppsListResponse, JSONRPCErrorError> {
    let total = connectors.len();
    if start > total {
        return Err(invalid_request(format!(
            "cursor {start} exceeds total apps {total}"
        )));
    }

    let effective_limit = limit.unwrap_or(total as u32).max(1) as usize;
    let end = start.saturating_add(effective_limit).min(total);
    let data = connectors[start..end]
        .iter()
        .cloned()
        .map(app_info_to_api)
        .collect();
    let next_cursor = if end < total {
        Some(end.to_string())
    } else {
        None
    };

    Ok(AppsListResponse { data, next_cursor })
}

async fn send_app_list_updated_notification(
    outgoing: &Arc<OutgoingMessageSender>,
    data: Vec<AppInfo>,
) {
    let data = data.into_iter().map(app_info_to_api).collect();
    outgoing
        .send_server_notification(ServerNotification::AppListUpdated(
            AppListUpdatedNotification { data },
        ))
        .await;
}

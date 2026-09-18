use super::*;
use crate::app_info::connector_metadata_to_api;
use codex_connectors::ConnectorMetadata;

pub(in crate::request_processors) const APP_READ_MAX_IDS: usize = 100;
const APPS_READ_DURATION_METRIC: &str = "codex.apps.read.duration_ms";

impl AppsRequestProcessor {
    pub(crate) async fn apps_read(
        &self,
        params: AppsReadParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let started_at = Instant::now();
        let AppsReadParams {
            app_ids,
            thread_id,
            include_tools,
        } = params;
        if app_ids.len() > APP_READ_MAX_IDS {
            return Err(invalid_params(format!(
                "app/read accepts at most {APP_READ_MAX_IDS} appIds"
            )));
        }

        let mut seen_app_ids = HashSet::new();
        let app_ids = app_ids
            .into_iter()
            .filter(|app_id| seen_app_ids.insert(app_id.clone()))
            .collect::<Vec<_>>();
        let config = self.load_apps_config(thread_id.as_deref()).await?;
        let auth = self.auth_manager.auth().await;
        if !config
            .features
            .apps_enabled_for_auth(auth.as_ref().is_some_and(CodexAuth::uses_codex_backend))
        {
            let response = AppsReadResponse {
                apps: Vec::new(),
                missing_app_ids: app_ids,
            };
            record_apps_read_duration(started_at, include_tools);
            return Ok(Some(response.into()));
        }
        let loaded_plugins = self
            .thread_manager
            .plugins_manager()
            .plugins_for_config(&config.plugins_config_input())
            .await;
        let connector_snapshot =
            codex_connectors::ConnectorSnapshot::from_plugin_capability_summaries(
                loaded_plugins.capability_summaries(),
            );
        let known_app_ids = connector_snapshot
            .connector_ids()
            .iter()
            .map(|connector_id| connector_id.0.as_str())
            .collect::<HashSet<_>>();
        let (apps, missing_app_ids): (Vec<_>, Vec<_>) = app_ids
            .into_iter()
            .partition(|app_id| known_app_ids.contains(app_id.as_str()));
        let apps = apps
            .into_iter()
            .map(|app_id| {
                let metadata = ConnectorMetadata {
                    id: app_id.clone(),
                    name: app_id,
                    description: None,
                    icon_url: None,
                    icon_url_dark: None,
                    distribution_channel: None,
                    tool_summaries: None,
                };
                let mut app = connector_metadata_to_api(metadata);
                app.install_url = None;
                app.plugin_display_names = connector_snapshot
                    .plugin_display_names_for_connector_id(app.id.as_str())
                    .to_vec();
                app
            })
            .collect();
        let response = AppsReadResponse {
            apps,
            missing_app_ids,
        };
        record_apps_read_duration(started_at, include_tools);
        Ok(Some(response.into()))
    }
}

fn record_apps_read_duration(started_at: Instant, include_tools: bool) {
    let include_tools = if include_tools { "true" } else { "false" };
    if let Some(metrics) = codex_otel::global() {
        let _ = metrics.record_duration(
            APPS_READ_DURATION_METRIC,
            started_at.elapsed(),
            &[("include_tools", include_tools)],
        );
    }
}

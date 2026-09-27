use super::*;
use codex_app_server_protocol::PluginSearchParams;
use codex_app_server_protocol::PluginSearchResponse;
use codex_app_server_protocol::PluginSearchResult;
use codex_app_server_protocol::PluginSearchScope;

const DEFAULT_PLUGIN_SEARCH_LIMIT: usize = 16;
const MAX_PLUGIN_SEARCH_LIMIT: usize = 1_000;

impl PluginRequestProcessor {
    pub(crate) async fn plugin_search(
        &self,
        params: PluginSearchParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let search_term = params.search_term.trim().to_lowercase();
        if search_term.is_empty() {
            return Ok(Some(
                PluginSearchResponse {
                    data: Vec::new(),
                    next_cursor: None,
                }
                .into(),
            ));
        }
        let config = self
            .load_catalog_config(params.cwds.as_deref().unwrap_or_default())
            .await?;
        if !config.features.enabled(Feature::Plugins)
            || params.cursor.is_some()
            || matches!(params.scope, Some(PluginSearchScope::Workspace))
        {
            return Ok(Some(
                PluginSearchResponse {
                    data: Vec::new(),
                    next_cursor: None,
                }
                .into(),
            ));
        }
        let roots = params.cwds.unwrap_or_default();
        let plugins_input = config.plugins_config_input();
        let outcome = self
            .thread_manager
            .plugins_manager()
            .list_marketplaces_for_config(&plugins_input, &roots)
            .map_err(|err| internal_error(format!("failed to list marketplace plugins: {err}")))?;
        let limit = params
            .limit
            .map(|limit| limit as usize)
            .unwrap_or(DEFAULT_PLUGIN_SEARCH_LIMIT)
            .clamp(1, MAX_PLUGIN_SEARCH_LIMIT);
        let mut data = outcome
            .marketplaces
            .into_iter()
            .filter(|_| marketplace_matches_search_scope(params.scope))
            .flat_map(|marketplace| {
                let marketplace_name = marketplace.name.clone();
                let marketplace_path = marketplace.path.clone();
                let query = search_term.clone();
                marketplace.plugins.into_iter().filter_map(move |plugin| {
                    let summary = super::plugin_to_summary(plugin);
                    let values = [
                        summary.name.to_lowercase(),
                        summary.id.to_lowercase(),
                        summary.keywords.join(" ").to_lowercase(),
                    ];
                    values.iter().any(|value| value.contains(&query)).then(|| {
                        plugin_search_result(
                            summary,
                            marketplace_name.clone(),
                            Some(marketplace_path.clone()),
                        )
                    })
                })
            })
            .take(limit)
            .collect::<Vec<_>>();
        data.sort_by(|left, right| left.plugin.name.cmp(&right.plugin.name));
        Ok(Some(
            PluginSearchResponse {
                data,
                next_cursor: None,
            }
            .into(),
        ))
    }
}

fn plugin_search_result(
    mut plugin: PluginSummary,
    marketplace_name: String,
    marketplace_path: Option<AbsolutePathBuf>,
) -> PluginSearchResult {
    plugin.enabled = false;
    PluginSearchResult {
        plugin,
        marketplace_name,
        marketplace_path,
    }
}

fn marketplace_matches_search_scope(scope: Option<PluginSearchScope>) -> bool {
    match scope {
        Some(PluginSearchScope::Global | PluginSearchScope::Workspace) => false,
        None | Some(PluginSearchScope::Personal) => true,
    }
}

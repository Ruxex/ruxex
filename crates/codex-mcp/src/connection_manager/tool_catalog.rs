use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use futures::future::join_all;
use tracing::Instrument;
use tracing::instrument;
use tracing::trace;
use tracing::trace_span;

use super::McpConnectionSet;
use super::McpServerMetadata;
use crate::binding::McpBinding;
use crate::binding::PreparedMcpCall;
use crate::binding_clients::McpBindingClients;
use crate::client_tool_catalog::ClientToolCatalogRevision;
use crate::client_tool_catalog::ToolCatalogSnapshot;
use crate::rmcp_client::ManagedClient;
use crate::tools::ToolInfo;
use crate::tools::filter_tools;
use crate::tools::normalize_tools_for_model_with_prefix;

const MCP_UI_META_KEY: &str = "ui";
const MCP_UI_VISIBILITY_META_KEY: &str = "visibility";
const MCP_UI_MODEL_VISIBILITY: &str = "model";
/// Returns whether a tool may be included in model-facing tool declarations.
///
/// Tools without visibility metadata remain visible. Tools with visibility
/// metadata are hidden unless they explicitly include `model`.
///
/// <https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx#resource-discovery>
pub fn tool_is_model_visible(tool: &ToolInfo) -> bool {
    let Some(visibility) = tool
        .tool
        .meta
        .as_deref()
        .and_then(|meta| meta.get(MCP_UI_META_KEY))
        .and_then(serde_json::Value::as_object)
        .and_then(|ui| ui.get(MCP_UI_VISIBILITY_META_KEY))
        .and_then(serde_json::Value::as_array)
    else {
        return true;
    };
    visibility
        .iter()
        .any(|target| target.as_str() == Some(MCP_UI_MODEL_VISIBILITY))
}

/// Catalog identity within one published connection set, including dormant servers.
#[derive(PartialEq, Eq)]
pub(crate) enum BindingCatalogRevision {
    Ready(ClientToolCatalogRevision),
    Dormant(u64),
}

impl McpConnectionSet {
    pub(crate) async fn stable_catalog_revisions(
        &self,
        required_servers: &[String],
        required_plugins: &HashSet<String>,
    ) -> Option<HashMap<String, BindingCatalogRevision>> {
        let mut revisions = HashMap::new();
        for (server_name, view) in &self.servers {
            if !view
                .connection
                .client
                .startup_complete
                .load(Ordering::Acquire)
            {
                // A cached catalog can remain stable without starting its server.
                // Explicit requirements must still start the server during binding capture.
                if !view.connection.startup_is_dormant()
                    || view.connection.client.cancel_token.is_cancelled()
                    || required_servers
                        .iter()
                        .any(|required| required == server_name)
                    || (self.is_selected_plugin_mcp_server(server_name)
                        && self
                            .plugin_id_for_mcp_server_name(server_name)
                            .is_some_and(|plugin_id| required_plugins.contains(plugin_id)))
                {
                    return None;
                }
                let revision = view
                    .connection
                    .client
                    .tool_catalog_cache_context
                    .as_ref()?
                    .current_revision()?;
                revisions.insert(
                    server_name.clone(),
                    BindingCatalogRevision::Dormant(revision),
                );
                continue;
            }
            let Some(client) = view.connection.client.ready_client() else {
                if self.required_servers.binary_search(server_name).is_err()
                    && matches!(view.connection.client.client.peek(), Some(Err(_)))
                {
                    continue;
                }
                return None;
            };
            if client.client.is_closed().await {
                return None;
            }
            let revision = client.tool_catalog.read(|catalog| catalog.revision).await;
            revisions.insert(
                server_name.clone(),
                BindingCatalogRevision::Ready(ClientToolCatalogRevision {
                    catalog: Arc::clone(&client.tool_catalog),
                    revision,
                }),
            );
        }
        Some(revisions)
    }

    /// Returns all tools with model-visible names normalized.
    pub async fn list_all_tools(&self) -> Vec<ToolInfo> {
        self.list_tools_with_errors().await.0
    }

    #[instrument(level = "trace", skip_all, fields(mcp_server_count = self.servers.len()))]
    pub(crate) async fn list_tools_with_errors(&self) -> (Vec<ToolInfo>, HashMap<String, String>) {
        let mut tools = Vec::new();
        let mut errors = HashMap::new();
        let mut available_server_count = 0;
        let mut unavailable_server_count = 0;
        let server_results = join_all(self.servers.iter().map(|(server_name, view)| async move {
            view.connection.client.reconnect_failed_startup().await;
            let has_cached_tools = view.connection.client.has_cached_tools();
            let startup_complete = view
                .connection
                .client
                .startup_complete
                .load(Ordering::Acquire);
            let server_tools = view
                .listed_tools(&self.tool_plugin_context)
                .instrument(trace_span!(
                    "list_tools_for_server",
                    server_name = %server_name,
                    has_cached_tools,
                    startup_complete
                ))
                .await;
            let result = match server_tools {
                Ok(server_tools) => Ok(server_tools
                    .into_iter()
                    .map(|tool| Self::with_server_metadata(tool, &view.metadata))
                    .collect::<Vec<_>>()),
                Err(error) => {
                    trace!(
                        server_name = %server_name,
                        has_cached_tools,
                        startup_complete,
                        "MCP server tools unavailable while building tool list"
                    );
                    Err(error)
                }
            };
            (server_name, result)
        }))
        .await;
        for (server_name, server_tools) in server_results {
            match server_tools {
                Ok(server_tools) => {
                    available_server_count += 1;
                    tools.extend(server_tools);
                }
                Err(error) => {
                    unavailable_server_count += 1;
                    errors.insert(server_name.clone(), error.to_string());
                }
            }
        }
        let tools = normalize_tools_for_model_with_prefix(
            tools,
            self.prefix_mcp_tool_names,
            &self.non_prefixed_mcp_tool_servers,
        );
        trace!(
            available_server_count,
            unavailable_server_count,
            tool_count = tools.len(),
            "built MCP tool list"
        );
        (tools, errors)
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) async fn capture_binding_with_metadata(
        self: &Arc<Self>,
        config: Arc<crate::McpConfig>,
        plugins_available: bool,
        required_servers: &[String],
        required_plugins: &HashSet<String>,
    ) -> McpBinding {
        let mut listed_tools = Vec::new();
        let mut clients = HashMap::new();
        let optional_mcp_startup_grace = config.optional_mcp_startup_grace;
        let server_snapshots = join_all(self.servers.iter().map(|(server_name, view)| async move {
            if !view
                .connection
                .client
                .startup_complete
                .load(Ordering::Acquire)
            {
                let required = self.required_servers.binary_search(server_name).is_ok();
                // Keep the catalog that lets us skip startup even if it expires during the wait.
                let cached_tools = view.connection.client.cached_tools().filter(|tools| {
                    !tools.is_empty()
                });
                let has_cached_tools = cached_tools.is_some();
                let must_wait_for_startup = (required
                    && (!view.connection.startup_is_dormant() || !has_cached_tools))
                    || required_servers
                        .iter()
                        .any(|required| required == server_name)
                    || (self.is_selected_plugin_mcp_server(server_name)
                        && self
                            .plugin_id_for_mcp_server_name(server_name)
                            .is_some_and(|plugin_id| required_plugins.contains(plugin_id)))
                    ;
                if !must_wait_for_startup && has_cached_tools {
                    return (server_name, view, cached_tools);
                }
                if !must_wait_for_startup && optional_mcp_startup_grace.is_zero() {
                    if let Some(cache) = view.connection.client.tool_catalog_cache_context.as_ref()
                    {
                        cache.optional_startup_deadline(
                            tokio::time::Instant::now(),
                            optional_mcp_startup_grace,
                        );
                    }
                    let _ = view.connection.client().await;
                } else if !must_wait_for_startup {
                    let optional_startup_deadline = if view.connection.startup_is_dormant() {
                        tokio::time::Instant::now() + optional_mcp_startup_grace
                    } else {
                        *self.optional_startup_deadline.get_or_init(|| {
                            tokio::time::Instant::now() + optional_mcp_startup_grace
                        })
                    };
                    let startup_deadline = view
                        .connection
                        .client
                        .tool_catalog_cache_context
                        .as_ref()
                        .map(|cache| {
                            cache.optional_startup_deadline(
                                optional_startup_deadline,
                                optional_mcp_startup_grace,
                            )
                        })
                        .unwrap_or(optional_startup_deadline);
                    if tokio::time::timeout_at(startup_deadline, view.connection.client())
                        .await
                        .is_err()
                    {
                        trace!(server_name = %server_name, "omitting pending optional MCP server");
                    }
                    return (server_name, view, cached_tools);
                }
                let _ = view.connection.client().await;
                return (server_name, view, cached_tools);
            }
            (server_name, view, None)
        }))
        .await;
        let server_results = join_all(server_snapshots.into_iter().map(|(server_name, view, cached_tools)| async move {
            let (client, server_tools) = if !view
                .connection
                .client
                .startup_complete
                .load(Ordering::Acquire)
            {
                (None, view.connection.client.cached_tools_or(cached_tools)?)
            } else {
                view.connection.client.reconnect_failed_startup().await;
                let Ok(mut client) = view.connection.client().await else {
                    trace!(server_name = %server_name, "omitting MCP server without an exact ready client");
                    return None;
                };
                client.tool_timeout = view.tool_timeout;
                let snapshot = client.tool_catalog.read(Arc::new).await;
                let server_tools = snapshot.tools.to_vec();
                (Some((Arc::new(client), snapshot)), server_tools)
            };
            let server_tools = filter_tools(server_tools, &view.tool_filter);
            let server_tools = crate::rmcp_client::prepare_mcp_tools_for_model(
                server_tools,
                &self.tool_plugin_context,
            );
            let server_tools = server_tools
                .into_iter()
                .map(|mut tool| {
                    if client.is_none()
                        && let Some(annotations) = tool.tool.annotations.as_mut()
                    {
                        annotations.read_only_hint = None;
                    }
                    Self::with_server_metadata(tool, &view.metadata)
                })
                .collect::<Vec<_>>();
            Some((server_name.clone(), client, server_tools))
        }))
        .await;
        for (server_name, client, server_tools) in server_results.into_iter().flatten() {
            if let Some((client, snapshot)) = client {
                clients.insert(server_name, (client, snapshot));
            }
            listed_tools.extend(server_tools);
        }
        let listed_tools = normalize_tools_for_model_with_prefix(
            listed_tools,
            self.prefix_mcp_tool_names,
            &self.non_prefixed_mcp_tool_servers,
        );
        let mut tools = Vec::with_capacity(listed_tools.len());
        let mut calls = std::collections::HashMap::with_capacity(listed_tools.len());
        for tool_info in listed_tools {
            let model_visible = crate::tool_is_model_visible(&tool_info);
            let Some((client, snapshot)) = clients.get(&tool_info.server_name) else {
                if model_visible {
                    tools.push(tool_info);
                }
                continue;
            };
            let Some(call) = self.prepare_call(
                &tool_info,
                Arc::clone(client),
                Arc::clone(&config),
                Arc::clone(snapshot),
            ) else {
                trace!(
                    server_name = %tool_info.server_name,
                    tool_name = %tool_info.tool.name,
                    "omitting MCP tool without an exact ready client"
                );
                continue;
            };
            calls.insert(
                (
                    tool_info.server_name.clone(),
                    tool_info.tool.name.to_string(),
                ),
                call,
            );
            if model_visible {
                tools.push(tool_info);
            }
        }
        let clients = Arc::new(McpBindingClients::new(
            clients
                .into_iter()
                .map(|(server_name, (client, _))| (server_name, client))
                .collect(),
        ));
        McpBinding::new(
            Arc::clone(self),
            clients,
            config,
            plugins_available,
            tools,
            calls,
        )
    }

    fn prepare_call(
        self: &Arc<Self>,
        tool_info: &ToolInfo,
        client: Arc<ManagedClient>,
        config: Arc<crate::McpConfig>,
        tool_catalog_snapshot: Arc<ToolCatalogSnapshot>,
    ) -> Option<PreparedMcpCall> {
        let server_name = &tool_info.server_name;
        let view = self.servers.get(server_name)?;
        PreparedMcpCall::new(
            client,
            config,
            tool_catalog_snapshot,
            tool_info.clone(),
            view.metadata.clone(),
            self.plugin_id_for_mcp_server_name(server_name)
                .map(str::to_string),
            self.is_selected_plugin_mcp_server(server_name),
        )
    }

    fn with_server_metadata(mut tool: ToolInfo, metadata: &McpServerMetadata) -> ToolInfo {
        tool.supports_parallel_tool_calls = metadata.supports_parallel_tool_calls;
        tool.server_origin = metadata
            .origin
            .as_ref()
            .map(|origin| origin.as_str().to_string());
        tool
    }
}

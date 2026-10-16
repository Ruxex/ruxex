use std::collections::HashMap;

use ruxex_protocol::mcp::ClientMcpExtensions;
use ruxex_protocol::mcp::MCP_APP_UI_EXTENSION_ID;
use serde_json::Value;

/// Applies the configured MCP extensions to one server connection.
pub(crate) fn server_mcp_extensions(
    extensions: &ClientMcpExtensions,
    _server_name: &str,
    _server: Option<&crate::catalog::ResolvedMcpServer>,
) -> ClientMcpExtensions {
    ClientMcpExtensions::new(
        extensions
            .iter()
            .map(|(id, settings)| (id.to_string(), settings.clone())),
    )
}

/// Selects the MCP extensions Ruxex supports from those declared by the app-server host.
///
/// App-server clients may declare unrelated extensions. Ruxex retains only the
/// provider-neutral extension namespaces it knows how to project downstream.
pub fn client_mcp_extensions(extensions: Option<&HashMap<String, Value>>) -> ClientMcpExtensions {
    let selected = extensions
        .into_iter()
        .flat_map(HashMap::iter)
        .filter(|(id, _)| id.as_str() == MCP_APP_UI_EXTENSION_ID)
        .map(|(id, value)| (id.clone(), value.clone()))
        .collect::<HashMap<_, _>>();
    ClientMcpExtensions::new(selected)
}

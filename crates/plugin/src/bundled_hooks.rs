//! Temporary cleanup-hook allowlist shared by local and executor plugin discovery.
//! This narrowly authorizes known MCP cleanup calls; it does not verify plugin signatures.

use codex_config::HookHandlerConfig;
use codex_protocol::protocol::HookEventName;

struct BundledHook {
    plugin_id: &'static str,
    events: &'static [HookEventName],
    target: BundledHookTarget,
}

enum BundledHookTarget {
    McpServer {
        server: &'static str,
        tool: &'static str,
    },
}

// Keep unsigned plugin exceptions together so they can be removed as signing lands.
const ALLOWLISTED_BUNDLED_HOOKS: &[BundledHook] = &[
    BundledHook {
        plugin_id: "browser@codex-bundled",
        events: &[
            HookEventName::Stop,
            HookEventName::Interrupt,
            HookEventName::SubagentStop,
        ],
        target: BundledHookTarget::McpServer {
            server: "node_repl",
            tool: "turn_ended",
        },
    },
    BundledHook {
        plugin_id: "chrome@codex-bundled",
        events: &[
            HookEventName::Stop,
            HookEventName::Interrupt,
            HookEventName::SubagentStop,
        ],
        target: BundledHookTarget::McpServer {
            server: "node_repl",
            tool: "turn_ended",
        },
    },
    BundledHook {
        plugin_id: "chrome-dev@codex-bundled",
        events: &[
            HookEventName::Stop,
            HookEventName::Interrupt,
            HookEventName::SubagentStop,
        ],
        target: BundledHookTarget::McpServer {
            server: "node_repl",
            tool: "turn_ended",
        },
    },
    BundledHook {
        plugin_id: "chrome-internal@codex-bundled",
        events: &[
            HookEventName::Stop,
            HookEventName::Interrupt,
            HookEventName::SubagentStop,
        ],
        target: BundledHookTarget::McpServer {
            server: "node_repl",
            tool: "turn_ended",
        },
    },
    BundledHook {
        plugin_id: "computer-use@codex-bundled",
        events: &[
            HookEventName::Stop,
            HookEventName::Interrupt,
            HookEventName::SubagentStop,
        ],
        target: BundledHookTarget::McpServer {
            server: "node_repl",
            tool: "turn_ended",
        },
    },
    BundledHook {
        plugin_id: "unified-computer-use@codex-bundled",
        events: &[
            HookEventName::Stop,
            HookEventName::Interrupt,
            HookEventName::SubagentStop,
        ],
        target: BundledHookTarget::McpServer {
            server: "cua_repl",
            tool: "turn_ended",
        },
    },
];

/// Matches a temporary unsigned cleanup exception.
pub fn is_allowlisted_bundled_cleanup_hook(
    plugin_id: &str,
    event: HookEventName,
    matcher: Option<&str>,
    handler: &HookHandlerConfig,
) -> bool {
    let HookHandlerConfig::McpTool { server, tool, .. } = handler else {
        return false;
    };

    matcher.is_none()
        && ALLOWLISTED_BUNDLED_HOOKS.iter().any(|hook| {
            hook.plugin_id == plugin_id
                && hook.events.contains(&event)
                && match hook.target {
                    BundledHookTarget::McpServer {
                        server: expected_server,
                        tool: expected_tool,
                    } => server == expected_server && tool == expected_tool,
                }
        })
}

# CLI-facing crate index

| Group | Crate/package | Responsibility |
| --- | --- | --- |
| Entrypoint | `codex-cli` | root clap grammar, dispatch, command-specific CLI helpers |
| Interactive UI | `codex-tui` | TUI args, event loop, session startup, rendering, interactive output |
| Non-interactive UI | `codex-exec` | exec/review args, request loop, event processing, JSONL/final output |
| Agent runtime | `codex-core` | config runtime, threads, turns, tools, approvals, policy integration |
| Shared CLI args | `codex-utils-cli` | shared option structs and override parsing |
| Config model | `codex-config` | config layer/types, diagnostics, thread config interfaces |
| Auth | `codex-login`, `codex-keyring-store` | credential loading, login/logout, refresh, storage |
| Provider/model | `codex-model-provider`, `codex-models-manager` | provider capability/configuration and model catalog |
| API boundary | `codex-api`, `codex-client` | request/response streams, provider endpoints, retries, transport telemetry |
| Protocol | `codex-protocol`, `codex-app-server-protocol` | domain and wire types, schema generation, experimental markers |
| Server | `codex-app-server`, `codex-app-server-transport`, `codex-app-server-client` | request processing, transports, client/server lifecycle |
| Tools | `codex-tools` | generic tool invocation/output traits and types |
| Policy/sandbox | `codex-execpolicy`, `codex-sandboxing`, platform sandbox crates | rules, approval/sandbox primitives, host enforcement |
| Persistence | `codex-thread-store`, `codex-rollout`, `codex-history`, `codex-state` | thread/queue stores, rollout/history, local state |
| Extensibility | `codex-mcp`, `codex-rmcp-client`, `codex-plugin`, `codex-core-plugins` | MCP and plugin config/runtime integration |
| Platform/process | `codex-arg0`, `codex-install-context`, `codex-terminal-detection`, `codex-process-hardening` | launch context, installation, terminal, process safety |

The package name and Rust crate name can differ; Cargo manifests are authoritative for that mapping. See [system map](../00-system-map.md) for runtime ownership and the [Trait index](trait-index.md) for interface-level ownership.

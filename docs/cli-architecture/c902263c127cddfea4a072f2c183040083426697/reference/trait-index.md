# Trait index

This index lists Traits that explain a CLI capability boundary or a shared runtime contract. Visibility is recorded from the defining declaration; a public re-export does not change the defining crate.

## API and provider boundaries

### `AuthProvider`

- Definition: `codex-api`, [`src/auth.rs:30`](../../../../crates/codex-api/src/auth.rs#L30)
- Visibility: `pub`
- Role: async request-time auth header provider consumed by API clients.
- Related users: `codex-login`, `codex-model-provider`, `codex-api` endpoints.

### `ModelProvider`

- Definition: `codex-model-provider`, [`src/provider.rs:123`](../../../../crates/model-provider/src/provider.rs#L123)
- Visibility: `pub`
- Role: provider capabilities/account state/recovery boundary used by model selection and core.

### `RequestTelemetry`

- Definition: `codex-client`, [`src/telemetry.rs:6`](../../../../crates/codex-client/src/telemetry.rs#L6)
- Visibility: `pub`
- Role: per-request transport observation hook.

### `SseTelemetry`

- Definition: `codex-api`, [`src/telemetry.rs:18`](../../../../crates/codex-api/src/telemetry.rs#L18)
- Visibility: `pub`
- Role: SSE polling observation hook.

### `WebsocketTelemetry`

- Definition: `codex-api`, [`src/telemetry.rs:35`](../../../../crates/codex-api/src/telemetry.rs#L35)
- Visibility: `pub`
- Role: WebSocket request/event observation hook.

## Configuration and persistence

### `ThreadConfigLoader`

- Definition: `codex-config`, [`src/thread_config.rs:86`](../../../../crates/config/src/thread_config.rs#L86)
- Visibility: `pub`
- Role: thread-scoped config loading boundary that keeps app-server/core independent from a concrete config source.

### `ThreadStore`

- Definition: `codex-thread-store`, [`src/store.rs:88`](../../../../crates/thread-store/src/store.rs#L88)
- Visibility: `pub`
- Role: persisted thread metadata/history/store boundary used by session lifecycle paths.

### `QueueStore`

- Definition: `codex-thread-store`, [`src/queue_store.rs:14`](../../../../crates/thread-store/src/queue_store.rs#L14)
- Visibility: `pub`
- Role: queued messages for existing sessions.

### `SessionTask`

- Definition: `codex-core`, [`src/tasks/mod.rs:176`](../../../../crates/core/src/tasks/mod.rs#L176)
- Visibility: `pub(crate)`
- Role: internal lifecycle contract for session work scheduled on background Tokio tasks.

## Tool and policy boundaries

### `ToolExecutor`

- Definition: `codex-tools`, [`src/tool_executor.rs:106`](../../../../crates/tools/src/tool_executor.rs#L106)
- Visibility: `pub`
- Role: generic invocation-to-tool-output handler boundary.

### `CoreToolRuntime`

- Definition: `codex-core`, [`src/tools/registry.rs:54`](../../../../crates/core/src/tools/registry.rs#L54)
- Visibility: `pub(crate)`
- Role: core registry's runtime view over a `ToolExecutor` implementation.

### `Approvable`

- Definition: `codex-core`, [`src/tools/sandboxing.rs:309`](../../../../crates/core/src/tools/sandboxing.rs#L309)
- Visibility: `pub(crate)`
- Role: tool runtime contract for approval decisions/review keys.

### `Sandboxable`

- Definition: `codex-core`, [`src/tools/sandboxing.rs:343`](../../../../crates/core/src/tools/sandboxing.rs#L343)
- Visibility: `pub(crate)`
- Role: tool runtime contract for sandbox preference/configuration.

### `ToolRuntime`

- Definition: `codex-core`, [`src/tools/sandboxing.rs:364`](../../../../crates/core/src/tools/sandboxing.rs#L364)
- Visibility: `pub(crate)`
- Role: combines approval and sandbox contracts for concrete tool runtimes.

### `Rule`

- Definition: `codex-execpolicy`, [`src/rule.rs:214`](../../../../crates/execpolicy/src/rule.rs#L214)
- Visibility: `pub`
- Role: executable rule matching abstraction used by policy evaluation.

## UI and command output boundaries

### `EventSource`

- Definition: `codex-tui`, [`src/tui/event_stream.rs:45`](../../../../crates/tui/src/tui/event_stream.rs#L45)
- Visibility: `pub`
- Role: terminal/event source abstraction consumed by the TUI event broker.

### `EventProcessor`

- Definition: `codex-exec`, [`src/event_processor.rs:13`](../../../../crates/exec/src/event_processor.rs#L13)
- Visibility: `pub(crate)`
- Role: internal conversion of response events/status into non-interactive output behavior.

### `ToolOutput`

- Definition: `codex-tools`, [`src/tool_output.rs:12`](../../../../crates/tools/src/tool_output.rs#L12)
- Visibility: `pub`
- Role: tool result/output boundary.

### `TurnItemEmitter`

- Definition: `codex-tools`, [`src/tool_call.rs:56`](../../../../crates/tools/src/tool_call.rs#L56)
- Visibility: `pub`
- Role: emits tool/turn items back into conversation/event handling.

## Protocol and authentication integration

### `ExperimentalApi`

- Definition: `codex-app-server-protocol`, [`src/experimental_api.rs:5`](../../../../crates/app-server-protocol/src/experimental_api.rs#L5)
- Visibility: `pub`
- Role: marker for protocol types/methods that signal experimental usage in schema/API generation.

### `AuthManagerConfig`

- Definition: `codex-login`, [`src/auth/manager.rs:1223`](../../../../crates/login/src/auth/manager.rs#L1223)
- Visibility: `pub`
- Role: keeps login manager configuration access independent from `codex-core::Config`.

## Exclusions

Rendering-only Traits such as `Renderable` and unrelated test-only helpers are not listed because they do not explain a CLI capability boundary. They remain valid internal interfaces; exclusion here is a documentation scope decision, not a claim that they do not exist.

# Transport and server boundaries

## Local and remote shapes

The CLI can reach runtime behavior through:

- direct TUI/core startup;
- a local app-server daemon;
- explicit remote app-server endpoint;
- app-server stdio, Unix socket, or WebSocket transport;
- exec-server or internal stdio-to-UDS plumbing.

`codex-cli` decides the command and endpoint. `codex-app-server` owns request processing; `codex-app-server-transport` owns connection/transport mechanics; `codex-app-server-protocol` owns serializable protocol types. See [app-server command](../commands/app-server.md).

## Protocol boundary

Protocol requests/notifications are typed values. Transport moves bytes/messages; server processors turn requests into core/session operations. Do not infer runtime ownership from a protocol type's crate.

## Lifecycle

Daemon lifecycle commands can install, start, restart, stop, update, or query a managed package. Proxy commands forward bytes to an existing control socket. These are operational paths and can affect running work; they are different from a single foreground request's connection lifetime.

Relevant Traits include [`RequestTelemetry`](../reference/trait-index.md#requesttelemetry) for client observations and [`EventSource`](../reference/trait-index.md#eventsource) for TUI input, while most server boundaries are concrete request/processor types rather than public Traits.

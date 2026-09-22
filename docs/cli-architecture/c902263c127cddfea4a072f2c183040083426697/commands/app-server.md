# `codex app-server`

## Entry modes

`AppServerCommand` is defined in [`cli/src/main.rs`](../../../../crates/cli/src/main.rs#L473). Without a nested subcommand, `codex app-server` starts the app-server runtime. Nested operations are:

- `daemon bootstrap|start|restart|update|stop|version`;
- `proxy`;
- `generate-ts`;
- `generate-json-schema`;
- hidden `generate-internal-json-schema`.

The root dispatch starts the runtime or calls the selected tool at [main.rs#L1119](../../../../crates/cli/src/main.rs#L1119).

## Foreground runtime

The CLI translates `--listen`/`--stdio`, strict-config, metrics default, auth, and code-mode-host options into `codex_app_server::run_main` at [`app-server/src/lib.rs#L417`](../../../../crates/app-server/src/lib.rs#L417). The transport-aware entry point is `run_main_with_transport_options` at [L473](../../../../crates/app-server/src/lib.rs#L473). It loads config, creates runtime paths, initializes the request/notification channels, and starts request processing.

Supported transport shapes in the CLI are `stdio://`, Unix sockets, WebSocket URLs, and `off`, subject to the command's platform/runtime validation.

## Daemon/proxy/schema tools

- Daemon commands manage a durable local app-server package and may restart or interrupt work.
- `proxy` forwards stdio bytes to the control Unix socket; it does not process protocol semantics itself.
- Schema generation calls `codex-app-server-protocol` generators and writes TypeScript or JSON Schema artifacts.
- Internal schema generation is tooling-facing and hidden from ordinary help.

See [transport/server](../common/transport-and-server.md), [protocol ownership](../reference/crate-index.md), [configuration](../common/configuration.md), and [telemetry](../common/telemetry.md).

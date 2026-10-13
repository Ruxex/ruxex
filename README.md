# Genex

Genex is a general-purpose AI agent foundation written in Rust. It provides
the agent loop, tools, MCP integration, sandboxing, persistence, and an
AppServer boundary without coupling the core runtime to a particular model
provider.

The repository is currently an unreleased workspace. The crates under
`crates/` are intended to be composed by a host binary or another workspace
crate rather than consumed as a published, versioned SDK.

## Architecture

Genex uses one concrete model provider per runtime/AppServer:

| Crate | Responsibility |
| --- | --- |
| `genex-model-provider` | Provider-neutral request, response, stream, error, capability, authentication, usage, and data-root contracts. |
| `genex-providers` | Built-in provider adapters and provider-specific HTTP, wire payload, credentials, and model-catalog behavior. Cloudflare lives under `genex_providers::cloudflare`. |
| `genex-app-server` | The shared runtime boundary. `AppServerRuntimeBuilder::new(SharedModelProvider)` accepts the provider selected by the host. |
| `genex-core` | Provider-neutral agent, session, thread, tool, and turn execution. |

Provider selection is a host-startup concern. Threads and rollouts do not
select providers, and a running runtime does not switch providers. To use a
different provider, construct another runtime/AppServer with another provider
and another provider-scoped data root.

## Run the provider-owned sample host

Install the Rust toolchain selected by `rust-toolchain.toml`, then build the
provider-owned sample host. The repository supports Linux and macOS. Windows
users should run Genex in WSL2.

```sh
cargo build -p genex-provider-host-example
cargo run -p genex-provider-host-example -- "explain this codebase to me"
```

There is no fixed-provider `genex` CLI binary. A host chooses its provider at
compile time; the sample host uses `CloudflareModelProvider`. Its default data
root is `~/.genex`; set `GENEX_HOME` to use an existing directory instead.

## Embed Genex in a Rust host

Add the workspace crates needed by the host:

```toml
[dependencies]
genex-app-server = { path = "crates/app-server" }
genex-http-client = { path = "crates/http-client" }
genex-model-provider = { path = "crates/model-provider" }
genex-model-provider-info = { path = "crates/model-provider-info" }
genex-providers = { path = "crates/providers" }
```

The built-in Cloudflare adapter can be constructed and injected like this:

```rust
use genex_app_server::runtime::AppServerRuntimeBuilder;
use genex_http_client::{HttpClientFactory, OutboundProxyPolicy};
use genex_model_provider_info::ModelProviderInfo;
use genex_providers::create_cloudflare_provider;

let provider = create_cloudflare_provider(
    Some("cloudflare".to_owned()),
    ModelProviderInfo::create_cloudflare_provider(),
    credential_manager,
    HttpClientFactory::new(OutboundProxyPolicy::ReqwestDefault),
);
let runtime = AppServerRuntimeBuilder::new(provider).build();
```

The configured-provider convenience constructor is available when the host
already uses `ModelProviderInfo`:

```rust
let runtime = AppServerRuntimeBuilder::from_provider_info(
    provider_info,
    credential_manager,
)
.build();
```

Hosts that construct a provider themselves should mount its data root before
opening credentials, SQLite, sessions, or caches:

```rust
use genex_app_server::runtime::mount_provider_home_for_runtime;

mount_provider_home_for_runtime(&config, &runtime)?;
```

The mount writes or validates `genex-home.json`. A marker for a different
provider or incompatible provider protocol is rejected before sensitive state
is opened.

## Host-owned daemon startup

A host that wants the default daemon behavior can compile the provider type
into both its foreground code and its AppServer entrypoint. The daemon process
reconstructs the provider; the foreground process never sends a provider
instance over the socket:

```rust
use genex_app_server::{ModelProviderFactory, ProviderDescriptor, ProviderInit};

impl ModelProviderFactory for MyProvider {
    fn provider_descriptor() -> ProviderDescriptor {
        ProviderDescriptor::new("my-provider", "1")
    }

    fn new(init: ProviderInit) -> Self {
        MyProvider::from_startup(init)
    }
}

// The host's hidden daemon mode calls this in the child process.
genex_app_server::run_main_with_transport_options_and_provider::<MyProvider>(
    arg0_paths,
    cli_config_overrides,
    loader_overrides,
    strict_config,
    false,
    genex_app_server::AppServerTransport::UnixSocket { socket_path },
    session_source,
    genex_app_server::AppServerWebsocketAuthSettings::default(),
    genex_app_server::AppServerRuntimeOptions::default(),
).await?;
```

The host can use `genex_app_server_daemon::run_with_host_executable` to start
that hidden mode in the same executable. Only serializable startup arguments
cross the process boundary; `P::new(ProviderInit)` runs inside the AppServer
daemon. The standalone `genex-app-server` binary remains available as a
compatibility and operational entrypoint, but it is not required by a
provider-owned host.

For a complete TUI host, call `genex_tui::run_main_with_provider::<P>(...)` in
the foreground branch and dispatch `APP_SERVER_DAEMON_MODE_ARG` to
`run_host_app_server_daemon::<P>(...)` before normal argument parsing in the
child branch. The runnable
[`examples/provider-host`](examples/provider-host/src/main.rs) crate shows this
layout with `CloudflareModelProvider`.

## Implement a custom provider

Provider implementations do not need to be registered in Genex. Implement the
public `genex_model_provider::ModelProvider` trait in the host or in another
workspace crate, store it as `SharedModelProvider`, and inject it into the
runtime:

```rust
use std::sync::Arc;

use genex_app_server::runtime::AppServerRuntimeBuilder;
use genex_model_provider::SharedModelProvider;

// MyProvider implements ModelProvider and owns its provider-specific client,
// credentials, wire payloads, and configuration.
let provider: SharedModelProvider = Arc::new(MyProvider::new(/* ... */));
let runtime = AppServerRuntimeBuilder::new(provider).build();
```

The contract keeps provider-specific details behind the implementation. The
host receives neutral chat/model-catalog/auth/usage operations and explicit
`Unsupported` results for capabilities the provider does not implement.
See [`genex-model-provider/src/contract.rs`](crates/model-provider/src/contract.rs)
for the complete trait and neutral types.

## Provider data and credentials

- Default data root: `~/.genex`.
- Runtime override: `GENEX_HOME`.
- Build-time default override: `GENEX_DEFAULT_HOME_DIR_NAME`.
- Provider marker: `genex-home.json` in the data root.
- Credential values are owned by the provider/runtime lifecycle and are not
  persisted in threads or rollouts.
- Credential changes affect new turns; an in-flight turn retains its
  start-time credential snapshot.

Use a separate runtime and data root when running different providers. This
prevents credentials, caches, sessions, and provider-specific state from being
accidentally shared across incompatible provider implementations.

## External provider contracts

The provider-neutral layer does not rewrite the external model API contract.
The existing OpenAI endpoint URLs, request/response fields, authentication
headers and environment variables, and model identifiers remain unchanged
where they are required by the endpoint.

Provider-specific HTTP and wire serialization belong in the adapter crate;
agent, MCP, sandbox, security, session, and thread code should depend only on
the neutral contract.

## Repository layout

- `Cargo.toml`, `Cargo.lock`: workspace definition and locked dependencies.
- `crates/`: Rust libraries, host-support APIs, AppServer, provider contracts,
  and built-in adapters.
- `docs/`: architecture, compatibility, provider-boundary, and generated
  protocol documentation.
- `scripts/`, `tools/`: development and generation helpers.
- `third_party/`: vendored native sources and metadata.

Standalone TypeScript/Python SDKs, package publishing, GitHub Actions, and
Bazel build infrastructure are not maintained in this fork. AppServer JSON and
TypeScript schemas remain under `crates/app-server-protocol/schema/`.

## Development

The repository uses `just` recipes for checks and generated artifacts:

```sh
just check
just check --tests
just generate
```

`just check --tests` compiles test targets without executing them. Use
`just test -p <crate>` when an actual test run is specifically needed.

For provider-boundary details, see
[`docs/provider-specific-crates.md`](docs/provider-specific-crates.md) and the
single-provider design in
[`docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`](docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md).

# Ruxex

Ruxex is a provider-neutral AI agent framework for building applications with
an App Server, CLI/TUI, tools, MCP, sandboxing, and persistence.

Ruxex started as a fork of OpenAI's Codex and is developed as an independent,
provider-neutral Rust library for custom hosts and model providers. See
[Attribution and notices](#attribution-and-notices) for provenance details.

## What Ruxex gives you

Ruxex provides the common building blocks of an AI agent, including model
conversations, threads and rollouts, tool execution, MCP integration,
sandboxing, persistence, and an embeddable App Server.

The application host chooses one model provider when it starts a runtime. A
thread or rollout does not choose a provider, and a running runtime does not
switch providers. This keeps provider credentials, HTTP details, and wire
formats out of the agent and session layers.

## Use Ruxex in your application

Ruxex provides an App Server and a CLI with an interactive TUI. A host
application supplies three independent pieces:

- [`ModelProvider`](MODEL-PROVIDER.md) for LLM endpoints, authentication,
  models, and wire formats;
- [`TuiBrandProfile`](TUI-BRAND.md) for terminal presentation; and
- [`ProductIdentity`](PRODUCT-IDENTITY.md) for the application's name,
  identity, data home, and App Server product metadata.

This keeps provider-specific HTTP details, product identity, and TUI
presentation out of threads, rollouts, and the shared runtime.

Ruxex is currently distributed from Git rather than crates.io. This is
intentional: the current implementation depends on Ruxex-maintained revisions
of some upstream crates, including proxy support that is not available from
the corresponding crates.io releases. The crates.io package name is reserved
by a [placeholder](placeholder/README.md); do not use a registry-only
dependency such as `ruxex = "0.1"`.

Add the facade crate to your application and pin it to a reviewed commit or
release tag:

```toml
[dependencies]
ruxex = {
    git = "https://github.com/Ruxex/ruxex.git",
    tag = "v0.1.0"
}
```

The default `full` feature preserves the App Server, CLI, and TUI facade. A
host that embeds the App Server without the terminal UI can opt out of the
default integration features:

```toml
[dependencies]
ruxex = {
    git = "https://github.com/Ruxex/ruxex.git",
    tag = "v0.2.0",
    default-features = false,
    features = ["app-server"]
}
```

The `cli` feature implies `app-server` and `tui`, because the current CLI
implementation uses the TUI integration. The provider-neutral contract and
protocol modules remain available without these facade features.

The CLI/TUI entry point accepts all three host-defined pieces. The CLI invokes
the provider factory and constructs the App Server internally:

```rust
use std::sync::Arc;

use ruxex::{
    AppServerRuntimeBuilder, ModelProviderFactory, ProductIdentity,
    ProviderRuntimeFactory, TuiBrandProfile,
};

// MyProvider implements ModelProvider and ModelProviderFactory.
// Implement the provider's LLM endpoint, credentials, and wire format here.
// MyTuiBrandProfile implements TuiBrandProfile.
let tui_brand: Arc<dyn TuiBrandProfile> = Arc::new(MyTuiBrandProfile);
let provider_factory: ProviderRuntimeFactory = (
    AppServerRuntimeBuilder::build_from_provider_factory::<MyProvider>,
    MyProvider::provider_descriptor(),
);
let product_identity = ProductIdentity::new(
    "Example Agent",
    "example-agent",
    ".example-agent",
    "example-home.json",
)?;

ruxex::run_with_provider_factory_and_identity(
    arg0_paths,
    provider_factory,
    product_identity,
    Some(Arc::new(MyTuiBrandProfile)),
)
.await?;
```

When the default product identity is sufficient, the shorter
`ruxex::run::<MyProvider>(arg0_paths, brand_profile)` entry point is also
available.

See [`crates/model-provider/src/contract.rs`](crates/model-provider/src/contract.rs)
for the provider contract and the linked documents above for the three host
integration points. If you embed the App Server without the CLI, you can use
its runtime builder directly.

Rusty V8 setup is required only for hosts that enable JavaScript code mode;
see [Rusty V8 setup](RUSTY-V8.md).

## Data, credentials, and product identity

By default, a Ruxex application stores data under `~/.ruxex`. A host can use
`RUXEX_HOME` or, preferably for a separately branded product, inject a
[`ProductIdentity`](PRODUCT-IDENTITY.md) with its own data directory and
marker filename. Provider credentials belong to the provider/runtime
lifecycle and are not stored in threads or rollouts.

Ruxex `0.1.0` is pre-1.0 and its APIs may evolve. Repository development
instructions are kept separate from this library-user guide.

## Attribution and notices

Ruxex is an independent derivative work based on OpenAI's Codex project. The
fork was created from Codex commit
[`e269f2164cbb9f499e4f22301c393500e2a831f3`](https://github.com/openai/codex/commit/e269f2164cbb9f499e4f22301c393500e2a831f3).
We are grateful to the Codex maintainers and contributors for the work on
which Ruxex builds. See [`NOTICE`](NOTICE) for fork provenance and third-party
attributions, and [`LICENSE`](LICENSE) for the applicable license terms.

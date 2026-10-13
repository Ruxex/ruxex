# Provider Credential Boundary Design

## Status

Implementation design for completing the provider-neutral boundary after the single-provider
runtime migration. The design covers provider authentication, Plugin marketplace credentials, and
MCP OAuth credentials as separate concerns.

## Goal

Make an injected `ModelProvider` the only owner of model-provider authentication while removing
`CredentialManager`, `GenexAuth`, and `ModelProviderInfo` from Core's provider-execution APIs. Keep
Plugin and MCP credentials available through narrowly scoped, provider-neutral interfaces.

The completion gate is:

- a custom `ModelProvider` can be injected without constructing a repository-specific
  `CredentialManager`;
- Core starts turns, obtains an opaque provider request snapshot, and never interprets or persists
  provider credentials;
- Provider login, logout, usage, model discovery, and auth recovery use the same injected provider;
- Plugin marketplace access uses a scoped generic credential broker rather than provider auth;
- MCP OAuth continues to use its own scoped OAuth store and does not depend on model-provider auth;
- in-flight turns keep their admission-time provider credential snapshot;
- new turns after provider logout are rejected;
- AppServer initialization and provider-home identity are derived from the injected runtime;
- no thread, rollout, history, or session-log record contains provider routing or credentials.

## Non-goals

- Do not change OpenAI endpoint URLs, request fields, authentication headers or environment names,
  or model identifiers.
- Do not redesign MCP OAuth protocol behavior or its keyring/file storage format.
- Do not make Plugin marketplace credentials part of the `ModelProvider` trait.
- Do not introduce a multi-provider registry or runtime provider switching.
- Do not remove security behavior, sandbox behavior, or provider-neutral agent capabilities.

## Target architecture

```text
Host / CLI / AppServer
  ├─ concrete ModelProvider adapter
  │    ├─ provider endpoint and wire format
  │    ├─ provider credential storage and refresh
  │    ├─ provider auth lifecycle
  │    └─ opaque ProviderRequestSnapshot
  ├─ PluginCredentialBroker
  │    └─ marketplace/plugin-scoped credentials
  └─ MCP OAuth store/runtime
       └─ MCP-server-scoped OAuth credentials

AppServerRuntime
  ├─ SharedModelProvider
  ├─ optional PluginCredentialBroker
  └─ existing MCP runtime dependencies

Core production code
  ├─ SharedModelProvider
  ├─ Provider-neutral model catalog inputs
  └─ scoped plugin/MCP credential interfaces where required
```

The runtime owns the provider handle. The provider adapter owns provider authentication. Core may
carry an opaque `SharedProviderCredentialSnapshot` through a turn but cannot downcast, serialize,
log, or use it as a routing selector.

## Contracts

### Provider authentication

Keep authentication lifecycle operations on `ModelProvider`:

```rust
pub trait ModelProvider: fmt::Debug + Send + Sync + 'static {
    fn descriptor(&self) -> ProviderDescriptor;
    fn capabilities(&self) -> ProviderCapabilities;
    fn auth_state(&self) -> ProviderAuthState;
    fn request_snapshot(&self) -> ProviderFuture<'_, ProviderResult<ProviderRequestSnapshot>>;
    fn login(&self, credentials: BTreeMap<String, String>)
        -> ProviderFuture<'_, ProviderResult<()>>;
    fn logout(&self) -> ProviderFuture<'_, ProviderResult<()>>;
    fn usage(&self, request: UsageRequest)
        -> ProviderFuture<'_, ProviderResult<ProviderUsage>>;
    fn chat(&self, request: ChatRequest)
        -> ProviderFuture<'_, ProviderResult<ProviderChatStream>>;
}
```

The concrete adapter may use `CredentialManager` internally during the transition, but that type
must not appear in Core, the public provider contract, or `AppServerRuntime::new` callers. The
configured OpenAI-compatible adapter remains a Host construction detail.

### Plugin marketplace credentials

Add a small provider-neutral credential crate with an opaque, scoped broker. It must not depend on
`genex-login`, `ModelProviderInfo`, `genex_api::Provider`, or HTTP implementation types.

```rust
pub struct CredentialRequest {
    pub owner: CredentialOwner,
    pub purpose: CredentialPurpose,
}

pub enum CredentialOwner {
    Plugin(String),
    Marketplace(String),
}

pub enum CredentialPurpose {
    MarketplaceRequest,
    PluginResource,
}

pub trait CredentialBroker: Send + Sync {
    fn resolve(
        &self,
        request: CredentialRequest,
    ) -> CredentialFuture<'_, CredentialResult<CredentialSnapshot>>;
}

pub struct CredentialSnapshot {
    // Provider-neutral opaque value. No Debug, Serialize, or Clone implementation that exposes it.
}
```

The initial implementation may adapt the existing local credential/keyring implementation, but
the adapter is owned by the Host/Plugin layer. Core-plugins receives `Arc<dyn CredentialBroker>`
and only requests credentials for the specific Plugin or marketplace owner. It must not use the
model provider's auth mode to decide whether a Plugin credential is available.

### MCP credentials

MCP OAuth remains an MCP-owned lifecycle. `genex-mcp` owns the OAuth credential name, keyring/file
store, refresh policy, and authentication status. The AppServer/Core path may receive a narrow MCP
credential resolver or an already-built MCP runtime, but must not pass the model provider's
`CredentialManager` into MCP. Bearer environment variables remain explicit MCP configuration and
are resolved only for the configured MCP server.

### Model catalog

`genex-models-manager` must not accept `Arc<CredentialManager>`. Model availability and discovery
are provider operations. The model catalog source calls `ModelProvider::list_models`; any provider
auth decision is made by the provider and represented by `ProviderError` or `ProviderUsage`/
`ProviderAuthState`. Static catalog filtering may receive a small neutral auth/capability snapshot,
but never a concrete auth manager.

## Runtime and Core API changes

`AppServerRuntime` becomes the shared owner of the injected provider and optional Plugin broker:

```rust
pub struct AppServerRuntime {
    provider: SharedModelProvider,
    plugin_credentials: Option<Arc<dyn CredentialBroker>>,
}
```

`AppServerRuntimeBuilder::new(provider)` must be sufficient for a custom provider. It must not
load credentials from `Config` as a fallback. Configured repository adapters use a separate Host
constructor that creates the concrete provider and its private auth state before calling `new`.

`ThreadManager`, `Session`, `SessionServices`, `ModelClient`, and model-catalog construction no
longer accept or store `CredentialManager`. Provider request admission uses only the injected
provider's `request_snapshot`. Plugin and MCP dependencies are passed only to the services that
need them, not through provider-execution arguments.

`Config` retains model selection and neutral runtime settings, but `model_provider` and
`model_provider_id` are not read by Core for routing or authentication. Host-side configuration may
continue to use `ModelProviderInfo` while constructing the configured adapter.

## Data flow

### New turn

1. AppServer selects the one provider from `AppServerRuntime`.
2. Core asks that provider for `ProviderRequestSnapshot` at turn admission.
3. If the provider reports unauthenticated, Core rejects the new turn with a neutral auth error.
4. Core places the opaque snapshot into `ChatTransportOptions`.
5. The provider adapter resolves the snapshot internally and builds the external request.
6. The snapshot is released after the turn and is never written to rollout, history, SQLite, or
   telemetry.

### Provider login/logout

1. AppServer account requests call the runtime provider directly.
2. The provider updates its own shared credential state and broadcasts its auth revision/state.
3. New turns observe the new state; in-flight turns retain their original snapshot.
4. Provider-specific auth recovery remains inside the provider adapter.

### Plugin marketplace request

1. The Plugin layer builds a `CredentialRequest` with the marketplace/plugin owner and purpose.
2. The scoped broker returns an opaque snapshot or `Unsupported`/`Unauthenticated`.
3. The Plugin layer creates the outbound request; Core never receives model-provider auth.

### MCP request

1. MCP resolves the configured server's auth mode and OAuth credential name.
2. MCP's own store/refresh runtime supplies the server-scoped credential.
3. MCP applies the credential only to that server connection and never consults model-provider auth.

## Migration phases

1. Add neutral credential contracts and tests; keep compatibility adapters at Host boundaries.
2. Move configured provider credential construction behind the provider adapter and make
   `AppServerRuntimeBuilder::new(provider)` self-sufficient.
3. Remove `CredentialManager` from models-manager and replace auth filtering with provider-neutral
   catalog/auth metadata.
4. Remove `CredentialManager` from ThreadManager, Session, turn context, and Core model execution.
5. Inject the scoped Plugin credential broker only into `core-plugins`/marketplace services.
6. Make MCP auth dependencies explicit and independent from provider auth.
7. Replace AppServer/user-verification/CLI provider-auth consumers with provider auth-state or
   provider lifecycle calls.
8. Make all initialization identity and provider-home validation use the injected runtime.
9. Audit and remove provider credential/routing persistence and update the compatibility inventory.
10. Run generated-schema checks where protocol types change, then run the required workspace checks.

Each phase must compile independently and record producer/consumer decisions in
`docs/pre-release-compatibility-inventory.md` before removing a repository-owned compatibility
field or API.

## Verification

Required source-level invariants:

- `rg` finds no production `CredentialManager`, `GenexAuth`, or `ModelProviderInfo` dependency in
  Core model execution/session APIs.
- `rg` finds no provider credential field in thread, rollout, history, SQLite, or session-log
  writers.
- An injected fake Provider with a descriptor different from config initializes successfully and
  serves `modelProvider/read` from the injected descriptor.
- A fake Provider can implement login/logout/auth snapshot without `CredentialManager`.
- A Plugin broker cannot resolve a credential for another Plugin or marketplace owner.
- MCP auth tests continue to pass without a model-provider credential manager.

Required commands follow `AGENTS.md`:

```sh
systemd-run --user --scope --collect --quiet \
  -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
  --setenv=CARGO_BUILD_JOBS=2 just check

systemd-run --user --scope --collect --quiet \
  -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G \
  --setenv=CARGO_BUILD_JOBS=2 just check --tests
```


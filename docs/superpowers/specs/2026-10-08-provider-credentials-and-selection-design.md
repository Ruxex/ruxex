# Provider Credentials and Selection Design (Historical / Retired)

> **Historical note (2026-10-09):** This design described multi-provider
> credentials and active-provider selection. It is retained for historical
> context only; the implemented Genex architecture mounts one built-in
> provider per runtime/AppServer and does not support provider switching or
> per-thread provider metadata. See the current single-provider design for
> the active contract.

## Goal

Replace the ChatGPT-era assumption that application login, model-provider
authentication, and MCP authentication are one credential domain.

Genex has no separate application account. The application manages credentials
for one or more model providers, selects one active provider for model
operations, and manages MCP credentials independently.

## Terminology

- `CredentialManager`: provider-scoped credential persistence and state
  management. This is the conceptual successor to the useful parts of
  `CredentialManager`.
- `ModelProvider`: a configured model backend. It resolves provider-specific
  credentials and exposes model, account, and usage capabilities.
- `active provider`: the model provider currently selected for chat and other
  model-backed operations.
- `McpCredentialManager`: MCP-server-scoped credential persistence and OAuth
  management. It is independent from model-provider credentials.

## Boundaries

### CredentialManager

`CredentialManager` owns:

- provider-scoped credential records;
- persistence through `auth.json`, keyring, and ephemeral stores;
- credential refresh and local deletion;
- active-provider selection;
- authentication-state change notifications used by runtimes and sessions.

It does not own a Genex account identity, provider-specific API semantics, model
requests, MCP OAuth, or MCP connection configuration.

The persisted model is conceptually:

```text
active_provider: Option<ProviderId>
credentials: Map<ProviderId, ProviderCredentialRecord>
```

Raw secrets should continue to use the existing secret-storage policy. The
configuration record may contain a credential reference, authentication mode,
and non-secret metadata, but should not require raw secrets in the JSON file.

### ModelProvider

`ModelProvider` consumes `CredentialManager` records through a provider-specific
credential resolver. Provider implementations translate their own credentials
into API keys, bearer tokens, headers, account IDs, or other request context.

Provider capabilities may include:

- credential validation;
- account information;
- usage, quota, or billing information;
- model listing;
- chat or responses execution.

Account and usage information are provider capabilities, not a global Genex
account. A provider may support only a subset of these capabilities. The UI and
protocol should represent unsupported information explicitly rather than
assuming every API-key provider has the same account model.

### MCP

MCP credentials are managed independently per MCP server. MCP owns:

- configured headers and bearer tokens;
- environment-backed MCP credentials;
- MCP OAuth credentials and refresh state;
- MCP connection identity and reconnect behavior.

MCP must not implicitly receive the active model provider's credentials or
`GenexAuth`. If a future MCP integration needs to use a model-provider
credential, that relationship must be explicit in MCP configuration and treated
as a separate integration feature.

MCP runtime invalidation should continue to react to changes in MCP credentials
and configuration, without depending on unrelated model-provider changes.

## Operations

The user-facing operations have distinct semantics:

```text
login(provider, credential)
  persist or update provider credentials
  validate when supported
  select the provider as active

switch(provider)
  select a provider whose credentials are already available
  do not mutate or delete credentials

logout()
  clear the active provider
  retain stored provider credentials

delete(provider)
  delete the provider's stored credentials
  clear active selection if it names that provider
```

`switch` should reject or guide the user toward `login` when no usable
credential exists. `logout` is intentionally not credential deletion. Remote
OAuth revocation, if needed later, should be an explicit provider operation and
not an accidental side effect of clearing the active provider.

## Runtime data flow

Model operations follow this path:

```text
active_provider
  -> CredentialManager
  -> provider credential resolver
  -> ModelProvider
  -> list_models / chat / provider-backed extensions
```

Provider-backed Web Search or History APIs belong on this path when they use the
same model-provider backend. They must not depend on a separate application
login concept.

MCP operations follow a separate path:

```text
MCP configuration
  -> McpCredentialManager
  -> MCP runtime
  -> MCP server connection
```

## Account and usage presentation

The app-server and CLI should report the active model provider's state rather
than a global Genex account. A status response may include:

- provider identifier and display name;
- active/inactive state;
- credential configured/validated/invalid state;
- provider account identifier or display name when available;
- usage or quota information when supported;
- a capability or unavailable state when the provider does not expose it.

When no provider is active, the application should report that model access is
not selected instead of presenting an unauthenticated Genex account.

## Migration order

1. Audit and remove implicit model-auth propagation into MCP; preserve MCP's
   own OAuth and credential behavior.
2. Introduce provider-scoped credential records and active-provider state while
   preserving existing persistence backends.
3. Narrow `CredentialManager` toward `CredentialManager` responsibilities and remove
   ChatGPT-specific managed-auth terminology.
4. Move provider validation, account information, and usage retrieval behind
   `ModelProvider` capabilities.
5. Update app-server, CLI, and TUI login/status operations to be provider
   scoped; retain `login`, add `switch`, make `logout` deactivate only, and add
   explicit `delete`.
6. Remove obsolete global-account assumptions and clean up unused auth
   parameters after producer/consumer audits.

Before each production change, record the affected producer/consumer audit and
an explicit `remove`, `retain`, or `migrate-in-place` decision in
`docs/pre-release-compatibility-inventory.md`, as required by the repository
direction.

## Non-goals

- Do not redesign provider request schemas or OpenAI-compatible endpoint
  fields.
- Do not merge MCP OAuth credentials into model-provider credentials.
- Do not delete keyring, `auth.json`, refresh, or authentication-change
  notification support.
- Do not assume that all providers expose the same account or usage data.

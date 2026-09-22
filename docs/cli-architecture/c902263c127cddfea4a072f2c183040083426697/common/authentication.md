# Common authentication flow

## Layers

Authentication has three distinct layers:

1. `codex login` / `logout` mutate or inspect local credential state through `codex-login::AuthManager`.
2. `codex-login` represents stored auth and refresh/recovery behavior.
3. `codex-api::AuthProvider` supplies request-time auth headers to API clients.

The CLI command boundary is in [`main.rs`](../../../../crates/cli/src/main.rs#L1392). Stored auth types and manager behavior are in [`login/src/auth/manager.rs`](../../../../crates/login/src/auth/manager.rs#L48). The request-facing Trait is [`AuthProvider`](../reference/trait-index.md#authprovider).

## Selection

`AuthManager` loads local state using `CODEX_HOME`, environment credentials, or configured auth modes. `model-provider` turns the selected auth into a shared auth provider for a concrete model provider. Auth selection is therefore upstream of provider/model and transport construction, but independent from the agent's thread/turn lifecycle.

## Security boundary

Credential storage and token refresh are not a general provider-neutral agent capability; they are an integration boundary. Core should consume the abstract request/auth interface and must not depend on CLI prompts or credential-file details.

# Provider and model selection

## Flow

```text
Config + AuthManager
  -> model provider configuration
  -> codex-model-provider::create_model_provider
  -> ModelProvider capability object
  -> codex-api Provider / client
  -> Responses or provider-specific stream
```

[`ModelProvider`](../reference/trait-index.md#modelprovider) is defined in `codex-model-provider`, while request/stream types and transport-facing clients are in `codex-api` and `codex-client`. The provider object carries capabilities, account state, recovery behavior, and request construction decisions; core consumes the resulting provider abstraction during turns.

The model catalog/debug path and runtime model selection are related but not identical: `codex debug models` inspects catalog data, while a turn creates the configured provider/model path.

## Provider-neutral rule

The common agent flow should describe "model provider" and "response stream" at this boundary. Provider-specific authentication, URL/header construction, and retry details belong below it. This keeps `codex-core` reusable when another provider implementation is selected.

## Relevant Traits

- [`ModelProvider`](../reference/trait-index.md#modelprovider): provider capabilities and lifecycle-facing behavior.
- [`AuthProvider`](../reference/trait-index.md#authprovider): request-time auth headers.
- [`SseTelemetry`](../reference/trait-index.md#ssetelemetry) / `WebsocketTelemetry`: transport observations.

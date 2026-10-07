# Model Catalog Boundaries Design

## Goal

Separate provider-specific model discovery from model catalog data and model catalog lifecycle management.

## Terminology

### Structs

- `ModelInfo`: metadata for one model.
- `ModelCatalog`: the serializable catalog value containing `Vec<ModelInfo>`. This replaces the misleading `ModelsResponse` Rust type; the JSON shape remains `{ "models": [...] }`.
- `ModelCatalogSnapshot`: one fetched catalog plus cache-validation metadata (`catalog`, `identity`, and optional `etag`). This replaces `ModelsEndpointResponse`.
- `ProviderModelCatalogSource`: an adapter that exposes a `SharedModelProvider` as a catalog source for the manager.
- `RemoteModelsManager`: the cache-backed catalog manager.
- `StaticModelsManager`: the in-memory authoritative catalog manager.

### Traits

- `ModelProvider`: provider runtime behavior, authentication, and provider-specific `list_models` implementation. It does not construct a manager.
- `ModelCatalogSource`: the manager-facing source abstraction. It provides identity/capability metadata and fetches `ModelCatalogSnapshot`.
- `ModelsManager`: refresh, cache, catalog merge, visibility filtering, and `ModelPreset` construction.

## Boundaries

`CloudflareModelProvider` and `MyModelProvider` implement `ModelProvider` only. Their `list_models` implementations normalize provider-specific responses into `ModelCatalogSnapshot`.

`ProviderModelCatalogSource` delegates catalog fetches to `ModelProvider` and supplies the manager-facing identity and source capabilities. `RemoteModelsManager` consumes `ModelCatalogSource`; it does not know which provider or endpoint produced the catalog.

`ModelsResponse` and `ModelsEndpointResponse` are removed as Rust type names. Existing serialized JSON, configuration fields, cache files, and external endpoint paths retain their existing wire shape and behavior.

## Cache metadata

- `identity` is an opaque digest of provider routing and effective authentication. It partitions cached catalogs by provider, account, endpoint, and credential scope without persisting raw credentials.
- `etag` is the upstream catalog version validator. When a refresh observes the same identity and ETag, the manager can revalidate freshness without treating the catalog as changed.

## Compatibility and behavior

- Preserve Online, Offline, and OnlineIfUncached refresh behavior.
- Preserve static configured catalogs and disk-cache behavior.
- Preserve API-key model discovery policy for My Provider and Cloudflare Workers AI.
- Preserve Cloudflare pagination, model filtering, normalization, and auth behavior.
- Do not change the JSON schema of `/models` or persisted cache entries.

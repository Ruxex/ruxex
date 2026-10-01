# Cloud Config Hosted Policy Removal Design

## Goal

Remove the ChatGPT backend-delivered cloud configuration and policy source
from local configuration loading, while preserving provider-neutral local
configuration and locally managed requirements.

## Scope

This is the first implementation unit of Level 1 Phase 3 in
`docs/provider-decoupling-plan.md`.

The change removes the hosted bundle source from `codex-config` and all
entrypoint setup paths. It then removes `codex-cloud-config` because its only
public outputs are loaders for that source.

## Architecture

`codex-config` will load configuration only from packaged defaults, local
system and managed files, user and project files, thread configuration, and
runtime overrides. Its `ConfigLoadOptions` will no longer carry a
`CloudConfigBundleLoader`, and the loader will no longer add remote config or
requirements layers.

The cloud-bundle domain types, TOML conversion logic, and fixtures currently
owned by `codex-config` exist only to support that removed source and will be
deleted. Local managed requirements retain their existing precedence and
validation behavior.

The CLI, TUI, exec, app-server, and Windows sandbox service will stop
constructing, refreshing, or passing cloud bundle loaders. This removes the
remaining consumers of `codex-cloud-config`; its workspace member,
dependencies, source, tests, and lockfile entries can then be removed.

## Data Flow

Before this change:

1. Entrypoints authenticate with the hosted service and build a cloud bundle
   loader.
2. `codex-config` fetches a remote bundle during local config loading.
3. The bundle contributes enterprise-managed config and requirements layers.

After this change:

1. Entrypoints construct `ConfigLoadOptions` without a hosted loader.
2. `codex-config` reads and merges only local configuration and managed
   requirements sources.
3. A remote provider cannot alter local configuration loading or policy.

## Compatibility and Error Handling

Local system/managed configuration, including local requirements, remains
supported. Missing or malformed local files retain their current diagnostics.
The former network, cache, signature, expiration, and remote-bundle errors no
longer occur during startup because no hosted bundle is loaded.

No provider-neutral replacement interface is introduced in this unit. Adding
one without an active non-hosted provider would preserve unnecessary provider
integration surface; a future provider adapter can define an explicit policy
interface when needed.

## Tests and Verification

Remove tests whose only assertion is cloud bundle application or cloud bundle
loading errors. Preserve and run tests for local managed configuration and
requirements. Verify the removal with:

- `just crate-dependency-graph`, confirming no `codex-cloud-config` node or
  incoming dependency edges remain;
- targeted `cargo check` commands for every former direct consumer and
  `codex-config`;
- repository-wide `just check`.

## Non-Goals

- Removing other backend integration features.
- Replacing hosted policy with another remote policy provider.
- Changing local configuration precedence or locally managed policy semantics.
- Redesigning authentication or the generic provider registry.

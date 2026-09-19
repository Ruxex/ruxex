# Pre-release Compatibility Inventory

This inventory is created and maintained by
`docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md`.

The repository is a pre-release fork with zero external users. The inventory
must distinguish historical compatibility from current runtime behavior before
any candidate is removed.

## Decision categories

- `remove`: exists only for an obsolete upstream, persisted, protocol, CLI, or
  configuration contract.
- `retain`: required by a current in-repository consumer, current platform,
  security behavior, or provider-neutral capability.
- `migrate-in-place`: has a current consumer, but the consumer and canonical
  representation will be updated in the same change.

## Inventory table

| Subsystem | Symbol/path | Historical contract | Current producer/consumer | Decision | Verification |
|---|---|---|---|---|---|
| Image generation | `image_generation*`, `image_gen.generation` | Legacy image-generation output/history | To be filled from the repository search | `remove` | Image-generation compatibility plan checks |
| Persistence | `crates/thread-store/src/local/rollout_migration/` | Old rollout/event shapes | To be filled from call-site audit | Pending | `cargo check -p codex-thread-store` |
| Protocol | `legacy`, `deprecated`, serde aliases | Old wire/config fields | To be filled from active-client audit | Pending | `cargo check -p codex-protocol -p codex-app-server-protocol` |
| CLI/config | Deprecated commands, fields, aliases | Pre-fork user surface | To be filled from current help/config surface | Pending | CLI help/config smoke checks |
| Runtime fallback | Platform/provider fallback paths | Current runtime behavior, not necessarily historical compatibility | To be filled from platform/provider audit | Pending | Affected crate checks |

Do not mark an entry complete based only on a name containing `legacy`,
`deprecated`, or `fallback`; record the producer and consumer first.

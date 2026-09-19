# Pre-release Compatibility Inventory

This inventory is maintained by
`docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md`.
The repository is a pre-release fork with zero external users, so historical
upstream contracts may be removed. A match is a candidate only when its
producer/consumer audit shows that it is not current runtime behavior.

## Decision categories

- `remove`: exists only to read, write, migrate, replay, or describe an
  obsolete contract. The owning cleanup task may delete it.
- `retain`: required by a current in-repository producer/consumer, supported
  platform, security behavior, or provider-neutral capability.
- `migrate-in-place`: has a current consumer, but the canonical producer and
  representation will be made explicit before the compatibility path is
  removed.

The search is intentionally broad:

```bash
rg -n -i 'legacy|compatib|deprecated|deprecat|migration|fallback|historical|serde\(alias' \
  crates docs --glob '*.rs' --glob '*.md' --glob '*.toml' --glob '*.json'
rg -n -i 'image_generation|image_generation_call|image_gen' crates \
  --glob '*.rs' --glob '*.json' --glob '*.ts'
```

Generated schemas and tests are grouped with the source symbol that produces
them. Ordinary names such as `rename_all`, dependency alias expansion,
heuristic fallback, and migration of a currently supported external-agent
import are not removed merely because the search matched them.

## Ordered cleanup slices

The slices are executed in this order so each later boundary can rely on the
earlier one being free of obsolete persisted data:

1. persisted rollout and thread-history readers/migrations;
2. protocol and app-server aliases/schemas;
3. CLI, config, and import compatibility;
4. UI, observability, and runtime shims;
5. final dependency and documentation audit.

## Candidates and decisions

### Image-generation history and protocol slice

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `codex_extension_items::image_generation::{ImageGenerationItem, ImageGenerationFailure}`; `ExtensionItem::ImageGeneration` | No current producer after the Images adapter removal; rollout/app-server/TUI consumers only deserialize or display old records | Generic image input/media and a future provider-neutral generation capability | `remove` | `cargo check -p codex-extension-items -p codex-core -p codex-app-server-protocol` |
| `ResponseItem::ImageGenerationCall`; `ImageGenerationBeginEvent`; `ImageGenerationEndEvent` | No current producer; Responses normalization, replay, and migration consume historical variants | Existing non-generation `ResponseItem`/`EventMsg` variants | `remove` | `cargo check -p codex-protocol -p codex-core` |
| `EventMsg::ImageGenerationEnd` arm in `rollout_migration/legacy_event.rs` and image branches in `thread_history.rs`/`policy.rs` | Historical rollout reader and SQLite projection only; no current writer | Current canonical paginated items | `remove` | `cargo check -p codex-thread-store -p codex-rollout` |
| `ThreadItem::ImageGeneration`, generated JSON/TypeScript schemas, notification redaction and resume branches | App-server history/replay consumers only; no active client producer | Generic `imageView` and ordinary thread items | `remove` | `cargo check -p codex-app-server-protocol -p codex-app-server` |
| Image-generation branches in TUI, analytics, OTEL, rollout-trace, guardian, and code-mode | Historical display/telemetry consumers; no live generation tool remains | Existing generic tool/turn/media accounting | `remove` | `cargo check -p codex-tui -p codex-analytics -p codex-rollout-trace -p codex-otel` |

This is one image-generation compatibility slice, not a decision to remove
generic image input, resizing/preparation, local media, or `view_image`.

### Persisted rollout and thread-history compatibility

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `RolloutMigrationMode`, `RolloutMigrationOptions`, `LegacyRolloutCanonicalizer`, `rollout_migration/{legacy_event,rollback_replay,telemetry}.rs` | CLI startup/manual migration consumes pre-pagination rollout files; no new canonical writer emits those files | Current paginated rollout writer and SQLite projection | `remove` | `cargo check -p codex-thread-store -p codex-rollout -p codex-cli` |
| `ThreadHistoryMode::Legacy`, `rollout::policy` legacy event persistence, `ordinal::Legacy`, legacy branches in `thread_store` | `ThreadStore::create_thread` still defaults to `Legacy`; live writer, metadata sync, resume, and tests consume it | `ThreadHistoryMode::Paginated` | `migrate-in-place` | `cargo check -p codex-thread-store -p codex-rollout`; add current-default audit before deletion |
| `strip_legacy_ghost_snapshot_rollout_line` and `is_legacy_ghost_snapshot_response_item` in `crates/rollout/src/recorder.rs` | Rollout reader filtered an obsolete snapshot artifact; no current writer found | Normal rollout line decoder | `remove` (cb46dfc8c) | `cargo test -p codex-rollout recorder` |
| `GhostSnapshotToml`, `GhostSnapshotConfig`, `ghost_snapshot` config field, and the migration-parser skip/test for `ghost_snapshot` | The snapshot producer was removed; these are no-op config/persisted-record compatibility paths with no current producer | Current config schema and ordinary response-item decoding | `remove` (current work) | `cargo check -p codex-config -p codex-core -p codex-thread-store -p codex-protocol`; `just write-config-schema` |
| `rollout_id_from_path_or_legacy_thread_id` and legacy filename fallback in `thread_rollout_resolver.rs` | Local thread-store lookup accepts noncanonical historical filenames | Canonical rollout ID/path from thread metadata | `remove` | `cargo check -p codex-thread-store` |
| `find_conversation_path_by_id_str` deprecated re-export in `crates/rollout/src/lib.rs` and `crates/core/src/lib.rs` | Search callers use `find_thread_path_by_id_str`, `ThreadItem`, and `ThreadsPage` | Canonical string/path and page types | `remove` (cb46dfc8c) | `cargo check -p codex-rollout -p codex-core` |
| Legacy metadata/title fallback in `local/helpers.rs`, `thread_metadata_sync.rs`, and `thread_history` | Legacy rollouts use stored metadata while the SQLite index fills missing fields | Canonical paginated metadata | `migrate-in-place` | `cargo test -p codex-thread-store local` |

Old rollout files are unsupported after cleanup but are not deleted by the
implementation.

### Protocol and app-server compatibility

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| Serde aliases in `config_toml.rs`: `max_threads`, `ignore_untracked_files_over_bytes`, `large_untracked_dir_warning_threshold`; `types.rs`: `no_memories_if_mcp_or_web_search`; `config_requirements.rs`: `feature_requirements` | Only old config files deserialize these names; current config producers use the canonical names | Canonical snake/camel-case config fields | `remove` | `cargo check -p codex-config -p codex-core` |
| `EventMsg` aliases `turn_started`/`task_started` and `turn_complete`/`task_complete` in `crates/protocol/src/protocol.rs` | Rollout and app-server event serializers/readers use the event enum; active clients must be checked before deleting aliases | Current `turn_*` event names | `migrate-in-place` | `cargo check -p codex-protocol -p codex-app-server` |
| `ResponseItem`/model aliases in `crates/protocol/src/models.rs`, `openai_models.rs`, and `codex-api` | Active Responses-compatible providers and app-server model catalog still consume some wire shapes | Provider-neutral model/response representation after call-site migration | `migrate-in-place` | `cargo check -p codex-protocol -p codex-api -p codex-app-server-protocol` |
| Deprecated personality, multi-agent, full-history, and `includeTurns` fields in app-server v2 thread/turn protocol | Current app-server tests and in-repository clients still exercise these fields and receive deprecation notices | Paginated thread/item APIs and reasoning configuration | `migrate-in-place` | `cargo check -p codex-app-server-protocol -p codex-app-server` |
| `DeprecationNoticeNotification` and `EventMsg::DeprecationNotice` | Current app-server request processors emit notices for active migration paths | Current protocol notification until all clients use canonical requests | `retain` | `cargo test -p codex-app-server --test suite` |
| `codex-mcp` aliases `callable_name`/`callable_namespace` and old MCP schema spellings | MCP compatibility deserialization is used by current MCP clients/tests; no replacement producer audit completed | Canonical MCP tool/schema names | `migrate-in-place` | `cargo check -p codex-mcp -p codex-protocol` |
| `app-server-protocol` generated schema files | Generated from the active Rust protocol and currently consumed by app clients/tests | Regenerated canonical schema after source removal | `migrate-in-place` | Repository schema generation plus `git diff --check` |

### CLI, configuration, and import compatibility

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| Deprecated ignored profile/style fields in `crates/config/src/profile_toml.rs` and `config_toml.rs` | Old config files only; current profiles use the canonical style/config fields | Remove ignored fields and warnings | `remove` | `cargo test -p codex-config` |
| `managed_config.toml` compatibility probe in `config/loader/layer_io.rs` | Windows loader still checks the old path to emit a migration warning; current supported requirement files remain separate | Current `%ProgramData%` requirements path/config.toml | `remove` | `cargo test -p codex-config loader` |
| Removed remote thread-store endpoint field and its rejection test in `crates/config/src/config_toml.rs`/`crates/core/src/config/config_tests.rs` | Only rejects a removed remote thread-store contract; no current producer; the active `RemoteThreadConfigLoader` is a separate current config service | Local thread store configuration | `remove` | `cargo test -p codex-core config` |
| `external-agent-migration` service, source detection, config/skills/MCP/hooks/subagent/session import | Current supported import workflow is called by CLI/app-server and has active tests | Native Codex config, skills, hooks, MCP, and session formats | `retain` | `cargo test -p codex-external-agent-migration` |
| `codex-mcp::McpProtocolMode::Legacy`, external-agent `MigrationFallback`, and source-specific `.cursorrules`/legacy files | Current external-agent import and MCP negotiation consume these paths | Imported native config plus current MCP protocol mode | `retain` | `cargo test -p codex-external-agent-migration -p codex-mcp` |
| CLI deprecated command/help guards and old top-level command rejection | Current CLI deliberately rejects removed OpenAI commands and keeps provider-neutral `login status`/`logout` | Current CLI command surface | `retain` | `cargo check -p codex-cli`; CLI help smoke checks |

### Runtime, UI, and observability shims

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `compatibility_sandbox_policy_for_permission_profile`, `to_legacy_sandbox_policy`, and `compatibility_workspace_write_policy` | Current sandbox permission profiles are projected for existing launcher/platform interfaces | Runtime `PermissionProfile` plus platform sandbox adapters | `retain` | `cargo test -p codex-sandboxing` |
| `use_legacy_landlock` and Linux `--use-legacy-landlock` path | Current Linux/WSL platform behavior and security policy still select this fallback | Current supported Landlock/bubblewrap selection | `retain` | `cargo test -p codex-sandboxing -- manager` |
| `LegacyAppPathString` and path URI conversion helpers | Current app-server, Windows, and sandbox protocol paths use the type at the platform boundary | Canonical native/path URI types after all clients migrate | `retain` | `cargo check -p codex-utils-path-uri -p codex-app-server-protocol` |
| TUI replay/status/transcript branches for `ImageGeneration*` | Historical image-generation records only; no current producer | Generic image/media and tool replay | `remove` | `cargo check -p codex-tui` |
| Analytics/OTEL/rollout-trace image-generation event counters and classifications | Historical image-generation events only | Generic turn/tool accounting | `remove` | `cargo check -p codex-analytics -p codex-otel -p codex-rollout-trace` |
| Generic `fallback` paths in LM Studio/Ollama/model-provider selection and exec policy heuristics | Current non-OpenAI providers and policy evaluation use them | Current provider/platform fallback behavior | `retain` | Affected crate `cargo check` |

## Inventory completion rule

Before each cleanup slice, refresh the broad searches, inspect every newly
introduced production match, and update the relevant row with an exact symbol,
producer/consumer, canonical replacement, decision, and verification command.
No row may be classified from its name alone. A slice is complete only when
its production matches are either removed or explicitly retained/migrated and
the listed verification commands have run.

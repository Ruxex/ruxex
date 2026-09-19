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
| `codex_extension_items::image_generation::{ImageGenerationItem, ImageGenerationFailure}`; `ExtensionItem::ImageGeneration` | Current extension-tool lifecycle emits the namespaced item through `CoreTurnItemEmitter`; extension, app-server, and TUI tests/consumers preserve its saved-path and failure fields | Provider-neutral extension item and its current app-server/TUI wrappers | `retain` | `cargo test -p codex-core image_generation_publication_preserves_extension_saved_path`; `cargo check -p codex-extension-items -p codex-app-server-protocol` |
| `ResponseItem::ImageGenerationCall`; `ImageGenerationBeginEvent`; `ImageGenerationEndEvent` | Active extension-tool bridge emits legacy event mirrors alongside canonical `ExtensionItem::ImageGeneration`; replay and UI consume them | Canonical extension item plus the current lifecycle bridge | `retain` | `cargo check -p codex-protocol -p codex-core` |
| `EventMsg::ImageGenerationEnd` bridge and image branches in `thread_history.rs`/`policy.rs` | The migration-only legacy-event module is gone, but active extension lifecycle items still use the event bridge and current history/policy code | Paginated extension item persistence with the current lifecycle bridge | `migrate-in-place` | `cargo check -p codex-thread-store -p codex-rollout` |
| `ThreadItem::ImageGeneration`, generated JSON/TypeScript schemas, notification redaction and resume branches | Current app-server history/replay and redaction consumers expose active extension-generated image items | Current typed extension item schema | `retain` | `cargo check -p codex-app-server-protocol -p codex-app-server` |
| Image-generation branches in TUI, analytics, OTEL, rollout-trace, guardian, and code-mode | Current extension lifecycle and UI tests still produce/display image-generation items; unused branches must be audited individually rather than removed by name | Current provider-neutral extension/media accounting | `retain` | `cargo check -p codex-tui -p codex-analytics -p codex-rollout-trace -p codex-otel` |

This is one image-generation compatibility slice, not a decision to remove
generic image input, resizing/preparation, local media, or `view_image`.

### Persisted rollout and thread-history compatibility

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `RolloutMigrationMode`, `RolloutMigrationOptions`, `LegacyRolloutCanonicalizer`, `rollout_migration.rs` and `rollout_migration/` | No current producer or consumer remains after the Paginated writer boundary; startup migration, manual CLI migration, migration telemetry, and migration-only state APIs were deleted | Current paginated rollout writer and SQLite projection | `remove` (current work) | `cargo check -p codex-thread-store -p codex-rollout -p codex-cli` |
| `0047_rollout_migration_state.sql`, `RolloutMigrationState`, and skipped-rollout state APIs | Only the deleted startup migration used these persisted tables and state records; no current runtime consumer remains | No migration cursor or skipped-rollout table | `remove` (current work) | `cargo check -p codex-state -p codex-thread-store` |
| `rollout::policy` legacy event persistence, `ordinal::Legacy`, live-writer legacy persistence/materialization branches, legacy rollout filename/title/name fallbacks, and legacy metadata SessionMeta writes | Producer/consumer audit: current local writers now emit canonical paginated ordinals and `ItemCompleted`/canonical items; SQLite is authoritative for selected rollout, display name, cwd, provider, git, memory, and metadata updates; old index/title/filename fallbacks had no current producer after the paginated boundary | `ThreadHistoryMode::Paginated`, canonical rollout path, and SQLite thread metadata/projection | `remove` (current work) | `cargo test -p codex-thread-store --lib` (190/190); `cargo test -p codex-rollout --lib` (129/129); `cargo check -p codex-thread-store -p codex-rollout` |
| `ThreadHistoryMode::Legacy` enum, `thread_history::read` unsupported guard, and the full-history/model-context reader for explicitly marked old rollout files | Producer/consumer audit: no current writer produces Legacy after the boundary, but staged Task 4 still uses the enum and explicit old-file rejection/read behavior to finish the protocol cleanup safely | Remove the enum and obsolete old-file reader surface in dedicated plan Task 4; retain current platform/security fallbacks unrelated to history mode | `migrate-in-place` | `cargo test -p codex-thread-store --lib`; remove only in Task 4 after protocol/app-server consumers are audited |
| `strip_legacy_ghost_snapshot_rollout_line` and `is_legacy_ghost_snapshot_response_item` in `crates/rollout/src/recorder.rs` | Rollout reader filtered an obsolete snapshot artifact; no current writer found | Normal rollout line decoder | `remove` (cb46dfc8c) | `cargo test -p codex-rollout recorder` |
| `GhostSnapshotToml`, `GhostSnapshotConfig`, `ghost_snapshot` config field, and the migration-parser skip/test for `ghost_snapshot` | The snapshot producer was removed; these are no-op config/persisted-record compatibility paths with no current producer | Current config schema and ordinary response-item decoding | `remove` (current work) | `cargo check -p codex-config -p codex-core -p codex-thread-store -p codex-protocol`; `just write-config-schema` |
| `rollout_id_from_path_or_legacy_thread_id` and legacy filename fallback in `thread_rollout_resolver.rs` | Local thread-store lookup accepts noncanonical historical filenames | Canonical rollout ID/path from thread metadata | `remove` | `cargo check -p codex-thread-store` |
| `find_conversation_path_by_id_str` deprecated re-export in `crates/rollout/src/lib.rs` and `crates/core/src/lib.rs` | Search callers use `find_thread_path_by_id_str`, `ThreadItem`, and `ThreadsPage` | Canonical string/path and page types | `remove` (cb46dfc8c) | `cargo check -p codex-rollout -p codex-core` |
| Legacy metadata/title/name fallback in `local/helpers.rs`, `read_thread.rs`, `thread_rollout_resolver.rs`, and `update_thread_metadata.rs` | Producer/consumer audit: current paginated reads and updates use canonical rollout paths plus SQLite metadata; the old session-index name, stale-path search, and legacy SessionMeta update consumers were removed; only the separate legacy sandbox-policy parser remains for current persisted permission compatibility | Canonical paginated metadata and current `PermissionProfile` projection | `remove` for thread metadata fallbacks; `retain` for legacy sandbox-policy parsing | `cargo test -p codex-thread-store --lib`; `cargo check -p codex-thread-store` |

Old rollout files are unsupported after cleanup but are not deleted by the
implementation.

### Protocol and app-server compatibility

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `crates/config/src/key_aliases.rs`, `AgentsToml::max_threads`, and `MemoriesToml::no_memories_if_mcp_or_web_search` | No current producer uses these names; the normalizer, serde aliases, origin projection, and merge tests existed only to accept old user config keys | Canonical `agents.max_concurrent_threads_per_session` and `memories.disable_on_external_context` | `remove` (current work) | `cargo test -p codex-config --lib`; `cargo check -p codex-core` |
| `ConfigRequirementsToml`'s `feature_requirements` serde alias and requirements-layer alias merge; credential-broker projection of the old path in `loader/local.rs` | Current managed requirements use `[features]`; the only current consumer of the old path was the credential-broker projection, which now reads the canonical path only | Canonical `[features]` requirements path | `remove` (current work) | `cargo test -p codex-config --lib`; `cargo check -p codex-core` |
| Removed config aliases `ignore_untracked_files_over_bytes` and `large_untracked_dir_warning_threshold` | Repository search found no production producer or consumer for these names; they are retained in this row only as audited historical candidates | Current config fields, if any, with no compatibility reader | `remove` (already absent) | `rg -n 'ignore_untracked_files_over_bytes|large_untracked_dir_warning_threshold' crates` |
| `EventMsg` aliases `turn_started`/`turn_complete` in `crates/protocol/src/protocol.rs` | Canonical serializers and current clients use `task_started`/`task_complete`; the old `turn_*` tags appeared only in the deserializer compatibility path, documentation, and observability labels | Canonical `task_started`/`task_complete` event tags | `remove` (already absent) | `cargo test -p codex-protocol old_turn_started_records_have_no_root_attribution`; `cargo check -p codex-app-server` |
| `ResponseItem`/model aliases in `crates/protocol/src/models.rs`, `openai_models.rs`, and `codex-api` | Active Responses-compatible providers and app-server model catalog still consume some wire shapes | Provider-neutral model/response representation after call-site migration | `migrate-in-place` | `cargo check -p codex-protocol -p codex-api -p codex-app-server-protocol` |
| Deprecated personality, multi-agent, full-history, and `includeTurns` fields in app-server v2 thread/turn protocol | Current app-server tests and in-repository clients still exercise these fields and receive deprecation notices | Paginated thread/item APIs and reasoning configuration | `migrate-in-place` | `cargo check -p codex-app-server-protocol -p codex-app-server` |
| `DeprecationNoticeNotification` and `EventMsg::DeprecationNotice` | Current app-server request processors emit notices for active migration paths | Current protocol notification until all clients use canonical requests | `retain` | `cargo test -p codex-app-server --test suite` |
| `codex-mcp` aliases `callable_name`/`callable_namespace` and old MCP schema spellings | MCP compatibility deserialization is used by current MCP clients/tests; no replacement producer audit completed | Canonical MCP tool/schema names | `migrate-in-place` | `cargo check -p codex-mcp -p codex-protocol` |
| `app-server-protocol` generated schema files | Generated from the active Rust protocol and currently consumed by app clients/tests | Regenerated canonical schema after source removal | `migrate-in-place` | Repository schema generation plus `git diff --check` |

### CLI, configuration, and import compatibility

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `ConfigProfile::js_repl_node_path`, `ConfigProfile::js_repl_node_module_dirs`, `ConfigToml::js_repl_node_path`, `ConfigToml::js_repl_node_module_dirs`, and `AgentsToml::job_max_runtime_seconds` | Repository-wide producer/consumer audit found no runtime reads; the fields were deserialized and ignored only to accept obsolete configuration | Current config schema without no-op fields | `remove` (current work) | `cargo test -p codex-config --lib`; `cargo check -p codex-core --lib` |
| Windows default `CODEX_HOME/managed_config.toml` metadata probe and startup warning in `loader/layer_io.rs` | Current Windows policy is sourced from the system `requirements.toml` path; the default user-home file was only checked to explain that it was ignored, while explicit managed paths and Unix `/etc/codex/managed_config.toml` remain active | Current Windows requirements/config paths without a historical-file probe | `remove` (current work) | `cargo test -p codex-config --lib`; `cargo check -p codex-core --lib` |
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

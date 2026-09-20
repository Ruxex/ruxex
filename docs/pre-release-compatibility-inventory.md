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

### Web-search provider boundary

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `WebSearchExtensionConfig::from` in `crates/ext/web-search/src/extension.rs`, `ModelProviderInfo::create_openai_provider`, and `uses_openai_actor_authorization` | The extension's `web.run` executor was enabled automatically for the built-in OpenAI provider and actor-authorized hosted requests; core then preferred that executor over the hosted `web_search` tool. Custom providers use the explicit `supports_standalone_web_search` opt-in and remain active | OpenAI uses the provider-capability-backed hosted `web_search` specification; custom providers opt into the standalone executor explicitly | `remove` for the OpenAI/actor-authorized branch; `retain` for custom standalone providers | `rg -n 'uses_openai_actor_authorization|OPENAI_ACTOR_AUTHORIZATION_HEADER' crates` returns no production matches; `just check` |

This slice removes the OpenAI hosted adapter path only. The shared search request
types, generic `SearchClient`, hosted tool specification, and custom-provider
standalone executor remain available.

### Persisted rollout and thread-history compatibility

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `RolloutMigrationMode`, `RolloutMigrationOptions`, `LegacyRolloutCanonicalizer`, `rollout_migration.rs` and `rollout_migration/` | No current producer or consumer remains after the Paginated writer boundary; startup migration, manual CLI migration, migration telemetry, and migration-only state APIs were deleted | Current paginated rollout writer and SQLite projection | `remove` (current work) | `cargo check -p codex-thread-store -p codex-rollout -p codex-cli` |
| `0047_rollout_migration_state.sql`, `RolloutMigrationState`, and skipped-rollout state APIs | Only the deleted startup migration used these persisted tables and state records; no current runtime consumer remains | No migration cursor or skipped-rollout table | `remove` (current work) | `cargo check -p codex-state -p codex-thread-store` |
| `rollout::policy` legacy event persistence, `ordinal::Legacy`, live-writer legacy persistence/materialization branches, legacy rollout filename/title/name fallbacks, and legacy metadata SessionMeta writes | Producer/consumer audit: current local writers emit canonical paginated ordinals and `ItemCompleted`/canonical items; SQLite is authoritative for selected rollout, display name, cwd, provider, git, memory, and metadata updates; no current producer or consumer needs the old branches | `ThreadHistoryMode::Paginated`, canonical rollout path, and SQLite thread metadata/projection | `remove` (completed in working tree) | `just check` (0 errors) |
| `ThreadHistoryMode::Legacy` enum, `thread_history::read` unsupported guard, and the full-history/model-context reader for explicitly marked old rollout files | Producer/consumer audit: all current creation, import, persistence, paging, replay, and app-server conversion paths use Paginated; old rollout files remain untouched but are unsupported after the reader boundary is removed | Single `ThreadHistoryMode::Paginated` contract and paginated lineage/model-context readers | `remove` (completed in working tree) | `just check` (0 errors) |
| `strip_legacy_ghost_snapshot_rollout_line` and `is_legacy_ghost_snapshot_response_item` in `crates/rollout/src/recorder.rs` | Rollout reader filtered an obsolete snapshot artifact; no current writer found | Normal rollout line decoder | `remove` (cb46dfc8c) | `cargo test -p codex-rollout recorder` |
| `GhostSnapshotToml`, `GhostSnapshotConfig`, `ghost_snapshot` config field, and the migration-parser skip/test for `ghost_snapshot` | The snapshot producer was removed; these are no-op config/persisted-record compatibility paths with no current producer | Current config schema and ordinary response-item decoding | `remove` (current work) | `cargo check -p codex-config -p codex-core -p codex-thread-store -p codex-protocol`; `just write-config-schema` |
| `RolloutItem::RealtimeItem`, `thread_realtime_items` migration/projection, and Realtime history readers | Producer/consumer audit: current rollout, thread-store, state, core, app-server, and TUI paths had no non-Realtime consumer; old Realtime rows are not read after this cleanup, while existing rollout files are not deleted automatically | Ordinary paginated rollout and thread-history items | `remove` (current work) | `cargo check -p codex-rollout -p codex-state -p codex-thread-store -p codex-core` |
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
| `FeedbackConfigToml`, `ConfigToml::feedback`, `Config::feedback_enabled`, managed `feedback.enabled` requirements, `FeedbackUpload` app-server request, `FeedbackUploadParams`, `FeedbackUploadResponse`, and generated feedback schemas | The TUI feedback command produced the request; the app-server processor consumed and uploaded it to the fixed Sentry endpoint. This pre-release fork has no external client that needs the removed contract | No feedback config or feedback/upload protocol surface | `remove` (completed) | `just write-config-schema`; `just write-app-server-schema`; `cargo check -p codex-config -p codex-app-server-protocol` |
| Deprecated personality, multi-agent, full-history, and `includeTurns` fields in app-server v2 thread/turn protocol | Current app-server tests and in-repository clients still exercise these fields and receive deprecation notices | Paginated thread/item APIs and reasoning configuration | `migrate-in-place` | `cargo check -p codex-app-server-protocol -p codex-app-server` |
| `DeprecationNoticeNotification` and `EventMsg::DeprecationNotice` | Current app-server request processors emit notices for active migration paths | Current protocol notification until all clients use canonical requests | `retain` | `cargo test -p codex-app-server --test suite` |
| `codex-mcp` aliases `callable_name`/`callable_namespace` and old MCP schema spellings | MCP compatibility deserialization is used by current MCP clients/tests; no replacement producer audit completed | Canonical MCP tool/schema names | `migrate-in-place` | `cargo check -p codex-mcp -p codex-protocol` |
| Realtime requests/notifications, Realtime protocol types, feature flag, and generated schema files | Producer/consumer audit: no current app-server, TUI, or in-repository client consumes the Realtime wire surface after the capability removal; ordinary thread/turn requests remain active | Current thread/turn protocol and generated schemas without Realtime types | `remove` (current work) | `cargo check -p codex-protocol -p codex-app-server-protocol -p codex-app-server`; `just write-app-server-schema` |
| `app-server-protocol` generated schema files for the remaining active protocol | Generated from the active Rust protocol and consumed by app clients/tests; ordinary thread/turn/history contracts remain current | Canonical generated JSON/TypeScript schemas | `migrate-in-place` | Updated generated JSON/TypeScript files, `just check`, and `git diff --check` |

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
| TUI replay/status/transcript branches for `ImageGeneration*` | Current extension lifecycle and app-server consumers still produce/display typed image-generation items; the removed code was the legacy history-mode transcript reader, not the active media path | Generic image/media and tool replay, backed by paginated history | `retain` | `just check` (active media consumers compile) |
| Analytics/OTEL/rollout-trace image-generation event counters and classifications | Current extension lifecycle and canonical compatibility events still feed active analytics/trace consumers | Generic turn/tool accounting with current media attribution | `migrate-in-place` | `just check` (active analytics/trace consumers compile) |
| Generic `fallback` paths in LM Studio/Ollama/model-provider selection and exec policy heuristics | Current non-OpenAI providers and policy evaluation use them | Current provider/platform fallback behavior | `retain` | Affected crate `cargo check` |
| TUI Realtime/voice commands, key bindings, settings, transcript replay/rendering, and voice snapshots | Producer/consumer audit: only the removed Realtime app-server/session lifecycle produced these states; ordinary composer, replay, status, generic audio, and screen-reader paths remain | Ordinary TUI input, replay, status, and media handling | `remove` (current work) | `cargo check -p codex-tui`; `git diff --check` |
| `query_feedback_logs_for_threads`, `query_feedback_logs`, feedback-only log formatter/tests, and feedback log export assertions in app-server tests | The removed feedback upload and Guardian feedback retention paths were the only production consumers; ordinary `query_logs`, SQLite retention, and sanitized log-body persistence remain active | Ordinary bounded `query_logs` and local log persistence | `remove` (completed) | `cargo check -p codex-state -p codex-app-server`; `rg -n 'query_feedback_logs|query_feedback_logs_for_threads' crates` returns no matches |
| `/feedback` tooltip, command-popup entry, interruption message, and related snapshots | TUI command/help and interruption rendering were the remaining user-facing producers after the feedback processor/UI removal; no current consumer requires the command | Generic interruption message and current slash-command list | `remove` (completed) | `cargo check -p codex-tui`; `git diff --check` |

## Inventory completion rule

Before each cleanup slice, refresh the broad searches, inspect every newly
introduced production match, and update the relevant row with an exact symbol,
producer/consumer, canonical replacement, decision, and verification command.
No row may be classified from its name alone. A slice is complete only when
its production matches are either removed or explicitly retained/migrated and
the listed verification commands have run.

## Phase 3 integration handoff

The following rows are capability-level cleanup units. They are intentionally
larger than a single compatibility symbol: each row must be audited from its
hosted producer through protocol, configuration, UI, and runtime consumers
before code is removed.

### ChatGPT Remote Control capability

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `crates/app-server-transport/src/transport/remote_control/` including `auth`, `clients`, `enroll`, `persistence`, `protocol`, and `websocket` | The transport allowlisted `chatgpt.com`/`chatgpt-staging.com`, called `/wham/remote/control`, and required ChatGPT account/OpenAI headers. No non-OpenAI producer/consumer was identified. | Current app-server transport and localhost/platform transport; no compatibility reader for remote-control enrollment | `remove` (completed) | `rg -n -i 'remote.?control|/wham/remote/control|chatgpt-account-id|x-openai-fedramp' crates --glob '*.{rs,json,ts,toml}'; just check` |
| `crates/app-server-protocol/src/protocol/v2/remote_control.rs`, v2 exports, generated schemas, and request/notification types | The app-server protocol exposed the Remote Control contract only for the hosted ChatGPT transport and its clients. | Active app-server protocol without Remote Control types | `remove` (completed) | `rg -n -i 'remote.?control|allowRemoteControl' crates/app-server-protocol --glob '*.{rs,json,ts}'; just check` |
| `crates/app-server-daemon/src/remote_control_client.rs`, `crates/cli/src/remote_control_cmd.rs`, `crates/state/src/runtime/remote_control.rs`, and remote-control config | Daemon, CLI, and state persistence existed only to enroll, pair, refresh, and run the hosted Remote Control session. | Current daemon/CLI/state surface without Remote Control | `remove` (completed) | `rg -n -i 'remote.?control|allow_remote_control|remoteControl' crates/cli crates/app-server-daemon crates/state crates/config crates/core --glob '*.{rs,json,toml}'; just check` |

### ChatGPT Apps / Connectors / host-owned Apps MCP capability

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `crates/connectors/src/lib.rs`, `ConnectorDirectoryCacheKey`, `/connectors/directory/list`, `/connectors/directory/list_workspace`, and `codex-connectors-extension` | Producer/consumer audit: ChatGPT backend directory responses were the only producer; Apps/plugin loading and connector UI were the only consumers. The crates, workspace edges, and consumers were removed. | Configured MCP/plugin metadata and ordinary provider-neutral tool discovery | `remove` (completed) | `rg -n 'ConnectorDirectory|connectors/directory|codex-connectors' crates --glob '*.rs'; just check` |
| `Feature::Apps`, `Feature::Psp`, Apps-related feature specs, `apps_mcp_product_sku`, and Apps config | Producer/consumer audit: ChatGPT-authenticated Apps/PSP configuration was the only producer; core, app-server, and TUI Apps paths were consumers. Feature, config, and requirements surfaces were removed. | Ordinary MCP configuration and current provider-neutral feature set | `remove` (completed) | `rg -n 'Feature::Apps|Feature::Psp|apps_mcp_product_sku|apps_enabled|EnableMcpApps|CodexAppsMcp' crates --glob '*.rs' --glob '*.toml'; just check` |
| `crates/app-server-protocol/src/protocol/v2/apps.rs`, Apps request processors, core Apps instructions, and TUI Apps/app mentions | Producer/consumer audit: app-server, core, and TUI were the only producers/consumers of the ChatGPT Apps product surface. RPCs, processors, instructions, UI, and generated schemas were removed. | Active app-server/core/TUI protocol and UI without Apps product types | `remove` (completed) | `rg -n 'AppsRequestProcessor|AppsList|AppsRequest|AppsInstalled|AppsRead' crates --glob '*.rs' --glob '*.json' --glob '*.ts'; just check` |
| `crates/codex-mcp/src/codex_apps.rs`, host-owned Apps MCP configuration, OpenAI `fileParams`/elicitation/source-attribution adapters, and ChatGPT MCP auth | Producer/consumer audit: ChatGPT-hosted Apps was the only producer; the dedicated MCP runtime, core, app-server, and TUI adapters were consumers. Those adapters were removed; generic MCP runtime, OAuth, standard elicitation, and security metadata remain. | Standard MCP runtime, generic OAuth/elicitation, and provider-neutral tool metadata | `remove` for Apps/OpenAI adapters; `retain` for generic MCP | `rg -n 'codex_apps|host_owned_codex_apps|CODEX_APPS_MCP|fileParams|openai/elicitation|McpServerAuth::ChatGpt' crates/codex-mcp crates --glob '*.rs'; just check` |

### Hosted Plugin Marketplace / Sharing / Runtime capability

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `crates/core-plugins/src/remote.rs`, `remote_legacy.rs`, `remote_bundle.rs`, `remote_mutations.rs`, remote search/share modules, and remote installed-plugin sync | Producer/consumer audit: these modules had no remaining local producer; their consumers were the hosted catalog, search, share, mutation, bundle, and installed-plugin paths in app-server, CLI, TUI, and core. | Local, bundled, Git/npm, and user-configured marketplace roots | `remove` (completed) | `cargo check -p codex-core-plugins -p codex-cli --bin codex`; `just check` |
| `Feature::RemotePlugin`, hosted plugin search/share/install request processors, CLI remote commands, Hosted Plugin Runtime routing, and remote analytics smoke client | Producer/consumer audit: only the hosted OpenAI/ChatGPT surfaces consumed these features. Local plugin install, hooks, tools, skills, trust policy, and the generic app-server test client remain active. | Provider-neutral local/plugin management surface | `remove` for Hosted surfaces; `retain` for local/plugin foundation | `rg -n 'Feature::RemotePlugin|remote_plugin_enabled|PluginShare|PluginSkillRead|openai-curated-remote|shared-with-me|created-by-me' crates --glob '*.rs' --glob '*.toml'; `cargo check -p codex-cli --bin codex` |
| `app-server-protocol` plugin share/remote-skill RPCs, remote marketplace fields, `PluginSource::Remote`, and generated JSON/TypeScript schemas | The protocol producer was `protocol/common.rs`; app-server and TUI were the only in-repository consumers, and those routes now have no callers. Generated schemas were regenerated from the reduced protocol. | Local marketplace path/source and ordinary plugin list/read/install/uninstall RPCs | `remove` (completed) | `cargo check -p codex-app-server --lib`; generated schema diff; `just check` |
| `crates/app-server-test-client/src/plugin_analytics_*` and corresponding README commands | These smoke clients exercised remote plugin install/uninstall and remote IDs only; after the hosted RPC removal they had no valid producer/consumer. | Existing provider-neutral app-server test-client commands | `remove` (completed) | `cargo check -p codex-app-server-test-client` |

### ChatGPT authentication / Agent Identity capability

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `crates/agent-identity/src/lib.rs`: ChatGPT issuer/JWKS/auth URLs, `ChatGptEnvironment`, `chatgpt_user_id`, and `chatgpt_account_is_fedramp` | Producer/consumer audit complete: the crate is consumed only by the ChatGPT Agent Identity paths in `codex-login` and `codex-model-provider`; no non-OpenAI identity producer or consumer remains. | Remove the identity crate; retain generic keyring, credential storage, workload identity, and external bearer primitives elsewhere | `remove` | `rg -n 'codex-agent-identity|ChatGptEnvironment|chatgpt_user_id|chatgpt_account_is_fedramp|CHATGPT_CODEX' crates --glob '*.{rs,toml}'; just check` |
| `AuthMode::AgentIdentity`, `CodexAuth::AgentIdentity`, `AgentIdentityStorage`, `AgentIdentityAuthRecord`, and the login/model-provider Agent Identity bootstrap path | Producer/consumer audit complete: persisted auth state, login bootstrap, model request auth, model telemetry, CLI/TUI account routing, and MCP auth-change comparison all exist only for the removed ChatGPT Agent Identity capability. | Current provider-neutral auth modes and credential storage without Agent Identity state | `remove` | `rg -n 'AuthMode::AgentIdentity|CodexAuth::AgentIdentity|AgentIdentityStorage|AgentIdentityAuthRecord|AgentIdentityAuthPolicy' crates --glob '*.{rs,toml,json,ts}'; just check` |
| `crates/login/src/server.rs`, `device_code_auth.rs`, ChatGPT OAuth/access-token branches, `AuthMode::Chatgpt`/`ChatgptAuthTokens`/`PersonalAccessToken`, `AuthManager` OAuth refresh/revoke/PAT paths, and ChatGPT auth-token refresh protocol/UI | Producer/consumer audit complete: these paths implemented ChatGPT OAuth, device authorization, token exchange/refresh, host-supplied token refresh, PAT handling, and their app-server/TUI consumers. `AuthMode::WorkloadIdentity` is now the host-managed token mode; Bedrock, external bearer, and generic storage/keyring have separate consumers. | Provider-neutral credential state plus `AuthMode::WorkloadIdentity`, Bedrock, external bearer, and generic storage; no ChatGPT login/refresh protocol | `remove` for ChatGPT OAuth/device/access-token, PAT, and refresh protocol/UI; `retain` for workload identity, Bedrock, external bearer, and generic auth storage | `rg -n 'AuthMode::Chatgpt|AuthMode::ChatgptAuthTokens|AuthMode::PersonalAccessToken|CodexAuth::Chatgpt|ChatgptAuthTokens|PersonalAccessToken|login_with_chatgpt_auth_tokens|auth\.openai\.com/oauth' crates/login crates/app-server crates/app-server-protocol crates/tui --glob '*.{rs,toml,json,ts}'; just check; just generate; git diff --check` |

### Residual hosted-specific code after capability removal

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `crates/app-server/src/turn_cost_worker_chatgpt.rs` and ChatGPT-only turn-cost imports | The active `turn_cost_worker.rs` is a disabled no-op; the ChatGPT worker source is not referenced by the current module graph and has no current producer. | No turn-cost worker; provider-neutral usage accounting only if a current producer is introduced | `remove` | `rg -n 'turn_cost_worker_chatgpt|query_chatgpt_turn_costs|WorkerRuntime' crates/app-server --glob '*.rs'; just check` |
| PSP cookies, `chatgpt_base_url`, `forced_chatgpt_workspace_id`, `apps_mcp_product_sku`, and ChatGPT Cloudflare host modules after the above slices | These settings/modules may become orphaned once Remote Control, Apps, Hosted Plugins, and ChatGPT auth are removed; OpenAI-compatible custom provider base URLs must be audited separately. | Provider-neutral provider URL/config and generic HTTP client | `remove` for orphaned hosted fields/modules; `retain` for active compatible-provider URL handling | `rg -n 'chatgpt_base_url|forced_chatgpt_workspace_id|apps_mcp_product_sku|with_chatgpt_cloudflare_cookies|chatgpt_hosts|oai-chat-psp' crates --glob '*.rs' --glob '*.toml'; just check` |

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `crates/core-plugins/src/startup_sync.rs`, `startup_sync/http_client.rs`, `startup_sync_tests.rs`, `CURATED_PLUGINS_STARTUP_SYNC_*`, and `PluginsManager::start_curated_repo_sync` | The producer only synchronized OpenAI's curated plugin repository from GitHub or the ChatGPT export archive during startup; no provider-neutral marketplace producer consumed it. | User-configured local/Git/npm marketplace sources and the existing bundled plugin loader | `remove` (completed) | `cargo check -p codex-core-plugins`; `git diff --check`; `just check` |

| `curated_marketplace.rs`; `.tmp/plugins` and `.tmp/plugins.sha` readers; `refresh_curated_plugin_cache`; local curated root injection in `PluginsManager::marketplace_roots`; curated-cache trust attribution | The startup producer and its cache refresh caller are gone. No current producer creates the local OpenAI curated cache, and the remaining readers only expose or trust unsupported old cache files. Existing files are left untouched. Hosted curated catalog/install is removed; bundled/runtime marketplace paths remain active. | User-configured local/Git/npm marketplace sources and bundled/runtime marketplace roots | `remove` (completed) | `cargo check -p codex-core-plugins`; `rg -n 'curated_plugins_(repo|api_marketplace)_path|read_curated_plugins_sha|refresh_curated_plugin_cache|plugins\.sha' crates/core-plugins/src crates/app-server/src crates/cli/src crates/core/src crates/external-agent-migration/src --glob '*.rs'`; `git diff --check`; `just check` |

### Analytics hosted sink slice

| Exact symbol/path | Current producer/consumer | Canonical replacement | Decision | Verification |
|---|---|---|---|---|
| `AnalyticsEventsDestination::Http`, `AnalyticsEventsClient::new` hosted URL/auth parameters, `send_track_events` auth filtering, HTTP request construction in `crates/analytics/src/client.rs`, and the `codex-model-provider` dependency | Core, app-server, and extensions still produce provider-neutral analytics facts and consume the client for local reduction/metadata gating, but no current non-OpenAI sink consumes the reduced events. The removed branch targeted `chatgpt.com/backend-api/codex/analytics-events/events` and required Codex auth/provider headers. | Provider-neutral event/fact/reducer APIs; optional debug capture via `CODEX_ANALYTICS_EVENTS_CAPTURE_FILE`; no hosted sink in this unit | `remove` for the OpenAI-hosted sink; `retain` for event/reducer processing and local debug capture | `cargo check -p codex-analytics -p codex-core -p codex-app-server`; `rg -n 'chatgpt\.com/backend-api/codex/analytics-events/events|auth_provider_from_auth|can_send_with_api_key_auth' crates/analytics/src/client.rs crates/analytics/src/events.rs crates/app-server/src/analytics_utils.rs` returns no production matches; `git diff --check` |

# 未使用・provider 依存候補の追加調査メモ

この文書は、[provider 固有機能の段階的な分離計画](provider-decoupling-plan.md) に対する 2026-09-27 時点の追加調査結果を記録する。

ここに記載した項目は **削除・保持・移行の決定ではない**。個別の producer/consumer 監査、別バイナリからの到達性、動的登録、生成物、外部利用の確認を行った後、[pre-release compatibility inventory](pre-release-compatibility-inventory.md) に `remove`、`retain`、または `migrate-in-place` として記録する。

## 調査範囲と検証

- workspace の `Cargo.toml`、`cargo metadata --no-deps` による内部依存元の確認
- `Feature` registry、`apply_map`、生成済み config schema の参照確認
- `provider`、`chatgpt`、固定 endpoint、provider 専用 header、managed auth の production code 検索
- `just check`：成功、エラーなし、未使用警告 114 件
- `just check --tests`：成功

既存の計画書・インベントリには、現在のソースツリーに残る実装を完了済みと記載している箇所もある。以下の候補は、その差分を解消するための再監査対象として扱う。

互換性クリーンアップ計画の Task 1〜8 は完了している。削除済み feature
の registry、schema、`apply_map`、CLI/TUI の一時的な consumer とテスト残骸も
`fb8d960a7` で処理済みであり、詳細な `remove` / `retain` /
`migrate-in-place` の判断は [pre-release compatibility inventory](pre-release-compatibility-inventory.md)
に記録した。以下では、未更新だった候補の状態と、まだ残る Phase 3
候補を区別して記録する。

## 0. Genex クレート名前空間移行の監査（2026-10-01）

workspace の `cargo metadata --no-deps` で 123 パッケージを確認し、121 個の
`codex-*` パッケージを `genex-*` へ移行する。workspace の依存宣言と各 Rust
crate namespace が producer/consumer であり、`codex` binary、protocol/schema、
永続化識別子、provider-owned contract は別の移行段階に属するため変更しない。
`app_test_support` と `core_test_support` は製品 namespace ではないため保持する。

対象ディレクトリは `crates/genex-api`、`crates/genex-client`、
`crates/genex-experimental-api-macros`、`crates/genex-home`、
`crates/genex-mcp` であり、これらを `crates/genex-*` へ移動する。

この slice の decision は `migrate-in-place` とし、provider capability の削除や
protocol/persisted-data の互換判断を混ぜ込まない。

### 0.1 Genex home instruction provider symbol migration (2026-10-01)

`genex-home` の `CodexHomeUserInstructionsProvider` は workspace 内の public
Rust API であり、`core-api` の re-export、`app-server`、`cli`、core の test
support/suite が consumer であることを確認した。private field と loader helper
も同じ crate 内だけで消費されるため、producer/consumer を同一 slice で移行する。
一方、`CODEX_HOME` 環境変数と `find_codex_home` は外部設定互換境界なので、今回の
Rust symbol 移行には含めない。decision は `migrate-in-place` とし、
`just check --tests` と `just check` で確認する。

### 0.2 Genex client Rust alias migration (2026-10-01)

`genex-client` と `genex-login` の `CodexHttpClient` / `CodexRequestBuilder` は、
汎用 `genex-http-client` 型への workspace-owned public re-export である。検索上の
consumer は re-export 自身だけで、HTTP実装、originator、環境変数、wire header は
別の境界として保持する。型エイリアスのみ `GenexHttpClient` /
`GenexRequestBuilder` へ `migrate-in-place` し、`just check --tests` と
`just check` で確認する。

## 0.3 Genex file and directory name audit (2026-10-01)

The remaining 73 tracked path entries containing `codex` were audited by
producer/consumer boundary:

| Path group | Producer/consumer audit | Decision |
|---|---|---|
| `crates/skills/**/agents/genex.yaml`, `references/genex_yaml.md`, and `generate_genex_yaml.py` | Skills loader, capability discovery, plugin/skill tests, sample assets, and skill-creator documentation are all repository-local consumers. The metadata filename has no required persisted-data compatibility in this pre-release fork. | `migrate-in-place` to `genex.yaml`, `genex_yaml.md`, and `generate_genex_yaml.py` |
| `crates/tui/frames/codex/` | `crates/tui/src/frames.rs` is the sole compile-time producer/consumer of the fixture directory; no external file contract is involved. | `migrate-in-place` to `frames/genex/`; keep the Rust constant rename separate if it changes symbol scope |
| `crates/app-server/src/codex_home_metrics.rs` and `crates/config/src/codex_home_symlink.rs` | Each file is consumed only by its owning crate's module declaration and local call sites. The metrics behavior and sandbox symlink security behavior remain unchanged. | `migrate-in-place` filenames/module paths; preserve public security function names until their API slice |
| `crates/core/src/codex_thread.rs` and `crates/core/src/codex_delegate.rs` | Both are private core modules. Their local `crate::codex_*` imports and module declarations are the only Rust path consumers; the re-exported `CodexThread` types and `run_codex_thread_*` functions remain unchanged in this filename/module-path slice. | `migrate-in-place` to `genex_thread.rs` / `genex_delegate.rs` and `crate::genex_*` module paths |
| `crates/core/tests/common/test_genex.rs`, `test_genex_exec.rs`, and `crates/core/tests/suite/genex_delegate.rs` | Test-support module declarations and workspace integration tests are the consumers. Hawk source-location records are generated audit artifacts and must be refreshed or explicitly recorded after the path move. | `migrate-in-place` filenames and module paths; preserve test helper function/type names unless separately audited |
| `scripts/mcp_conformance/run_genex_compliance.py`, `genex_conformance_adapter.py`, and `test_genex_compliance.py` | Conformance README, Python imports, subprocess path construction, and tests are repository-local consumers. The `codex features enable` command inside the scripts is a binary surface and is not renamed here. | `migrate-in-place` script filenames/imports to Genex names; preserve CLI command names |
| `crates/skills/src/assets/samples/imagegen/references/genex-network.md` | The sample imagegen skill and its local CLI reference docs are the only consumers; this is repository-local documentation bundled as a sample asset, not a network or security protocol contract. | `migrate-in-place` to `references/genex-network.md` |
| `codex-cli.json`, CLI `codex__*.snap`, `crates/core/src/session/snapshots/genex_core__codex_tests__*.snap`, and `hawk.json`/Hawk source-location records | These are generated findings or snapshots keyed to the current `codex` binary, test target, or source symbols. Renaming them now would mix the later binary/observability/generated-audit boundary into the file-name slice. | `retain` until binary/observability and generated-artifact stages; record source-path refresh separately |
| `crates/code-mode-protocol/src/grpc/codex.code_mode.v1.proto`, `crates/execpolicy/examples/example.codexpolicy`, `crates/windows-sandbox-rs/codex-windows-sandbox-setup.manifest` | Protocol filename, security policy extension, and Windows setup manifest/binary contract are consumed by wire/build/security paths. | `retain` external/protocol/security contract |
| `crates/core/*codex*prompt*`, model-instruction templates, `docs/hawk-codex-cli-result-analysis.md`, `docs/superpowers/plans/*codex-only*`, and `scripts/debug-codex.sh` | Model identifiers, historical audit/plan filenames, and a user-configured VS Code script path are external or historical documentation/tooling names, not internal Genex namespace boundaries. | `retain`; re-audit in user-facing/docs or model-contract stages |

## 0.4 All-crate Rust symbol coverage (2026-10-02)

`cargo metadata --no-deps` の全 123 workspace member を対象に Rust source を走査した。
82 member は `Codex` / `codex_*` の Rust source match を持ち、41 member は
`no-match` と確認した。詳細な symbol、match count、ファイル一覧は
`target/genex-symbol-audit/rust-symbols.tsv` と
`target/genex-symbol-audit/workspace-coverage.tsv` に生成している。

今回の全クレート対応の coverage contract は次のとおり。

- 82 の symbol-bearing member は、leaf/private、low-fan-out public、crate-local
  shared model、high-fan-out core/API、protocol/auth/CLI/UI のいずれかの wave に割り当てる。
- 41 の `no-match` member は、Rust symbol の対象外として監査済みとする。
- `genex-core`、`genex-protocol`、`genex-app-server-protocol`、`genex-login`、
  `genex-config`、`genex-rollout`、`genex-rollout-trace`、`genex-state`、
  `genex-thread-store`、`genex-cli`、`genex-tui` は高 fan-out または外部境界候補として
  初回 rename wave から分離する。
- `genex-app-server-test-client`、`genex-git-utils`、`genex-network-proxy`、
  `genex-linux-sandbox`、`genex-install-context`、`genex-history` は、private/test-only
  または低 fan-out consumer を先に監査する初回候補とする。

`no-match` member は次の 41 件である。

`genex-agent-roles`, `genex-ansi-escape`, `genex-app-server-protocol-noop-macros`,
`genex-attachment-store`, `genex-aws-auth`, `genex-code-mode`, `genex-code-mode-runtime`,
`genex-collaboration-mode-templates`, `genex-config-schema`, `genex-context-fragments`,
`genex-diagnostics`, `genex-exec-server-test-support`, `genex-experimental-api-macros`,
`genex-file-search`, `genex-file-system`, `genex-file-watcher`, `genex-home`,
`genex-keyring-store`, `genex-mermaid`, `genex-otel-trace-websocket`,
`genex-response-debug-context`, `genex-shell-escalation`, `genex-stdio-to-uds`,
`genex-tcp-tunnel`, `genex-terminal-detection`, `genex-uds`, `genex-user-verification`,
`genex-utils-absolute-path`, `genex-utils-audio`, `genex-utils-cache`, `genex-utils-elapsed`,
`genex-utils-fuzzy-match`, `genex-utils-git-discovery`, `genex-utils-image`,
`genex-utils-json-to-toml`, `genex-utils-output-truncation`, `genex-utils-pty`,
`genex-utils-redacted-string`, `genex-utils-sandbox-summary`, `genex-utils-stream-parser`,
`genex-utils-string`.

## 0.5 Genex Rust symbol waves (2026-10-02)

| Symbol family | Producer/consumer audit | Decision |
|---|---|---|
| `genex-app-server-test-client::CodexClient`, leaf private helpers, CLI test command helpers, and TUI resize test helper | Definitions and consumers are private to test/client crates; no wire, persisted, or external binary name is changed. | `migrate-in-place` to Genex names (completed in Task 1) |
| `genex-install-context::{CodexPackageLayout,CodexPackageManifest}` | Consumers are `genex-arg0`, `genex-linux-sandbox`, `genex-cli`, and `genex-app-server-daemon`; manifest field names, install paths, and binary names remain unchanged. | `migrate-in-place` to `GenexPackage*` (completed in Task 2) |
| `genex-core::{CodexResponsesMetadata,CodexResponsesRequestKind,CodexTurnMetadataPayload}` and `genex-exec::CodexStatus` | All workspace consumers were enumerated. Rust type names are not wire keys or JSONL status values; `x-codex-*` metadata keys and event values remain unchanged. | `migrate-in-place` to `Genex*` (completed in Task 3) |
| `genex-core::{CodexThread,CodexThreadSettingsOverrides}` and private `run_codex_thread_*` runners | Consumers include core-api re-exports, app-server, TUI, extensions, CLI, and integration tests. Thread IDs, rollout/session paths, persisted fields, and protocol names are separate boundaries and remain unchanged. | `migrate-in-place` to `Genex*` (completed in Task 4) |
| `genex-rollout-trace::{CodexTurn,CodexTurnId,CodexTurnTraceEvent,codex_turn_trace_event}` and `codex_turn_id` serialized fields | Trace model producers/reducers and raw event payloads use these names in serialized/persisted structures. | `defer` to protocol/persisted-data audit; retain serialized names until that decision |
| `genex-protocol::{CodexErr,CodexErrorDetails}` and the public `CodexResult` alias used by protocol request/reply channels | `genex-protocol` defines the public error/result surface; core, app-server, CLI, TUI, extensions, and integration tests consume it. Error display text, discriminants, and request/reply types cross crate boundaries even where the Rust names are not serialized directly. | `defer` to the protocol/error contract audit; retain current names until the public error surface and generated consumers are reviewed |
| `genex-protocol::protocol::CodexErrorInfo` and `genex-app-server-protocol::protocol::v2::CodexErrorInfo` | Core maps runtime errors into the protocol enum; app-server translates that enum into the camelCase v2 schema and `codexErrorInfo` response/event fields. Legacy rollout replay also deserializes `ThreadRollbackFailed`. | `defer` to the app-server schema and persisted-history audit; preserve Rust and serialized compatibility names for now |
| `genex-login::CodexAuth` and its `core-api` re-export | Login/auth storage, external bearer auth, model-provider auth selection, Bedrock auth, models-manager, core-plugins, CLI, extensions, and tests consume the enum. Its variants encode active API-key, header, and Bedrock capabilities; removing or renaming it without an auth-contract migration would mix naming with capability changes. | `retain` current auth contract and `defer` a provider-neutral auth naming task; do not rename in the Genex symbol wave |
| `genex-history::CodexHarnessMetadata` | History envelopes and rollout payloads persist the metadata; history tests assert its JSON schema definition. The type name is therefore coupled to persisted/schema consumers despite being workspace-owned. | `defer` to the persisted-data/schema audit; preserve serialized metadata compatibility until migration policy is explicit |
| Runtime/config fields `codex_home`, `codex_self_exe`, and `codex_linux_sandbox_exe` | Auth storage, rollout/history paths, exec-server, sandbox/security, core config, CLI, TUI, and app-server produce and consume these values. They are path, environment, and security contracts rather than internal Rust namespace names. | `retain` current path/environment/security names; defer any change to the CLI/config and platform-contract audit |

## 0.6 User/integration boundary slices (2026-10-02)

| Name family | Producer/consumer audit | Decision |
|---|---|---|
| TUI-internal `AppEvent::{CodexOp,GenexOp}` transition, `CodexOpTarget`, and private `codex_bin`/`codex_pane` locals | `AppEvent` and its operation target are confined to the TUI event bus; session logging explicitly skips operation events, and the names are not serialized into app-server or rollout contracts. The two locals are test/CLI implementation details. | `migrate-in-place` to `GenexOp`/`GenexOpTarget` and Genex locals (completed in Task 6) |
| `codex` executable/argv-0 names, CLI command/help text, and CLI snapshot basenames | CLI launchers, install layouts, `assert_cmd` tests, shell/tooling entry points, and snapshots consume the executable and user-facing names. A rename changes the binary contract and must be coordinated with packaging and generated snapshots. | `defer` to an independent binary/CLI slice; retain current executable and command names until producer/consumer migration is explicit |
| `CODEX_HOME`, local `codex_home` bindings, config filenames, rollout/session roots, and `codex_self_exe`/`codex_linux_sandbox_exe` | Environment/config/path values are produced by CLI, auth, history, sandbox, exec-server, app-server, and tests. Local variable spelling is mixed with persisted and security paths, so a textual bulk replacement is unsafe. | `retain` environment/path contracts; defer config/env/platform slice and keep local-only renames separate |
| TUI `TerminalTitleItem::CodexVersion`, `StatusSurfacePreviewItem::CodexVersion`, and visible “Codex application” labels | Status-line configuration, preview rendering, terminal titles, and snapshots consume these identifiers and labels. They are user-visible UI/config vocabulary, not private Rust-only symbols. | `defer` to a UI/docs/fixture/snapshot slice; preserve current display/config vocabulary for now |
| `codex_otel`, `codex_plugin_*`, `codex_apps`, and `codex_app_server_daemon` strings | These occur in telemetry/analytics event names, MCP connector tool/server names, client identity fixtures, and app-server integration tests. They are external observability, MCP, or integration contracts rather than internal namespace names. | `retain` or `defer` per telemetry/MCP/integration producer; do not rename in the user-boundary preparation slice |
| `codex` content and filenames in docs, fixtures, snapshots, generated schemas, wire fields, rollout/session records, and SQLite/JSON identifiers | These names are consumed by documentation/tooling, generated artifacts, protocol clients, persisted readers, and replay/state stores. | `defer` to Task 7/8 audits; inventory both content and path names before any migration |

## 0.7 Protocol, schema, persisted-data, and external-contract audit (2026-10-02)

| Contract family | Producer/reader/writer audit | Decision |
|---|---|---|
| `crates/code-mode-protocol/src/grpc/codex.code_mode.v1.proto`, `package codex.code_mode.v1`, and `tonic::include_proto!` | The proto package and generated gRPC bindings are wire/build inputs; code-mode session types also describe a durable Codex thread/session boundary. | `retain` current package, file, and generated binding names; any migration requires a versioned protocol task |
| app-server generated schema titles `CodexAppServerProtocol` / `CodexAppServerProtocolV2`, checked-in JSON/TypeScript schema, `codexHome`, and schema-generator environment keys | Rust producers generate the checked-in schema, app-server clients consume it, and schema fixture tests compare the generated output. `just generate` produced no diff in this audit. | `retain` generated and wire names; defer any product-title or field rename to a generated-schema migration with consumer coordination |
| `codexErrorInfo`, core/app-server `CodexErrorInfo`, `codex_turn_id`, `CodexHarnessMetadata`, and `ThreadRollbackFailed` | Core produces error/turn events; app-server converts them to v2 responses and thread-history items; rollout/history/thread-store readers and writers replay persisted data and schema fixtures cover legacy values. | `retain` serialized fields/variants; defer Rust-name changes to an explicit protocol and persisted-data migration |
| `codex_home` across rollout/session index, thread-store, state SQLite, auth storage, and app-server | CLI/config produces the home root; rollout and state write/read files and databases below it; thread-store and app-server resolve the same root. | `retain` path/config contract; no local-variable bulk rename in the protocol task |
| persisted origin/observability values such as `codex_cli_rs`, `codex_state`, and existing `codex_*` metadata keys | State extraction, log filters, telemetry, and replay fixtures produce or inspect these values. They are data/observability identifiers, not unused Rust aliases. | `retain` current values; defer any cleanup to the persisted/observability compatibility inventory |
| `.codex` project folders, code-mode/API URLs, marketplace/plugin identifiers, and `codex_apps` fixture names | Config discovery, app-server fixtures, MCP/plugin consumers, and external URLs use these strings. Some represent third-party or historical fixtures rather than an internal Genex namespace. | `retain` or defer per producer/consumer; do not rename as part of the Rust symbol migration |

## 0.8 Final all-crate Genex audit (2026-10-02)

The final coverage artifact contains all 123 workspace members: 82 members with Rust `Codex`/`codex_*` matches are `reviewed-in-inventory`, and 41 members are explicit `no-match`; `unreviewed=0`. Every residual name is covered by a recorded `migrate-in-place`, `retain`, or `defer` decision in the wave tables above. The remaining 19 path names are the previously audited generated, protocol, security, model/history, and tooling contracts; they are not unclassified path leftovers.

The final audit intentionally leaves `CODEX_HOME`, persisted identifiers, wire/schema names, security/platform fields, provider-owned/external contracts, and user-facing binary/config vocabulary unchanged until their independent migration decisions are approved.

## 1. 参照元が見つからないクレート・ソース

| 対象 | 現時点の観測 | 現時点の扱い |
|---|---|---|
| `codex-utils-readiness` | workspace に登録されていたが、他の workspace member からの依存・参照がなく、実装と自身のテストだけが残っていた。producer/consumer 監査後に workspace 登録、本体、lockfile、checked-in Hawk finding を削除済み | 置換なし。未参照クレートを削除 | `remove (completed)` |
| `codex-thread-manager-sample` | 他の workspace member から利用されず、製品 runtime や必須 test entrypoint からも利用されない standalone binary。README と entrypoint 文書は配布する場合の説明に留まり、現行の必須 consumer ではない。workspace、lockfile、ソース、entrypoint 文書から削除済み | 置換なし。ThreadManager サンプルを削除 | `remove (completed)` |
| `LoopbackResponsesServer` と `plugin_analytics_capture_tests.rs` | いずれも削除済みの plugin analytics smoke test の残骸。前者は module 登録だけが残り、後者は module 登録されず削除済みシンボルを参照していた。`genex-cli.json` には前者の古い Hawk finding が2件残っていた | 置換なし。孤立した test helper、孤立 test source、生成済み finding を削除 | `remove (completed)` |
| `crates/tui/src/app/backend_banner_fallback.rs`、`BackendBanner`、`ActionableBanner` | production caller がなく、ChatGPT の利用量・課金導線・Reserve 自動復旧に結び付いた TUI 実装と孤立 snapshot だけが残っていた | 汎用 rate-limit/status と通常の model picker は残し、Banner と Reserve 自動復旧を削除 | `remove (completed)` |

確認対象の主なパスは次のとおり。

- `crates/utils/readiness/`
- `crates/thread-manager-sample/`
- `crates/app-server-test-client/src/loopback_responses_server.rs`
- `crates/app-server-test-client/src/plugin_analytics_capture_tests.rs`
- `crates/tui/src/app/backend_banner_fallback.rs`（削除済み）
- `crates/tui/src/chatwidget/backend_banners.rs`（削除済み）
- `crates/tui/src/chatwidget/luna_reserve_return.rs`（削除済み）

## 2. デフォルトオフまたは削除済み feature の残存候補（監査完了）

### production consumer が見つからない、または互換 no-op に見えるもの

次の feature は、現行 Rust production code から直接利用されていないか、`Features::apply_map` の互換 no-op として残っていた。producer/consumer
監査の結果、旧 feature の registry/spec/schema/no-op は削除済みである。

- `transcript_v2`
- `code_mode_buffered_exec`
- `js_repl`
- `js_repl_tools_only`
- `search_tool`
- `codex_git_commit`
- `local_thread_store_shared_compression`
- `apply_patch_freeform`
- `request_rule`
- `multi_agent_mode`
- `enable_fanout`
- `unavailable_dummy_tools`
- `plugin_hooks`
- `external_migration`
- `skill_env_var_dependency_prompt`
- `image_detail_original`
- `workspace_owner_usage_nudge`
- `responses_websockets`
- `responses_websockets_v2`
- `tui_app_server`

削除したのは旧 feature compatibility layer だけであり、同名に近い現行
capability は次のように保持している。

- `multi_agent_mode`、`plugin_hooks`、`external_migration` は、現行の
  multi-agent、plugin hooks、external-agent import の consumer を保持
- `search_tool`、`image_detail_original`、`responses_websockets*` は、現行の
  provider/model capability と `supports_websockets` に移行
- `use_legacy_landlock`、`recommended_plugins`、`workspace_dependencies`、
  code-mode、MCP、plugin runtime は現行 consumer または security fallback として保持

検証は `crates/features/src/lib.rs`、`crates/core/config.schema.json`、
CLI/managed-feature consumer の参照監査、`just generate`、
`just check --tests`、`just check` で完了している。

### 名前だけでは削除しないもの

- `EnableMcpApps` はApps capability削除時に残った未使用のfeature登録であり、今回の残存コード掃除で削除する。
- `use_legacy_landlock` は Linux の実行時 security fallback と結び付くため、互換 no-op とみなさない。
- `in_app_*`、`browser_use_*`、`computer_use` は requirements-only gate として config/app-server 側に consumer があるため、registry だけを見て削除しない。
- `recommended_plugins`、`workspace_dependencies`、code-mode、MCP、plugin runtime など、デフォルトオフでも現行 entrypoint やテストから到達できる機能は別扱いにする。

## 3. provider バックエンド依存の capability 候補

### 3.1 ChatGPT の利用量・Reserve 自動切替・復旧 UI（Banner 部分は削除済み）

対象候補は TUI の rate limit、thread usage、backend banner、Reserve model return state、status card、ChatGPT usage URL、`requires_provider_auth` 判定である。

主な確認先:

- `crates/tui/src/chatwidget/rate_limits.rs`
- `crates/tui/src/chatwidget/thread_usage.rs`
- `crates/tui/src/chatwidget/backend_banners.rs`（削除済み）
- `crates/tui/src/chatwidget/luna_reserve_return.rs`（削除済み）
- `crates/tui/src/backend_banners/actions.rs`（削除済み）
- `crates/tui/src/status/card.rs`
- `crates/tui/src/status/account.rs`

通常の status、model catalog、provider-neutral な usage 表現まで削除するのではなく、ChatGPT account/plan、固定 URL、provider 専用 recovery state の到達経路を分けて監査する。

BackendBanner の parser/CTA/render、bottom pane の専用 inline banner、Reserve 自動切替・復帰状態・関連 snapshot は `remove (completed)` とした。通常の rate-limit 表示、status card、手動の model catalog はこの削除に含めていない。

### 3.2 provider Curated / Bundled marketplace

監査の結果、provider 固有 marketplace surface は次の active boundary と test-only
残骸に分かれた。

- `crates/core-plugins/src/lib.rs` の `codex-bundled`、`codex-primary-runtime` と、対応する
  `marketplace_policy` の予約名・managed path 判定は、bundled plugin ローダー、trusted
  script attribution、artifact telemetry が現在も消費しているため保持する。
- `crates/app-server/src/request_processors/plugins/search.rs` に provider 固有の built-in
  scope はなく、local/Git/npm marketplace の検索基盤だけが残る。
- `crates/tui/src/chatwidget/plugin_catalog.rs` の provider Curated tab は既に削除済みで、
  残っていた Apps help URL と disclosure line も削除した。
- `core-plugins` と TUI の test-only curated marketplace 名は `test-curated`/
  `test-api-curated`/`Test Marketplace` へ移行した。

汎用の local/Git/npm plugin loader、hook、tool、trust/security policy は別の保持候補であり、`genex-core-plugins` 全体を削除する根拠にはしない。

### 3.3 first-party auth と model provider adapter

次のクレートは provider 固有部分と provider-neutral 部分が混在している。

- `genex-login`: ChatGPT JWT、plan/account ID、managed auth、`PROVIDER_API_KEY`
- `genex-model-provider-info`: `create_responses_provider`、`requires_first_party_auth`
- `genex-model-provider`: provider models endpoint、ChatGPT account header、provider auth predicate
- `genex-app-server-protocol`: ChatGPT auth/account や provider 固有 protocol 型

Bedrock、custom `model_providers`、汎用 credential storage、Responses-compatible provider の consumer があるため、クレート全体の削除ではなく first-party adapter の分割・削除を先に検討する。

モデル一覧については、`MyModelProvider` も `ModelProvider::models_manager()` 経由で provider の `/models` を動的取得するよう移行済み。provider の bundled catalog と暗黙 fallback は削除し、明示的な `model_catalog_json` と Bedrock の provider-owned catalog だけを静的経路として保持する。

### 3.4 Apps / host-owned MCP

Apps、`codex_apps`、Apps entitlement、host-owned Apps MCP の現行経路とApps専用の孤立テストは削除済みである。`genex-mcp` 全体は通常のMCP、OAuth/elicitation、provider-neutral metadata/security behaviorのため保持する。

provider account/entitlement、host-owned Apps MCP、Apps 固有 UI/protocol と、通常の MCP、OAuth/elicitation、provider-neutral metadata/security behavior を分けて監査する。

## 4. 現時点で削除候補としない共有基盤

- `genex-api`、`genex-model-provider-info`、`genex-model-provider` の全体
- `genex-http-client`、`genex-mcp`、`genex-core-plugins` の全体
- `genex-otel`、通常の MCP、local/stdio exec-server、sandbox/security behavior
- `crossterm`、`tokio-tungstenite`、`tungstenite` の provider fork dependency

これらは provider 由来の名前や adapter を含むものの、現行の custom provider、Bedrock、local execution、plugin/MCP、platform fallback から利用されている。provider 固有 capability を外した後に依存元がなくなった場合のみ、クレート単位の削除を再評価する。

## 5. 次回以降の精査順序

各単位で、先に [pre-release compatibility inventory](pre-release-compatibility-inventory.md) の順序に従って producer/consumer を記録する。

1. 参照元のないクレート・ソースと generated artifact を確認する。`codex-utils-readiness`、`codex-thread-manager-sample`、`LoopbackResponsesServer`、plugin analytics の孤立 source、BackendBanner と関連 snapshot、Apps専用の孤立テスト群は削除済み。
2. persisted/config/schema に残る削除済み feature の互換層を確認する。**完了**。
3. protocol/app-server の互換 alias・schema は**完了**。ただし ChatGPT account/auth の現行 protocol surface は、次の first-party auth 境界で別途判断する。
4. TUI の BackendBanner/Reserve recovery と hosted/curated marketplace UI は**完了**。
   通常の status/account 表示と active bundled/runtime marketplace boundary は保持する。
5. login/model-provider の first-party auth adapter は、共有 auth capability の名称を
   `requires_auth` に移行済みで、ChatGPT/workload-identity の runtime consumer と
   token auth は `d22689112` で削除済み。API Key、Bedrock、外部Bearer、
   provider-neutralなcredential/MCP OAuthは保持する。残る作業は未登録テストソース、
   obsolete な環境変数マーカーの cleanup であり、generated index の更新は Hawk の
   更新をユーザーが明示的に依頼した場合に行う。
   Apps/MCP の専用 adapter は削除済みで、通常の MCP/OAuth/elicitation/security は保持済み。
   Session/history compatibility の監査では、`crates/rollout/src/lib.rs` の
   ChatGPT/Atlas 専用 interactive filter entries を削除し、汎用の persisted
   reader、remote resume の payload redaction、Codex-only product filtering は保持した。
   詳細は compatibility inventory の `Persisted session/history compatibility`
   セクションに記録した。
6. 未完了の各項目について producer/consumer を台帳に記録し、`remove`、`retain`、`migrate-in-place` を決めた後、`just check`、必要に応じて `just check --tests`、生成物があれば `just generate` を実行する。

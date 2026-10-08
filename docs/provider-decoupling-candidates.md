# 未使用・provider 依存候補の追加調査メモ

この文書は、[provider 固有機能の段階的な分離計画](provider-decoupling-plan.md) に対する 2026-09-27 時点の追加調査結果を記録する。

ここに記載した項目は **削除・保持・移行の決定ではない**。個別の producer/consumer 監査、別バイナリからの到達性、動的登録、生成物、外部利用の確認を行った後、[pre-release compatibility inventory](pre-release-compatibility-inventory.md) に `remove`、`retain`、または `migrate-in-place` として記録する。

## 調査範囲と検証

- workspace の `Cargo.toml`、`cargo metadata --no-deps` による内部依存元の確認
- `Feature` registry、`apply_map`、生成済み config schema の参照確認
- `provider`、`chatgpt`、固定 endpoint、provider 専用 header、managed auth の production code 検索
- `just check`：成功、エラーなし、未使用警告 114 件
- `just check --tests`：成功

## Genex 全クレート監査の基準更新（2026-10-03）

Genex 命名移行では、repository-owned の protocol/schema、永続化識別子、
CLI/config、UI、package/build 名も、producer/consumer 監査後に
`migrate-in-place` または `remove` の対象とする。public、高 fan-out、
serialized という理由だけで `retain` や `defer` にはしない。

`cargo metadata --no-deps` で 123 package を確認し、各 package directory を
`rg -ni 'codex'` で再走査した結果、76 member に match、47 member は
`no-match` だった。Rust source の識別子走査では 74 member に match、49 member
は `no-match` だった。以前の 82/41 の Rust coverage 記録は stale として、
`target/genex-symbol-audit/workspace-coverage.tsv`、
`target/genex-symbol-audit/rust-symbols.tsv`、
`target/genex-symbol-audit/dependency-order.tsv` の結果で置き換える。

保持できるのは、OpenAI endpoint 固有の URL/request-response field/auth/model、
外部 OpenAI/upstream・第三者 protocol/package、歴史的・法的記録、別単語内の
部分文字列、外部 repository URL、または変更により既存の外部/security stateを
壊す識別子に限る。命名監査中に provider-specific capability、endpoint、auth、
telemetry、hosted assumption を見つけた場合は、命名変更へ混ぜず本メモの独立
follow-up と compatibility inventory の decision に記録する。

### Genex Rust 境界移行の再計画と実装 (2026-10-04)

Rust source の残存 `codex` を、repository-owned な内部名と外部契約に分けて
再監査した。内部の env/定数、protocol・persisted marker、telemetry の内部名、
originator、CLI/TUI/test fixture は producer/consumer を同一 wave で
`migrate-in-place` とする。Responses API の `codex_rollout_budget_units` は
`genex-api` の serde adapter だけに残し、内部の usage 型は
`genex_rollout_budget_units` に変換する。

明示的に `retain` するのは、Responses の wire field/header/auth/model、
third-party MCP/Agent Plugin/package 識別子、OpenAI/upstream URL、OS sandbox・
security namespace、既存の外部 telemetry/MCP contract、履歴・法的記録、および
旧形式を検証する test fixture である。
この分類に基づく実装では、Rust の repository-owned residual audit は
`scripts/audit_genex_rust_names.py` で実行し、内部 Rust 名の未分類残存を失敗扱いに
する。実装前の RED audit は 175 行、移行後の Rust source の `codex` 残存は
1,747 occurrences（1,539 lines）で、残りは上記の retain boundary として個別に
台帳化する。

### Genex repository boundary expansion (2026-10-04)

前節の「外部契約として保持」とした項目を、Responses API の汎用化方針と
repository-owned な plugin/auth 境界の再監査に基づいて更新する。この節が前節の
該当する暫定判断を上書きする。

- `.codex-plugin` は Agent Plugin の外部標準ディレクトリではなく、Genex の
  manifest overlay、private metadata、command/skill migration metadata を読む
  repository-owned な内部境界である。`.genex-plugin` へ移行する。
- `codex-bundled`、`codex-bundled-alpha`、`codex-primary-runtime`、
  `codex-runtimes` は、bundled hooks、artifact runtime、marketplace cache を
  同一 repository 内で登録・解決する namespace/path である。それぞれ
  `genex-*` へ移行する。`.claude-plugin`、`.cursor-plugin`、root `plugin.json`
  と Agent Plugin schema URL は外部標準なので変更しない。
- Responses の `codex_rollout_budget_units`、`x-codex-*` と Responses 用の
  internal header/metadata key は endpoint 固有の必須名ではなく、Genex 側で
  汎用化する wire producer/consumer として `genex` へ移行する。
- `CODEX_API_KEY`、`CODEX_ACCESS_TOKEN`、GitHub/Connectors token、`CODEX_AUTH`、
  `Codex Auth` keyring service、`CODEX_INTERNAL_ORIGINATOR_OVERRIDE` は、本 fork の login、proxy、keyring、
  test/config consumers を同時更新できる repository-owned auth namespace として
  `GENEX_*` へ移行する。token の意味や secret redaction/security behavior は変更しない。

この追加 wave の decision は全て `migrate-in-place` とし、旧値を読む互換層は追加しない。

### 0.6 依存順 leaf wave の decisions (2026-10-03)

- `genex-execpolicy` の standalone binary、README、description、local migration
  variable は repository-owned のため `migrate-in-place` とした。policy matcher と
  security behaviorは維持する。
- `genex-shell-escalation` の wrapper binary、内部 escalation socket env、test
  variable、in-repository zsh helper/release labels は producer/consumer が全て
  repository 内にあるため Genex へ移行した。execve/escalation protocolの挙動は変更しない。
- `genex-tcp-tunnel` の README に残っていた `codex tcp-tunnel` は CLI producer と一体で
  `genex tcp-tunnel` に移行済み。`genex-user-verification` の `com.codex.codex` は OS
  keychain/lock namespaceのため `retain` とする。
- `CODEX_HOME` / `.codex` / `find_codex_home` は high fan-out を理由に保留せず、
  home/config/persisted consumerを同じ横断 waveで `migrate-in-place` する対象とした。
  `codex-env` は path URI compatibility scheme のため retainする。

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
`CODEX_HOME` 環境変数、`find_codex_home`、`.codex`、および内部の
`codex_home` bindings は repository-owned の home/config/persisted family として、
producer/consumer を同一 cross-crate wave で `migrate-in-place` する。外部 URL、
`codex-env` scheme、OS/security namespace、`.codex-plugin` のような別単語の部分文字列は
変更しない。

### 0.2 Genex client Rust alias migration (2026-10-01)

`genex-client` と `genex-login` の `CodexHttpClient` / `CodexRequestBuilder` は、
汎用 `genex-http-client` 型への workspace-owned public re-export である。検索上の
consumer は re-export 自身だけで、HTTP実装、originator、環境変数、wire header は
別の境界として保持する。型エイリアスのみ `GenexHttpClient` /
`GenexRequestBuilder` へ `migrate-in-place` し、`just check --tests` と
`just check` で確認する。

## 0.3 Genex file and directory name audit (2026-10-01)

The remaining product-scoped path entries containing `codex` were audited by
producer/consumer boundary:

| Path group | Producer/consumer audit | Decision |
|---|---|---|
| `crates/skills/**/agents/genex.yaml`, `references/genex_yaml.md`, and `generate_genex_yaml.py` | Skills loader, capability discovery, plugin/skill tests, sample assets, and skill-creator documentation are all repository-local consumers. The metadata filename has no required persisted-data compatibility in this pre-release fork. | `migrate-in-place` to `genex.yaml`, `genex_yaml.md`, and `generate_genex_yaml.py` |
| `crates/tui/frames/codex/` | `crates/tui/src/frames.rs` is the sole compile-time producer/consumer of the fixture directory; no external file contract is involved. | `migrate-in-place` to `frames/genex/`; keep the Rust constant rename separate if it changes symbol scope |
| `crates/app-server/src/codex_home_metrics.rs` and `crates/config/src/codex_home_symlink.rs` | Each file is consumed only by its owning crate's module declaration and local call sites. The metrics behavior and sandbox symlink security behavior remain unchanged. | `migrate-in-place` filenames/module paths; preserve public security function names until their API slice |
| `crates/core/src/codex_thread.rs` and `crates/core/src/codex_delegate.rs` | Both are private core modules. Their local `crate::codex_*` imports and module declarations are the only Rust path consumers; the re-exported `CodexThread` types and `run_codex_thread_*` functions remain unchanged in this filename/module-path slice. | `migrate-in-place` to `genex_thread.rs` / `genex_delegate.rs` and `crate::genex_*` module paths |
| `crates/core/tests/common/test_genex.rs`, `test_genex_exec.rs`, and `crates/core/tests/suite/genex_delegate.rs` | Test-support module declarations and workspace integration tests are the consumers. Hawk source-location records are generated audit artifacts and must be refreshed or explicitly recorded after the path move; the shared helper declaration wave was separately audited across these consumers. | `migrate-in-place` filenames, module paths, and shared test helper declarations to Genex names |
| `scripts/mcp_conformance/run_genex_compliance.py`, `genex_conformance_adapter.py`, and `test_genex_compliance.py` | Conformance README, Python imports, subprocess path construction, and tests are repository-local consumers. Official MCP scenario terminology and active auth variables are separate external contracts. | `migrate-in-place` local executable/home/parameter names to Genex; preserve official scenario terminology and `CODEX_API_KEY`/`CODEX_ACCESS_TOKEN` contracts |
| `crates/skills/src/assets/samples/imagegen/references/genex-network.md` | The sample imagegen skill and its local CLI reference docs are the only consumers; this is repository-local documentation bundled as a sample asset, not a network or security protocol contract. | `migrate-in-place` to `references/genex-network.md` |
| `codex-cli.json`, CLI `codex__*.snap`, `crates/core/src/session/snapshots/genex_core__codex_tests__*.snap`, and `hawk.json`/Hawk source-location records | These are generated findings or snapshots keyed to the current `codex` binary, test target, or source symbols. Renaming them now would mix the later binary/observability/generated-audit boundary into the file-name slice. | `retain` until binary/observability and generated-artifact stages; record source-path refresh separately |
| `crates/code-mode-protocol/src/grpc/codex.code_mode.v1.proto`, `crates/execpolicy/examples/example.codexpolicy`, `crates/windows-sandbox-rs/codex-windows-sandbox-setup.manifest` | Protocol filename, security policy extension, and Windows setup manifest/binary contract are consumed by wire/build/security paths. | `retain` external/protocol/security contract |
| `crates/core/*codex*prompt*`, model-instruction templates, `docs/hawk-codex-cli-result-analysis.md`, `docs/superpowers/plans/*codex-only*`, and `scripts/debug-codex.sh` | Model identifiers, historical audit/plan filenames, and a user-configured VS Code script path are external or historical documentation/tooling names, not internal Genex namespace boundaries. | `retain`; re-audit in user-facing/docs or model-contract stages |

The path count above is intentionally scoped to product, protocol, security, model/history,
and repository tooling paths. `.codex/` agent infrastructure, third-party fixtures, and
historical generated reports are counted separately and must not be treated as unresolved
Genex product paths.

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

### Contract-boundary vocabulary for the remaining declarations

ここでいう「外部」は単にインターネット上の利用者という意味ではない。対象を次のように分けて記録する。

| Declaration family | 具体的な契約相手 | 保持理由 |
|---|---|---|
| `GenexAuth` | workspace 内の `genex-core-api`、core、CLI、MCP、model-provider、および実行時の auth.json/keyring・provider 認証経路 | 公開 workspace API と認証状態・provider auth の producer/consumer が結合している |
| `GenexResponsesHeaders` | workspace 内の core/client・thread extension と、実行時の Responses HTTP/WebSocket endpoint | model-scoped auth/routing header の wire/endpoint 契約である |
| `GenexErrorInfo` | app-server protocol の workspace 内 producer/translator、workspace 外の app-server client、checked-in JSON schema、thread history reader | JSON/RPC schema と既存履歴の読み手が同じ名前を要求する |
| `GenexTurn*` / `GenexTurnTraceEvent` | rollout-trace reducer/writer、trace viewer/search/replay、ディスク上の serialized rollout trace | `genex_turn_id` を含む保存形式と replay の契約である |
| `GenexHarnessMetadata` | history/rollout/core の writer/reader、ディスク上の thread history、JSON Schema consumer | 保存済み履歴と schema definition 名に結合している |

したがって、以下の表で `retain` / `defer` としたものは、workspace 内の crate 間 APIだけでなく、上記の実行時・保存済み・workspace 外 consumer を含む。

| Symbol family | Producer/consumer audit | Decision |
|---|---|---|
| `genex-app-server-test-client::CodexClient`, leaf private helpers, CLI test command helpers, and TUI resize test helper | Definitions and consumers are private to test/client crates; no wire, persisted, or external binary name is changed. | `migrate-in-place` to Genex names (completed in Task 1) |
| `genex-install-context::{GenexPackageLayout,GenexPackageManifest}` and package-kind labels | Consumers are `genex-arg0`, `genex-linux-sandbox`, `genex-cli`, and `genex-app-server-daemon`; repository-owned package layout, test executables, fixture metadata, and path labels were migrated. The npm/bun/pnpm/Vite+ `codex.js` shim and Homebrew `codex` cask are external package contracts. | `migrate-in-place` repository-owned names; `retain` external `codex.js`/Homebrew package contracts |
| `genex-core::{CodexResponsesMetadata,CodexResponsesRequestKind,GenexTurnMetadataPayload}` and `genex-exec::CodexStatus` | All workspace consumers were enumerated. Rust type names are not wire keys or JSONL status values; `x-codex-*` metadata keys and event values remain unchanged. | `migrate-in-place` to `Genex*` (completed in Task 3) |
| `genex-core::{CodexThread,CodexThreadSettingsOverrides}` and private `run_codex_thread_*` runners | Consumers include core-api re-exports, app-server, TUI, extensions, CLI, and integration tests. Thread IDs, rollout/session paths, persisted fields, and protocol names are separate boundaries and remain unchanged. | `migrate-in-place` to `Genex*` (completed in Task 4) |
| `genex-rollout-trace::{GenexTurn,GenexTurnId,GenexTurnTraceEvent,genex_turn_trace_event}` and `genex_turn_id` serialized fields | Trace model producers/reducers, raw event payloads, and replay readers were migrated together; persisted field/tag names now use the Genex spelling. | `migrate-in-place` (completed) |
| `genex-protocol::{GenexErr,GenexErrorDetails}` and the public `GenexResult` alias used by protocol request/reply channels | Declaration-led audit found these are internal Rust error/result/detail names consumed across core, app-server, model-provider, sandboxing, linux-sandbox, extensions, and TUI tests; their type names are not wire keys, persisted fields, CLI/env names, security contracts, or external protocol values. `GenexErrorInfo` and `genex_error_info` remain separate schema boundaries. | `migrate-in-place` `CodexErr`/`CodexErrorDetails`/`CodexResult` to `GenexErr`/`GenexErrorDetails`/`GenexResult`; defer external error/schema names to their dedicated audits |
| `genex-protocol::protocol::GenexErrorInfo` and `genex-app-server-protocol::protocol::v2::GenexErrorInfo` | Core, app-server, checked-in JSON schemas, thread-history fixtures, and replay readers were migrated together; the external schema field is now `genexErrorInfo`. | `migrate-in-place` (completed) |
| `genex-core` shared test-support declarations `TestCodex*`, `TestCodexResponsesRequestKind`, `TestCodexHome`, `TempCodexHomeGuard`, and `test_codex*` | Cross-crate integration-test producers and consumers were audited across core, exec, app-server, queue, skills, and extension tests. These are test-only Rust fixtures; no production API, wire/schema field, persisted value, CLI/config name, or security contract depends on the helper identifier spelling. | `migrate-in-place` to `TestGenex*`, `TestGenexResponsesRequestKind`, `TestGenexHome`, `TempGenexHomeGuard`, and `test_genex*` |
| Test-only declarations `PtyCodex`, `TempCodexHome`, `TestCodexHelperPaths`, and `test_codex_helper_paths` | TUI PTY tests, rmcp OAuth tests, and exec-server filesystem/integration tests are the complete producer/consumer sets. The helper names are not production APIs; the guarded `CODEX_HOME` environment variable and executable paths remain unchanged contracts. | `migrate-in-place` to `PtyGenex`, `TempGenexHome`, `TestGenexHelperPaths`, and `test_genex_helper_paths` |
| Test-only helper functions `test_codex_home`, `temp_codex_home`, `test_codex_auth`, plugin fixture builders, remote/Bedrock builders, and tool/thread builders | Cross-crate consumers were enumerated in model-provider, state, core-plugins, core integration suites, and skills tests. These function identifiers only construct test fixtures; their `CODEX_HOME`, auth variants, Bedrock endpoint behavior, and plugin/runtime contracts are unchanged. | `migrate-in-place` to the corresponding `test_genex_*`, `*_genex_*`, and `build_genex_*` helpers |
| `genex-core::GenexResponsesHeaders` | The public core re-export, Responses client, thread-extension initialization, and model-scoped HTTP/WebSocket header path were migrated together; provider behavior and header values remain unchanged. | `migrate-in-place` (completed; provider endpoint/auth behavior retained) |
| `genex-login::GenexAuth` and its `core-api` re-export | Login/auth storage, external bearer auth, model-provider auth selection, Bedrock auth, models-manager, core-plugins, CLI, extensions, and tests were migrated together. API-key, header, and Bedrock capabilities remain unchanged. | `migrate-in-place` (completed; auth capabilities retained) |
| `genex-history::GenexHarnessMetadata` | History envelopes, rollout payloads, persisted thread history, and the JSON Schema definition were migrated together. | `migrate-in-place` (completed) |
| Runtime/config fields `codex_home`, `codex_self_exe`, and `codex_linux_sandbox_exe` | Auth storage, rollout/history paths, exec-server, sandbox/security, core config, CLI, TUI, and app-server produce and consume these fields. The field names are repository-owned; active executable fallback values and security behavior are separate retained boundaries. | `genex_home`, `genex_self_exe`, and `genex_linux_sandbox_exe`; preserve active fallback values and security behavior | `migrate-in-place` for repository-owned field/binding names; `retain` active platform fallback values | Cross-crate producer/consumer audit; `just check --tests`; `just check` |

## 0.6 User/integration boundary slices (2026-10-02)

| Name family | Producer/consumer audit | Decision |
|---|---|---|
| TUI-internal `AppEvent::{CodexOp,GenexOp}` transition, `CodexOpTarget`, and private `codex_bin`/`codex_pane` locals | `AppEvent` and its operation target are confined to the TUI event bus; session logging explicitly skips operation events, and the names are not serialized into app-server or rollout contracts. The two locals are test/CLI implementation details. | `migrate-in-place` to `GenexOp`/`GenexOpTarget` and Genex locals (completed in Task 6) |
| `codex` executable/argv-0 names, CLI command/help text, and CLI snapshot basenames | CLI launchers, install layouts, `assert_cmd` tests, shell/tooling entry points, and snapshots consume the executable and user-facing names. The repository-owned CLI bin target, test launchers, command/help text, doctor PATH detection, `genex-version`, and snapshot basenames were migrated to Genex; package-manager shims, Homebrew/desktop identities, and legacy install layouts remain retained contracts. | `migrate-in-place` for repository-owned CLI surface; `retain` external/package/security boundaries |
| `CODEX_HOME`, `CODEX_SQLITE_HOME`, local `codex_home` bindings, config filenames, rollout/session roots, and `codex_self_exe`/`codex_linux_sandbox_exe` | Environment/config/path producers and readers span CLI, auth, history, sandbox, exec-server, app-server, and tests. In this pre-release fork the home/config family is repository-owned; active platform fallback values and security behavior remain separate. | `GENEX_HOME`, `GENEX_SQLITE_HOME`, `.genex`, `genex_home`, `genex_self_exe`, and `genex_linux_sandbox_exe`; retain active platform fallback values | `migrate-in-place` for repository-owned home/config/binding names; `retain` active platform fallback/security values | Full producer/consumer home wave; `just generate`; `just check --tests`; `just check` |
| TUI `TerminalTitleItem::CodexVersion`, `StatusSurfacePreviewItem::CodexVersion`, and visible “Codex application” labels | Status-line configuration, preview rendering, terminal titles, history cells, update/version notices, and snapshots consume these identifiers and labels. They are repository-owned UI/application vocabulary, not private Rust-only symbols or external contracts. | `migrate-in-place` to `GenexVersion`, `GENEX_CLI_VERSION`, and visible Genex labels (completed in the Task 6 UI slice) |
| `codex_otel`, `codex_plugin_*`, `codex_apps`, and `codex_app_server_daemon` strings | Internal OTEL target names and the repository README/tests were migrated to `genex_otel`; remaining telemetry metric/event values and MCP/client identity strings still have external observability or integration consumers. The daemon client identity is workspace-owned and was migrated to `genex_app_server_daemon`; `codex_cli_rs` originator values remain a separate persisted/telemetry contract. | `migrate-in-place` internal OTEL and daemon client identity; `retain` remaining external telemetry/MCP/originator values |
| app-server daemon `managed_codex_*`, `managedCodex*`, `codex app-server daemon ...`, and version-parser fixtures | The daemon is the producer of lifecycle/bootstrap/update JSON and the CLI integration tests are its in-repository consumers. These implementation, serialized, and user-facing names are repository-owned. The standalone `current/codex` path remains an active fallback for previously launched legacy packages and must not be removed as part of this rename. | Genex-named daemon implementation/JSON/commands and `current/codex` fallback retained | `migrate-in-place` for repository-owned names; `retain` for active legacy-layout fallback | Audit all daemon/CLI producers and consumers before editing; run `just check --tests` and `just check` |
| CLI doctor `DoctorReport.codex_version`, JSON `codexVersion`, `codex doctor` help/remediation text, and remaining user-facing `Codex CLI` labels | `genex-cli` produces both doctor report shapes; the doctor renderers/tests and TUI notices consume them. The report field and user-facing command/application labels are repository-owned and are not OpenAI endpoint, auth, model, or persisted-data identifiers. | `genex_version`, `genexVersion`, `genex doctor`, and Genex CLI labels | `migrate-in-place` | Audit all CLI doctor/TUI producers and consumers; run `just check --tests` and `just check` |
| core MCP turn metadata `codex_version` in `_meta.x-codex-turn-metadata` | Core writes the key into external MCP `tools/call` metadata and MCP servers consume the extension. Renaming it would change an active third-party integration boundary; it is distinct from repository-owned CLI doctor version fields. | Existing MCP metadata key | `retain` | Preserve exact key/envelope; no code change in the Genex rename |
| TUI user-facing `Codex` labels, prompts, task descriptions, and test expectations | TUI renderers, prompts, task descriptions, and tests consume these labels. The application vocabulary is repository-owned; external URLs, OS/security identifiers, MCP namespaces, and pet/CDN catalog names must remain under their own audits. | Genex UI vocabulary; retain separately audited external/security/catalog names | `migrate-in-place` for UI vocabulary; `retain` for external/security/catalog names | Audit TUI source/tests with exact-word exclusions; run `just check --tests` and `just check` |
| TUI `has_genex_backend_auth`, `genex_rate_limit_reached_type`, and `genex_spend_control_reached` state | Startup/app-server event producers initialize these fields; usage/status and turn-error consumers use them to gate account activity and render provider-shaped quota semantics. The names identify a remaining provider-dependent capability, not just UI vocabulary. | Keep current capability for now; separately remove or neutralize the provider-dependent auth/quota path after its producer/consumer audit | `retain` pending the tracked hosted usage/billing follow-up; the internal symbols were renamed to Genex while behavior remains unchanged | Audit `crates/tui/src/chatwidget/{settings,rate_limits,turn_runtime}.rs`, app startup/event producers, and protocol/genex-api rate-limit payloads |
| TUI dynamic task JSON `thread.kind` and task-management response fields | `crates/tui/src/dynamic_tools.rs` is the producer; the task-management model/tool consumer is outside the repository, while no in-repository deserializer depends on the `kind` value. The response is repository-owned task tooling rather than the external MCP namespace/client identity. | `kind: "genex"` and Genex task-management response vocabulary | `migrate-in-place` | Audit producer and in-repository absence of consumers; `just check --tests`; `just check` |
| `codex` content and filenames in docs, fixtures, snapshots, generated schemas, wire fields, rollout/session records, and SQLite/JSON identifiers | These names are consumed by documentation/tooling, generated artifacts, protocol clients, persisted readers, and replay/state stores. | `defer` to Task 7/8 audits; inventory both content and path names before any migration |

## 0.7 Protocol, schema, persisted-data, and external-contract audit (2026-10-02)

| Contract family | Producer/reader/writer audit | Decision |
|---|---|---|
| `crates/code-mode-protocol/src/grpc/codex.code_mode.v1.proto`, `package codex.code_mode.v1`, and `tonic::include_proto!` | The proto package and generated gRPC bindings are wire/build inputs; code-mode session types also describe a durable Codex thread/session boundary. The executable/service name is a separate repository-owned process identity and was migrated independently. | `retain` current package, file, and generated binding names; any migration requires a versioned protocol task |
| `genex-code-mode-host` Cargo binary, install lookup, launchers, transport logs, and host tests | All executable-name producers and consumers are inside the workspace; no external wire or persisted contract requires `codex-code-mode-host`. | `migrate-in-place` to `genex-code-mode-host`; retain the separate `codex.code_mode.v1` protocol package |
| app-server generated schema titles `CodexAppServerProtocol` / `CodexAppServerProtocolV2`, checked-in JSON/TypeScript schema, `codexHome`, and schema-generator environment keys | Rust producers generate the checked-in schema, app-server clients consume it, and schema fixture tests compare the generated output. The repository-owned titles, `codexHome`, and schema-generator environment keys were migrated together; the Bedrock `usesGenexManagedCredentials` field was migrated with its Rust producers/consumers and generated schemas. | `migrate-in-place` for `GenexAppServerProtocol` / `GenexAppServerProtocolV2`, `genexHome`, `GENEX_APP_SERVER_SCHEMA_*`, and `usesGenexManagedCredentials`; retain separately audited external/client/provider contracts |
| `genexErrorInfo`, core/app-server `GenexErrorInfo`, `genex_turn_id`, `GenexHarnessMetadata`, and `ThreadRollbackFailed` | Core produces error/turn events; app-server converts them to v2 responses and thread-history items; rollout/history/thread-store readers and writers were migrated together. `ThreadRollbackFailed` remains a protocol variant because it is not part of the Codex identity family. | `migrate-in-place` for Codex-named declarations/fields (completed); retain `ThreadRollbackFailed` |
| `codex_home` across rollout/session index, thread-store, state SQLite, auth storage, and app-server | CLI/config produces the home root; rollout and state write/read files and databases below it; thread-store and app-server resolve the same root. The repository-owned field/local/config family was migrated together in the home wave. | `migrate-in-place` to `genex_home`, `GENEX_HOME`, `GENEX_SQLITE_HOME`, and `.genex`; retain only external/security/protocol boundaries |
| persisted origin/observability values such as `codex_cli_rs`, `codex_state`, and existing `codex_*` metadata keys | State extraction, log filters, telemetry, and replay fixtures produce or inspect these values. They are data/observability identifiers, not unused Rust aliases. | `retain` current values; defer any cleanup to the persisted/observability compatibility inventory |
| `.codex` project folders, code-mode/API URLs, marketplace/plugin identifiers, and `codex_apps` fixture names | Repository-owned config discovery and app-server fixtures now use `.genex`; external URLs, marketplace/plugin identifiers, and provider-specific `codex_apps` contracts remain separate boundaries. | `migrate-in-place` for repository-owned `.genex` paths; `retain` or defer external/provider contracts |

## 0.8 Final all-crate Genex audit (2026-10-02)

The final coverage artifact contains all 123 workspace members: 82 members with Rust `Codex`/`codex_*` matches are `reviewed-in-inventory`, and 41 members are explicit `no-match`; `unreviewed=0`. Every residual name is covered by a recorded `migrate-in-place`, `retain`, `remove`, or `defer` decision in the wave tables above. The scoped product-path residuals are the previously audited generated, protocol, security, model/history, and tooling contracts; `.codex/` agent infrastructure and third-party fixtures are counted separately and are not unclassified product leftovers.

The final audit intentionally leaves persisted identifiers, wire/schema names, security/platform fields, provider-owned/external contracts, and unrelated user-facing binary/config vocabulary unchanged. The repository-owned home/config family was migrated to `GENEX_HOME`, `GENEX_SQLITE_HOME`, `genex_home`, and `.genex`; retained home-like strings are limited to external/security/protocol boundaries and are recorded below.

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

- `multi_agent_mode`、`plugin_hooks` は、現行の multi-agent と plugin hooks
  の consumer を保持
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

### 3.1.1 CLI doctor の update/install と desktop security boundary

`crates/cli/src/doctor/updates.rs` は削除済みで、GitHub/Homebrew/npm/Bun/Vite+/pnpm と Desktop appcast の更新検査は存在しない。`crates/cli/src/doctor/desktop/{macos_security.rs,windows_security.rs,platform.rs}` と `crates/cli/src/doctor/security.rs` の署名、sandbox helper、endpoint exclusion の producer/consumer は更新機能とは独立して残っている。`crates/cli/src/doctor/output.rs` と各テスト/snapshot は残存する診断表示を消費する。

更新検査は `genex` CLI の更新不要方針に従って削除した。一方、OS bundle/process identity、security policy、署名、sandbox helper、endpoint exclusion の診断は更新機能とは別の安全性境界として維持する。

| 対象 | Producer/consumer audit | Decision |
|---|---|---|
| `genex-cli` doctor update/install, Desktop diagnostics, endpoint security checks | Doctor update checks and their output/tests were removed after auditing their external release/package/appcast probes. Desktop/platform/security modules still consume OS package, bundle, executable, signing, and exclusion identities; output/tests render those retained results. | `remove` doctor update/install checks and fixed hosted probes; `retain` current security behavior and external contracts |

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

モデル一覧については、Cloudflare と任意の設定済み provider が provider 所有の endpoint 経由でモデルを動的取得する。provider の bundled catalog と暗黙 fallback は削除し、明示的な `model_catalog_json` だけを静的なローカル上書き経路として保持する。旧 `MyModelProvider` は専用の組み込み実装・既定値・`APIKEY` 認証経路を持つため、CloudflareProvider への一本化に伴う削除候補として監査する。

### 3.4 Apps / host-owned MCP

Apps、`codex_apps`、Apps entitlement、host-owned Apps MCP の現行経路と、`all.rs`/`chatwidget.rs` から登録解除された Apps 依存テストソースは削除対象である。`genex-mcp` 全体は通常のMCP、OAuth/elicitation、provider-neutral metadata/security behaviorのため保持する。

provider account/entitlement、host-owned Apps MCP、Apps 固有 UI/protocol と、通常の MCP、OAuth/elicitation、provider-neutral metadata/security behavior を分けて監査する。

### 3.5 Genex 内部 API・OS セキュリティ・UI の残存名（2026-10-05）

producer/consumer をリポジトリ内で再監査した結果、次の範囲は外部利用者向け互換層ではなく、本リポジトリ内の producer と consumer を同時に変更できる対象とした。

- `api/genex`、`backend-api/genex`、`genex/analytics-events`、`genex/alpha` への内部 route/fixture を `migrate-in-place`。OpenAI endpoint の `/v1/responses`、wire field、認証 header/env、モデル ID は対象外。
- Windows/macOS/Linux の Genex-owned sandbox/package/config/security identifiers（`GenexSandbox`、`com.genex`、`/etc/genex`、`genex-linux-sandbox`、Genex WFP/provisioning/helper names）を `migrate-in-place`。trust/enforcement の判定ロジックは変更しない。
- 内部テストの temporary path、model slug、workspace/network fixture、app-server/TUI の Genex-owned client/backend labels、`dotGenexFolder`、Genex UI 文言を `migrate-in-place`。
- VS Code client identities、Agent Plugin の `.codex-plugin`/`com.codex` manifest namespace、GitHubの本番attachment契約、Product::Codex、外部 package/installer/upstream URL、OpenAI/Bedrock wire values は `retain`。これらは外部プロトコル、既存サービス、第三者パッケージ、履歴データ、または provider endpoint の契約であり、内部名の残存数には含めるが未対応バグとは扱わない。一方、GitHub/PR URLやGit originを単なるテストfixtureとして使っていた箇所は、検証する形式を保ったまま `owner/repo` へ `migrate-in-place` した。MCP repository-owned identifiers は下記の移行記録で扱う。

### MCP 名前空間のGenex化（2026-10-05）

MCPについては、ユーザー判断により本リポジトリが所有するクライアント識別子、User-Agent、
capability、notification、tool-result metadata、UI表示メタデータ、OAuth表示名、resource URI、
テストfixtureを `migrate-in-place` とした。`genex-mcp` と `genex-rmcp-client` のproducer、core/protocol
consumer、code-mode説明、テストサーバー、OTELタグを同時に更新する。

保持するのは、MCPとは別契約の `codex.code_mode.v1`、外部OAuthの事前登録client ID、VS Code拡張の
`codex_vscode`/`Codex VS Code Extension`、およびVS Codeの `vscode-remote://`/`.vscode` だけである。

### VS Code拡張ソースの公開状況（2026-10-05）

公式 `openai/codex` の公開Issueでは、VS Code拡張のソースは公開されておらず、公開要望が提出されている
状態だった。したがって拡張側producerを同期更新できる公開リポジトリは確認できず、`codex_vscode`の
契約変更は保留する。将来ソースが公開された場合は、app-serverのClientInfo/User-Agent、OTEL originator、
拡張側実装をまとめて `genex_vscode` へ移行する。

## 4. 現時点で削除候補としない共有基盤

- `genex-api`、`genex-model-provider-info`、`genex-model-provider` の全体
- `genex-http-client`、`genex-mcp`、`genex-core-plugins` の全体
- `genex-otel`、通常の MCP、local/stdio exec-server、sandbox/security behavior
- `crossterm`、`tokio-tungstenite`、`tungstenite` の provider fork dependency

これらは provider 由来の名前や adapter を含むものの、現行の custom provider、Bedrock、local execution、plugin/MCP、platform fallback から利用されている。provider 固有 capability を外した後に依存元がなくなった場合のみ、クレート単位の削除を再評価する。

### Protocol experimental/deprecated surface audit (2026-10-06)

The app-server protocol audit confirmed that most experimental methods are unique
capabilities rather than duplicate stable APIs. TUI production callers use
`thread/settings/update`, `thread/backgroundTerminals/clean`,
`thread/backgroundTerminals/list`, `environment/info`, `thread/search`,
`collaborationMode/list`, `thread/queue/add`, and
`userVerification/verify`; the CLI `queue` command delegates to the TUI helper
and ultimately sends `thread/queue/add`. The stable-looking neighbors
`command/exec`, `fuzzyFileSearch`, `thread/list`, and `turn/diff/updated` have
different sandbox, streaming, search, or diff scopes and are not drop-in
replacements.

The following decisions are recorded for the next cleanup slice:

| Surface | Producer/consumer audit | Decision |
|---|---|---|
| `gitDiffToRemote` | No TUI/CLI caller; only app-server dispatch/processor and protocol/schema definitions remain. The internal Git utility is still consumed by core Git-state tests. | `remove` wire API; retain internal utility |
| `mock/experimentalMethod` | Test-only method for capability/schema gating; no product caller. | `remove` method, payloads, processor, helpers, and generated entries |
| `getAuthStatus` | Auth integration tests only; explicitly deprecated in favor of `account/read`, with legacy token fields not carried forward. | `migrate-in-place` tests, then `remove` |
| `getConversationSummary` | Deprecated implementation and integration tests only; no TUI/CLI caller and no exact stable wire replacement. | `remove` |
| `applyPatchApproval` / `execCommandApproval` | No production producer; canonical v2 approval requests are emitted instead. | `remove` legacy server requests and dead consumer arms |
| `item/fileChange/outputDelta` / `thread/compacted` | Raw-event/rollout compatibility rationale remains, and notification consumers still exist. | `retain` pending replay audit |

### Windows support removal audit (2026-10-06)

The repository-wide platform audit found 122 workspace packages. Two workspace
packages are Windows-specific (`genex-windows-sandbox` and
`genex-mxc-sandbox`); 18 additional packages have direct Windows-targeted
dependencies or Windows-only conditional modules. Tracked Windows-named paths
are concentrated in `crates/windows-sandbox-rs`, app-server protocol, core,
CLI, TUI, low-level platform crates, `third_party/voice`, CI, and setup/docs.

The user-approved target is Linux/macOS only. The following producer/consumer
families are the implementation queue; earlier rows that retained ordinary
Windows support are superseded by this audit for the current slice.

| Family | Producer/consumer audit | Decision |
|---|---|---|
| `genex-windows-sandbox` and `genex-mxc-sandbox` | Workspace members are consumed by `core`, `sandboxing`, `app-server`, `cli`, `tui`, `arg0`, `network-proxy`, and `exec-server-protocol`. Their public exports and binaries are Windows-only; no Linux/macOS consumer requires them. | `remove` |
| Windows sandbox protocol/config/app-server surface | Windows sandbox protocol module, generated JSON/TypeScript schema, app-server request processor/events, config loader/types, feature registry, and tests form one in-repository producer/consumer graph. | `remove` |
| Core, sandboxing, CLI, and TUI Windows paths | Windows sandbox state/config/grants, setup/readiness branches, doctor diagnostics, prompts/events, console/screen-reader/terminal-probe/IDE-pipe modules, and snapshots are guarded Windows producers with no supported-platform consumer. Unix/macOS managers and security paths are separate consumers. | `remove` Windows branches; `migrate-in-place` supported-platform code |
| Low-level Windows modules | `exec-server`, `http-client`, `network-proxy`, `uds`, `utils/pty`, `utils/sleep-inhibitor`, `path-utils`, `arg0`, `hooks`, `rmcp-client`, `keyring-store`, and `app-server-daemon` each have Windows-only files or target dependencies. Their Linux/macOS implementations remain reachable independently. | `remove` Windows modules/dependencies; `migrate-in-place` shared selectors |
| PowerShell parser and Windows shell tooling | `tree-sitter-powershell` is a direct dependency of `shell-command`; parser branches and fixtures are consumed only by shell parsing/tests. `just-shell.py` PowerShell execution and Windows Just recipes are consumed only by Windows recipes. | `remove` |
| CI, setup, and native Windows packaging | GitHub issue templates, `justfile` Windows recipes, `scripts/setup-windows.ps1`, Windows artifact/package paths, and voice Windows build helpers are Windows-specific operational producers. | `remove`; update active docs to recommend WSL |
| Third-party/transitive Windows target entries | Cargo lockfile may retain Windows target packages pulled by supported third-party crates. They are not repository-owned Windows support and have no direct supported-path producer in this slice. | `retain` only when Cargo metadata shows a supported consumer |
| Historical, legal, upstream, and external protocol strings | Existing upstream fixtures, legal/history records, external protocol/package names, and non-Windows external contracts require separate consumers and are not native Windows support. | `retain` and document exact path |

## 5. 次回以降の精査順序

各単位で、先に [pre-release compatibility inventory](pre-release-compatibility-inventory.md) の順序に従って producer/consumer を記録する。

1. 参照元のないクレート・ソースと generated artifact を確認する。`codex-utils-readiness`、`codex-thread-manager-sample`、`LoopbackResponsesServer`、plugin analytics の孤立 source、BackendBanner と関連 snapshot、Apps専用の孤立テスト群は削除済み。
2. persisted/config/schema に残る削除済み feature の互換層を確認する。**完了**。
3. protocol/app-server の互換 alias・schema は**完了**。ただし ChatGPT account/auth の現行 protocol surface は、次の first-party auth 境界で別途判断する。
4. TUI の BackendBanner/Reserve recovery と hosted/curated marketplace UI は**完了**。
   通常の status/account 表示と active bundled/runtime marketplace boundary は保持する。
5. login/model-provider の first-party auth adapter は、すべての model provider が認証対象であることを前提に整理し、ChatGPT/workload-identity の runtime consumer と
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

### Generic model boundary cleanup (2026-10-04)

Producer/consumer audit for the model picker, reasoning controls, rate-limit parser and TUI,
status-surface previews, model migration notices, and Rust test fixtures found that the following
items were repository-owned assumptions rather than required external endpoint names:

- `codex-auto-*` picker partitioning/order and reasoning-label suppression;
- model warning checks for built-in model prefixes and the Plus-plan wording;
- hardcoded reserve-model TUI branches;
- the default rate-limit bucket identifier `codex` and its TUI/core consumers;
- built-in model status previews and model test fixtures;
- stale provider-specific migration notice keys and config fields.

The model catalog remains the producer of available model IDs and reasoning capabilities. Rate-limit
nudges now consume the server-provided `normal_model_slug`, warnings are based on advertised
reasoning effort, and the default bucket is `default`. These are `remove`/`migrate-in-place`.
External provider model IDs may still be supplied by a provider response, but no built-in model ID
is shipped by Genex. Product::Codex restrictions, GitHub/upstream fixtures, security and OS
identities, and required endpoint wire names remain separate retained contracts.

### OS security, history, and telemetry genericization (2026-10-04)

The pre-release audit confirmed that the desktop security product identifiers, owned history
metadata namespace, world-state hash namespace, OTEL meter scope, and authentication-environment
telemetry fields are repository-owned. They were migrated together with their in-repository
producers, consumers, tests, and snapshots. Trusted-process and enforcement predicates are
unchanged. GitHub/upstream repository values, Product::Codex, OpenAI/provider wire names, and
third-party originator identifiers remain external contracts.

### Amazon Bedrock capability (2026-10-05)

Producer/consumer audit is complete for the repository-owned Amazon Bedrock capability. This entry
supersedes earlier candidate notes that treated Bedrock as retained; those notes remain
historical context, while the current decision is the completed `remove` below. The
Mantle/Runtime model-provider adapters, Bedrock auth/login, AWS credential export and refresh,
app-server protocol/setup routes, config/schema, CLI/TUI onboarding, test clients, and generated
artifacts form one reachable feature and are selected for `remove`. `genex-aws-auth` has no
unrelated production consumer and is selected for `remove` with the feature. OpenAI endpoint
wire/auth/model contracts, custom providers, generic model discovery, platform fallbacks, and
security behavior are explicitly `retain` and must not be changed by this slice.

### MCP/model-provider credential boundary (2026-10-08)

The current MCP discovery and runtime paths receive `GenexAuth`/`CredentialManager`
values even where the effective MCP server projection ignores them. The active
MCP credential consumers are the server's configured headers, environment
variables, bearer-token variables, and MCP OAuth stores. The following cleanup
is tracked separately from the ModelProvider credential redesign:

| Area | Producer/consumer audit | Decision | Verification |
|---|---|---|---|
| Unused model-auth parameters in MCP discovery/status/resource helpers | `genex-mcp::effective_mcp_servers`, `compute_auth_statuses`, and threadless resource/status paths accept model auth, but MCP projection/status behavior consumes MCP config and MCP OAuth state instead. | `remove` | Cross-crate signature/caller audit; `just check --tests`; `just check` |
| MCP-owned headers, bearer-token environment variables, OAuth credentials, and reconnect identity | `genex-config`, `genex-mcp`, `genex-rmcp-client`, CLI MCP login/logout, and app-server MCP processors produce/consume these active credentials and connection states. | `retain` | MCP OAuth/status/transport audit; `just check --tests`; `just check` |
| Host `genex/auth-change` notification and `AuthChangeState` watcher | `genex-login` produced the watcher and `genex-mcp` forwarded it to opt-in MCP servers. No MCP-owned credential producer or consumer uses this path; MCP OAuth refresh and connection identity already own their lifecycle. | `remove` | Capability producer/consumer audit completed; `just check --tests`; `just check` |
| Global `AuthDotJson` persisted credential shape | `genex-login` storage backends and `CredentialManager` read/write the current single-provider `auth_mode`/`api_key` payload; CLI doctor and test fixtures consume its status and persistence helpers. Provider-scoped storage now owns keyed records, while the legacy shape remains readable and migrates on provider-scoped write. | `migrate-in-place` to provider-keyed records with redacted debug/status views; retain file, keyring, secret, ephemeral, and legacy-read behavior | Login/storage producer-consumer audit; `just check --tests`; `just check` |
| Provider credential lifecycle and active selection | `genex-login` now provides provider-scoped `login`, `switch`, `logout`, and `delete`; `genex-core::Config` validates configured provider IDs and coordinates selection with `CredentialManager`. Logout retains records and delete removes only the targeted provider. | `migrate-in-place`; retain generic auth revision, refresh, and storage backends | Core/login producer-consumer audit; lifecycle transition tests compiled by `just check --tests`; `just check` |
| Provider-scoped account and usage state | `ModelProvider::account_state` and the app-server `account/read` response expose the selected provider ID, optional account identity, authentication status, and optional usage snapshot. Providers without a usage endpoint report an explicit unsupported capability and are omitted from the wire usage field. | `migrate-in-place`; retain provider-owned account/usage capabilities and avoid a global Genex account abstraction | Model-provider/app-server/protocol producer-consumer audit; `just generate`; `just check --tests`; `just check` |
| Provider-aware `AuthChangeState` | `genex-login` produces provider ID and active-selection metadata for app-server/auth watchers without exposing credential payloads; existing generation and owner-generation consumers remain valid. | `migrate-in-place`; retain generic invalidation semantics | Change-state producer/consumer audit; `just check --tests`; `just check` |
| Implicit global ModelProvider auth lookup and cache identity | ModelProvider, models endpoint/catalog, core Chat, Web Search, and History/Notes previously received provider metadata without a stable provider ID and could resolve the shared manager snapshot implicitly. Runtime construction now carries `Config.model_provider_id`; provider-local stored auth and cache identity are resolved from that ID, while explicit provider credentials remain higher priority. | `migrate-in-place`; retain provider-neutral request and cache behavior | ModelProvider producer/consumer audit; cross-provider isolation/identity checks compiled by `just check --tests`; `just check` |
| `McpServerAuth::EmaAuth` | `genex-config` validates a trusted enterprise IdP registration and prevents project/plugin credential substitution; `genex-mcp` catalog resolution and `genex-rmcp-client` retain EMA identity/token-exchange primitives. CLI/app-server login paths reject ordinary OAuth login for this mode, and runtime startup rejects unconfigured/unsupported EMA connections. No path reads ModelProvider credentials. | `retain` as an explicitly configured MCP enterprise-IdP mode; keep its separate credential and provenance boundary | Config/catalog/RMCP producer-consumer audit; no schema change; `just check --tests`; `just check` |
### Global authentication consumer audit (2026-10-08)

The remaining `CredentialManager` references were classified before editing. Model-provider,
models-manager, core Chat, Web Search, History/Notes, account/usage, and catalog paths
need provider-scoped credentials and remain active. `ConnectionAuth`, user verification,
and the app-server verification watcher use auth revisions/account identity for security
and request invalidation; they do not represent an application login gate and are retained.
The exec/TUI login-restriction calls enforce explicit managed-auth policy and remain a
separate security/configuration capability pending its own hosted-auth decision. Plugin
skill metadata still consumes the effective provider auth mode, so its auth capability is
retained while its cache no longer touches an auth snapshot solely to invalidate a cache.

The external-agent migration service and its app-server/TUI/state/protocol
consumers were removed together. Marketplace plugin discovery remains active in
the ordinary plugin paths and no longer depends on a migration-only service.
The now-unused `effective_plugins_changed_callback` auth parameter was removed
as part of the provider-neutral plugin cleanup.

| Consumer | Producer/consumer audit | Decision |
|---|---|---|
| Model request, model catalog, account/usage, Web Search, History/Notes | Provider instances and request builders resolve credentials through configured provider IDs; account/usage is provider-scoped. | `retain` provider credential capability |
| Transport connection ownership and user verification | Auth revision/owner generation and provider account identity invalidate stale work and bind device verification. | `retain` generic security invalidation |
| Exec/TUI login restrictions | Explicit forced login method/workspace policy is still read from configuration; no generic “logged in” UI gate is introduced. | `retain` explicit policy; audit hosted policy separately |
| Core plugin skill metadata | Skill projection uses effective auth mode to render capability metadata; cache loading did not consume auth data. | `retain` auth-mode capability; remove dead snapshot read |
| External-agent migration auth field | The migration service, its `PluginsManager` dependency, and every app-server/TUI/state/protocol consumer were deleted after a producer/consumer audit. Ordinary plugin discovery has independent producers and consumers. | `remove (completed)` migration-only dependency; retain ordinary plugin capability |

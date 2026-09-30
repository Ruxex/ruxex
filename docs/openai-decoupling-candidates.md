# 未使用・OpenAI 依存候補の追加調査メモ

この文書は、[OpenAI 固有機能の段階的な分離計画](openai-decoupling-plan.md) に対する 2026-09-27 時点の追加調査結果を記録する。

ここに記載した項目は **削除・保持・移行の決定ではない**。個別の producer/consumer 監査、別バイナリからの到達性、動的登録、生成物、外部利用の確認を行った後、[pre-release compatibility inventory](pre-release-compatibility-inventory.md) に `remove`、`retain`、または `migrate-in-place` として記録する。

## 調査範囲と検証

- workspace の `Cargo.toml`、`cargo metadata --no-deps` による内部依存元の確認
- `Feature` registry、`apply_map`、生成済み config schema の参照確認
- `openai`、`chatgpt`、固定 endpoint、OpenAI 専用 header、managed auth の production code 検索
- `just check`：成功、エラーなし、未使用警告 114 件
- `just check --tests`：成功

既存の計画書・インベントリには、現在のソースツリーに残る実装を完了済みと記載している箇所もある。以下の候補は、その差分を解消するための再監査対象として扱う。

互換性クリーンアップ計画の Task 1〜8 は完了している。削除済み feature
の registry、schema、`apply_map`、CLI/TUI の一時的な consumer とテスト残骸も
`fb8d960a7` で処理済みであり、詳細な `remove` / `retain` /
`migrate-in-place` の判断は [pre-release compatibility inventory](pre-release-compatibility-inventory.md)
に記録した。以下では、未更新だった候補の状態と、まだ残る Phase 3
候補を区別して記録する。

## 1. 参照元が見つからないクレート・ソース

| 対象 | 現時点の観測 | 現時点の扱い |
|---|---|---|
| `codex-utils-readiness` | workspace に登録されていたが、他の workspace member からの依存・参照がなく、実装と自身のテストだけが残っていた。producer/consumer 監査後に workspace 登録、本体、lockfile、checked-in Hawk finding を削除済み | 置換なし。未参照クレートを削除 | `remove (completed)` |
| `codex-thread-manager-sample` | 他の workspace member から利用されず、製品 runtime や必須 test entrypoint からも利用されない standalone binary。README と entrypoint 文書は配布する場合の説明に留まり、現行の必須 consumer ではない。workspace、lockfile、ソース、entrypoint 文書から削除済み | 置換なし。ThreadManager サンプルを削除 | `remove (completed)` |
| `LoopbackResponsesServer` と `plugin_analytics_capture_tests.rs` | いずれも削除済みの plugin analytics smoke test の残骸。前者は module 登録だけが残り、後者は module 登録されず削除済みシンボルを参照していた。`codex-cli.json` には前者の古い Hawk finding が2件残っていた | 置換なし。孤立した test helper、孤立 test source、生成済み finding を削除 | `remove (completed)` |
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

## 3. OpenAI バックエンド依存の capability 候補

### 3.1 ChatGPT の利用量・Reserve 自動切替・復旧 UI（Banner 部分は削除済み）

対象候補は TUI の rate limit、thread usage、backend banner、Reserve model return state、status card、ChatGPT usage URL、`requires_openai_auth` 判定である。

主な確認先:

- `crates/tui/src/chatwidget/rate_limits.rs`
- `crates/tui/src/chatwidget/thread_usage.rs`
- `crates/tui/src/chatwidget/backend_banners.rs`（削除済み）
- `crates/tui/src/chatwidget/luna_reserve_return.rs`（削除済み）
- `crates/tui/src/backend_banners/actions.rs`（削除済み）
- `crates/tui/src/status/card.rs`
- `crates/tui/src/status/account.rs`

通常の status、model catalog、provider-neutral な usage 表現まで削除するのではなく、ChatGPT account/plan、固定 URL、OpenAI 専用 recovery state の到達経路を分けて監査する。

BackendBanner の parser/CTA/render、bottom pane の専用 inline banner、Reserve 自動切替・復帰状態・関連 snapshot は `remove (completed)` とした。通常の rate-limit 表示、status card、手動の model catalog はこの削除に含めていない。

### 3.2 OpenAI Curated / Bundled marketplace

監査の結果、OpenAI 固有 marketplace surface は次の active boundary と test-only
残骸に分かれた。

- `crates/core-plugins/src/lib.rs` の `openai-bundled`、`openai-primary-runtime` と、対応する
  `marketplace_policy` の予約名・managed path 判定は、bundled plugin ローダー、trusted
  script attribution、artifact telemetry が現在も消費しているため保持する。
- `crates/app-server/src/request_processors/plugins/search.rs` に OpenAI 固有の built-in
  scope はなく、local/Git/npm marketplace の検索基盤だけが残る。
- `crates/tui/src/chatwidget/plugin_catalog.rs` の OpenAI Curated tab は既に削除済みで、
  残っていた Apps help URL と disclosure line も削除した。
- `core-plugins` と TUI の test-only curated marketplace 名は `test-curated`/
  `test-api-curated`/`Test Marketplace` へ移行した。

汎用の local/Git/npm plugin loader、hook、tool、trust/security policy は別の保持候補であり、`codex-core-plugins` 全体を削除する根拠にはしない。

### 3.3 first-party auth と model provider adapter

次のクレートは OpenAI 固有部分と provider-neutral 部分が混在している。

- `codex-login`: ChatGPT JWT、plan/account ID、managed auth、`OPENAI_API_KEY`
- `codex-model-provider-info`: `create_openai_provider`、`requires_first_party_auth`
- `codex-model-provider`: OpenAI models endpoint、ChatGPT account header、OpenAI auth predicate
- `codex-app-server-protocol`: ChatGPT auth/account や OpenAI 固有 protocol 型

Bedrock、custom `model_providers`、汎用 credential storage、Responses-compatible provider の consumer があるため、クレート全体の削除ではなく first-party adapter の分割・削除を先に検討する。

モデル一覧については、`MyModelProvider` も `ModelProvider::models_manager()` 経由で provider の `/models` を動的取得するよう移行済み。OpenAI の bundled catalog と暗黙 fallback は削除し、明示的な `model_catalog_json` と Bedrock の provider-owned catalog だけを静的経路として保持する。

### 3.4 Apps / host-owned MCP

Apps、`codex_apps`、Apps entitlement、host-owned Apps MCP の現行経路とApps専用の孤立テストは削除済みである。`codex-mcp` 全体は通常のMCP、OAuth/elicitation、provider-neutral metadata/security behaviorのため保持する。

OpenAI account/entitlement、host-owned Apps MCP、Apps 固有 UI/protocol と、通常の MCP、OAuth/elicitation、provider-neutral metadata/security behavior を分けて監査する。

## 4. 現時点で削除候補としない共有基盤

- `codex-api`、`codex-model-provider-info`、`codex-model-provider` の全体
- `codex-http-client`、`codex-mcp`、`codex-core-plugins` の全体
- `codex-otel`、通常の MCP、local/stdio exec-server、sandbox/security behavior
- `crossterm`、`tokio-tungstenite`、`tungstenite` の OpenAI fork dependency

これらは OpenAI 由来の名前や adapter を含むものの、現行の custom provider、Bedrock、local execution、plugin/MCP、platform fallback から利用されている。OpenAI 固有 capability を外した後に依存元がなくなった場合のみ、クレート単位の削除を再評価する。

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

# 未使用・OpenAI 依存候補の追加調査メモ

この文書は、[OpenAI 固有機能の段階的な分離計画](openai-decoupling-plan.md) に対する 2026-09-26 時点の追加調査結果を記録する。

ここに記載した項目は **削除・保持・移行の決定ではない**。個別の producer/consumer 監査、別バイナリからの到達性、動的登録、生成物、外部利用の確認を行った後、[pre-release compatibility inventory](pre-release-compatibility-inventory.md) に `remove`、`retain`、または `migrate-in-place` として記録する。

## 調査範囲と検証

- workspace の `Cargo.toml`、`cargo metadata --no-deps` による内部依存元の確認
- `Feature` registry、`apply_map`、生成済み config schema の参照確認
- `openai`、`chatgpt`、固定 endpoint、OpenAI 専用 header、managed auth の production code 検索
- `just check`：成功、エラーなし、未使用警告 114 件
- `just check --tests`：成功

既存の計画書・インベントリには、現在のソースツリーに残る実装を完了済みと記載している箇所もある。以下の候補は、その差分を解消するための再監査対象として扱う。

## 1. 参照元が見つからないクレート・ソース

| 対象 | 現時点の観測 | 現時点の扱い |
|---|---|---|
| `codex-utils-readiness` | workspace に登録されているが、他の workspace member からの依存・参照がない。実装と自身のテストだけが残る | クレート削除可否を精査する候補 |
| `codex-thread-manager-sample` | workspace member だが、他の workspace member から利用されない独立サンプル | 製品 workspace に残すサンプルかどうかを確認する候補 |
| `LoopbackResponsesServer` | `app-server-test-client` で module 登録されているが、型・`start`・`base_url` の利用箇所がない | module、関連 generated artifact、古い snapshot の扱いを精査する候補 |
| `crates/tui/src/app/backend_banner_fallback.rs` | production `app.rs` から module 化されておらず、production caller も見つからない。関連する fallback snapshot は残る | 孤立ソースと Reserve fallback の残存範囲を精査する候補 |

確認対象の主なパスは次のとおり。

- `crates/utils/readiness/`
- `crates/thread-manager-sample/`
- `crates/app-server-test-client/src/loopback_responses_server.rs`
- `crates/tui/src/app/backend_banner_fallback.rs`
- `crates/tui/src/chatwidget/backend_banners.rs`
- `crates/tui/src/chatwidget/luna_reserve_return.rs`

## 2. デフォルトオフまたは削除済み feature の残存候補

### production consumer が見つからない、または互換 no-op に見えるもの

次の feature は、現行 Rust production code から直接利用されていないか、`Features::apply_map` の互換 no-op として残っている。

- `transcript_v2`
- `code_mode_buffered_exec`
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

これらは、次の互換処理と生成物を producer/consumer 監査する。

- `crates/features/src/lib.rs` の `FeatureSpec` と `apply_map`
- `crates/core/config.schema.json` の設定 schema
- feature 設定を読む config layer、CLI、app-server、TUI
- tests、fixture、説明文、生成済み schema

### 名前だけでは削除しないもの

- `enable_mcp_apps` は feature enum の直接参照が少なくても、Apps/MCP/plugin の現行経路があるため、Apps capability と一緒に監査する。
- `use_legacy_landlock` は Linux の実行時 security fallback と結び付くため、互換 no-op とみなさない。
- `in_app_*`、`browser_use_*`、`computer_use` は requirements-only gate として config/app-server 側に consumer があるため、registry だけを見て削除しない。
- `recommended_plugins`、`workspace_dependencies`、code-mode、MCP、plugin runtime など、デフォルトオフでも現行 entrypoint やテストから到達できる機能は別扱いにする。

## 3. OpenAI バックエンド依存の capability 候補

### 3.1 ChatGPT の利用量・Reserve 自動切替・復旧 UI

対象候補は TUI の rate limit、thread usage、backend banner、Reserve model return state、status card、ChatGPT usage URL、`requires_openai_auth` 判定である。

主な確認先:

- `crates/tui/src/chatwidget/rate_limits.rs`
- `crates/tui/src/chatwidget/thread_usage.rs`
- `crates/tui/src/chatwidget/backend_banners.rs`
- `crates/tui/src/chatwidget/luna_reserve_return.rs`
- `crates/tui/src/backend_banners/actions.rs`
- `crates/tui/src/status/card.rs`
- `crates/tui/src/status/account.rs`

通常の status、model catalog、provider-neutral な usage 表現まで削除するのではなく、ChatGPT account/plan、固定 URL、OpenAI 専用 recovery state の到達経路を分けて監査する。

### 3.2 OpenAI Curated / Bundled marketplace

次の OpenAI 固有 marketplace surface が残っている。

- `crates/core-plugins/src/lib.rs` の `openai-curated`、`openai-api-curated`、`openai-bundled`、`openai-primary-runtime`
- `crates/core-plugins/src/marketplace_policy.rs` の予約名・managed path 判定
- `crates/app-server/src/request_processors/plugins/search.rs` の built-in marketplace scope
- `crates/tui/src/chatwidget/plugin_catalog.rs` の OpenAI Curated tab と help URL

汎用の local/Git/npm plugin loader、hook、tool、trust/security policy は別の保持候補であり、`codex-core-plugins` 全体を削除する根拠にはしない。

### 3.3 first-party auth と model provider adapter

次のクレートは OpenAI 固有部分と provider-neutral 部分が混在している。

- `codex-login`: ChatGPT JWT、plan/account ID、managed auth、`OPENAI_API_KEY`
- `codex-model-provider-info`: `create_openai_provider`、`requires_openai_auth`
- `codex-model-provider`: OpenAI models endpoint、ChatGPT account header、OpenAI auth predicate
- `codex-app-server-protocol`: ChatGPT auth/account や OpenAI 固有 protocol 型

Bedrock、custom `model_providers`、汎用 credential storage、Responses-compatible provider の consumer があるため、クレート全体の削除ではなく first-party adapter の分割・削除を先に検討する。

### 3.4 Apps / host-owned MCP

`codex-mcp`、app-server、core、tests には Apps、`codex_apps`、managed auth、Apps entitlement に関係する現行コードが残る。したがって `codex-mcp` 全体は未使用とは扱わない。

OpenAI account/entitlement、host-owned Apps MCP、Apps 固有 UI/protocol と、通常の MCP、OAuth/elicitation、provider-neutral metadata/security behavior を分けて監査する。

## 4. 現時点で削除候補としない共有基盤

- `codex-api`、`codex-model-provider-info`、`codex-model-provider` の全体
- `codex-http-client`、`codex-mcp`、`codex-core-plugins` の全体
- `codex-otel`、通常の MCP、local/stdio exec-server、sandbox/security behavior
- `crossterm`、`tokio-tungstenite`、`tungstenite` の OpenAI fork dependency

これらは OpenAI 由来の名前や adapter を含むものの、現行の custom provider、Bedrock、local execution、plugin/MCP、platform fallback から利用されている。OpenAI 固有 capability を外した後に依存元がなくなった場合のみ、クレート単位の削除を再評価する。

## 5. 次回以降の精査順序

各単位で、先に [pre-release compatibility inventory](pre-release-compatibility-inventory.md) の順序に従って producer/consumer を記録する。

1. 参照元のないクレート・ソースと generated artifact を確認する。
2. persisted/config/schema に残る削除済み feature の互換層を確認する。
3. protocol/app-server の OpenAI 固有 alias・schema・account surface を確認する。
4. TUI の usage/recovery UI と OpenAI Curated marketplace を確認する。
5. login/model-provider の first-party auth adapter と Apps/MCP の OpenAI 固有部分を確認する。
6. 各項目について `remove`、`retain`、`migrate-in-place` を決め、変更後に `just check`、必要に応じて `just check --tests`、生成物があれば `just generate` を実行する。

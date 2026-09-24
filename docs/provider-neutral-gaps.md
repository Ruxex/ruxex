# 汎用プロバイダ版クライアントの不足機能

## 目的

OpenAI 固有機能の除去後に残ったコードを調査し、汎用プロバイダから利用する場合に不足している機能、実装済みだが接続されていない機能、既に別経路で実装済みの機能を整理する。

この文書では、次の状態を区別する。

- **未実装:** 汎用経路に必要な処理が存在しない。
- **未接続:** 基盤コードはあるが、現行の汎用経路から呼び出されていない。
- **部分実装:** 一部の値・経路だけが実装されている。
- **実装済み:** OpenAI 固有経路を削除しても、汎用経路が残っている。

## 結論

| ID | 機能 | 状態 | 優先度 | 判断 |
|---|---|---|---|---|
| G-1 | MCP ツールカタログの動的 refresh | 実装済み | 高 | server 単位の汎用 refresh 経路を追加済み |
| G-2 | 自作マーケットプレイスの publish/upload | 部分実装 | 中 | bundle pack を保持し、公開 API は別途設計する |
| G-3 | 拡張機能からの turn 中断 | 未接続 | 中 | 汎用 extension API として接続する |
| G-4 | リトライ終了後の再開情報 | 未接続 | 中 | producer と consumer を定義する |
| G-5 | 画像 detail の `low` / `auto` | 部分実装 | 中 | provider capability と変換方針を定義する |
| G-6 | Prefix compaction | 部分実装 | 低 | 現行 Memento との差分を設計してから実装する |
| G-7 | 非 Responses wire API の完全なサポート | 未実装 | 高 | provider transport 境界を再設計する |

## G-1: MCP ツールカタログの動的 refresh

### 現状

[`ClientToolCatalog::refresh`](../crates/codex-mcp/src/client_tool_catalog.rs) は、fetch・publication・revision 管理・実行中 call との整合性を実装している。現在は `McpConnectionSet` の server 単位 API からこの処理を呼び出せる。

以前の呼び出し側は ChatGPT Apps/Connectors の専用カタログ更新処理だった。これは OpenAI のモデルプロバイダ機能ではなく、Apps の削除時に呼び出し側が削除されたものだが、通常の MCP server に対しても次のような更新要求は汎用機能になり得る。

- MCP server の tools/list が変更された場合の再取得
- plugin の有効化・無効化や MCP 設定変更後の再構築
- server 側の更新通知を受けた場合の refresh
- refresh 中の既存 call、失敗、stale snapshot の扱い

### 対応済み

`McpConnectionSet`、MCP runtime、core session、app-server の `mcpServer/reload` を接続し、`serverName` を指定した場合に live な `tools/list` を再取得・公開する。initialize 時の server instruction も再利用するため、旧 OpenAI Apps 経路と同じくツール準備・cache ticket・revision 更新を通る。

refresh 前に作られた binding は既存 snapshot を保持し、refresh 後に作られた binding は新しい revision を参照する。refresh 中の call と stale snapshot は既存の `ClientToolCatalog` 契約で処理する。`serverName` を省略した既存の `mcpServer/reload` は従来どおり設定 reload として動作する。

server 側通知を自動的に refresh へ変換する producer はまだなく、必要になった時点で MCP notification の契約を追加する。

## G-2: 自作マーケットプレイスの publish/upload

### 現状

自作 marketplace の読み込み・利用は既に実装済みである。

- `marketplace.json` の読み込みと plugin entry の解決
- local marketplace の利用
- Git marketplace の追加・upgrade・remove
- marketplace 内 plugin の local/Git/npm source
- CLI の `codex plugin marketplace add/list/upgrade/remove`

[`plugin_bundle_archive.rs`](../crates/core-plugins/src/plugin_bundle_archive.rs) には、manifest を検証して tar.gz bundle を作る pack 処理もある。ただし、これは現在 private かつ主にテストから利用されており、次の publish 経路はない。

- upload 先となる registry/server API
- 認証・権限・namespace 管理
- plugin version の公開・上書き規則
- bundle の署名、検証、size limit の公開契約
- publish 用 CLI/app-server API

したがって、**自作 marketplace を Git/local で運用することは現在可能**だが、HTTP registry に plugin をアップロードして公開する機能は未実装である。将来の upload 用の土台として bundle pack は保持する。

## G-3: 拡張機能からの turn 中断

[`Session::interrupt_turn_with_warning`](../crates/core/src/session/extension_interruption.rs) は turn を中断し、warning event を送る処理を持つが、現行の extension 呼び出し側がない。

拡張機能が外部状態の変化や安全上の判断で turn を停止する必要がある場合、汎用 extension API に次を公開する必要がある。

- 中断対象 turn の識別
- warning の表示先と永続化方針
- 中断と通常の user abort / provider error の区別
- race と cancellation の扱い

## G-4: リトライ終了後の再開情報

Responses stream のリトライ自体は実装されている。最大リトライ回数を超えた時点で [`ExhaustedResponseRetry`](../crates/core/src/responses_retry.rs) を thread extension data に保存するが、現在その値を読む consumer はない。

そのため、次のどちらを提供するかが未決定である。

- 次回 turn で retry-at を考慮して自動再試行する
- UI/extension に「再試行可能時刻」を通知するだけにする

また、型名と metadata が Responses API 前提なので、汎用 retry state と provider-specific retry advice を分離する必要がある。

## G-5: 画像 detail の provider capability

現在の画像処理は `high` と `original` を実際に扱う。`auto` は `high` に正規化され、`low` は unsupported として画像を省略する。

これは OpenAI 専用機能の残骸というより、provider capability の不足である。provider によって `low`、`auto`、`original` の意味や上限が異なるため、次の capability が必要になる可能性がある。

- 対応する detail 値
- provider ごとの最大画像サイズ・変換規則
- unsupported 時に省略するか、近い detail に変換するか
- モデル入力とメタデータに記録する effective detail

## G-6: Prefix compaction

compaction の共通型には `Memento` と `PrefixCompaction` があるが、現行の生成経路は `Memento` のみである。ユーザー入力の identity も `Regenerate` が現行経路で、`Preserve` は使われていない。

ローカル compaction 自体は汎用 agent 機能だが、compaction metadata と request transport は現在 Responses API 形状を強く持つ。Prefix compaction を追加する場合は、先に次を決める必要がある。

- provider が履歴 prefix を保持する場合の契約
- message identity を保持・再生成する条件
- thread history / rollback との互換性
- local compaction と provider-backed compaction の capability 表現

## G-7: 非 Responses wire API の完全なサポート

現在の provider abstraction には `ModelProvider`、provider capability、任意の `model_providers` 設定、Amazon Bedrock 実装がある。一方、core の model client と request metadata は主に次を前提にしている。

- Responses API の request/item/event 形式
- Responses HTTP と Responses WebSocket
- `codex_protocol::openai_models::ModelInfo`
- Responses 用の compaction、retry、metadata

したがって、現在の「汎用プロバイダ」は **Responses互換の provider を差し替えられる汎用性** であり、Chat Completions、独自 streaming protocol、非 OpenAI 形式の tool call を同じ抽象で扱える状態ではない。

完全な provider-neutral 化には、少なくとも次の境界が必要になる。

- provider-independent な turn request/response/event model
- tool call、structured output、reasoning、画像、streaming の capability negotiation
- provider-independent な model metadata と token usage
- transport と wire protocol adapter の分離
- provider-specific auth、retry、error mapping の隔離

これは小さな warning cleanup ではなく、別のアーキテクチャ作業として扱う。

## 既に実装済みで、不足機能と扱わないもの

| 機能 | 現状 |
|---|---|
| MCP server instruction | initialize 時の instruction はツールの namespace description に反映され、live refresh の再取得にも再利用する |
| MCP tool provenance | 通常の `prepare_mcp_tools_for_model` が plugin 名を付与する |
| context window token status | 現行の `context_window_token_status` が model info を取得して計算する。旧 wrapper は削除済み |
| managed requirements | `apply_exact_requirement` が現行経路。旧 leaf/helper は削除済み |
| full-access 判定 | protocol 側の `has_full_access` が現行経路。snapshot の旧 wrapper は削除済み |
| MCP approval / sandbox / network policy | 現行の安全境界として利用中。未使用 warning だけを理由に削除しない |

## 推奨する実装順

1. **Marketplace publish contract**: bundle、署名、認証、version、registry API を決めてから pack を production API に昇格する。
2. **Provider capability 拡張**: image detail、tool/structured output、streaming、compaction を capability として整理する。
3. **Extension context**: extension interruption の producer/consumer を接続する。
4. **Retry / compaction metadata**: Responses 固有の型を provider-neutral な state と adapter に分離する。
5. **非 Responses wire API**: 最後に transport と request/event model を再設計する。

## 判断保留

次のコードは汎用性のある補助機能に見えるが、現時点で現行 consumer がない。将来の publish、HTTP registry、provider adapter の設計で必要になる可能性があるため、直ちには削除しない。

- `HttpClientSelector`
- HTTP status の telemetry subtype helper
- plugin installation の `plugin_id` 保持
- provider-specific HTTP client / error mapping の分離候補

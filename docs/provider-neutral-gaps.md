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
| G-2 | 自作マーケットプレイスの publish/upload | 部分実装 | 中 | 汎用 HTTP registry 向け CLI publish client を追加。registry server と app-server API は別途実装する |
| G-3 | 拡張機能からの turn 中断 | 実装済み | 中 | extension event sink から listener 経由で Core の対象 turn を中断する |
| G-4 | リトライ終了後の再開情報 | 対象外 | - | consumer 不在の未使用 producer は削除し、必要になった時点で再設計する |
| G-5 | 画像 detail の `low` / `auto` | 部分実装 | 中 | provider capability と変換方針を定義する |
| G-6 | Prefix compaction | 実装済み | 低 | コンテキスト超過時に自動適用。ユーザー選択設定は追加しない |
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

[`plugin_bundle_archive.rs`](../crates/core-plugins/src/plugin_bundle_archive.rs) には、manifest を検証して tar.gz bundle を作る pack 処理がある。削除前の hosted 実装は、この bundle を署名付き upload URL に PUT してから backend API で finalize していた。

- upload 先となる registry/server API
- 認証・権限・namespace 管理
- plugin version の公開・上書き規則
- bundle の署名、検証、size limit の公開契約
- publish 用 CLI/app-server API

したがって、**自作 marketplace を Git/local で運用することは現在可能**であり、HTTP registry への plugin upload は CLI の汎用 client で利用できるようになった。ただし、registry server、署名鍵管理、namespace/version policy、app-server 経由の publish API はこのリポジトリにはまだない。

### 対応済み（client-side publish）

`codex plugin marketplace publish` は、次の provider-neutral な三段階契約を利用する。

1. `POST {registry}/v1/plugins/uploads`
2. 応答の `upload_url` に `application/gzip` の bundle を `PUT`
3. `POST {registry}/v1/plugins` に `file_id`、`etag`、任意の `plugin_id` を送信して finalize

control API の bearer token は `--token-env-var` で指定した環境変数から読み取り、署名付き `upload_url` には転送しない。registry は upload reservation に `upload_headers` を返して追加の署名ヘッダーを要求できる。bundle は最大 50 MiB で、symbolic link と path traversal を許可しない既存の archive 検証を利用する。

この実装は tar.gz を HTTP で配布するもので、Docker/OCI コンテナイメージの build・push・pull は提供しない。現在の marketplace source は local/Git/npm であり、npm の `registry` も plugin package の取得先であって marketplace コンテナ registry ではない。

## G-3: 拡張機能からの turn 中断

[`Session::interrupt_turn_with_warning`](../crates/core/src/session/extension_interruption.rs) の既存処理を、`ExtensionEventSink::request_turn_interrupt` から利用できるようにした。app-server は要求を `ThreadListenerCommand` に変換して listener の順序に入れ、対象 `turn_id` がまだ active な場合だけ warning を送って中断する。

拡張機能は次の汎用 API で外部状態の変化や安全上の判断による中断を要求できる。

- `ExtensionTurnInterrupt { thread_id, turn_id, message }`
- `ExtensionEventSink::request_turn_interrupt(...)`

host が中断 API を提供しない場合は no-op とし、app-server では thread listener が順序を保証する。対象 turn が終了済み、別 turn に置き換わった、または listener が存在しない場合は要求を破棄する。中断理由は既存の `TurnAbortReason::Interrupted` を使うため、通常の user interrupt と同じ provider-neutral な lifecycle を通る。

## G-4: リトライ終了後の再開情報

Responses stream のリトライ自体は既存のリトライループで完結している。リトライ終了後の
`retry_at` を後続処理へ渡す consumer はこのフォークに存在せず、自動再試行や UI／extension
通知を追加しても、ツール実行などの副作用を重複させる安全な再開契約がない。

そのため、G-4 は現時点では実装対象外とする。consumer 不在の retry advice producer、thread
extension data、protocol／UI accessor などの未使用コードが見つかった場合は残さず削除する。
将来、安全な再開 consumer と重複実行を防ぐ契約が必要になった時点で、provider-neutral な
retry advice として改めて設計する。

## G-5: 画像 detail の provider capability

Responses API の `input_image.detail` で定義される `auto`、`low`、`high`、`original` を受け付ける。
`auto` は指定値を wire request に保持し、ローカル前処理では安全側の `high` 相当を使う。
`low` は 512px 四方相当へ縮小してから送信し、未対応値として画像を省略しない。

これは OpenAI Responses 互換経路の実装であり、provider capability の差異を表現する仕組みは別途必要である。provider によって `low`、`auto`、`original` の意味や上限が異なるため、次の capability が必要になる可能性がある。

- 対応する detail 値
- provider ごとの最大画像サイズ・変換規則
- unsupported 時に省略するか、近い detail に変換するか
- モデル入力とメタデータに記録する effective detail

## G-6: Prefix compaction

通常の compaction は `Memento` として要約を生成して履歴を置換する。要約リクエスト自体がコンテキスト上限を超えた場合は、元の OpenAI 版にあるフォールバックに合わせ、履歴の先頭から項目を削って直近の履歴を残したまま再試行する。この再試行以降の request metadata には `strategy: "prefix_compaction"` を記録する。

これは利用者が圧縮方式を選ぶ設定ではなく、コンテキスト超過時だけ自動適用されるリトライ経路である。ローカル compaction と Responses API 形状の metadata は引き続き provider-neutral な共通処理から利用し、provider-specific な履歴保持契約は追加しない。

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

1. **Marketplace publish contract**: server 側の bundle、署名、認証、version、registry API を決め、現在の client-side contract と接続する。
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

# Chat Completion wire adapter 設計

## Status

設計承認済み。実装前レビュー待ち。

## 目的

組み込み `my-provider` を Chat Completions 経路で動かし、provider 固有の SSE と
Usage payload を既存の Core の応答・Usage 記録経路へ接続する。

Responses API は現在の具象実装を維持する。任意の設定 provider が独自の adapter を
登録する汎用 plugin 機構は今回の対象外とする。

## 現状

現在の実行時 dispatch は `ModelProviderInfo.wire_api` を `ModelClient::stream` が直接
match している。

```text
WireApi::Responses -> Responses HTTP/WebSocket の既存具象経路
WireApi::Chat     -> ChatRequestBuilder -> ChatClient -> process_chat_sse
```

Chat 経路の `process_chat_sse` は通常の text/tool event を `ResponseEvent` に変換するが、
Chat response の top-level `usage` は保持せず、完了イベントの `token_usage` は現在
`None` になる。

## 設計方針

### 1. 設定値とランタイム実装を分離する

`WireApi` は TOML/config/catalog に保存できる provider の wire protocol 宣言として残す。
ただし、Core の実行時分岐は `WireApi` を直接参照せず、`ModelProvider` が解決した
runtime API descriptor を利用する。

概念的な型は次のとおりとする。

```rust
pub enum ProviderApi {
    Responses,
    Chat(Arc<dyn ChatCompletionApi>),
}
```

`Responses` は既存の `ResponsesClient` / Responses WebSocket 実装を選ぶマーカーとし、
Responses 用 Trait を新設しない。

`ConfiguredModelProvider` は既存の `wire_api` から次のように解決する。

```text
wire_api = responses -> ProviderApi::Responses
wire_api = chat     -> ProviderApi::Chat(StandardChatCompletionApi)
```

`MyModelProvider` は `ProviderApi::Chat(MyProviderChatCompletionApi)` を直接返す。
これにより、組み込み provider だけが独自 adapter を使い、通常の設定 provider は
従来どおり標準 Chat adapter を使える。

### 2. Chat の wire adapter だけを Trait 化する

Trait は HTTP transport、認証、リトライを所有しない。これらは既存の
`EndpointSession` と `ChatClient<T>` に残す。

Trait は `codex-api` に置き、object-safe にする。`HttpTransport` が generic/RPITIT
ベースであるため、Trait のメソッドに transport の型パラメータや async return を
露出させない。

想定する責務は次の2つである。

```rust
pub trait ChatCompletionApi: Send + Sync {
    fn build_request(
        &self,
        input: ChatRequestInput<'_>,
    ) -> Result<ChatRequest, ApiError>;

    fn spawn_stream(
        &self,
        response: StreamResponse,
        idle_timeout: Duration,
        telemetry: Option<Arc<dyn SseTelemetry>>,
    ) -> ResponseStream;
}
```

`ChatRequestInput` は model、instructions、history、tools、conversation/session metadata
を保持する provider-neutral な入力値とする。標準 adapter は現在の
`ChatRequestBuilder` の変換を委譲する。

`spawn_stream` は HTTP response の SSE bytes を Core が消費する
`ResponseEvent` stream に変換する。標準 adapter は現在の `process_chat_sse` を利用し、
`my-provider` adapter は共通の Chat protocol primitives を再利用しながら provider 固有
Usage を処理できる構造にする。

低レベルの SSE/`StreamResponse` を扱う `MyProviderChatCompletionApi` の実装は
`codex-api` に置く。`codex-api` が response stream と parser の所有者であり、
`model-provider` に HTTP/SSE parser の依存を広げないためである。`model-provider` の
`MyModelProvider` はこの adapter を選択・注入するだけにする。

### 3. ChatClient は adapter を受け取る

`ChatClient<T>` は `Arc<dyn ChatCompletionApi>` を保持する。

```text
ChatClient<T>::stream_request
  -> EndpointSession<T>::stream_encoded_json_with
  -> ChatCompletionApi::spawn_stream
```

Core の Chat request 作成も adapter 経由にし、`ModelClient` が標準の
`ChatRequestBuilder` を直接呼ばないようにする。これにより将来 request payload の
差分が必要になっても、adapter の実装に閉じ込められる。

### 4. Provider API の解決場所

`ModelProvider` に runtime API descriptor を返すメソッドを追加する。

```rust
fn api(&self) -> ProviderApi;
```

デフォルト実装は `self.info().wire_api` から標準 adapter/Responses を解決する。
`MyModelProvider` だけがこのメソッドを override し、`MyProviderChatCompletionApi` を
注入する。

`ProviderApi` は `Arc` を使って clone 可能にする。provider object 自体は既存どおり
`Arc<dyn ModelProvider>` で共有し、adapter は request 間で mutable state を持たない。
SSE の途中状態は `spawn_stream` が生成する stream task のローカル state に置く。

## Usage の設計

### 入力

Chat Completions の各 SSE JSON event に top-level `usage` が存在する場合、adapter は
その値を読み取る。`my-provider` が毎回 Usage を返す場合も、完了までの最新値を保持し、
最後に観測した値を採用する。

標準形の変換は次のとおりとする。

| Chat Completions | `TokenUsage` |
| --- | --- |
| `prompt_tokens` | `input_tokens` |
| `prompt_tokens_details.cached_tokens` | `cached_input_tokens` |
| `completion_tokens` | `output_tokens` |
| `completion_tokens_details.reasoning_tokens` | `reasoning_output_tokens` |
| `total_tokens` | `total_tokens` |

未知の provider-specific field は token usage の計算に使わない。必要な場合だけ
`ResponseUsageMetadata.metadata` に raw JSON を保持する。

### 出力経路

```text
Chat SSE usage
  -> ChatCompletionApi adapter
  -> ResponseEvent::Completed { token_usage, usage_metadata, ... }
  -> Session::record_observed_response_completed
  -> RawResponseCompletedEvent

  -> Session::record_token_usage_info / update_token_usage_info
  -> TokenCountEvent / thread usage aggregation
```

`finish_reason = stop`、`finish_reason = tool_calls`、`[DONE]`、stream EOF のいずれで
完了しても、保持中の Usage を `Completed` に付与する。Usage が一度も観測されなければ
従来どおり `None` とし、入力からの推定値で補完しない。

標準 Chat Completions の final usage-only chunk（`choices: []`）も破棄せず、Usage を
更新してから通常の完了処理へ進む。

## エラー処理

- Usage が欠落、一部欠損、または型不正の場合、テキスト/tool stream 全体は継続する。
- `total_tokens` など数値の変換不能時は、その Usage snapshot を無視して `None` または
  直前の valid snapshot を使う。
- SSE protocol の破損、transport error、idle timeout は現在と同じ `ApiError` とする。
- Usage の存在だけを理由に完了イベントを早めて発行しない。
- adapter は provider 固有の HTTP 認証復旧を担当せず、既存の `ModelProvider`/Core の
  unauthorized recovery を利用する。

## 実装対象ファイル

### `crates/codex-api`

- `src/chat_completion.rs`（新規）
  - `ChatCompletionApi`
  - `ChatRequestInput`
  - 標準 adapter
- `src/lib.rs`
  - 新しい Trait/型の export
- `src/endpoint/chat.rs`
  - adapter を `ChatClient` に注入
- `src/requests/chat.rs`
  - 標準 request builder の adapter 内への移動または委譲
- `src/sse/chat.rs`
  - Usage snapshot の解析・保持
  - final usage-only chunk 対応
- 既存の Chat SSE/request tests
  - adapter と Usage の単体テストを追加

### `crates/model-provider`

- `src/provider.rs`
  - `ProviderApi` と `ModelProvider::api()` の runtime interface
- `src/my_provider.rs`
  - `MyProviderChatCompletionApi` の選択・注入
- `src/lib.rs`
  - 必要な runtime API 型の export
- provider tests
  - built-in `my-provider` が Chat adapter を返すことを検証

### `crates/core`

- `src/client.rs`
  - `ModelProvider::api()` を使った dispatch
  - Chat request construction を adapter 呼び出しへ変更
  - Responses branch は既存の具象呼び出しを維持
- Chat/Core integration tests
  - `Completed.token_usage` が session usage に伝播することを検証

## 非対象

- Responses API の Trait 化
- 任意の設定 provider が外部から adapter を登録する plugin/registry API
- Chat と Responses の共通 request model の大規模な再設計
- Usage がない場合の tokenizer 推定
- provider-specific Usage の一般的な課金モデル抽象化

## 検証計画

Rust の production code を追加・移動・refactor するため、実装後は repository 方針に
従い次を実行する。

```text
just check
just check --tests
```

checked-in generated artifact や protocol schema が変更された場合は追加で
`just generate` を実行し、生成差分を確認する。単独の test command は実行しない。

主要な検証ケースは次のとおりとする。

1. built-in `my-provider` の runtime API が `Chat(MyProviderChatCompletionApi)` になる。
2. standard Chat adapter が従来の request payload と text/tool event を維持する。
3. Usage が通常 chunk にある場合、完了イベントに反映される。
4. Usage-only chunk（`choices: []`）を処理できる。
5. Usage が複数 chunk にある場合、最後の valid snapshot を使う。
6. Chat Usage が Core の raw completed event と token-count aggregation に届く。
7. Responses の HTTP/WebSocket 経路に変更がない。

## 実装順序

1. `ChatCompletionApi`、`ChatRequestInput`、標準 adapter の境界を追加する。
2. 既存 Chat request/SSE 処理を標準 adapter に移し、既存挙動を固定するテストを追加する。
3. `ChatClient` に adapter を注入する。
4. `ProviderApi` と `ModelProvider::api()` を追加し、`my-provider` のみ custom adapter を選択する。
5. Core の Chat dispatch を runtime API descriptor に切り替える。
6. Usage parser と Core propagation tests を追加する。
7. `just check`、`just check --tests` を実行し、生成物の影響があれば `just generate` を実行する。

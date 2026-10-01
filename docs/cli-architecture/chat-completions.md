# ChatCompletionAPI のアーキテクチャ

このページは、`wire_api = "chat"` を指定したモデルプロバイダーが利用する
Chat Completions 経路の、現行ソースに基づく実装マップです。固定コミットの CLI
アーキテクチャスナップショットとは異なり、Chat Completions 実装の更新に合わせて
更新します。

## 全体フロー

```text
model_providers.<name>.wire_api = "chat"
  -> codex-core::ModelClient::stream_chat_completions
  -> ChatRequestBuilder
  -> ChatClient::stream
  -> POST {provider.base_url}/chat/completions
  -> SSE (`data: ...`, `[DONE]`)
  -> Chat Completions SSE adapter
  -> codex_api::ResponseEvent
  -> core の通常の turn / tool 実行
```

Responses API のデフォルト経路とは分離されており、`wire_api` が `responses` の
プロバイダーはこの経路を通りません。プロバイダーの URL、認証、HTTP ヘッダー、
リトライ設定、stream idle timeout は共通の provider/client 設定を利用します。

## 対応機能

| 機能 | 対応範囲 | 実装上の扱い |
| --- | --- | --- |
| テキスト入出力 | 対応 | system instructions と履歴を `messages` に変換し、assistant の content delta を通常の出力イベントにする |
| ストリーミング | 対応 | リクエストに `stream: true` を設定し、SSE を逐次処理する。Core では通常、turn 単位の集約ストリームを利用する |
| 画像入力 | 対応 | `InputImage` を Chat Completions の `image_url` content part に変換する。inline URL と file reference を入力として扱う |
| function tools | 対応 | Responses 形式の function tool 定義を Chat Completions 形式の `{ type: "function", function: ... }` に変換する |
| tool call | 対応 | `tool_calls` の複数 call、delta をまたぐ関数名・引数の連結、index/ID による追跡を行い、`FunctionCall` に戻す |
| tool output | 対応 | `FunctionCallOutput` を `role: "tool"` と `tool_call_id` に変換する。テキストと画像の output content part を扱う |
| reasoning | 部分対応 | `delta.reasoning` または `message.reasoning` の文字列・`text`・`content` を reasoning event に変換する。Chat Completions 標準の reasoning 契約を定義するものではない |
| 複数ターンの履歴 | 対応 | assistant message、function call、tool output を Chat Completions の message 列へ変換する。連続する tool call は一つの assistant message にまとめる |
| 認証復旧 | 対応 | Chat 経路でも共通の auth manager による unauthorized recovery を利用し、必要な場合にリクエストを再試行する |
| リクエスト/SSE テレメトリ | 対応 | `/chat/completions` の route、HTTP request、SSE poll、認証リトライを既存の telemetry/inference trace に接続する |

### ツール定義とツール履歴の境界

Chat Completions に送信する tool 定義は function type のみです。Responses API の
tool 定義を変換する際、function 以外の tool は送信対象から除外されます。そのため、
Chat 経路で利用できるモデル側の tool 呼び出しは function tool が中心です。

一方、過去の turn を再構成する request builder は `FunctionCall`、
`FunctionCallOutput`、`CustomToolCall`、`CustomToolCallOutput`、`LocalShellCall`
を内部履歴として表現できます。これは履歴を共通 `ResponseItem` から request に
写像するための処理であり、Chat Completions サーバーが custom tool や local shell
tool をネイティブに提供することを意味しません。

## リクエスト構築

`codex-api::ChatRequestBuilder` が次の payload を構築します。

```json
{
  "model": "<model>",
  "messages": [
    {"role": "system", "content": "<instructions>"},
    "..."
  ],
  "stream": true,
  "tools": ["<function tools>"]
}
```

会話 ID がある場合は `session_id` ヘッダーを付与し、sub-agent セッションでは
`x-provider-subagent` ヘッダーも付与します。実際の認証ヘッダーや provider 固有の
ヘッダーは `EndpointSession` と共通の API client が追加します。

履歴の主な変換は次のとおりです。

- `Message` は `role` と text content に変換する。ユーザーの画像を含む場合は text と `image_url` の content parts を使う。
- `FunctionCall` は assistant message 内の `tool_calls` に変換する。連続する call は同じ assistant message にまとめる。
- `FunctionCallOutput` と `CustomToolCallOutput` は `role: "tool"` の message に変換する。
- `Reasoning` は隣接する assistant message または tool call に `reasoning` として関連付ける。
- 同一内容の連続する assistant message は重複を避けるため一部を抑制する。

## SSE から Core event への変換

`codex-api::sse::chat::process_chat_sse` は SSE の各 JSON event の `choices` を読み、
次の event に正規化します。

| Chat Completions の入力 | Core event |
| --- | --- |
| `delta.content` の文字列、または text content part | `OutputTextDelta` と assistant `Message` |
| `delta.reasoning` / `message.reasoning` | `ReasoningContentDelta` と reasoning item |
| `delta.tool_calls` | 引数を delta 間で連結した `FunctionCall` |
| `finish_reason = "stop"` | assistant/reasoning item を flush して `Completed` |
| `finish_reason = "tool_calls"` | function call item を出力して次の tool 実行へ進む |
| `finish_reason = "length"` | `ContextWindowExceeded` エラー |
| `[DONE]` または `DONE` | 未送信 item を flush して `Completed` |

サーバーが接続を閉じずに sentinel を送る実装にも対応するため、sentinel を受けた
時点で完了イベントを発行します。JSON として解釈できない SSE data はログに記録
してスキップし、stream idle timeout や transport error は `ApiError` に変換します。

Core はこの stream を `AggregateStreamExt::aggregate()` に通します。これにより、
assistant text と reasoning の delta は turn の完了時に集約された item として扱われ、
tool call は個別の出力 item として保持されます。

## 非対応・制約

| 項目 | 状態 |
| --- | --- |
| `output_schema` / Chat 経路の structured output | 非対応。prompt に output schema がある場合は `UnsupportedOperation` を返す |
| Responses WebSocket | 非対応。Chat 経路は HTTP SSE のみ |
| Chat 経路の token usage / response ID | 限定的。現在の SSE adapter は `Completed` に token usage と response ID を設定しない |
| Responses 固有の reasoning summary | 非対応。Chat 側で扱うのは `reasoning` の文字列系 payload |
| Responses 固有の compaction、service tier、verbosity | Chat request には渡さない |
| 非 function のモデル側 tool 定義 | 非対応。送信前に function tool 以外を除外する |
| 独自 streaming protocol | 非対応。Chat Completions 形式の SSE を前提にする |

したがって、ChatCompletionAPI は Responses API の全機能を置き換える実装ではなく、
共通の turn/tool lifecycle を Chat Completions の request/message/SSE 形式へ接続する
限定的な wire adapter です。

## 主要ソース

- [`WireApi`](../../crates/model-provider-info/src/lib.rs): `chat` / `responses` の設定値と既定値
- [`ModelClient::stream_chat_completions`](../../crates/core/src/client.rs): Chat 経路の選択、認証復旧、Core への接続
- [`ChatRequestBuilder`](../../crates/codex-api/src/requests/chat.rs): 履歴・tool・ヘッダーの request 変換
- [`ChatClient`](../../crates/codex-api/src/endpoint/chat.rs): `/chat/completions` への HTTP streaming
- [`process_chat_sse`](../../crates/codex-api/src/sse/chat.rs): SSE の解析と `ResponseEvent` への正規化
- [`create_tools_json_for_chat_completions_api`](../../crates/tools/src/tool_spec.rs): function tool 定義の変換
- [`G-7`](../provider-neutral-gaps.md#g-7): 非 Responses wire API の現状と今後の provider-neutral 化課題

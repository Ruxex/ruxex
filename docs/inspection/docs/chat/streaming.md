---
title: Streamingとイベント処理
---

# Streamingとイベント処理

## Streamingの役割

Providerは完成した1つのJSONを返すのではなく、Response Itemの生成途中にイベントを送ります。CoreはそれをUIイベント、履歴、Tool実行の3方向へ変換します。

```text
Provider stream
  -> event parser
  -> response item started / delta / completed
       -> UI lifecycle events
       -> assistant text / reasoning display
       -> completed item persistence
       -> Tool Call runtime
```

## 完了したItemの扱い

Assistant messageやReasoningは、ストリーム中にDeltaを表示しつつ、Itemが完了した時点で正規の履歴へ記録します。Tool Callは、完了したCallを先に履歴へ記録し、その後にTool Futureを起動します。

この順序により、Tool実行中にTurnがキャンセルされても「モデルがToolを要求した」という事実をRolloutから復元できます。

## Retryと同じTurnのSession

`ModelClientSession` はTurnスコープです。Streaming失敗が再試行可能な場合、同じTurnのSessionとRetry Stateを使って再試行します。Retryしても、ユーザーから見たTurnは増えません。

一方、Context Window超過や認証エラーのように再試行できないエラーは、Turn Error lifecycleを発行して終了します。

## 主なソース

- [`crates/core/src/session/turn.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/session/turn.rs): `run_sampling_request`、`try_run_sampling_request`
- [`crates/core/src/stream_events_utils.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/stream_events_utils.rs): Itemの完了・履歴記録・Tool Call振り分け
- [`crates/core/src/client.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/client.rs): Provider Client Session境界

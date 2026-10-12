---
title: エラーとキャンセル
---

# エラーとキャンセル

Turn中のエラーは、すべて同じように扱われるわけではありません。

| 種類 | 典型的な扱い |
| --- | --- |
| Retry可能なStream/Transport Error | Turn-scoped Client Sessionを使って再試行 |
| Context Window超過 | Token状態を更新し、Compactionまたはエラーへ |
| 認証・Provider Error | Protocol Error Eventを発行してTurnを終了 |
| Invalid Image / Input | ユーザーに修正を促すError Event |
| Tool拒否 | Tool Outputとしてモデルへ返し、Follow-upを許す場合がある |
| `TurnAborted` | キャンセルを保持し、通常の継続処理を行わず終了 |
| Stop Hookの継続 | Hook Promptを履歴へ追加し、同じTurnを継続 |

Cancellation Tokenは、Provider Stream、MCP起動、Tool実行、Context収集などへ伝播します。非同期処理の途中で止まっても、すでに完了して永続化されたItemは消えません。

## Pending Inputとキャンセルの違い

UIからTurn中に入力を送ると、Pending InputとしてQueueに入り、現在のSamplingが完了したあとに次のStepへ取り込まれます。これはTurnのキャンセルではありません。

明示的なInterruptはTurnを中断します。したがって、次のユーザー入力が「同じTurnのFollow-up」なのか「新しいTurn」なのかは、QueueとTurn admissionの状態で決まります。

## 主なソース

- [`crates/core/src/session/turn.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/session/turn.rs)
- [`crates/app-server/src/turn_admission.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/app-server/src/turn_admission.rs)
- [`crates/core/src/responses_retry.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/responses_retry.rs)

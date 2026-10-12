---
sidebar_position: 5
title: 障害調査と観測性
---

# 障害調査と観測性

## どの境界で失敗したかを切り分ける

```text
Client / TUI
  -> App Server Protocol
  -> Turn admission
  -> Context capture
  -> Provider request / stream
  -> Tool approval / sandbox / executor
  -> Rollout write
  -> SQLite projection
```

表示されない、応答が遅い、履歴が欠ける、Toolが拒否される、DBが起動しない、という症状は別の境界で起きます。最初に「どこまで進んだか」を確認します。

## ログ

Logs DBにはLevel、Target、Thread ID、Process UUID、推定サイズを持つログ行が保存されます。Threadに紐づくログでも、それは会話履歴ではありません。

検索時はThread IDだけでなく、Process UUID、Target、時刻を使います。複数ClientやApp Serverが同じThreadを扱う場合、Thread IDだけでは足りないことがあります。

## Metrics

SQLite初期化、Migration、Backfill、Fallback、Log Queue、Rollout Persistence、Provider Requestなどの境界にはMetricsがあります。Metricは「何が遅いか」より先に「どの段階で失敗したか」を見るために使います。

代表的な観測対象は次です。

- DB open / migrate
- Backfill gate
- Rollout write / projection
- Provider stream request / retry
- Turn TTFTと総時間
- Tool CallとApproval
- Queue dropped / pending input

## Rolloutを直接見る

RolloutはJSONLなので、まずPrefixや末尾を確認します。正規記録を読むときは、SQLiteの一覧表示より先にRolloutのOrdinal、Turn、Itemを確認します。

```bash
jq -C . /path/to/.genex/sessions/*.jsonl | less -R
```

圧縮済みRolloutやWriter Lockがある場合は、`genex-rollout` のReader・Materialize・Lock処理を確認します。

## DBを調べる

DBはProvider Home Markerを確認したうえで、読み取り専用で調べます。

```bash
sqlite3 /path/to/.genex/state_5.sqlite '.tables'
sqlite3 /path/to/.genex/thread_history_1.sqlite '.schema thread_items'
```

DBにレコードがないことだけで「データがない」と判断しません。RolloutがCanonicalで、Projectionが未完了・遅延している可能性があります。

## 主なソース

- [`crates/state/src/log_db.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/log_db.rs)
- [`crates/state/src/telemetry.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/state/src/telemetry.rs)
- [`crates/rollout/src/persistence_metrics.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/rollout/src/persistence_metrics.rs)
- [`crates/core/src/turn_timing.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/turn_timing.rs)

---
title: Compaction
---

# Compaction

## 目的

Compactionは、履歴がContext WindowやAuto CompactのToken上限に近づいたときに、過去の内容を要約・圧縮し、同じThreadの次のSamplingを可能にする処理です。

## 2つのタイミング

- Pre-sampling: 新しいユーザー入力を履歴へ確定する前に、これから追加されるContextを考慮して実行する
- Mid-turn: Sampling後にFollow-upが必要だが、Token上限に達したときに実行する

Compaction後もTurn Context、World State、必要なTool/Environmentは維持されます。新しいTurnを作るのではなく、Context Windowを進めます。

## 実装の分岐

現在のコードでは、Feature設定とProvider Capabilityに応じて、Token Budget、Remote Compaction V2、Local Compactionの経路が選ばれます。ProviderがRemote Compactionを実装していない場合でも、CoreのLocal経路で明示的に処理されます。

## 主なソース

- [`crates/core/src/session/turn.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/session/turn.rs): `run_pre_sampling_compact`、`run_auto_compact`
- [`crates/core/src/compact.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/compact.rs)
- [`crates/core/src/compact_remote_v2.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/compact_remote_v2.rs)

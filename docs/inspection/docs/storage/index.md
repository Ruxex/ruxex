---
sidebar_position: 1
title: 永続化
---

# 永続化

Genexの会話状態は、Rollout JSONLと複数のSQLite Projectionに分かれて保存されます。SQLiteだけを見ても会話全体は分からず、Rolloutだけを見ても一覧・検索・Queueの現在状態は分かりません。

まず [保存領域全体図](./storage-map) を読み、その後 [SQLite](./sqlite) と [Rollout](./rollouts) を読み進めてください。

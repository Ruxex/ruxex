---
sidebar_position: 1
title: チャット処理
---

# チャット処理

チャット処理の中心は、1つのTurnの中で複数のSamplingとTool実行を繰り返すCoreのループです。モデルに1回リクエストして終わり、ではありません。

読む順番は [チャットループ](./chat-loop) → [チャット前のコンテキスト収集](./context-assembly) → [ストリーミング](./streaming) → [Tool Loop](./tool-loop) です。

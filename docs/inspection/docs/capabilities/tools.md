---
sidebar_position: 2
title: Tools
---

# Tools

Toolは、モデルが外部世界へ作用するためのCoreの実行単位です。Tool Routerがモデルに公開する仕様と、Tool Runtimeが実際に行う処理を接続します。

## 2つの責務

- Model-visible spec: 名前、説明、Input Schema、Deferred LoadingなどをProvider Requestへ載せる
- Execution: CallをApproval、Exec Policy、Sandbox、Network Policyの下で実行する

Tool Callは、モデルの権限昇格ではありません。実行環境が許可しない操作は拒否され、拒否や出力も次のSamplingへ返されます。

## 主なソース

- [`crates/core/src/tools/spec_plan.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/tools/spec_plan.rs)
- [`crates/core/src/tools/parallel.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/tools/parallel.rs)
- [`crates/tools/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/tools/src/lib.rs)

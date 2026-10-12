---
sidebar_position: 6
title: Sandboxと権限
---

# Sandboxと権限

Toolの実行可否は、Approval、Exec Policy、File System Sandbox、Network Policy、Environmentの組み合わせで決まります。

```text
Model Tool Call
  -> argument validation
  -> approval decision
  -> exec policy
  -> filesystem / network sandbox
  -> executor
  -> output
```

Permission ProfileはStepContextとEnvironmentに結びつきます。現在のCWDやWorkspace Rootが変わると、同じTool名でも実行できるパスが変わります。

## 主なソース

- [`crates/execpolicy/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/execpolicy/src/lib.rs)
- [`crates/sandboxing/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/sandboxing/src/lib.rs)
- [`crates/core/src/exec_policy.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/exec_policy.rs)
- [`crates/core/src/network_policy_decision.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/network_policy_decision.rs)

---
sidebar_position: 3
title: MCP
---

# MCP

MCPは外部ServerのToolやResourceを、現在のTurnのCapabilityへ接続します。

入力中の `mcp://` Mention、Plugin依存、Skill依存を解析して必要なServerを起動・Refreshし、その結果をStepContextのMCP BindingとTool Routerへ渡します。

MCPのCredentialや接続状態は、モデル履歴へそのまま保存されるものではありません。モデルへ見えるのは、公開されたTool仕様と実行結果です。

## 主なソース

- [`crates/core/src/mcp.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/mcp.rs)
- [`crates/core/src/session/mcp_runtime.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/session/mcp_runtime.rs)
- [`crates/genex-mcp/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/genex-mcp/src/lib.rs)

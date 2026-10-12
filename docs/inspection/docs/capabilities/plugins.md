---
sidebar_position: 4
title: Plugins
---

# Plugins

Pluginは、MCP Server、Skill、追加Prompt、Capability設定をまとめて提供する構成単位です。

Turn開始時には、ユーザー入力のMentionとConfigを使って有効Pluginを決めます。Pluginは必要なMCP Serverを要求でき、Capability Summaryを通してTool計画へ参加します。

## 主なソース

- [`crates/core/src/plugins/mod.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/plugins/mod.rs)
- [`crates/core/src/plugins/injection.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/plugins/injection.rs)
- [`crates/plugin/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/plugin/src/lib.rs)

---
sidebar_position: 5
title: Skills
---

# Skills

Skillは、モデルへ追加指示を注入し、必要なら依存ToolやPluginを要求する仕組みです。

入力に明示的なSkill Mentionがあると、Skill SnapshotからPrompt Fragmentを読み、依存するMCP ToolやPluginを解決します。読み込まれたSkillの指示は、通常のユーザーメッセージとは異なる注入項目として履歴へ追加されます。

## 主なソース

- [`crates/core/src/skills.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/skills.rs)
- [`crates/skills/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/skills/src/lib.rs)
- [`crates/core/src/session/turn.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/core/src/session/turn.rs)

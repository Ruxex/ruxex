---
sidebar_position: 4
title: 変更時の読み順
---

# 変更時の読み順

## 変更したい場所から逆引きする

| 変更対象 | 先に確認する境界 | 影響しやすい保存・Protocol |
| --- | --- | --- |
| モデルへのPrompt | `run_turn`、`StepContext`、`build_prompt` | Rollout、Token Count、Provider Request |
| 新しいTool | `ToolRouter`、`ToolCallRuntime`、Approval | Tool Schema、Tool Call Item、Sandbox |
| MCP連携 | MCP Manager、Plugin/Skill依存解決 | Credential isolation、Tool events、Rollout |
| Provider機能 | `ModelProvider` Contract、Capability | App Server catalog/auth/usage、Runtime共有状態 |
| Login/Logout | Credential Manager、Provider snapshot | 全Client、In-flight Turn、Auth Error |
| Thread metadata | State schema、`apply_rollout_item` | SQLite Migration、Backfill、一覧Protocol |
| 履歴形式 | Rollout Item、Thread History Projection | Resume、Fork、Rollback、Migration |
| Turnイベント | Core Event Mapping、App Server Protocol | TUI buffer、Desktop client、Rollout |
| Sandbox/権限 | Permission Profile、Exec Policy、Environment | Tool実行、PromptへのContext、Security |
| DB分割 | `SqliteConfig`、StateRuntime、各Migrator | Lock、Backfill、Recovery、Telemetry |

## ProducerとConsumerを先に探す

型やFieldを変更する前に、次の順で調べます。

1. 型・Fieldを定義するProducer
2. JSON/SQLite/Rolloutへ書き込むWriter
3. Resume/List/Protocolで読むReader
4. UIや外部Clientで表示するConsumer
5. テストFixtureと生成Schema

特に永続Fieldは、Rustの型を変更するだけでは不十分です。Migration、Backfill、Rollout extractor、App Server serialization、TUI表示まで同時に確認します。

## 変更後のチェック

サイト本文のソース説明を変更した場合は、コード上の実装も再確認します。DB Schemaを変更した場合は、`crates/state` のSQLと `docs/inspection/docs/storage/tables.md` の両方を更新します。

このサイト自身を変更した場合は、次を実行します。

```bash
cd docs/inspection
pnpm typecheck
pnpm build
```

## 現行境界を壊さないための問い

- この状態はThreadに属するのかRuntimeに属するのか
- Provider固有情報をCoreやProtocolへ漏らしていないか
- Credentialを履歴・Rollout・Threadへ保存していないか
- 1つのTurn内の別StepでSnapshotの一貫性が崩れないか
- RolloutよりSQLite Projectionが先に進んでいないか
- Client表示の変更がProtocolのProducer/Consumer双方に反映されているか

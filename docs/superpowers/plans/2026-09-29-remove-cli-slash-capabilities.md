# CLI スラッシュ機能削除 主計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `/app`、`/experimental`、`/fast` の入口と機能本体を段階的に削除し、関係しない Desktop 診断、通常の feature/config、model selection、他の service tier、provider-neutral capability を維持する。

**Architecture:** 小さな `/app` の Desktop セッション引き継ぎを先に削除し、次に app-server protocol/schema を含む `/experimental`、最後に Core/config/TUI を横断する `/fast` を削除する。各機能は独立した子計画で producer/consumer を監査し、共通の compatibility inventory と最終監査で統合する。

**Tech Stack:** Rust workspace、Cargo、TUI、app-server protocol、生成 JSON/TypeScript schema、`just generate`、`just check --tests`、`just check`。

**Spec:** [`docs/superpowers/specs/2026-09-29-remove-cli-slash-capabilities-design.md`](../specs/2026-09-29-remove-cli-slash-capabilities-design.md)

## Global Constraints

- `AGENTS.md` に従い `main` ブランチで作業し、Git worktree は作成しない。
- 各 production match はコード変更前に producer/consumer を監査し、`docs/pre-release-compatibility-inventory.md` に `remove` / `retain` / `migrate-in-place` を記録する。
- `/app` では Desktop アプリ本体、CLI `doctor` の Desktop 診断、別用途の Desktop protocol 型を削除しない。
- `/experimental` では通常の `Feature` registry/config と個別 feature runtime を、`/fast` では通常の model selection、Flex/他の service tier、一般の `service_tier` transport を保持する。
- Rust source/test target の変更後は `just check --tests` と `just check` を実行する。
- checked-in generated artifact に影響する変更後は `just generate` を実行して diff を確認する。
- standalone test は実行しない。完了主張の前に `just check --tests`、`just check`、必要な `just generate`、`git diff --check` の結果を確認する。

## Review Focus

- `/app` の削除で Desktop 本体や CLI `doctor` の診断経路を誤って削除しないこと。子計画 Task 1–3。
- `/experimental` の RPC と generated schema を削除しても、通常の config feature enablement と残存 app-server API が一致すること。子計画 Task 2–4。
- `/fast` の削除で Flex/他の service tier、一般の model selection、provider-neutral な request transport を壊さないこと。子計画 Task 2–5。
- 既存設定・履歴・キャッシュを読み書きする互換層を誤って追加・保持しないこと。各子計画の inventory task。
- 3機能に由来する古い snapshots、generated schema、workspace/dependency、ドキュメントが残らないこと。主計画 Task 5。

## 子計画

| 順序 | 機能 | 子計画 | 主な境界 |
|---|---|---|---|
| 1 | `/app` | [`2026-09-29-remove-app-slash-command.md`](2026-09-29-remove-app-slash-command.md) | CLI/TUI の Desktop セッション引き継ぎのみ |
| 2 | `/experimental` | [`2026-09-29-remove-experimental-features.md`](2026-09-29-remove-experimental-features.md) | TUI popup、app-server RPC、protocol/schema |
| 3 | `/fast` | [`2026-09-29-remove-fast-mode.md`](2026-09-29-remove-fast-mode.md) | FastMode、Fast tier、Core/config/TUI 表示 |
| 4 | 最終監査 | 本書 Task 5 | workspace、generated artifact、文書、inventory |

## Tasks

### Task 1: 共通 producer/consumer 監査と inventory の準備

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/openai-decoupling-candidates.md`（必要な Phase 3 handoff のみ）

**Interfaces:**
- Produces: 3子計画が参照する decision、保持境界、検証コマンド。

- [x] `/app`、`experimentalFeature`、`FastMode`、Fast service tier、関連 generated artifact の production match を `rg` と `cargo metadata --no-deps` で収集する。
- [x] 各 match を producer/consumer、削除対象、保持対象に分類し、inventory に exact path と decision を記録する。
- [x] `/app` の Desktop 本体/doctor、`/experimental` の通常 feature registry、`/fast` の Flex/一般 service tier を retain として明記する。
- [x] 子計画の実施順と、inventory decision がコード変更より先に必要であることを確認する。

### Task 2: `/app` 子計画の実行

**Files:**
- Reference: `docs/superpowers/plans/2026-09-29-remove-app-slash-command.md`

- [x] `/app` 子計画の Task 1–4 を順に実行する。
- [x] 子計画完了時点で `OpenDesktopThread`、`codex://threads` の専用 producer/consumer、`SlashCommand::App` が対象範囲から消えていることを確認する。

### Task 3: `/experimental` 子計画の実行

**Files:**
- Reference: `docs/superpowers/plans/2026-09-29-remove-experimental-features.md`

- [x] `/experimental` 子計画の Task 1–5 を順に実行する。
- [x] app-server protocol の Rust source と generated JSON/TypeScript schema を同じ変更単位として確認する。
- [x] 通常の feature registry/config/CLI 経路が残っていることを子計画の保持監査で確認する。

### Task 4: `/fast` 子計画の実行

**Files:**
- Reference: `docs/superpowers/plans/2026-09-29-remove-fast-mode.md`

- [x] `/fast` 子計画の Task 1–6 を順に実行する。
- [x] Fast 固有の feature、tier、keymap/status、Core 判定を削除し、Flex/一般 tier と model selection を残す。
- [x] Fast 固有の config/schema/test fixture を削除または更新し、既存設定を削除する migration は追加しない。

### Task 5: 最終 dependency・generated artifact・documentation audit

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/openai-decoupling-plan.md`、`docs/openai-specific-crates.md`（Phase 3 status/handoff のみ）
- Modify: `docs/cli-architecture/`（削除した command/capability の記述がある場合のみ）
- Modify/Delete: 残存 snapshots、fixtures、generated schema、workspace/dependency references

- [x] `rg -n 'SlashCommand::(App|Experimental)|OpenDesktopThread|codex://threads|experimentalFeature/(list|enablement/set)|ExperimentalFeature|Feature::FastMode|fast_mode|SPEED_TIER_FAST|toggle_fast_mode' crates Cargo.toml Cargo.lock docs --glob '*.{rs,json,ts,toml,md}'` を実行し、残存 match を production/test/history に分類する。
- [x] `/app`、`/experimental`、`/fast` の専用 production match がないこと、retain とした Desktop doctor、通常 feature、Flex/一般 service tier のみが残ることを確認する。
- [x] `cargo metadata --no-deps` と必要な dependency graph を確認し、削除した機能専用 crate/dependency/module の孤立参照がないことを確認する。
- [x] `just generate` を実行し、生成物の差分が計画した protocol/config/model schema の変更だけであることを確認する。
- [x] `just check --tests`、`just check`、`git diff --check` を実行する。
- [x] 設計書の完了条件、3子計画の完了条件、inventory の全 decision を照合し、Phase 3 の handoff を更新する。

## 実行上の注意

各子計画は独立してレビュー可能な単位だが、実装順は `/app` → `/experimental` → `/fast` とする。`/experimental` と `/fast` で共有される feature/config/protocol の境界に新たな依存が見つかった場合は、先に inventory と設計書を更新し、子計画の削除範囲を再確認してからコードを変更する。

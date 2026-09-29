# `/experimental` 機能削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `/experimental` の TUI popup と、実験的 feature catalog/enablement の app-server protocol・processor・generated schema・専用テストを削除する。

**Architecture:** TUI の popup を入口として、`FetchExperimentalFeatures` / `SaveExperimentalFeatures` event、`experimentalFeature/list` / `experimentalFeature/enablement/set` RPC、protocol v2 型、app-server processor、generated JSON/TypeScript schema を一つの capability として撤去する。通常の `Feature` registry、config、`codex features` CLI、個別 feature runtime は保持する。

**Tech Stack:** Rust app-server/TUI、JSON-RPC protocol、generated JSON/TypeScript schemas、`just generate`。

**Spec:** [`docs/superpowers/specs/2026-09-29-remove-cli-slash-capabilities-design.md`](../specs/2026-09-29-remove-cli-slash-capabilities-design.md)

## Global Constraints

- `main` で作業し、worktree は作成しない。
- protocol/config/source の production match ごとに inventory decision を先に記録する。
- 通常の `Feature` registry、feature config、`codex features list/enable/disable`、個別 feature runtime は保持する。
- generated JSON/TypeScript は手編集せず、Rust source 更新後に `just generate` で再生成する。
- Rust/test target の変更後は `just check --tests` と `just check` を実行する。standalone test は実行しない。

## Review Focus

- `/experimental` の popup、候補、dispatch が消え、残存 TUI が専用 event を発行しないこと。
- app-server protocol の RPC、request dispatch、processor、client helper、suite registration が一貫して消えること。
- generated schema から experimental feature 専用型・method が消え、他の schema が壊れていないこと。
- 通常の feature config と個別 feature runtime が保持され、`codex features` の経路が壊れないこと。
- skills/model list など、enablement RPC を test setup に使う別 consumer を見落とさず、保持または代替を明記すること。

## File Map

- Modify: `crates/tui/src/slash_command.rs`、`crates/tui/src/chatwidget/slash_dispatch.rs`、`settings_popups.rs`、`experimental_features.rs`、`bottom_pane/experimental_features_view.rs`、`app_event.rs`、`app/event_dispatch.rs`。
- Modify/Delete: TUI popup tests/snapshots、`experimental_features_tests.rs`、専用 imports。
- Modify: `crates/app-server-protocol/src/protocol/v2/experimental_feature.rs`、`protocol/v2/mod.rs`、`protocol/common.rs`、exports/tests。
- Modify: `crates/app-server/src/request_processors.rs`、`catalog_processor.rs`、`config_processor.rs`、`message_processor.rs`、suite registration/tests/common helpers。
- Modify/Delete: `crates/app-server-protocol/schema/json/`、`schema/typescript/` の generated artifacts。
- Modify: `docs/pre-release-compatibility-inventory.md`、必要な CLI architecture docs。

### Task 1: RPC/UI producer/consumer audit と inventory decision

- [ ] `rg -n 'SlashCommand::Experimental|open_experimental_popup|ExperimentalFeaturesView|FetchExperimentalFeatures|SaveExperimentalFeatures' crates/tui` で TUI producer/consumer を列挙する。
- [ ] `rg -n 'ExperimentalFeature|experimentalFeature/(list|enablement/set)' crates/app-server crates/app-server-protocol` で protocol、dispatch、processor、test helper、suite consumer を列挙する。
- [ ] `rg -n 'Feature::|FEATURES|codex features|enable_feature_for_new_threads' crates/cli crates/tui crates/core crates/features crates/config` で通常の feature registry/config/runtime の保持境界を確認する。
- [ ] `docs/pre-release-compatibility-inventory.md` に専用 RPC/UI を `remove`、通常の feature/config/runtime を `retain` として記録する。skills/model list などの別 consumer は個別 decision を付ける。

### Task 2: TUI popup と slash command を削除

- [ ] `SlashCommand::Experimental` と description/availability、`slash_dispatch.rs` の popup dispatch、composer の allowlist/queue match を削除する。
- [ ] `settings_popups.rs` の `open_experimental_popup` と、専用 event を発行するコードを削除する。
- [ ] `experimental_features.rs`、`experimental_features_view.rs` の runtime wiring、専用 module/export、関連 TUI tests/snapshots を削除する。通常の settings/feature UI は残す。
- [ ] `app_event.rs` の `FetchExperimentalFeatures` / `SaveExperimentalFeatures` と `event_dispatch.rs` の専用 match arm を削除する。

### Task 3: Protocol と app-server processor を削除

- [ ] `protocol/v2/experimental_feature.rs`、module/export、`protocol/common.rs` の request/response variants と serialization tests を削除する。
- [ ] `message_processor.rs` の request dispatch、`catalog_processor.rs` の list response、`config_processor.rs` の enablement mutation を削除する。
- [ ] `request_processors.rs` の imports、app-server suite registration、test client helper、`experimental_feature_list.rs` と enablement 専用 test case を削除または通常の config test へ移行する。
- [ ] `skills_list.rs`、`model_list.rs` など残存する enablement RPC consumer を再監査し、現行テスト・機能を壊さない代替経路へ移すか、専用シナリオなら削除する。

### Task 4: Generated schema を再生成して整合性を確認

- [ ] Rust protocol source と app-server dispatch の削除後に `just generate` を実行する。
- [ ] `rg -n 'ExperimentalFeature|experimentalFeature/(list|enablement/set)' crates/app-server-protocol/schema` で専用 generated type/method が残っていないことを確認する。
- [ ] generated diff が専用 RPC/type の削除とそれに伴う index/export 更新に限定され、通常の protocol schema を変更していないことを確認する。

### Task 5: 検証と保持境界の確認

- [ ] `rg -n 'SlashCommand::Experimental|ExperimentalFeaturesView|FetchExperimentalFeatures|SaveExperimentalFeatures|experimentalFeature/(list|enablement/set)|ExperimentalFeature' crates Cargo.toml Cargo.lock docs --glob '*.{rs,json,ts,toml,md}'` の残存 match を production/test/history に分類する。
- [ ] 通常の `Feature` registry、config schema、`codex features` CLI、個別 feature runtime の compile path が残っていることを確認する。
- [ ] `just check --tests` を実行する。
- [ ] `just check` を実行する。
- [ ] `git diff --check` を実行し、inventory row と本計画の完了条件を照合する。

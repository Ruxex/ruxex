# `/app` スラッシュ機能削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** TUI の `/app` による現在セッションの Desktop アプリ引き継ぎ機能を、slash command、event、OS 別 launcher、専用テストまで削除する。

**Architecture:** `SlashCommand::App` → `AppEvent::OpenDesktopThread` → `App::open_desktop_thread` → `codex://threads/<id>` launcher の専用経路を切断する。Desktop アプリ本体、CLI `doctor` の Desktop 診断、別用途の Desktop protocol 型は保持する。

**Tech Stack:** Rust TUI、crossterm event loop、macOS `open`、Windows PowerShell/package launcher、TUI snapshots。

**Spec:** [`docs/superpowers/specs/2026-09-29-remove-cli-slash-capabilities-design.md`](../specs/2026-09-29-remove-cli-slash-capabilities-design.md)

## Global Constraints

- `main` で作業し、worktree は作成しない。
- `docs/pre-release-compatibility-inventory.md` に producer/consumer audit と decision を先に記録する。
- Desktop 本体、`crates/cli/src/doctor/desktop*`、Desktop app-server の診断、IDE context は削除しない。
- Rust/test target の変更後は `just check --tests` と `just check` を実行する。standalone test は実行しない。

## Review Focus

- `/app` の parser/popup/dispatch から候補と実行経路が完全に消えること。
- `OpenDesktopThread` を削除しても他の `AppEvent` dispatch が壊れないこと。
- `history_ui.rs` の通常の履歴・browser URL 処理を残し、Desktop launcher だけを除去すること。
- macOS/Windows/unsupported platform の cfg 分岐と専用 error message/snapshot が残らないこと。
- Desktop doctor と `DesktopOnboardingEntrypoint` を誤って削除しないこと。

## File Map

- Modify: `crates/tui/src/slash_command.rs` — `App` variant/description/availability/visibility。
- Modify: `crates/tui/src/chatwidget/slash_dispatch.rs`、`crates/tui/src/bottom_pane/chat_composer.rs` — dispatch/queue/parent-owned command list。
- Modify: `crates/tui/src/app_event.rs`、`crates/tui/src/app/event_dispatch.rs` —専用 event と handler。
- Modify: `crates/tui/src/app/history_ui.rs` — Desktop URL launcher と専用 message/test のみ。
- Modify/Delete: `crates/tui/src/chatwidget/tests/slash_commands.rs`、history UI snapshots/tests。
- Modify: `docs/pre-release-compatibility-inventory.md`。

### Task 1: `/app` 経路の audit と inventory decision

- [ ] `rg -n 'SlashCommand::App|OpenDesktopThread|codex://threads|open_desktop_thread|Desktop app' crates/tui crates/cli crates/app-server-protocol` で producer/consumer を列挙する。
- [ ] `rg -n 'DesktopOnboardingEntrypoint|doctor/desktop|desktop application' crates` で別用途の Desktop 経路を確認する。
- [ ] `docs/pre-release-compatibility-inventory.md` に `/app` 専用経路を `remove`、Desktop doctor・別用途 protocol を `retain` として記録する。

### Task 2: Slash command と event bridge を削除

- [ ] `SlashCommand::App`、description、`available_during_task`、platform visibility を `crates/tui/src/slash_command.rs` から削除する。
- [ ] `chat_composer.rs` と `slash_dispatch.rs` の parent-owned allowlist、queue drain match、dispatch arm、`OpenDesktopThread` 送信を削除する。
- [ ] `AppEvent::OpenDesktopThread` と `event_dispatch.rs` の専用 match arm を削除する。
- [ ] `rg` で `SlashCommand::App` と `OpenDesktopThread` が production/test source に残っていないことを確認する。

### Task 3: OS 別 Desktop launcher と専用テストを削除

- [ ] `history_ui.rs` の `open_desktop_thread`、`open_desktop_thread_url` の macOS/Windows/unsupported 実装、専用定数と error message を削除する。
- [ ] history UI の通常の browser open、session history、model/service-tier 表示を残す。
- [ ] `/app` dispatch test、Desktop opened/error snapshots、未使用 import/fixture を削除または更新する。
- [ ] `rg -n 'codex://threads|open_desktop_thread|Opened this session in the Desktop app|Failed to open this session in the Desktop app' crates/tui` が空であることを確認する。

### Task 4: 検証と境界確認

- [ ] `rg -n -i 'Desktop|desktop' crates/cli/src/doctor crates/tui/src/ide_context crates/app-server-protocol/src` を確認し、非対象の Desktop 診断・protocol・IDE context を保持していることを確認する。
- [ ] `just check --tests` を実行する。
- [ ] `just check` を実行する。
- [ ] `git diff --check` を実行する。
- [ ] 親計画 Task 3 に進む前に inventory の `/app` row と本計画の完了条件を照合する。

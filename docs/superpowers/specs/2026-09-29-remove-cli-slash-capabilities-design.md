# CLI スラッシュ機能削除 設計書

作成日: 2026-09-29

## 概要

Codex TUI の `/app`、`/experimental`、`/fast` を、入力候補だけでなく機能本体まで削除する。3機能は依存範囲が異なるため、共通の主計画から独立した3本の実装計画へ分割する。

## 目的

- 現在のセッションを Desktop アプリへ引き継ぐ `/app` 機能を削除する。
- 実験的機能の一覧取得・有効化・無効化を行う `/experimental` 機能を削除する。
- Fast service tier と FastMode の設定・切替・表示・Core 判定を削除する。
- 削除後に残る provider-neutral な TUI、通常の feature/config、app-server、MCP、sandbox/security の機能を維持する。

## 非目的

- Desktop アプリ本体の削除。Desktop 本体の UI/アプリケーションコードはこのリポジトリにない。
- CLI `doctor` の Desktop インストール・セキュリティ・app-server 診断の削除。これは `/app` のセッション引き継ぎとは別の機能として保持する。
- 全ての `experimental` という名前の protocol や feature の削除。対象は実験的 feature catalog と enablement API/UI であり、他の現行 API の実験フラグ表現は個別監査する。
- 全ての service tier、モデル選択、provider-neutral な設定処理の削除。対象は Fast service tier と FastMode に限定し、他の tier が現行 consumer を持つ場合は保持する。
- 既存の設定、履歴、キャッシュ、ユーザーデータを削除する migration の追加。

## 共通制約

- `AGENTS.md` に従い `main` ブランチで作業し、Git worktree は作成しない。
- 各 production match について producer/consumer を監査し、`docs/pre-release-compatibility-inventory.md` に `remove`、`retain`、`migrate-in-place` の decision をコード変更前に記録する。
- Rust の source/test target を変更した場合は `just check --tests` と `just check` を実行する。
- protocol/config の checked-in generated artifact に影響する場合は `just generate` を実行して diff を確認する。
- standalone test は実行せず、リポジトリの検証方針に従う。
- `legacy`、`fallback`、`experimental` という名前だけを根拠に、現行の platform fallback、security behavior、provider-neutral capability を削除しない。

## 機能別の削除境界

### `/app`: CLI の Desktop セッション引き継ぎ

削除対象は `SlashCommand::App`、`OpenDesktopThread` イベント、TUI の `codex://threads/<id>` 起動処理、macOS/Windows/unsupported platform の専用 launcher、関連メッセージ・snapshot・テストである。

`crates/cli/src/doctor/desktop*`、Desktop app-server の診断、IDE context、protocol に別用途で使われる Desktop 型は producer/consumer を確認したうえで保持する。

### `/experimental`: 実験的 feature catalog と enablement

削除対象は `/experimental` の TUI popup/view、popup 専用の event、`experimentalFeature/list` と `experimentalFeature/enablement/set` の app-server protocol/processor、専用 Rust 型、generated JSON/TypeScript schema、専用テストである。

通常の `Feature` registry、config の feature enable/disable、`codex features` CLI、個別 feature の runtime consumer は別の経路で利用されているため、全削除を前提にせず監査する。残す feature の UI/config/API は canonical な現行経路へ整理する。

### `/fast`: FastMode と Fast service tier

削除対象は Fast service tier を slash command/popup から選択する経路、`Feature::FastMode`、FastMode の config/schema/feature metadata、keymap/status/title 表示、TUI の service-tier toggle、Core の FastMode による turn/session/review 判定、Fast 固有 protocol/model metadata と関連テストである。

通常の model selection、他の service tier、一般の reasoning/model configuration、既存の設定データを削除する migration は保持する。Fast 固有の persisted config は pre-release 方針に従い、互換 no-op や reader を新設せず、未知設定として扱う境界を計画で確認する。

## Producer/consumer の初期監査結果

### `/app`

- Producer: `crates/tui/src/chatwidget/slash_dispatch.rs` の `/app` dispatch。
- Transport: `AppEvent::OpenDesktopThread`。
- Consumer: `crates/tui/src/app/event_dispatch.rs`、`crates/tui/src/app/history_ui.rs` の OS 別 URL launcher。
- Tests/artifacts: `crates/tui/src/chatwidget/tests/slash_commands.rs`、history UI snapshots。
- 非対象: CLI doctor の Desktop 診断と Desktop 本体。

### `/experimental`

- Producer: TUI の `/experimental` dispatch と `open_experimental_popup`。
- Transport: `FetchExperimentalFeatures`、`SaveExperimentalFeatures`。
- Protocol consumers: app-server protocol の `ExperimentalFeature*` 型、`catalog_processor`、request dispatch、client/test helpers。
- Artifacts: app-server JSON/TypeScript schemas と protocol/app-server/TUI tests/snapshots。
- 保持候補: 通常の feature registry/config、個別 feature runtime。

### `/fast`

- Producer: model catalog の service tier、TUI の `ServiceTierCommand` と slash input。
- Consumers: `FastMode` feature gate、service-tier persistence/override、keymap action、status/title surfaces、Core session/turn/review/config logic。
- Artifacts: config schema、model/protocol metadata、TUI snapshots、Core/app-server tests。
- 保持候補: 一般の model/service-tier capability。ただし Fast 専用型・定数・分岐は削除対象。

## 実施順序

1. `/app` を削除する。境界が小さく、Desktop 本体や doctor を誤って削除しないことを検証しやすい。
2. `/experimental` を削除する。protocol/app-server と generated schema を一体で更新する。
3. `/fast` を削除する。Core/config/TUI の横断変更として最後に実施し、他の model/service-tier 経路を保持する。
4. 主計画の最終監査で、3機能の production match、workspace/dependency、generated artifact、ドキュメントを再確認する。

## 完了条件

1. `/app`、`/experimental`、`/fast` が TUI の候補・入力解析・dispatch から消えている。
2. 各機能の専用 runtime、protocol/config/UI/observability が計画の境界どおり削除されている。
3. Desktop 本体、CLI doctor の Desktop 診断、通常の feature/config、model selection、他の service tier、provider-neutral capability、security/platform fallback が保持されている。
4. compatibility inventory に全 production match の audit と decision がある。
5. 必要な generated artifact が `just generate` で source と一致している。
6. `just check --tests`、`just check`、`git diff --check` が成功している。

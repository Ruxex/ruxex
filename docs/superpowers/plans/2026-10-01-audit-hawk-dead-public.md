# Hawk dead_public 個別監査 実行計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 最終 Hawk レポートに残る `dead_public` を個別に producer/consumer 監査し、各項目を `remove`、`retain`、`migrate-in-place` に対応づける。監査開始時点では526 unique items（診断530件、通常/test 重複を含む）を対象とし、最新の二つの production root では525診断・521 unique itemを確認した。

**Architecture:** `hawk.json` の診断を安定した package/item/location 単位の監査台帳へ展開する。生成コード、test-only target、production binary、外部/プロトコル/安全性APIを分離し、Rust参照だけでなく re-export、serde/schema、文字列dispatch、`cfg`/target差分を確認する。削除候補は監査記録を先に追加してから一件ずつ変更し、約20%ごとに Hawk を再実行する。

**Tech Stack:** Rust workspace、Hawk JSON、`jq`/`rg`、Markdown監査表、`just check`、`just check --tests`。

**Spec:** `AGENTS.md`、`docs/provider-decoupling-candidates.md`、`docs/pre-release-compatibility-inventory.md`。

## Global Constraints

- 生成コードは触らない。
- codex CLI と app-server の両 production 起点を対象にする。
- 外部拡張API、protocol/schema、persisted-data、security、platform fallback、別アーキテクチャの利用可能性を保持する。
- Rust変更前に `docs/pre-release-compatibility-inventory.md` へ producer/consumer と decision を記録する。
- Rust変更後は `just check --tests` と `just check` を実行し、standalone test は実行しない。
- Hawk は約20%の処理ごと、または削除バッチ完了時に再実行する。

## Review Focus

- `OUT_DIR` protobuf/prost の重複診断は生成元・wire contractを確認し、生成物を編集しない。
- `test_compiled_only` の test-support API は runtime dead code と混同せず、テストtargetのconsumerを確認する。
- app-server/TUI/core の公開型は sibling binary、core-api、外部hostのre-exportを確認する。
- serde、schema、macro、文字列dispatch、環境変数、`cfg`/target-specific参照を `rg` だけで見落とさない。
- OS/security/compatibility surface は Linux起点の未到達だけで削除しない。

### Task 1: 監査台帳の作成

**Files:**
- Create: `docs/hawk-dead-public-audit.tsv`
- Modify: `docs/pre-release-compatibility-inventory.md`（台帳運用ルールまたは初回分類）

- [x] `hawk.json` から `dead_public` の全診断を抽出し、package/item/kind/location/test flag/duplicate group を台帳化する。
- [x] 生成コード、test-only、production、重複診断を明示する。
- [x] 監査開始時点の診断数530件・unique数526件を検算し、最新の二つの production root では525診断・521 unique itemへ更新した。

### Task 2: 生成コード・テスト支援の個別監査

**Files:**
- Modify: `docs/hawk-dead-public-audit.tsv`
- Modify: `docs/pre-release-compatibility-inventory.md`

- [x] `codex-code-mode-protocol` の生成protobuf項目を、proto/build.rs/consumer/re-export単位で確認し、個別に `retain` と記録する。
- [x] `app_test_support` と `core_test_support` の各項目について、テストtargetからの参照またはfixture用途を確認する。
- [x] 参照のない test-only helper だけを削除候補として抽出し、監査台帳に削除結果を記録する。

### Task 3: production候補の個別監査と低リスク削除

**Files:**
- Modify: 対象Rustファイル（候補ごと）
- Modify: `docs/hawk-dead-public-audit.tsv`
- Modify: `docs/pre-release-compatibility-inventory.md`

- [x] production項目を package/source 単位で約20%ずつ処理する。
- [x] 各項目について workspace参照、re-export、public crate API、protocol/serde/schema、target条件を確認する。
- [x] obsolete alias、完全重複、内部専用の未使用wrapperだけを `remove` または `migrate-in-place` とする。
- [x] 削除ごとに `git diff --check`、`just check --tests`、`just check` を実行する。
- [x] 削除候補の監査結果と削除可否を監査台帳とインベントリへ記録する。

### Task 4: 定期Hawk再計測と台帳照合

**Files:**
- Modify: `hawk.json`
- Modify: `docs/hawk-dead-public-audit.tsv`

- [x] 処理済み約20%ごと、または削除バッチ完了時に `just hawk` を実行する。
- [x] 診断の消滅、重複、追加、location変化を台帳へ反映する。
- [x] stale reportを根拠に判定せず、直近の成功した二つの production root の Hawk JSONだけを採用する。

### Task 5: 最終照合

**Files:**
- Modify: `docs/hawk-dead-public-audit.tsv`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `hawk.json`

- [x] 最新Hawkの全 unique `dead_public` が `remove`、`retain`、`migrate-in-place` のいずれかになっていることを確認する。
- [x] 未対応項目をゼロにし、生成コード・test-only・productionの件数を別々に報告する。
- [x] `git diff --check`、`just fmt-check`、`just check --tests`、`just check`、JSON妥当性確認を実行する。
- [x] 最終件数、削除件数、retain理由、未変更の生成コード、検証結果を監査台帳とインベントリへ記録する。

監査完了: 最新の `hawk.json` は `codex` と `codex-app-server` の二つの production root を含み、`dead_public` は525診断・521 unique item。現行521件は全て `retain`、監査中に削除した過去項目は6件である。単一rootの旧 `codex-cli.json` は履歴資料として保持する。

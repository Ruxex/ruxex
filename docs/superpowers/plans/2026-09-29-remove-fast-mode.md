# `/fast` / FastMode 機能削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `/fast` と FastMode による Fast service tier の選択・永続化・表示・Core 判定を削除し、一般の model selection、Flex/他の service tier、provider-neutral な request transport を維持する。

**Architecture:** Fast 固有の `Feature::FastMode` gate、model catalog の Fast 判定、TUI の dynamic service-tier slash command、keymap/status/title、config default/validation、Core の session/turn/review 分岐を producer から consumer まで撤去する。`service_tier` という一般の transport/config field は残し、Fast 固有の enum/constant/metadata は現行 consumer を監査して削除する。

**Tech Stack:** Rust protocol/config/features/core/TUI、model catalog、TUI keymap/status surfaces、generated config schema、Cargo。

**Spec:** [`docs/superpowers/specs/2026-09-29-remove-cli-slash-capabilities-design.md`](../specs/2026-09-29-remove-cli-slash-capabilities-design.md)

## Global Constraints

- `main` で作業し、worktree は作成しない。
- Fast 固有の production match をコード変更前に inventory へ記録する。
- `ServiceTier::Flex`、custom/unknown service-tier strings が現行 consumer を持つ場合は保持する。一般の model/service-tier request field は削除しない。
- 既存の `service_tier = "fast"` を消す migration やユーザーデータ削除を追加しない。pre-release の未知設定扱い・validation 境界を明記する。
- FastMode 以外の feature registry/config、通常の reasoning/model selection、provider-neutral core runtime を保持する。
- Rust/test target の変更後は `just check --tests` と `just check`、config/model schema に影響する場合は `just generate` を実行する。standalone test は実行しない。

## Review Focus

- dynamic service-tier list から Fast だけが消え、Flex/他の現行 tier と model picker が残ること。
- `Feature::FastMode` 削除が Core の turn/session/review、TUI の effective tier、app-server config に一貫して反映されること。
- keymap/status/title/config schema の Fast 専用項目が消え、他の項目の順序・既定値・設定 parsing が壊れないこと。
- Fast 固有の OpenAI model metadata/test fixture を削除しても、一般の `ModelServiceTier` と custom provider の service tier が残ること。
- 既存設定や履歴を破壊的に削除せず、削除後の unsupported/unknown config の扱いが明確であること。

## File Map

- Modify: `crates/features/src/lib.rs`、feature schema/generated config metadata。
- Modify: `crates/protocol/src/config_types.rs`、`openai_models.rs`、model/protocol tests。
- Modify: `crates/config/src/` の FastMode/config default/requirements/keymap/schema consumers。
- Modify: `crates/core/src/` の FastMode gate、agent/session/turn/review/config logic と関連 tests。
- Modify/Delete: `crates/tui/src/service_tier_resolution.rs`、`chatwidget/service_tiers.rs`、slash input/composer/popup、keymap、status/title、tooltips、snapshots/tests。
- Modify: `crates/app-server/src/` と app-server/core tests の FastMode-specific config consumers。
- Modify: `docs/pre-release-compatibility-inventory.md`、必要な decoupling/architecture docs。

### Task 1: FastMode と Fast tier の producer/consumer audit

- [ ] `rg -n 'FastMode|fast_mode|SPEED_TIER_FAST|ServiceTier::Fast|supports_fast_mode|toggle_fast_mode|/fast|service_tier.*fast' crates Cargo.toml Cargo.lock docs --glob '*.{rs,json,ts,toml,md}'` で全 match を収集する。
- [ ] `cargo metadata --no-deps` と model/config dependency を確認し、Fast 固有の crate/module/generated artifact と一般 service-tier consumer を区別する。
- [ ] Core の `agent/role`、session/turn/review、config の `service_tier` validation、TUI の service-tier resolution/keymap/status/title を producer/consumer 表にする。
- [ ] inventory に Fast 固有を `remove`、Flex/一般 service tier/model transport を `retain`、現行 consumer が移行を要する共有 helper を `migrate-in-place` として記録する。

### Task 2: Feature registry/config schema と managed defaults を削除

- [ ] `Feature::FastMode`、feature spec/key/stage/default、feature display/schema/apply mapping の producer/consumer を削除する。
- [ ] `crates/config/src/` の FastMode-specific validation、managed requirement fixture、Fast 固有 default/compatibility branch を削除し、一般 `service_tier` と Flex/custom value の parsing を残す。
- [ ] `crates/config/src/tui_keymap.rs` の `toggle_fast_mode` field と config schema entry を削除する。既存 keymap の他 binding は保持する。
- [ ] generated config schema が管理対象なら `just generate` を実行し、FastMode/keymap の削除 diff を確認する。

### Task 3: Protocol/model metadata から Fast 固有の型・判定を削除

- [ ] `ServiceTier::Fast`、`SPEED_TIER_FAST`、Fast 固有の `request_value`/parse branch を、Flex/一般 string transport を壊さない形で削除する。
- [ ] `ModelPreset::supports_fast_mode`、Fast 固有の `additional_speed_tiers` 判定、Fast 固有 model fixture/test を削除または一般 service-tier capability 判定へ移行する。
- [ ] `ModelServiceTier`、`service_tier_for_request`、Flex/custom provider の protocol serialization は保持する。
- [ ] `just generate` を実行し、protocol/model generated artifact に Fast 固有 schema がある場合だけ更新する。

### Task 4: TUI の `/fast`、service-tier command、keymap、表示を削除

- [ ] `current_model_service_tier_commands` から Fast 固有の slash command生成・FastMode gate・`toggle_service_tier_from_ui` の不要経路を削除し、他の tier を残す。
- [ ] `InputResult::ServiceTierCommand`、service-tier command popup、slash input の dynamic command support を、他の tier が残るかを確認したうえで最小化または削除する。
- [ ] `toggle_fast_mode` keymap action/runtime/input dispatch、FastMode keymap picker/tests/snapshots を削除する。
- [ ] status line、terminal title、status preview、model header、tooltip、Fast 固有 snapshots/tests を削除し、一般 model/service-tier 表示を保持する。

### Task 5: Core/app-server runtime の FastMode 判定を削除

- [ ] Core の agent role、session step activation、turn context、review、startup/default、client request で `Feature::FastMode` または Fast 固有 tier を参照する分岐を削除する。
- [ ] app-server/TUI の config/session initialization で FastMode-specific effective-tier resolution を削除し、一般 service_tier の明示値・Flex/custom value の transport を保持する。
- [ ] Fast 固有の tests/fixtures を削除または一般 tier の regression coverage に更新する。Fast 以外の service-tier serialization tests は保持する。
- [ ] `rg` で FastMode gate が残っていないことを確認し、残存する `fast` は unrelated wording/history/test data として分類する。

### Task 6: 最終検証と設定・依存境界の確認

- [ ] `rg -n 'Feature::FastMode|fast_mode|SPEED_TIER_FAST|ServiceTier::Fast|supports_fast_mode|toggle_fast_mode|/fast' crates Cargo.toml Cargo.lock docs --glob '*.{rs,json,ts,toml,md}'` を実行し、専用 production match が残っていないことを確認する。
- [ ] `rg -n 'ServiceTier::Flex|ModelServiceTier|service_tier' crates/core crates/config crates/protocol crates/tui --glob '*.rs'` を確認し、一般 service-tier/model path が保持されていることを確認する。
- [ ] `cargo metadata --no-deps` で Fast 専用 crate/dependency/module の孤立参照がないことを確認する。
- [ ] 必要な `just generate` の diff を確認する。
- [ ] `just check --tests` を実行する。
- [ ] `just check` を実行する。
- [ ] `git diff --check` を実行し、inventory row と本計画の完了条件を照合する。

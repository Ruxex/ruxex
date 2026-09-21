# Guardian OpenAI統合削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** OpenAI Responses backendに依存するGuardian自動レビュー・自動承認経路を削除し、通常の手動承認、sandbox、permission security behaviorを保持する。

**Architecture:** `codex-guardian-v2`のResponses HTTP/WebSocket scorerとapp-server extension wiringを削除する。Guardian専用のreviewer/context/config/protocol/UI surfaceも削除し、coreの一般的なapproval requestとpermission evaluationをprovider-neutralな基盤として残す。

**Tech Stack:** Rust, `codex-guardian-v2`, `codex-guardian-reviewer`, `codex-guardian-context`, core approval flow, app-server protocol, TUI, `just check`。

**Spec:** `docs/openai-decoupling-plan.md`, `docs/pre-release-compatibility-inventory.md`

## Global Constraints

- `CodexAuth`、Responses WebSocket、OpenAI internal headers、Codex backend route専用の自動レビューを削除する。
- 手動approval request、sandbox policy、permission deny/allow behaviorは保持する。
- Guardian削除を理由にprovider-neutralなMCP、core tool approval、通常のreviewable actionを削除しない。
- テストコマンドは実行せず、`just check`を使用する。

## Review Focus

- Guardianが失敗したときだけ許可される経路を残さず、通常の手動承認に明確に戻ること。
- `approvals_reviewer`、`review_model`、`auto_review`などの設定がprotocol/schema/TUIに残らないこと。
- `codex-guardian-*`のworkspace reverse dependencyが消えること。
- OpenAI internal headerやbackend-route判定がproduction codeに残らないこと。
- sandbox/permission/security behaviorの一般経路がGuardian削除で弱くならないこと。

### Task 1: Guardian capabilityのproducer/consumer監査を完了する

**Files:**
- Inspect: `crates/ext/guardian-v2/`
- Inspect: `crates/ext/guardian-reviewer/`, `crates/guardian-context/`
- Inspect: `crates/core/src/guardian/`, `crates/core/src/permissions*`
- Inspect: `crates/app-server/src/extensions.rs`, app-server request/notification processors
- Inspect: `crates/config`, `crates/protocol`, `crates/tui`, `crates/prompts`
- Modify: `docs/pre-release-compatibility-inventory.md`

  - [x] **Step 1: auto-reviewとmanual approvalの境界を記録する**

  Guardian専用のapproval reviewer、review session、context composer、通知、設定、TUI操作を列挙し、通常approval/sandboxの保持対象と分離する。

  - [x] **Step 2: OpenAI依存の根拠をinventoryに記録する**

  `ResponsesWebsocketClient`、`ResponsesClient`、`CodexAuth`、`x-openai-subagent`、`x-codex-guardian`、`supports_codex_backend_routes`のproducer/consumerを`remove`として記録する。

  - [x] **Step 3: Guardian capability削除の範囲を固定する**

  Guardian自動レビューとその設定/UI/protocol surfaceを削除し、手動approvalとsandboxをretainすることをinventoryに明記する。

### Task 2: backend-dependent extensionとGuardian cratesを削除する

**Files:**
- Modify: `crates/app-server/src/extensions.rs`
- Modify: workspace `Cargo.toml`
- Delete: `crates/ext/guardian-v2/`
- Delete: `crates/ext/guardian-reviewer/`
- Delete: `crates/guardian-context/`
- Modify: `crates/core/Cargo.toml`, `crates/prompts/Cargo.toml`, `crates/app-server/Cargo.toml`

  - [x] **Step 1: app-server installationを削除する**

  `codex_guardian_v2::install`とGuardian extension wiringを削除し、他のapp-server extensionsは保持する。

  - [x] **Step 2: crate reverse dependenciesを除去する**

  core/prompts/app-server/test-supportのGuardian依存を削除し、workspace memberとpackage metadataから3クレートを除去する。

  - [x] **Step 3: backend-specific errors/importsを整理する**

  Guardian crate由来のauth/http/provider imports、OpenAI internal header、Responses route判定、Guardian専用telemetryをproduction codeから除去する。

### Task 3: core/config/protocolのGuardian surfaceを削除する

**Files:**
- Modify/Delete: `crates/core/src/guardian/`
- Modify: `crates/core/src/config/`
- Modify: `crates/config/src/`
- Modify: `crates/protocol/src/`
- Modify: app-server request/notification processors and generated schema sources

  - [x] **Step 1: Guardian専用core stateとreview sessionを削除する**

  `GuardianThreadContext`、Guardian review session、Guardian approval request/notificationのproduction経路を削除する。一般approval requestとpermission evaluationの実装は残す。

  - [x] **Step 2: Guardian専用configを削除する**

  `approvals_reviewer`、`review_model`、`auto_review`、`guardian_policy_config`、managed Guardian requirementsと関連validator/defaultを削除する。

  - [x] **Step 3: protocol/schemaを更新する**

  Guardian専用request、event、status、config schemaを削除し、通常のapproval/permission protocolは保持する。生成物がある場合は`just generate`対象にする。

### Task 4: TUI/prompts/tests/docsを整理する

**Files:**
- Modify/Delete: Guardian menus, popups, shortcuts, notifications under `crates/tui`
- Modify: `crates/prompts`
- Delete/Modify: Guardian fixtures and tests under `crates/core`, `crates/app-server`, `crates/tui`
- Modify: `docs/openai-decoupling-plan.md`
- Modify: `docs/pre-release-compatibility-inventory.md`

  - [x] **Step 1: Guardian UIを削除する**

  Guardian reviewer選択、auto-review表示、Guardian専用failure/status表示を削除し、通常のpermission promptとapproval UIを残す。

  - [x] **Step 2: Guardian prompt/contextを削除する**

  Guardian専用のcontext/prompt builderとfailure-review保持処理を削除し、一般的なtool/permission promptを変更しない。

  - [x] **Step 3: fixturesとdocsを整理する**

  Guardian専用fixture、dummy auth、OpenAI-specific review scenario、関連docsを削除する。一般approval/security scenarioは残す。

### Task 5: 生成物とrepository checkを実行する

**Files:**
- Modify: checked-in schemas, snapshots, generated bindings affected by Task 3 or 4
- Modify: `docs/pre-release-compatibility-inventory.md`

  - [x] **Step 1: generated artifactsを更新する**

  protocol/config/schemaの変更があるため`just generate`を実行し、Guardian専用生成物が消えたことを確認する。

  - [x] **Step 2: production symbol scanを実行する**

  `rg -n -i 'guardian|auto_review|review_model|approvals_reviewer|x-openai-subagent|x-codex-guardian|supports_codex_backend_routes' crates --glob '*.{rs,toml,json}'`を実行し、保持対象の一般的なapproval/security参照を除いてGuardian production surfaceが残っていないことを確認する。

  - [x] **Step 3: repository checkを実行する**

  `just check`を実行する。テストコマンドは実行しない。

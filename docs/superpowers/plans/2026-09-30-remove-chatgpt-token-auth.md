# ChatGPT系トークン認証削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** ChatGPT系の保存トークンおよびWorkload Identity認証を削除し、API Key、Amazon Bedrock、汎用Headers認証、MCP OAuthなどの非ChatGPT認証だけを残す。

**Architecture:** 認証の中心を `ApiKey`、Bedrock認証、プロバイダー固有の外部Bearer/Headersに限定する。`requires_first_party_auth` は `requires_auth` に移行し、起動時の認証要求を表すフィールドとして再定義する。Workload Identityのproducerを先に各consumerから外し、依存関係がなくなったことを確認してからクレート本体を削除する。

**Tech Stack:** Rust workspace、Cargo、app-server protocol、TUI、TOML/JSON schema、`just check`、`just check --tests`、`just generate`

**Spec:** `docs/provider-decoupling-candidates.md` の first-party auth/model-provider adapter候補、および `docs/pre-release-compatibility-inventory.md` の ChatGPT authentication / Agent Identity capability監査結果。ユーザー要求により、現行 `AuthMode::WorkloadIdentity` のretain判断をremoveへ更新する。

## Global Constraints

- API Key、Amazon Bedrock、汎用Headers認証、通常のMCP OAuth、provider-neutralなcredential storageは保持する。
- ChatGPT OAuth、device login、PAT、ChatGPT JWT claim、Workload Identity token exchange、managed account専用経路は削除する。
- `requires_auth` は「起動時にCodexの認証フローで認証確認・入力を要求するか」を表す。環境変数、コマンド、AWS設定で認証するproviderにAPI Key入力画面を誤表示しない。
- 既存のトークン専用 `auth.json` は認証済みとして扱わない。既存ファイルを起動時に自動削除せず、`logout` またはAPI Key保存で明示的に更新する。
- 既存のprovider-neutral capability、セキュリティ挙動、プラットフォームfallbackを名前だけで削除しない。
- 作業は `main` ブランチで行い、worktreeは作成しない。
- Rustの変更後は `just check --tests` と `just check` を実行する。protocol/config/schemaの生成物に影響する場合は `just generate` も実行する。

## Review Focus

- トークンだけを持つ旧 `auth.json` が起動時にAPI Key認証済みと判定されないこと（Task 2）。
- API Key保存後は認証画面からチャット画面へ遷移し、API Keyがない場合は認証画面へ戻ること（Task 4）。
- Bedrock providerの専用セットアップが汎用API Key画面に置き換わらないこと（Task 4）。
- command-backed external bearer/Headers providerとMCP OAuthがWorkload Identity削除の巻き添えにならないこと（Task 2、Task 5）。
- 生成済みprotocol/schemaとCargo workspaceから削除済み型・クレートへの参照が残らないこと（Task 5）。

### Task 1: Producer/consumer台帳と認証境界の更新

**Files:**

- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`
- Inspect: `crates/login/src/auth/manager.rs`
- Inspect: `crates/login/src/auth/workload_identity.rs`
- Inspect: `crates/model-provider/src/auth.rs`
- Inspect: `crates/codex-mcp/`

**Interfaces:**

- Consumes: 現行のWorkload Identity producer/consumer監査結果。
- Produces: `WorkloadIdentity`、ChatGPT claim、汎用Headers、Bedrock、MCP OAuthのremove/retain判断。

- [x] **Step 1: 既存台帳のWorkload Identity判断を更新する**

`AuthMode::WorkloadIdentity`、`CodexAuth::WorkloadIdentity`、`codex-workload-identity`、host-managed token exchange、managed account consumerを `remove` として記録する。`AuthMode::Headers`、external bearer、Bedrock、API Key、MCP OAuthは `retain` として明記する。

- [x] **Step 2: consumer分類を確認する**

`rg` と `cargo metadata --no-deps` で、削除対象と保持対象の境界を記録する。特にuser verification、attestation、managed model catalog、ChatGPT usage/rate-limit、cyber accessの各consumerを削除対象として列挙する。

### Task 2: `codex-login` の認証モデルと保存形式をAPI Key中心へ移行

**Files:**

- Modify: `crates/login/src/auth/manager.rs`
- Modify: `crates/login/src/auth/storage.rs`
- Modify: `crates/login/src/auth/mod.rs`
- Delete: `crates/login/src/auth/workload_identity.rs`
- Delete: `crates/login/src/token_data.rs`
- Modify: `crates/login/src/lib.rs`
- Test: `crates/login/src/auth/manager.rs` の認証・保存テスト、必要に応じて専用テストモジュール

**Interfaces:**

- Consumes: API Key、Bedrock、Headersの既存認証インターフェース。
- Produces: `CodexAuth::{ApiKey, Headers, BedrockApiKey, BedrockAccessKeys}` に限定された `AuthManager`。

- [x] **Step 1: トークン専用認証が未認証になるテストを書く**

トークンとChatGPT claimだけを含む保存データを読み込んだ場合、`AuthManager` がAPI Key認証を返さず、起動時アカウントが未認証になるテストを追加する。API KeyとBedrockの既存テストも維持する。

- [x] **Step 2: テスト対象が意図どおり失敗することを確認する**

Run: `just check --tests`

実装前の確認は、リポジトリ方針に従いスタンドアロンテストではなくテストターゲットのコンパイルを用いる。

- [x] **Step 3: Workload Identityとtoken refreshを削除する**

`CodexAuth`、`AuthManager`、auth storageからWorkload Identity、JWT/token data、refresh/revoke、managed account identityを削除する。Headersを使うexternal authの更新経路は残す。

- [x] **Step 4: 保存形式をAPI Key／Bedrockに限定する**

`AuthDotJson` からtoken専用フィールドを削除し、旧トークン専用データは読み込んでも認証状態にならないようにする。自動削除や秘密情報のログ出力は追加しない。

- [x] **Step 5: 対象テストターゲットをコンパイルする**

Run: `just check --tests`

Expected: token-onlyは未認証、API Key／Bedrock／Headersのテストターゲットがコンパイルできる。

### Task 3: protocol/configのmanaged authと命名を整理

**Files:**

- Modify: `crates/protocol/src/auth.rs`
- Modify: `crates/protocol/src/config_types.rs`
- Modify: `crates/config/src/auth_policy.rs`
- Modify: `crates/config/src/config_requirements.rs`
- Modify: `crates/config/src/config_toml.rs`
- Modify: `crates/model-provider-info/src/lib.rs`
- Modify: `crates/core/config.schema.json`（生成物の場合は手編集せず生成）
- Test: `crates/config/src/`、`crates/model-provider-info/src/` の該当テスト

**Interfaces:**

- Consumes: Task 2で限定された認証モード。
- Produces: `requires_auth` を持つprovider metadataと、Workload Identityを含まないprotocol/config型。

- [x] **Step 1: `requires_auth` の期待動作をテストで固定する**

`requires_auth = true` のproviderは起動時に認証要求対象、`false` のproviderはAPI Key画面を表示しないことをテストする。`env_key`、command-backed auth、Bedrockの専用経路が誤ってAPI Key画面を出さないことも確認する。

- [x] **Step 2: `requires_first_party_auth` を `requires_auth` に移行する**

production code、config schema、protocol bootstrap、provider testsの名前と説明を更新する。意味は「起動時のCodex認証フローを要求するか」に限定する。

- [x] **Step 3: Workload Identity／managed-only policyを削除する**

`AuthMode::WorkloadIdentity` と Workload Identity 専用の protocol/config surface を削除する。汎用 provider の認証ポリシー、Headers の provider auth、Bedrock、MCP OAuth は残す。

- [x] **Step 4: protocol/configテストと生成確認を行う**

Run: `just generate`

Expected: generated JSON/TypeScript/schemaからWorkload Identityと旧フィールドが消え、`requires_auth` が反映される。

### Task 4: app-server、TUI、CLIの起動認証フローを移行

**Files:**

- Modify: `crates/app-server/src/request_processors/account_processor.rs`
- Modify: `crates/app-server/src/auth_mode.rs`
- Modify: `crates/app-server/src/in_process.rs`
- Modify: `crates/tui/src/lib.rs`
- Modify: `crates/tui/src/app_server_session.rs`
- Modify: `crates/tui/src/onboarding/auth.rs`
- Modify: `crates/tui/src/startup_orchestration.rs`
- Modify: `crates/cli/src/login.rs`
- Modify: `crates/cli/src/main.rs`
- Test: `crates/app-server/tests/suite/v2/account.rs`
- Test: `crates/tui/src/` の起動・onboardingテスト

**Interfaces:**

- Consumes: Task 2/3のAPI Key／Bedrock／Headers認証モデルと `requires_auth`。
- Produces: 認証情報がない場合だけAPI Key入力画面を表示し、成功後にチャットへ遷移する起動フロー。

- [x] **Step 1: 起動時の失敗ケースを追加する**

認証不要providerではチャット画面、`requires_auth=true` かつ認証なしではAPI Key画面、token-only保存データではAPI Key画面になるテストを追加する。

- [x] **Step 2: managed auth分岐を削除する**

Workload Identity選択時の起動 bypass、account/status表示、token refresh要求、managed login制約を削除する。Bedrock setupとAPI Key loginは保持する。

- [x] **Step 3: CLI status/logoutを整理する**

`codex login status` はAPI Key／Bedrock／未認証のみを表示し、Workload Identityの分岐を削除する。`logout` はAPI Key／Bedrock storageを対象にする。

- [x] **Step 4: 起動テストターゲットをコンパイルする**

Run: `just check --tests`

Expected: 認証なし・API Keyあり・token-only・Bedrockの各テストターゲットがコンパイルできる。

### Task 5: Workload Identity consumer、依存、クレート本体を削除

**Files:**

- Modify: `crates/model-provider/src/auth.rs`
- Modify: `crates/model-provider/src/provider.rs`
- Modify: `crates/model-provider/src/models_endpoint.rs`
- Modify: `crates/core/src/` のmanaged account／ChatGPT claim consumer
- Modify: `crates/app-server/src/` のverification、attestation、managed account consumer
- Modify: `crates/tui/src/` のmanaged account、usage、status consumer
- Modify: `crates/cli/src/doctor.rs`
- Modify: `Cargo.toml`
- Modify: `crates/login/Cargo.toml`
- Delete: `crates/workload-identity/`
- Update: `Cargo.lock`
- Test/Delete: Workload Identity専用fixtureと統合テスト

**Interfaces:**

- Consumes: Task 2〜4で削除された認証型とprotocol surface。
- Produces: Workload Identityへの依存がないworkspaceと、API Key／Bedrock／Headersのテスト網羅性。

- [x] **Step 1: runtime consumerを置換または削除する**

ChatGPT account ID、plan、managed auth predicate、専用rate-limit、user verification、attestation、cyber accessなどを、汎用provider capabilityに置換できるものは置換し、ChatGPT専用で代替のないものは削除する。

- [x] **Step 2: test consumerを整理する**

Workload Identity fixture、token exchange、refresh、managed account専用ケースを削除する。API Key、Bedrock、Headers、MCP OAuthのテストは残す。

- [x] **Step 3: dependency edgeを削除する**

`crates/login/Cargo.toml`、workspace `Cargo.toml` から `codex-workload-identity` の依存とmember登録を削除する。

- [x] **Step 4: 参照がないことを確認してからクレートを削除する**

Run: `cargo metadata --no-deps`、`cargo tree -i codex-workload-identity`、`rg -n 'WorkloadIdentity|workload_identity|codex-workload-identity' crates Cargo.toml`

Expected: production/test consumer、workspace member、dependency edgeが残っていないこと。

- [x] **Step 5: 生成物とworkspaceを検証する**

Run: `just generate`

Expected: lockfile、schema、TypeScript出力から削除済み型・依存が消える。

### Task 6: 全体検証とドキュメント確定

**Files:**

- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-candidates.md`

- [x] **Step 1: repository-wide残存参照を確認する**

ChatGPT OAuth、device login、PAT、Workload Identity、managed auth、`requires_first_party_auth` のproduction参照がないことを確認する。保持対象のHeaders、Bedrock、MCP OAuthは除外条件を明記する。

- [x] **Step 2: 必須検証を実行する**

Run: `just check --tests`

Expected: test targetのコンパイル成功。

Run: `just check`

Expected: production workspaceのtype-check成功。

- [x] **Step 3: 最終差分を確認する**

Run: `git diff --check` および `git status --short`

Expected: 意図した削除・移行だけが残り、秘密情報や不要な生成差分がないこと。`codex-cli.json` の Hawk 出力は、ユーザー指定により更新せず現状を保持する。

## 進捗追記（2026-09-30）

- Workload Identity 削除後に残っていた未登録テストソース `crates/core/tests/suite/client.rs` と `code_mode.rs` を削除した。
- Workload Identity の環境変数スクラブ、obsolete な認証 claim、ChatGPT 専用テストヘルパーを削除した。
- `docs/pre-release-compatibility-inventory.md` と `docs/provider-decoupling-candidates.md` に producer/consumer 監査結果と判断を反映した。
- `codex-cli.json` は Hawk 更新を行わず、残存する古い finding は明示的に保留した。
- 検証済み: `just check --tests`、`just check`、`git diff --check`。

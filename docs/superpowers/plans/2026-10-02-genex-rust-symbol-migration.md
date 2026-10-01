# Genex Rust Symbol Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** workspace の全クレートを対象に `Codex` / `codex_*` の Rust シンボルを棚卸しし、fan-out と互換性リスクの低い順に Genex へ移行するか、根拠付きで `retain` / `defer` として確定する。

**Architecture:** まず repository-wide inventory を作り、private・test-only・leaf crate のシンボルから移行する。次に低 fan-out の workspace 公開型、crate 内部の共有型、最後に高 fan-out の core/API 型を個別に移行する。`CODEX_HOME`、永続化識別子、wire/protocol 型、外部認証契約はこの計画の後段へ明示的に分離する。

**Tech Stack:** Rust workspace、Cargo metadata、`rg`、`just check`、`just check --tests`、`just generate`、Markdown inventory。

**Spec:** [`docs/provider-decoupling-plan.md`](../../provider-decoupling-plan.md)、[`docs/provider-decoupling-candidates.md`](../../provider-decoupling-candidates.md)、[`docs/pre-release-compatibility-inventory.md`](../../pre-release-compatibility-inventory.md)

## Global Constraints

- repository 全体への機械的な `codex` 全置換は行わない。
- producer/consumer と互換性範囲を監査し、各 slice に `remove`、`retain`、`migrate-in-place` の decision を記録する。
- `CODEX_HOME`、`find_codex_home`、`codex_home` の設定・認証・rollout・SQLite 境界は、private な局所変数と証明できない限り変更しない。
- provider 名、外部 wire 契約、既存 persisted data、security boundary は名前だけを理由に変更しない。
- Rust 変更ごとに `just check --tests` と `just check` を実行し、生成物に影響する場合は `just generate` を実行して差分を確認する。
- standalone test は cleanup の既定検証にせず、test target のコンパイルは `just check --tests` で確認する。
- 各 wave は独立した review 単位としてコミットし、次の wave の前に workspace を clean にする。
- `main` で作業し、worktree は作成しない。
- `cargo metadata` に含まれる全 workspace member を coverage matrix に一度ずつ記録し、Rust source に該当名がないクレートも `no-match` として監査済みにする。

## Review Focus

- **外部境界の誤変更:** `CodexAuth`、`CODEX_HOME`、`codex_home`、auth.json、rollout/SQLite の名前を内部 symbol と誤認しない。Task 0 の分類監査で retain/defer を固定する。
- **高 fan-out 型の一括変更:** `CodexErr`、`CodexErrorDetails`、`CodexErrorInfo`、`CodexResult`、`CodexThread` は一度に扱わず、consumer 一覧と crate 単位の diff を確認する。Task 4/5 の `just check --tests` が検証する。
- **test-only consumer の見落とし:** integration test、fixture helper、別 crate の `tests/common` を含めて検索する。Task 1/2 の old/new symbol invariant と `just check --tests` で固定する。
- **生成・公開 API の連鎖:** public re-export、app-server protocol、schema generator に到達する型は、生成物の差分を確認してから変更する。Task 3/5 の `just generate` と staged diff review で検証する。
- **provider capability の混入:** 命名変更中に auth、endpoint、telemetry、hosted assumption を見つけた場合は実装を混ぜず、候補メモと compatibility inventory に別 task として記録する。全 Task の audit step が担当する。

## Dependency and fan-out policy

対象は次の順で wave 分けする。fan-out は `cargo metadata --no-deps` の package graph と repository-wide `rg` の consumer 数を合わせて判断する。

1. crate-private / test-only、consumer 5 件以下の leaf symbol
2. workspace 公開だが consumer が少ない補助型
3. crate-local の共有型と medium fan-out の internal API
4. `genex-core` の public re-export と高 fan-out model/client 型
5. protocol/app-server/auth/CLI/UI に露出する名前

`CodexHome*`、`CodexAuth`、`CodexErr` 系、`CodexThread`、`CodexTurn*`、`CodexResponses*` は初回 wave の一括対象にせず、Task 0 の分類結果に応じて後続 Task へ割り当てる。

## All-crate coverage contract

Task 0 の coverage matrix は workspace member ごとに次のいずれかを必須とする。

- `migrate-in-place`: Genex 名へ変更する symbol/file consumer を列挙する。
- `retain`: 現行名を残す producer/consumer と互換性理由を列挙する。
- `defer`: 後続の CLI、protocol、persisted-data、auth、外部契約 task を指定する。
- `no-match`: 対象 Rust symbol が存在しないことを検索結果で確認する。

`unreviewed` の crate/member/symbol を完了時に残さない。今回の implementation では、少なくとも workspace member の全件を `target/genex-symbol-audit/rust-symbols.tsv` と両 inventory 文書のいずれかに対応付ける。

---

### Task 0: Rust symbol inventory と retain/defer 境界の固定

**Files:**
- Create: `target/genex-symbol-audit/rust-symbols.tsv`（checked-in しない一時監査成果物）
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: `cargo metadata --no-deps --format-version 1`、Rust source、既存の path audit。
- Produces: symbol ごとの crate、visibility、definition、consumer 数、カテゴリ、decision、次 wave。

- [x] **Step 1: Inventory を生成する**

  `rg` で `Codex[A-Z]`、`codex_[a-z]`、`Codex` を含む Rust identifier を収集し、definition/import/re-export/test-only を区別する。`target/`、generated JSON、履歴資料の文字列は symbol inventory に混ぜない。

- [x] **Step 2: Cargo graph と consumer 数を突合する**

  `cargo metadata --no-deps --format-version 1` と workspace dependency graph を使い、crate fan-out と symbol consumer 数を記録する。`genex-core`、`genex-protocol`、`genex-login`、`genex-tui`、`genex-cli` は高 fan-out 候補として印を付ける。

- [x] **Step 3: 各 symbol に decision を記録する**

  少なくとも `migrate-in-place` 候補（private/test-only/補助型）、`retain` 候補（`CODEX_HOME`、persisted/wire/security）、`defer` 候補（高 fan-out public API）を inventory 両文書へ記録する。OpenAI/provider capability を発見した場合は命名変更から分離する。

- [x] **Step 4: 監査 invariant を保存する**

  wave ごとに old symbol count、new symbol count、consumer list、対象外パターンを記録する。ここではコードを変更しない。

---

### Task 1: Leaf crate の private / test-only symbol

**Files:**
- Modify: `crates/app-server-test-client/src/lib.rs`
- Modify: `crates/git-utils/src/baseline.rs`
- Modify: `crates/network-proxy/src/proxy.rs`
- Modify: `crates/linux-sandbox/src/{landlock.rs,bwrap.rs}` と対応 test helper
- Modify: leaf crate の `tests/`、`tests/common/`、private helper 定義箇所（Task 0 の inventory で確定）

**Interfaces:**
- Consumes: Task 0 の consumer 一覧と old/new invariant。
- Produces: crate-private / test-only の `Genex*` / `genex_*` symbol。公開型、CLI command、環境変数、protocol 名は変更しない。

- [x] **Step 1: private/test-only 候補を crate 単位で分割する**

  `CodexClient`、`codex_signature`、`codex_proxy_git_ssh_command`、`codex_linux_sandbox_exe`、`codex_command` など、external consumer がない候補だけを選ぶ。crate をまたぐ helper は Task 2 に送る。

- [x] **Step 2: old/new invariant を確認する**

  対象 crate ごとに旧 symbol の参照数と新 symbol の参照数を記録し、旧定義が private/test-only であることを確認する。

- [x] **Step 3: symbol と同一 crate の consumers を rename する**

  function、private struct、module-local constant、test helper の定義・呼び出し・import を変更する。`CODEX_HOME`、CLI binary 名、snapshot の expected user text は変更しない。

- [x] **Step 4: compile verification**

  Run: `just check --tests` and `just check`

  Expected: 0 errors. 対象が generated fixture を参照する場合は `just generate` も実行し、意図しない差分がないことを確認する。

- [x] **Step 5: Commit**

  `git commit -m "Rename leaf Rust symbols to Genex"`

---

### Task 2: Low-fan-out workspace public helper types

**Files:**
- Modify: `crates/install-context/src/lib.rs`
- Audit only: `crates/history/src/lib.rs`、`crates/history/src/{rollout_payload.rs,retained_context.rs}` と tests; `CodexHarnessMetadata` rename is deferred to schema/persisted audit
- Modify: `crates/cli/src/doctor.rs`
- Modify: `crates/app-server-daemon/src/prepare_install.rs`
- Modify: `crates/rollout/src/` と core test support の `CodexHarnessMetadata` consumers（Task 0 で確定）

**Interfaces:**
- Consumes: Task 1 完了後の clean workspace と Task 0 の public consumer map。
- Produces: `GenexPackageLayout`、`GenexPackageManifest`; `CodexHarnessMetadata` remains explicitly deferred because its schema definition name is a compatibility boundary。

- [x] **Step 1: public consumer と external boundary を確認する**

  `CodexPackageLayout`、`CodexPackageManifest`、`CodexHarnessMetadata` が wire/schema/persisted identifier ではなく workspace-owned API であることを確認する。別 binary、extension、生成 code の参照を列挙する。`CodexHarnessMetadata` は history JSON schema の definition 名に現れるため、schema/persisted audit が完了するまで Task 3/7 へ defer してよい。

- [x] **Step 2: API 名と全 consumer を migrate-in-place する**

  型名、import、re-export、doc comment、test helper を `Genex*` へ変更する。package filename、install layout、rollout filename、JSON field 名は変更しない。

- [x] **Step 3: generated/compile verification**

  Run: `just check --tests`, `just check`。

  app-server protocol/schema に到達する場合のみ `just generate` を実行し、schema の意図しない wire name 変更を reject する。

- [x] **Step 4: Commit**

  `git commit -m "Rename low-fanout Rust helper types to Genex"`

---

### Task 3: Crate-local shared models

**Files:**
- Modify: `crates/rollout-trace/src/{model/mod.rs,model/session.rs,protocol_event.rs}`
- Modify: `crates/exec/src/{event_processor.rs,event_processor_with_human_output.rs,event_processor_with_jsonl_output.rs}`
- Modify: `crates/core/src/responses_metadata.rs`
- Modify: direct consumers in `crates/core/src/` and `crates/rollout-trace/src/`

**Interfaces:**
- Consumes: Task 2 の public helper rename と Task 0 の trace/protocol boundary audit。
- Produces: crate-local の `GenexTurn*`、`GenexStatus`、`GenexResponsesMetadata` 系。serialized field、event tag、rollout source name は変更しない。

- [ ] **Step 1: trace/protocol/persistence の境界を audit する**

  `CodexTurnId`、`CodexTurn`、`CodexTurnTraceEvent`、`codex_turn_trace_event`、`CodexStatus`、`CodexResponsesMetadata`、`CodexResponsesRequestKind` の各 producer/consumer を確認する。保存データ、JSONL、app-server protocol に露出するものは retain/defer として記録する。

- [ ] **Step 2: private/crate-local subset を先に migrate する**

  public re-export や serde tag に到達しない `CodexResponsesRequestKind`、`CodexTurnMetadataPayload` などを先に `Genex*` へ変更する。共有型は一つの symbol family ごとに分ける。

- [ ] **Step 3: non-wire public utility subset を audit 後に migrate する**

  `CodexStatus` など、JSONL/event processor の external contract でないものだけを consumer map に従って変更する。wire output の key/value、snapshot、CLI output は変更しない。

- [ ] **Step 4: Verification**

  Run: `just check --tests`, `just check`。rollout-trace または schema generator の入力を変更した場合は `just generate` を実行し、persisted/protocol 差分がないことを確認する。

- [ ] **Step 5: Commit**

  `git commit -m "Rename crate-local Rust models to Genex"`

---

### Task 4: `genex-core` の high-fan-out public Rust API

**Files:**
- Modify: `crates/core/src/genex_thread.rs`
- Modify: `crates/core/src/lib.rs`、`crates/core-api/src/lib.rs`
- Modify: `crates/app-server/src/`、`crates/tui/src/`、`crates/ext/`、`crates/cli/src/` の consumer
- Modify: `crates/core/tests/` と cross-crate test support

**Interfaces:**
- Consumes: Task 3 の crate-local model decisions と Task 0 の complete consumer map。
- Produces: `GenexThread`、`GenexThreadSettingsOverrides` など、workspace 内で完全に列挙された Genex public API。外部 wire/persisted name は保持する。

- [ ] **Step 1: public re-export と dynamic/test consumer を列挙する**

  `genex-core`、`genex-core-api`、app-server、TUI、extension、integration test の import/re-export を一覧化する。Hawk の source-location は生成監査資料として別扱いにする。

- [ ] **Step 2: `CodexThread` family を一単位で migrate する**

  `CodexThread`、`CodexThreadSettingsOverrides`、対象の `run_codex_thread_*` private/public functions を、consumer map に沿って変更する。thread id、rollout path、session history、SQLite field は変更しない。

- [ ] **Step 3: compile and generated verification**

  Run: `just check --tests`, `just check`, `just generate`。generated schema、Hawk source path、snapshot の変更は内容を確認し、symbol rename に必須でない差分は戻す。

- [ ] **Step 4: Commit**

  `git commit -m "Rename core thread APIs to Genex"`

---

### Task 5: Shared error/result and auth names — audit gate only

**Files:**
- Audit: `crates/protocol/src/{error.rs,protocol.rs}`
- Audit: `crates/app-server-protocol/src/protocol/v2/{shared.rs,thread_data.rs}`
- Audit: `crates/login/src/auth/manager.rs`
- Audit: all consumers of `CodexErr`、`CodexErrorDetails`、`CodexErrorInfo`、`CodexResult`、`CodexAuth`
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: Tasks 1–4 の migrated internal APIs。
- Produces: Rename / retain / defer decision。実装変更は decision が `migrate-in-place` の family だけに限定する。

- [ ] **Step 1: protocol/schema/auth boundary を監査する**

  serde name、JSON error shape、app-server notification/request、auth storage、external extension API、provider-specific behavior を producer/consumer 単位で確認する。

- [ ] **Step 2: internal-only subset と external subset を分ける**

  `CodexResult` のような type alias でも、protocol crate の public export や外部 extension consumer があれば high-risk として defer する。`CodexAuth` は認証契約と provider capability を含むため、命名変更に capability removal を混ぜない。

- [ ] **Step 3: decision を inventory に記録する**

  `retain` または `defer` の理由、consumer、次の protocol/auth task を記録する。ここで実装変更を行わない場合も audit completion を commit する。

- [ ] **Step 4: Commit**

  `git commit -m "Audit shared Rust symbol compatibility boundaries"`

---

### Task 6: User/integration boundary preparation

**Files:**
- Audit/modify: `crates/cli/`
- Audit/modify: `crates/tui/`
- Audit/modify: `crates/app-server/`
- Audit/modify: docs, fixtures, snapshots, generated schema inputs identified by Task 5
- Modify: both compatibility inventory documents

**Interfaces:**
- Consumes: stable internal Rust names and explicit protocol/auth decisions from Tasks 1–5.
- Produces: separate plan entries for binary, CLI, config, environment, UI, docs, fixture, and snapshot names; no implicit rename of `codex` binary or `CODEX_HOME`.

- [ ] **Step 1: classify every remaining user/integration name**

  `codex_command`、`codex_binary`、`codex_home`、CLI command/help、snapshot basename、UI/docs text を binary/config/env/UI/docs/protocol/persisted に分類する。

- [ ] **Step 2: choose independent slices**

  Binary/CLI、config/env、UI/docs/fixtures/snapshots を別 task にし、each slice に producer/consumer、migration risk、verification command を記録する。

- [ ] **Step 3: Do not implement protocol/persisted changes here**

  wire name、serialized field、rollout/session source、SQLite/JSON identifier は Task 7 へ送る。retain の名前は content と filename の両方を inventory に残す。

---

### Task 7: Protocol, schema, persisted data, external contract decision

**Files:**
- Audit: `crates/code-mode-protocol/`
- Audit: `crates/app-server-protocol/` and generated schema
- Audit: `crates/rollout/`, `crates/thread-store/`, `crates/state/`
- Audit: external URLs, auth/config keys, provider-owned contracts
- Modify: compatibility inventory and follow-up plan documents

**Interfaces:**
- Consumes: Task 6 の user/integration classification。
- Produces: per-contract migration/alias/retain decision and, only where necessary, a separately reviewed compatibility migration.

- [ ] **Step 1: enumerate producers, readers, writers, and existing data**
- [ ] **Step 2: decide whether old reader/alias is required**
- [ ] **Step 3: implement only approved contract changes in separate plans**
- [ ] **Step 4: run `just generate`, `just check --tests`, and `just check`; inspect schema/persisted diff**

---

### Task 8: Final generated artifact and residual-name audit

**Files:**
- Modify: generated artifacts only when their producer requires it
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`

- [ ] **Step 1: Run `just generate` and inspect all checked-in diffs**
- [ ] **Step 2: Search all repository paths and contents for unclassified `Codex` / `codex_`**
- [ ] **Step 3: Verify every workspace member and remaining name has an explicit `migrate-in-place`, `retain`, `defer`, or `no-match` decision**
- [ ] **Step 4: Run `just check --tests`, `just check`, and `git diff --check`**
- [ ] **Step 5: Commit final audit separately**

## Completion Gate

- Every workspace member and internal Rust symbol is either Genex-named, explicitly retained/deferred, or explicitly marked `no-match`.
- No `CODEX_HOME`、persisted identifier、wire name、security boundary、provider-owned contract is changed without its own producer/consumer audit.
- Each wave has a focused commit and passed the repository verification policy.
- The next user-facing/protocol work is represented as a separate plan before implementation begins.

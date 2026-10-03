# 全クレート Genex 命名移行 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `crates/` 配下の全 workspace member を依存の少ない順に監査し、リポジトリ所有の `codex` 名称を producer/consumer 単位で Genex へ移行するか、根拠付きで保持する。

**Architecture:** 最初に Cargo metadata と依存グラフから全123 memberの順序・fan-out・coverage matrixを再生成する。その後、各 member を一つずつ `rg -ni 'codex'` で監査し、低依存の private/test-only 境界から、crate API、protocol/schema、永続化、CLI/config/UI、最後に高 fan-out runtime へ進む。名前 family が複数クレートにまたがる場合は、現在の member の監査記録を起点に全 producer/consumerを同じ waveで変更する。

**Tech Stack:** Rust workspace、Cargo metadata、`rg`、`jq`、既存の `scripts/generate_crate_dependency_graph.py`、Markdown inventory、`just check`、`just generate`。

**Spec:** [`docs/superpowers/specs/2026-10-03-genex-all-crates-migration-design.md`](../specs/2026-10-03-genex-all-crates-migration-design.md)、[`docs/provider-decoupling-plan.md`](../../provider-decoupling-plan.md)

## Global Constraints

- `cargo metadata --no-deps` の全123 workspace memberを一度ずつ coverage matrix に記録し、`no-match` memberも監査済みにする。
- 各 member は依存される側の少ない topological layerから処理し、layer内は transitive dependent数、direct dependent数、match数、package名の順で並べる。
- repository-wide の機械的な `codex` 全置換は行わず、各 match の producer/consumer を監査してから変更する。
- repository-owned の API、protocol/schema、persisted field、CLI/config、UI、package/build 名は、保持理由がなければ `migrate-in-place` とする。
- OpenAI endpoint 必須の URL、request/response field、authentication variable/header、model identifier、外部 fork/upstream・第三者 protocol/package、歴史的帰属・法的記録、別単語内の部分文字列、外部 repository URLは変更しない。
- 保持・移行・削除の決定を、変更前に `docs/pre-release-compatibility-inventory.md` へ `retain`、`migrate-in-place`、`remove` として記録する。旧 `defer` 行は最終的にこの三分類へ解消する。
- 命名監査で provider-specific capability、endpoint、auth、telemetry、hosted assumption を発見した場合、コード変更に混ぜず `docs/provider-decoupling-candidates.md` へ別 follow-up として記録する。
- main branchで作業し、worktreeは作成しない。既存のユーザー変更がある場合は保持し、今回の変更と混ぜない。
- `target/genex-symbol-audit/` は一時監査成果物として使用し、source/docsの完了記録とは別に扱い、通常はコミットしない。
- Rust変更後は `just check --tests` と `just check` を実行する。checked-in generated artifact、schema、snapshot、Cargo metadataに影響する場合は `just generate` も実行する。standalone testは既定検証にしない。

## Review Focus

- **部分文字列の誤変更:** `codex` が別単語の内部にあるだけの matchを identity名と誤認しない。Task 0で語境界・所有者・canonical replacementを記録する。
- **高 fan-out の横断漏れ:** public re-export、別binary、integration test、schema generator、reader/writerをconsumer mapから落とさない。Task 6/7で old symbolの残存を0にする。
- **外部契約の誤変更:** OpenAI endpoint fields/auth/model、第三者protocol、外部URL、OS/security namespaceは保持理由を検証する。Task 4/5で明示的に確認する。
- **永続化読者の不整合:** rollout、thread-history、SQLite、JSON schemaのproducer/reader/writerを同じ familyで更新する。Task 4で生成物とreaderの整合を確認する。
- **台帳と実装の乖離:** 変更前 decision、変更後 exact-match、検証コマンドを同じ台帳行へ揃える。Task 7で全123 memberと全残存matchを突合する。

## Coverage and wave policy

Task 0で作成する `target/genex-symbol-audit/workspace-coverage.tsv` が唯一の処理順の入口になる。各行は package、manifest path、dependency layer/rank、direct/transitive dependent数、crate-local match数、cross-crate consumer数、wave、decision、verification statusを持つ。実装者はこの表のrank順に一行ずつ処理し、consumerが別rankにある場合はそのmember行へ参照を追加する。

各memberの処理は次の4段階で完了とする。

1. crate directory、manifest、source、tests、fixtures、build script、README、生成入力の `rg -ni 'codex'` を保存する。
2. matchごとに producer、consumer、境界、canonical replacement、decisionを台帳へ追加する。
3. `migrate-in-place` / `remove` familyを producer と全consumerで変更し、保持対象は exact理由を記録する。
4. old/new exact search、必要な生成、`just check --tests`、`just check`、`git diff --check`を通してcoverage行を完了にする。

---

### Task 0: 全123 memberの依存順・coverage・既存台帳の再基準化

**Files:**
- Create: `target/genex-symbol-audit/metadata.json`（一時監査成果物）
- Create: `target/genex-symbol-audit/dependency-order.tsv`（一時監査成果物）
- Modify: `target/genex-symbol-audit/workspace-coverage.tsv`
- Modify: `target/genex-symbol-audit/rust-symbols.tsv`
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: `cargo metadata --no-deps`、`scripts/generate_crate_dependency_graph.py`、既存のsymbol/path audit、両 inventory。
- Produces: 全 memberのrank、match分類、旧 `defer` 行の再評価対象、各後続waveの入力。

- [ ] **Step 1: Cargo metadataと依存グラフを再生成する**

  `cargo metadata --no-deps --format-version 1` を `target/genex-symbol-audit/metadata.json` に保存し、既存の `scripts/generate_crate_dependency_graph.py` で workspace graph を生成する。Cargo metadataの123 packageと、path workspace packageを含むことを確認する。

- [ ] **Step 2: 依存順を決める**

  各packageについて、依存先・依存元・topological layer・direct/transitive dependent数を `dependency-order.tsv` に出力する。leafから始め、同一layerは dependent数、match数、package名で安定ソートする。cycleがある場合はSCC全体を一つのwaveとして記録する。

- [ ] **Step 3: 全memberを一行ずつ監査する**

  `rg -ni 'codex'` を各manifest directoryで実行し、Rust sourceだけでなくpath、Cargo.toml、build/test、fixture、README、生成入力を分けて記録する。matchがないmemberは `no-match` としてcoverageに残す。

- [ ] **Step 4: 既存 `defer` を再評価する**

  既存 inventory の「高 fan-outだから保留」「protocol/永続化/CLIだから保留」を、外部契約・部分文字列・provider-specific capability・security namespaceのいずれかに該当するか確認する。該当しない repository-owned nameは `migrate-in-place`、不要なものは `remove`、保持境界だけを `retain` に更新する。

- [ ] **Step 5: coverage invariantを保存する**

  member数123、`unreviewed=0`、各行のrank/wave/decision/verification statusが埋まっていることを確認する。ここではsource codeを変更しない。

- [ ] **Step 6: Commit**

  ```bash
  git add docs/provider-decoupling-candidates.md docs/pre-release-compatibility-inventory.md
  git commit -m "Rebase Genex crate audit on repository-owned contracts"
  ```

### Task 1: 低依存 leaf crateのpath/package/build/test境界

**Files:**
- Modify: Task 0のrank最小層に含まれる `crates/*/Cargo.toml`、source、tests、fixtures、README、build script
- Modify: `Cargo.toml`、`Cargo.lock`、関連生成入力
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: Task 0の `dependency-order.tsv` と各memberのmatch ledger。
- Produces: Genex package/path/module/build/test names、または根拠付き `remove` / `retain`。

- [ ] **Step 1: rank順に一つのleaf crateを選ぶ**

  coverageの未完了行から最小rankのmemberだけを選び、そのcrate内とrepository-wide consumerを検索する。次のcrateへ進む前にdecisionをinventoryへ記録する。

- [ ] **Step 2: repository-owned path/package/build名を移行する**

  Cargo package名、workspace dependency alias、crate directory、module filename、fixture directory、build-time variable、README/exampleを、外部URL・security namespace・第三者名でないことを確認してGenexへ変更する。不要な compatibility fixtureはproducer/consumerを確認して削除する。

- [ ] **Step 3: 全consumerを更新する**

  `Cargo.toml`、`Cargo.lock`、`include_bytes!`、`include_str!`、build script、test command、snapshot basename、justfile/docs参照を同じfamilyで更新する。別crate consumerはそのcrateのcoverage rowにもcross-crate edgeを記録する。

- [ ] **Step 4: crate単位で検証する**

  旧path/package/nameのexact searchが保持行以外で0、`git diff --check`、`just check --tests`、`just check`を実行する。生成入力を変更した場合は `just generate` を実行する。

- [ ] **Step 5: coverageを完了して次のleafへ進む**

  ledgerにverification resultを記録し、同じrankの次のmemberを処理する。rank最小層が全て完了するまで高 fan-out crateへ進まない。

### Task 2: crate-private / test-only Rust symbolと低fan-out API

**Files:**
- Modify: Task 0/1で該当した `crates/*/src/**/*.rs`、`crates/*/tests/**/*.rs`
- Modify: cross-crate test support (`crates/app-server/tests/common/`, `crates/core/tests/common/`, `crates/exec-server/tests/support/`)
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: 完了したleaf package/path移行とproducer/consumer map。
- Produces: `Genex*`、`genex_*` のprivate/test-only symbol、低fan-out public helper。

- [ ] **Step 1: symbol familyごとに定義と全参照を列挙する**

  `Codex[A-Z]`、`codex_[a-z]`、module/file nameをdefinition/import/re-export/call/test fixtureに分解する。`CODEX_HOME`、OpenAI wire field、外部URL、security namespaceを内部symbolと混同しない。

- [ ] **Step 2: low-fan-out familyを移行または削除する**

  private helper、test builder、local module、fixture builder、未使用wrapperはそれぞれ `migrate-in-place` または `remove` として、同一familyの全consumerを更新する。挙動、auth、sandbox、endpoint、telemetryの変更は行わない。

- [ ] **Step 3: public helperのrepository-owned名を移行する**

  consumerが全てrepository内で、wire/persisted/security契約でない型・関数・re-exportは、別crateのconsumerを含めてGenexへ変更する。publicであることだけを理由にretainしない。

- [ ] **Step 4: old/new invariantとcompile verification**

  old exact identifierが0、new identifierの定義/import/call数が期待値と一致することを確認し、`just check --tests`、`just check`を実行する。

- [ ] **Step 5: Commit**

  ```bash
  git add crates Cargo.toml Cargo.lock docs/pre-release-compatibility-inventory.md
  git commit -m "Migrate low-fanout crate symbols to Genex"
  ```

### Task 3: medium-fan-out shared modelとcross-crate test support

**Files:**
- Modify: `crates/core/src/`、`crates/protocol/src/`、`crates/rollout-trace/src/`、`crates/exec/src/`
- Modify: `crates/*/tests/` と extension test supportでTask 0が列挙したconsumer
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: Task 2のGenex low-fan-out symbols、medium-fan-out consumer map。
- Produces: wire/persisted fieldとは独立したGenex shared model、error/result alias、test-support API。

- [ ] **Step 1: declaration-led auditを実施する**

  `CodexErr`、`CodexErrorDetails`、`CodexResult`、`CodexStatus`、`CodexResponses*`、test-support helperなどを、型名とserde/wire/persisted valueに分離する。`GenexErrorInfo`、`genex_error_info`、turn/history fieldsはTask 4のfamilyとして接続する。

- [ ] **Step 2: repository-owned shared modelを一単位で移行する**

  public re-export、extension、app-server、TUI、integration testを含む全consumerを変更し、display textやwire fieldの意図しない変更がないことを確認する。

- [ ] **Step 3: test supportを横断移行する**

  `TestCodex*`、`test_codex_*`、`TempCodex*`、`PtyCodex`などのtest-only helperを、core/app-server/exec/skills/extension tests全体でGenexへ変更する。`CODEX_HOME`、実行ファイル名、外部auth値は別境界として保持する。

- [ ] **Step 4: verify and record**

  old exact symbol search、`just check --tests`、`just check`、`git diff --check`を実行し、台帳に全consumerとverificationを記録する。schema/persisted producerに到達した場合はTask 4へ移送し、retain理由なしの保留を残さない。

### Task 4: protocol/schema/persisted data/reader-writer family

**Files:**
- Modify: `crates/protocol/`
- Modify: `crates/app-server-protocol/` と `crates/app-server-protocol/schema/`
- Modify: `crates/rollout/`、`crates/rollout-trace/`、`crates/history/`、`crates/state/`、`crates/thread-store/`、`crates/message-history/`
- Modify: related fixtures, snapshots, JSON/TypeScript generated outputs
- Modify: compatibility inventory documents

**Interfaces:**
- Consumes: Task 3のdeclaration auditと全reader/writer map。
- Produces: repository-owned Genex wire/schema/persisted identifiers、または外部契約だけを残したretain boundary。

- [ ] **Step 1: producer/reader/writerを列挙する**

  app-server request/response、JSON schema、rollout/session source、thread history、SQLite/JSON field、trace event/tagについて、生成元・書き込み元・読み手・replay/test fixtureを全て記録する。

- [ ] **Step 2: repository-owned protocol/schema名を移行する**

  workspace内のproducerとconsumerが揃う `CodexErrorInfo`、app-server schema title/field、request/reply alias、生成TypeScript/JSONをGenexへ更新する。OpenAI endpoint wire field、第三者 proto/package、外部 clientだけが要求する名前はretainする。

- [ ] **Step 3: persisted reader/writerを同じfamilyで更新する**

  pre-release forkの方針に従い、repository-owned rollout/history/state identifiersはwriter、reader、migration、replay fixtureを一緒に移行する。互換readerを残す場合は、外部状態・既存読者・変更解除条件を台帳に明記する。`CODEX_HOME`のようなactive path/security contractは名前だけで変更しない。

- [ ] **Step 4: generated artifactsを再生成する**

  `just generate`を実行し、checked-in schema/snapshot/TypeScript diffを確認する。期待するGenex差分以外の変更は原因を監査して戻す。

- [ ] **Step 5: verify**

  old serialized/protocol identifierの検索結果をretain行と突合し、`just check --tests`、`just check`、`git diff --check`を実行する。

### Task 5: CLI/config/env/UI/docs/build/securityのrepository-owned surface

**Files:**
- Modify: `crates/cli/`
- Modify: `crates/config/`、`crates/app-server/`、`crates/tui/`
- Modify: `crates/install-context/`、`crates/arg0/`、`crates/build-info/`、`crates/bwrap/`、related build scripts/manifests
- Modify: `docs/`、`justfile`、fixtures、snapshots、generated config/schema inputs
- Modify: compatibility inventory documents

**Interfaces:**
- Consumes: Task 4のprotocol/persisted decisionsとCLI/UI producer/consumer map。
- Produces: repository-owned Genex binary/help/config/UI/docs/build names、保持が必要な外部/auth/security namesの明示的な台帳行。

- [ ] **Step 1: CLI/config/envを分類する**

  binary名、argv[0]、help/command、config key、environment variable、import path、snapshotを、repository-owned surface、OpenAI endpoint/auth、persisted/security path、外部 URLに分類する。

- [ ] **Step 2: repository-owned CLI/config/UIを移行する**

  producer（CLI parser/config loader/UI renderer）と全consumer（daemon、TUI、tests、docs、justfile、packaging）をGenexへ変更する。ユーザー向け名称も保持理由がなければ移行し、snapshotを更新する。

- [ ] **Step 3: build/package/security boundaryを監査する**

  install layout、manifest、sandbox executable、bwrap build override、OS keychain/lock namespace、active security fallbackを個別に確認する。内部 build labelは移行し、外部状態・security protocol・binary contractはretainとして記録する。

- [ ] **Step 4: verify generated and user-facing outputs**

  `just generate`、`just check --tests`、`just check`、`git diff --check`を実行し、help/config/schema/snapshotの変更を確認する。OpenAI auth/endpoint文字列の保持をexact searchで検証する。

### Task 6: high-fan-out core/API/login/app-server runtime family

**Files:**
- Modify: `crates/core/`、`crates/core-api/`
- Modify: `crates/login/`、`crates/genex-api/`
- Modify: `crates/app-server/`、`crates/tui/`、`crates/cli/`、`crates/ext/`の全consumer
- Modify: `crates/model-provider/`、`crates/models-manager/`、`crates/genex-mcp/`、`crates/otel/`の該当symbol family
- Modify: compatibility inventory documents

**Interfaces:**
- Consumes: Task 2–5で確定した low/medium/shared/protocol/user boundary namesとcomplete consumer map。
- Produces: workspace内の高fan-out Genex API、thread/client/auth/runtime names。ただしOpenAI endpoint contract、external auth values、security behaviorは保持する。

- [ ] **Step 1: public API/re-export/feature reachabilityを再確認する**

  `CodexThread`、`CodexThreadSettingsOverrides`、`CodexAuth`、`CodexResponses*`、`CodexClient`、remaining `Codex*` public namesについて、workspace binary、dynamic registration、integration test、generated schemaの全consumerを列挙する。

- [ ] **Step 2: repository-owned high-fan-out familyを移行する**

  coreのdefinitionからconsumerへ、re-export、trait bound、macro、test support、docsを一つのfamilyで変更する。型名の移行と同時に provider capability、auth method、endpoint routing、telemetry sinkを削除・変更しない。

- [ ] **Step 3: auth/header/endpoint boundaryを分離する**

  内部の repository-owned auth typeやwrapperはGenexへ移行できるか確認する一方、OpenAI endpointが要求するheader/env/model/URLと、active provider behaviorは保持する。保持理由または別follow-upを両inventoryへ記録する。

- [ ] **Step 4: full workspace verification**

  `rg -n -w`で旧public symbolを検索し、retain行以外を0にする。`just check --tests`、`just check`、必要な `just generate`、`git diff --check`を実行する。

- [ ] **Step 5: Commit**

  ```bash
  git add crates docs Cargo.toml Cargo.lock
  git commit -m "Migrate high-fanout runtime names to Genex"
  ```

### Task 7: 全クレート・全パス・台帳の最終監査

**Files:**
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Audit only: `target/genex-symbol-audit/workspace-coverage.tsv`
- Modify: checked-in generated artifacts only when final producers require it

**Interfaces:**
- Consumes: Task 0–6の全wave、generated output、old/new search results、Cargo metadata。
- Produces: `unreviewed=0`、unclassified match=0、Genex移行完了または明示retainのworkspace。

- [ ] **Step 1: 全memberを依存順に再走査する**

  123 memberそれぞれで `rg -ni 'codex'` を再実行し、coverage rowのdecision、changed paths、verification statusと一致させる。`no-match` rowを含めて欠落がないことを確認する。

- [ ] **Step 2: repository-wide residual auditを行う**

  `rg --files`でpath/package、`rg -ni`でcontent、Cargo metadataとCargo.lockでpackage/dependency、生成schema/snapshot/docs/justfileで参照を確認する。残存matchは exact path/symbol/string単位でinventoryのretain行に紐付ける。

- [ ] **Step 3: old `defer` と未分類行を解消する**

  台帳に残る `defer`、`hold`、`pending` を、`retain`、`migrate-in-place`、`remove`のいずれかへ更新する。provider follow-upは候補台帳の独立項目にし、命名移行の未完了扱いと混同しない。

- [ ] **Step 4: 最終生成・型検証を行う**

  `just generate`、`just check --tests`、`just check`、`git diff --check`を実行する。生成物差分を確認し、期待するGenex変更以外を解消する。

- [ ] **Step 5: completion invariantを記録する**

  coverageに全123 member、unreviewed=0、retain/migrate/removeのdecision、verification resultを記録する。台帳と実際の残存matchを突合し、最終監査のcommitを作成する。

- [ ] **Step 6: Commit**

  ```bash
  git add docs
  git commit -m "Complete all-crate Genex naming audit"
  ```

## Completion Gate

- 全123 workspace memberが依存順に監査され、`no-match`を含めてunreviewedがない。
- 保持理由のない repository-owned `codex` nameはGenexへ移行されている。
- protocol/schema、persisted data、CLI/config/UI、package/build、high-fan-out APIが、監査済みなら移行されている。
- OpenAI endpoint固有契約、外部URL/識別子、部分文字列、法的記録、必要なsecurity/OS namespaceだけがretainとして台帳に残っている。
- provider-specific capabilityを命名変更に混ぜず、候補台帳に独立して記録している。
- `just generate`、`just check --tests`、`just check`、`git diff --check`が最終状態で成功している。

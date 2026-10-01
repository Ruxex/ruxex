# メモリーズ機能削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** メモリーズの生成・保存・読み込み・注入・専用操作を runtime から削除し、通常の thread history、provider-neutral capability、Chronicle、一般の external-agent import を維持する。

**Architecture:** memory 専用 crate と runtime pipeline を、persisted rollout/thread-history、protocol/app-server、CLI/config/import、UI/runtime、build/dependency の境界順に切り離す。既存の memory ファイル、memory SQLite、main state DB の既存 rows は削除せず、過去の main state migration 履歴だけは checksum/order 維持のため残す。

**Tech Stack:** Rust workspace、Cargo、app-server protocol の Rust source と生成 JSON/TypeScript schema、TUI、SQLite state、`just generate` / `just check`。

**Spec:** [`docs/superpowers/specs/2026-09-25-remove-memories-feature-design.md`](../specs/2026-09-25-remove-memories-feature-design.md)

## Global Constraints

- `AGENTS.md` の指示どおり `main` ブランチで作業し、Git worktree は作成しない。
- pre-release fork で外部ユーザーがいないため、memory protocol/config API の後方互換性は保証しない。
- 既存の memory ファイル、memory SQLite、main state DB の memory rows は削除しない。
- Chronicle、一般の external-agent import、通常の thread history、generic MCP/extension、platform fallback、security behavior は保持する。
- production match ごとに `docs/pre-release-compatibility-inventory.md` へ producer/consumer audit と `remove` / `retain` / `migrate-in-place` の decision を記録する。
- standalone test は実行せず、`just check --tests` と `just check` を検証基準にする。
- Rust source または test-only code を変更した段階では `just check --tests` と `just check` を実行する。生成 artifact を変更した段階では `just generate` を実行して diff を確認する。

## Review Focus

- memory-specific rollout/history fields を除去しても、通常の response item、thread history、rollout metadata の保存・再生が壊れないこと。
- app-server の memory API と generated schema を除去しても、残存する通常の turn/thread/import API が一致すること。
- external-agent import は memory source だけを失い、config、skills、hooks、marketplace などの一般 import は残ること。
- `Feature::MemoryTool` / `Feature::ExternalAgentMemoryImport` を除去しても、Chronicle と他の feature の設定・doctor/UI 表示が残ること。
- 専用 crate と state runtime を除去しても、既存データを削除する処理や、一般の SQLite/thread state の open path を追加・破壊しないこと。

## File Map

- **Persisted/runtime contracts:** `crates/rollout/src/{policy,recorder,config,metadata,lib}.rs`、`crates/protocol/src/{items,lib,protocol,memory_version}.rs`、`crates/state/src/{runtime,model,sqlite,migrations}.rs`、`crates/thread-store/src/` の memory fields/call sites。
- **Memory implementation:** `crates/memories/`、`crates/ext/memories/`、`crates/codex-api/src/endpoint/memories.rs`。
- **Protocol/app-server:** `crates/app-server-protocol/src/protocol/v2/`、`crates/app-server/src/request_processors/` と generated schema、history conversion。
- **CLI/config/import:** `crates/features/src/lib.rs`、`crates/core/src/config/`、`crates/cli/src/`、`crates/external-agent-migration/src/`、関連 protocol/config schema。
- **UI/runtime/telemetry:** `crates/core/src/context/`、`crates/core/src/session/`、`crates/core/src/stream_events_utils.rs`、`crates/tui/src/`、metrics/doctor/telemetry の関連箇所。
- **Build/docs:** `Cargo.toml`、`Cargo.lock`、README/template/fixture、`docs/pre-release-compatibility-inventory.md`、`docs/provider-decoupling-plan.md`、`docs/provider-specific-crates.md`。

### Task 1: Persisted rollout/thread-history と state の memory 経路を除去

**Files:**
- Modify: `crates/rollout/src/policy.rs`, `recorder.rs`, `config.rs`, `metadata.rs`, `lib.rs` と関連 tests
- Modify: `crates/protocol/src/items.rs`, `protocol.rs`, `memory_version.rs`, `lib.rs` と関連 models
- Modify: `crates/state/src/runtime.rs`, `runtime/memories.rs`, `runtime/memory_versions.rs`, `model/`, `sqlite.rs`, `migrations.rs`
- Modify: `crates/thread-store/src/` の memory mode/citation/usage fields と call sites
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: 現行 canonical rollout、thread history、SQLite thread metadata。
- Produces: memory fields/citation/mode に依存しない通常の rollout/history/state interface。後続 task はこの interface を使い、一般の item/history conversion を保持する。

- [x] `rg` で rollout item、metadata、thread mode、citation、memory state の producer/consumer を確認し、inventory に decision を先に記録する。
- [x] memory-specific persistence policy、input sanitization、mode/citation/usage fields、`MemoryStore`・memory readiness/version/job runtime を削除する。通常の canonical persistence と既存 migration history は残す。
- [x] memory-only state runtime/module/tests を削除し、main state migration の過去 SQL は migration checksum/order を壊さない範囲で保持する。
- [x] `rg` で残った production references を再監査し、`Chronicle` と一般の metadata/history fields が保持されていることを確認する。
- [x] `just check --tests` と `just check` を実行する。

### Task 2: memory protocol、app-server API、history/schema を除去

**Files:**
- Modify: `crates/app-server-protocol/src/protocol/v2/`、common/history conversion、generated JSON/TypeScript schema
- Modify: `crates/app-server/src/request_processors/` と app-server tests/fixtures
- Modify: `crates/protocol/src/` の memory request/response/item types
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: Task 1 の memory-free protocol item/history/state contracts。
- Produces: memory status/read/reset/inject、`ThreadMemoryMode`、`MemoryCitation`、memory history fields を持たない app-server protocol。通常の turn/thread/import methods は従来どおり利用可能。

- [x] memory status/read/reset/inject processor、request/response、thread history citation、memory-only enum/field の current callers を確認し inventory に記録する。
- [x] memory-specific Rust protocol と app-server request processors/history conversion を削除し、残存 API の field/variant conversion を canonical contract に揃える。
- [x] Rust source から generated JSON/TypeScript schema を `just generate` で再生成し、生成 diff が memory 専用削除に限定されることを確認する。
- [x] `just check --tests` と `just check` を実行する。

### Task 3: feature、config、CLI、external-agent memory import を除去

**Files:**
- Modify: `crates/features/src/lib.rs`、`crates/core/src/config/`、`crates/cli/src/`、config schema/docs
- Modify: `crates/external-agent-migration/src/` と generated import schema/tests
- Modify: `crates/tui/src/` の config migration/import wiring（専用 UI は Task 4）
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: Task 2 の memory-free protocol/config shape。
- Produces: memory feature key/config/CLI/import item を公開しない canonical configuration と一般 external-agent import。

- [x] `Feature::MemoryTool`、`Feature::ExternalAgentMemoryImport`、`MemoriesToml`、`MemoriesConfig`、`Config::memories`、CLI reset/clear、memory import item の producer/consumer を audit する。
- [x] memory feature metadata、`[memories]` reader/schema、doctor/config output、memory reset/clear command を削除する。既存 `[memories]` を互換受理する layer は追加しない。
- [x] `memory_import.rs` の source 検出、`MEMORY.md`/resource copy、workspace/consolidation enqueue、memory-only protocol item だけを削除し、一般 config/skills/hooks/marketplace import は残す。
- [x] import/config generated schema を `just generate` で再生成し、diff を確認する。
- [x] `just check --tests` と `just check` を実行する。

### Task 4: core runtime、専用 tools、TUI、metrics/telemetry を除去

**Files:**
- Delete: `crates/ext/memories/`、`crates/memories/read/`、`crates/memories/write/`、memory-only templates/fixtures
- Modify: `crates/core/src/context/`、session/turn/event mapping/tool registry、rollout integration
- Modify: `crates/tui/src/bottom_pane/memories_settings_view.rs`、関連 popup/snapshot/test、config persistence/experimental features
- Modify: metrics、doctor、telemetry、hooks の memory-only integration
- Modify: `Cargo.toml`、`Cargo.lock`、関連 README と inventory

**Interfaces:**
- Consumes: Task 1–3 の memory-free persistence/protocol/config contracts。
- Produces: memory startup extraction/read/injection/tools/UI/observability が存在しない core/TUI runtime。Chronicle と generic MCP/extension は残る。

- [x] startup memory task、memory context injection、citation parser/usage recording、専用 extension registration、memory reset/settings UI/notice/snapshots/metrics の producer/consumer を確認する。
- [x] memory runtime wiring と専用 crates/files/templates/tests を削除し、通常の session/turn/event/tool paths と Chronicle を維持する。
- [x] workspace members/dependencies/Cargo.lock を更新し、`cargo metadata` で専用 crate が消えたことを確認する。既存 memory files/DB を削除するコードは追加しない。
- [x] memory-related `rg` の全 production match を分類して inventory を更新する。残す match は OS memory buffer 等の無関係なもの、Chronicle 等の別機能に限定する。
- [x] `just generate`、`just check --tests`、`just check`、`git diff --check` を実行する。

### Task 5: 最終監査と Phase 3 handoff

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-plan.md`、`docs/provider-specific-crates.md`（削除状況と次の integration の handoff のみ）

- [x] `rg -n -i 'memories|memory_mode|memory_citation|MemoryStore|MemoryTool|ExternalAgentMemoryImport' crates Cargo.toml Cargo.lock` を実行し、production code に機能参照が残っていないことを確認する。
- [x] `cargo metadata` と dependency graph を確認し、memory 専用 crate/dependency がないことを確認する。
- [x] `just generate` の生成物差分を再確認し、`just check --tests`、`just check`、`git diff --check` を最終実行する。
- [x] 設計書の完了条件と inventory の全 decision を照合し、Phase 3 を未完了のまま次の integration handoff を記録する。

## Execution Notes

各 task の production match は先に inventory に記録する。計画と実コードに差異がある場合は、設計書を優先し、`progress.md` に `Ruling: ...` として理由・影響を残す。専用 crate/既存データの削除は対象コードと repository metadata のみとし、ユーザーの `~/.codex` 配下には触れない。

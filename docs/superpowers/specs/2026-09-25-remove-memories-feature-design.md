# メモリーズ機能削除 設計書

作成日: 2026-09-25

## 概要

Codex のメモリーズ機能を、機能単位で削除する。対象は、スレッドから知識を抽出して保存する startup pipeline、その保存結果をコンテキストへ注入する read path、専用 memory tools、設定、state、app-server protocol、TUI UI、および external-agent memory import である。

この機能は `Feature::MemoryTool` がデフォルト無効であるが、設定で有効化された場合には、turn 処理、rollout 保存、SQLite state、memory ファイル、MCP/extension tool、app-server、TUI が連携して動作する。したがって、個別の未使用 `pub` を削除するのではなく、producer から consumer まで一つの capability として撤去する。

## 目的

- スレッド内容からの自動的な知識抽出・統合・保存をなくす。
- 保存済み memory の prompt 注入、引用追跡、専用ツールをなくす。
- memory 専用の設定、feature flag、state、protocol、UI、crate をなくす。
- 現行の provider-neutral な thread history、通常の MCP/extension、sandbox/security behavior は維持する。

## 非目的

- ユーザーの `~/.codex/memories` 配下のファイルや既存 SQLite ファイルを削除すること。
- 一般の thread history、rollout、SQLite thread metadata を削除すること。
- `Chronicle` の screen-context 機能を削除すること。これはメモリーズの thread knowledge pipeline とは別の feature として扱う。
- external-agent の一般的な config/import 機能を削除すること。削除するのは memory の検出・コピー・consolidation 経路だけとする。

## 現在の producer/consumer 境界

### Producer

- `crates/app-server/src/request_processors/turn_processor.rs` の startup memory task 起動。
- `crates/memories/write` の stage 1 thread extraction と phase 2 global consolidation。
- `crates/memories/write/src/rollout_input.rs` と `crates/rollout/src/policy.rs` の memory 向け rollout input/persistence 判定。
- `crates/external-agent-migration/src/memory_import.rs` の外部 agent memory の workspace 同期と consolidation enqueue。
- `crates/tui` の memory 設定変更、memory reset、設定 persistence。
- `crates/app-server` の memory read/status/reset/inject request processor。

### Consumer

- `crates/memories/read` の memory root、citation、usage metric、prompt/read path。
- `crates/ext/memories` の read/list/search/ad-hoc-note tools と extension registration。
- `crates/core/src/context/memory.rs`、`stream_events_utils.rs`、`memory_usage.rs`、session/turn の memory mode・citation・usage 処理。
- `crates/state/src/runtime/memories.rs`、`memory_readiness.rs`、`memory_versions.rs`、`state::MemoryStore`。
- `crates/protocol/src/memory_version.rs`、`memory_citation.rs`、thread memory mode と rollout item の memory fields。
- `crates/app-server-protocol/src/protocol/v2/memory.rs`、thread history の memory citation、生成 JSON/TypeScript schema。
- `crates/genex-api/src/endpoint/memories.rs` と memory extraction/consolidation request types。
- TUI の `memories_settings_view`、memory settings snapshot、reset confirmation、enable prompt。

## 設計方針

### 1. 機能の入口を削除する

`Feature::MemoryTool` と `Feature::ExternalAgentMemoryImport` を削除する。`memories` feature key、external-agent memory import の feature key、memory capability の feature metadata、doctor/config output を削除する。

`Chronicle` は独立した feature として残す。単に `memory` という語を含む名前だけを理由に削除しない。

### 2. runtime pipeline を削除する

以下を削除する。

- `codex-memories-write` crate 全体。
- `codex-memories-read` crate 全体。
- `codex-memories-extension` crate 全体。
- startup extraction、stage 1 output、phase 2 consolidation、memory agent の起動・heartbeat・job claim。
- core の memory context injection、citation parsing/usage recording、memory mode 判定。
- rollout の memory-specific persistence policy と memory-specific input sanitization。
- CLI/app-server の memory reset、clear、status、read、startup task wiring。

memory crate の削除に伴う workspace member、dependency、Cargo.lock の不要な package、README、template、専用 test fixture も削除する。

### 3. 設定と protocol を削除する

`MemoriesToml`、`MemoriesConfig`、`Config::memories`、`MemoryVersion`、memory-specific defaults/validation/schema を削除する。`config.toml` に残る `[memories]` は、pre-release fork に外部ユーザーがいないという方針に従い、互換 reader を残さず未知設定として扱う。

app-server protocol から memory status/read/reset/inject API、`ThreadMemoryMode`、`MemoryCitation` と memory-specific history fields を削除する。Rust protocol source を変更した後に JSON/TypeScript schema を再生成し、生成物を手編集しない。

一般の thread item/history conversion から memory citation を除く。ただし通常の response item、thread history、rollout metadata は保持する。

### 4. state と永続データの扱い

`MemoryStore`、memory DB の open/close、memory runtime migrator、memory job/state model、memory readiness/version runtime を削除する。

既存の `~/.codex/memories`、`memories.sqlite`、および main state DB 内の既存 memory rows は、この変更で削除しない。以後の runtime はそれらを読み書きしないため、既存データは未使用データとして残る。

`crates/state/memory_migrations` と `MEMORIES_MIGRATOR` は memory DB を開く経路とともに削除する。main state migrator の過去の migration ファイル（memory tables を作成・更新した履歴）は、適用済み migration の順序・checksum を壊さないため、active runtime code から切り離したうえで履歴として保持する。新しい memory migration やデータ削除 migration は追加しない。

### 5. external-agent import の扱い

external-agent の一般 config import、skills、hooks、marketplace などは残す。memory source の検出、`MEMORY.md`/resource のコピー、memory workspace 準備、consolidation enqueue、memory-specific protocol item は削除する。

external-agent import protocol の memory enum/field が memory 専用なら削除し、他の import item type と generated schema を再生成する。

### 6. UI と observability

TUI の memory settings popup、enable notice、reset confirmation、config persistence、関連 snapshot/test を削除する。memory-specific metrics、usage attribution、doctor output、telemetry event も削除する。

通常の config editor、feature list、thread settings、generic metrics、Chronicle の UI/telemetry は保持する。

## 実施順序

リポジトリの互換性 cleanup 順序に従い、次の順番で小さな変更単位に分ける。

1. **Persisted rollout/thread-history**: memory-specific rollout policy、citation、memory mode、state rows/jobs の producer/consumer を削除する。
2. **Protocol/app-server**: Rust protocol、request processor、history conversion、JSON/TypeScript generated schema を削除・再生成する。
3. **CLI/config/import**: feature、`MemoriesConfig`、config schema、CLI reset/clear、external-agent memory import を削除する。
4. **UI/observability/runtime**: TUI、memory metrics、doctor output、startup task、core runtime wiring を削除する。
5. **Build/dependency**: 専用 crate、workspace/dependency/Cargo.lock、template、専用 fixture、README を削除する。
6. **最終監査**: `rg` による production match の棚卸し、inventory 更新、生成物と dependency の差分確認を行う。

各段階で producer/consumer を確認し、`remove`、`retain`、`migrate-in-place` を inventory に記録する。`legacy` や `fallback` という名前だけでは削除理由にしない。

## 既存データと互換性の決定

- 既存 memory ファイル、memory SQLite ファイル、main state DB の memory rows は削除しない。
- 旧 memory を読み込んで新形式へ移行する reader は追加しない。
- 既存 `[memories]` 設定を受け付ける互換 layer は残さない。
- main state migration の履歴は migration engine の整合性維持のため保持し、memory runtime からは参照しない。
- pre-release fork で外部ユーザーがいないため、削除後の memory protocol/config API の後方互換性は保証しない。

## 検証方針

実装単位ごとに次を実行する。

```text
just generate
just check --tests
just check
git diff --check
```

最終的には次を確認する。

- `cargo metadata` に memory 専用 crate が残っていない。
- production code に memory extraction/read/injection/tool/state/protocol/UI の参照が残っていない。
- `Chronicle`、一般の external-agent import、通常の thread history、generic MCP/extension は残っている。
- generated config/protocol schema が source と一致している。
- 既存 memory ファイルを削除する処理が追加されていない。
- 既存の app-server、TUI、CLI、exec、MCP、sandbox の通常経路が compile check を通過する。

standalone test は実行せず、リポジトリの cleanup policy に従って `just check`、`just check --tests`、生成物確認を検証の基準にする。

## 完了条件

この設計の実装は、次をすべて満たした時点で完了とする。

1. メモリーズの生成・読み込み・注入・専用操作が runtime に存在しない。
2. memory 専用 crate、設定、feature、state runtime、protocol、generated schema、TUI が削除されている。
3. external-agent memory import だけが削除され、一般の external-agent import は維持されている。
4. Chronicle と provider-neutral capability が保持されている。
5. 既存の memory files/DB を削除していない。
6. inventory に全 production match の producer/consumer audit と decision が記録されている。
7. `just generate`、`just check --tests`、`just check`、`git diff --check` が成功している。

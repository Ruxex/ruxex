# codex-analytics クレート削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (recommended) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hosted analytics sink が削除された後に残っている `codex-analytics` のイベント収集・reducer・型依存を段階的に除去し、実行状態・plugin・OTEL metrics の責務を正しいクレートへ戻したうえでクレートを削除する。

**Architecture:** まず `codex-analytics` の公開型と直接 consumer を inventory に固定する。次に、core/plugin が本来所有すべき型を producer 側へ移し、分析専用の client・queue・reducer・wire serializer と、その呼び出し経路を削除する。分析フラグと OTEL metrics の混在は provider-neutral な `metrics_enabled` に分離し、既存の metrics 有効化挙動を維持する。

**Tech Stack:** Rust workspace、Cargo、app-server protocol、OTEL metrics、生成 config/schema artifacts。

**Spec:** `docs/openai-decoupling-plan.md` および `docs/pre-release-compatibility-inventory.md`

## Global Constraints

- `main` ブランチ上で作業し、Git worktree は作成しない。
- pre-release fork で外部ユーザーは存在しないため、obsolete な config/protocol/API compatibility は保持しない。
- `codex-analytics` の削除を理由に、現行の provider-neutral capability、OTEL、security behavior、platform fallback、plugin runtime を削除しない。
- core が分析クレートの型を所有する依存方向を残さない。分析専用型は削除し、現行の実行モデルは core または producer 側に置く。
- `analytics.enabled` と `--analytics-default-enabled` の意味を名前だけ変更して残さず、metrics の有効化責務として `metrics_enabled` に移す。既存の明示的な有効/無効挙動は維持する。
- この cleanup ではテストを追加・実行しない。対象 crate の `cargo check`、`just check`、生成物に影響する場合の `just generate`、`git diff --check` を検証に使う。
- production match ごとに producer、consumer、`remove` または `migrate-in-place` の判断を inventory に記録してからコードを変更する。

## Review Focus

- 本番 build で discard される analytics queue/reducer が全て呼び出し側から除去されていること。
- `TurnProfile` や `TurnAnalyticsMetadata` を削除しても、core の turn timing、metadata、TTFT/TTFM metrics が壊れないこと。
- 重複する `ArtifactOperation` は `codex-core-plugins` 側を正とし、OTEL の artifact metrics を維持すること。
- `analytics.enabled` の移行で OTEL metrics の既定値、明示的な無効化、daemon/app-server の flag が変わらないこと。
- `AnalyticsEventsClient` を session、extension、app-server、core-api から除去しても、通常の MCP、Guardian、plugin、external-agent import の動作を残すこと。

## File Map

- **Plan and inventory:** `docs/openai-decoupling-plan.md`、`docs/pre-release-compatibility-inventory.md`、この plan。
- **Analytics implementation:** `crates/analytics/Cargo.toml`、`crates/analytics/src/lib.rs`、`client.rs`、`facts.rs`、`events.rs`、`reducer.rs`、`guardian_v2.rs`、`thread_hint.rs`、`accepted_lines.rs`、debug capture、analytics-only tests。
- **Canonical producer types:** `crates/core-plugins/src/artifact_operation.rs`、`crates/core/src/tools/events.rs`、`crates/core/src/turn_timing.rs`、`crates/core/src/turn_metadata.rs`。
- **Metrics/config boundary:** `crates/config/src/config_toml.rs`、`profile_toml.rs`、`types.rs`、`crates/core/src/config/mod.rs`、`crates/core/src/otel_init.rs`、`crates/app-server/src/lib.rs`、`otel_reloader.rs`、`crates/cli/src/main.rs`、`daemon_telemetry.rs`。
- **Metadata/API boundary:** `crates/core/src/responses_metadata.rs`、`crates/core-api/src/lib.rs`、`crates/core-api/Cargo.toml`、app-server protocol/config schema files affected by `analytics_enabled`.
- **Consumers:** `crates/core/src/`、`crates/app-server/src/`、`crates/core-plugins/src/manager.rs`、`crates/ext/{goal,guardian-reviewer,guardian-v2,history-notes,skills}/`、`crates/external-agent-migration/`、`crates/core/tests/common/`、`crates/thread-manager-sample/`。
- **Workspace edges:** root `Cargo.toml` and `Cargo.lock` when the workspace member and dependency edges disappear.

### Task 1: Freeze the producer/consumer inventory

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/openai-decoupling-plan.md`
- Create: `docs/superpowers/plans/2026-09-21-remove-analytics-crate.md`

- [x] **Step 1: Enumerate the crate boundary and reverse dependencies**

  Run:

  ```bash
  cargo tree -p codex-analytics --depth 1
  cargo tree -i codex-analytics --depth 1
  rg -n 'codex-analytics|codex_analytics|AnalyticsEventsClient|AnalyticsFact|TurnProfile|TurnAnalyticsMetadata|ArtifactOperation' crates Cargo.toml docs --glob '*.{rs,toml,md}'
  ```

  Record production consumers separately from tests and fixtures. The current reverse dependency set includes core, app-server, core-api, core-plugins, external-agent migration, and the active extensions; no consumer may be omitted merely because it only calls `disabled()`.

- [x] **Step 2: Add explicit inventory decisions before implementation**

  Add the following decisions to the analytics section of `docs/pre-release-compatibility-inventory.md`:

  | Exact symbol/path | Decision |
  |---|---|
  | `crates/analytics/src/client.rs`, `facts.rs`, `events.rs`, `reducer.rs`, `accepted_lines.rs`, `guardian_v2.rs`, `thread_hint.rs`, and debug capture | `remove`: the release path has no hosted sink and the debug capture is not a product runtime consumer |
  | `AnalyticsEventsClient` injection in core, app-server, extensions, core-api, and external-agent migration | `remove`: no current event sink consumes the queue output |
  | `TurnProfile` and profile-only state in `crates/core/src/turn_timing.rs` | `remove` for analytics-only profile output; `retain` for existing TTFT/TTFM and other active OTEL timing behavior |
  | `TurnAnalyticsMetadata` and `turn_metadata: Arc<dyn TurnAnalyticsMetadata>` fields | `remove`: the trait has no consumer outside analytics; core retains its own metadata state |
  | `codex_analytics::ArtifactOperation` and its construction in `crates/core/src/tools/events.rs` | `migrate-in-place`: use `codex-core-plugins::ArtifactOperation` as the canonical producer type and retain the existing OTEL counters/histograms |
  | `analytics.enabled`, `Config::analytics_enabled`, `analytics_enabled` response metadata, and `--analytics-default-enabled` | `migrate-in-place` for OTEL metrics gating, `remove` for analytics response metadata; rename the surviving control to `metrics_enabled` and `--metrics-default-enabled` |

- [x] **Step 3: Update the phase handoff**

  Add a new Phase 3 implementation item after the shared provider-boundary audit in `docs/openai-decoupling-plan.md`, linking this plan and stating that `codex-analytics` remains a staged deletion candidate until its consumers and misplaced types are removed.

- [x] **Step 4: Verify the inventory diff**

  Run `git diff --check` and confirm every `codex_analytics` production match is covered by one of the tasks below.

### Task 2: Separate metrics configuration from analytics naming

**Files:**
- Modify: `crates/config/src/types.rs`
- Modify: `crates/config/src/config_toml.rs`
- Modify: `crates/config/src/profile_toml.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/core/src/otel_init.rs`
- Modify: `crates/app-server/src/lib.rs`
- Modify: `crates/app-server/src/otel_reloader.rs`
- Modify: `crates/app-server/src/main.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/daemon_telemetry.rs`
- Modify: config schema and generated artifacts reached by the existing `just generate` flow

- [x] **Step 1: Rename the effective configuration field and TOML section**

  Rename `AnalyticsConfigToml` to `MetricsConfigToml`, the `[analytics].enabled` section to `[metrics].enabled`, and the effective `Config::analytics_enabled` field to `Config::metrics_enabled`. Update profile merging, schema generation, comments, and all config construction sites together. Do not add serde aliases for `[analytics]` because the fork has no external users.

- [x] **Step 2: Rename the app-server and CLI default plumbing**

  Rename `default_analytics_enabled` to `default_metrics_enabled` through app-server startup, OTEL reload, CLI daemon telemetry, and the flag `--analytics-default-enabled` to `--metrics-default-enabled`. Preserve the current rule: an explicit config value wins; when unset, the caller-provided default controls metrics export.

- [x] **Step 3: Keep OTEL metrics behavior and remove analytics coupling**

  Make `crates/core/src/otel_init.rs` read `config.metrics_enabled` while leaving the selected `config.otel.metrics_exporter` unchanged. Keep the existing `OtelProvider` exporter and runtime metrics behavior; only the name and ownership of the gate change.

- [x] **Step 4: Remove the obsolete analytics response metadata field**

  Remove `ANALYTICS_ENABLED_KEY`, the `analytics_enabled` field in `CodexResponsesMetadata`, and its serialized request field. The repository-wide search currently shows no active consumer for this field after the hosted analytics sink removal. Do not remove unrelated Responses metadata.

- [x] **Step 5: Verify configuration generation and references**

  Run `just generate`, then:

  ```bash
  rg -n 'analytics_enabled|AnalyticsConfigToml|\[analytics\]|analytics-default-enabled|default_analytics_enabled' crates docs --glob '*.{rs,toml,md,json,ts}'
  cargo check -p codex-config -p codex-core -p codex-app-server -p codex-cli
  ```

  Expected production result: no old analytics configuration names; any remaining match must be an inventory entry explaining why it is not production code.

### Task 3: Move or remove misplaced execution and plugin types

**Files:**
- Modify: `crates/core-plugins/src/artifact_operation.rs`
- Modify: `crates/core/src/tools/events.rs`
- Modify: `crates/core/src/turn_timing.rs`
- Modify: `crates/core/src/turn_timing_tests.rs` only for imports or dead profile assertions, without running tests
- Modify: `crates/core/src/turn_metadata.rs`
- Modify: `crates/core/src/tools/code_mode/telemetry.rs`
- Modify: `crates/core/src/tools/control_tool_analytics.rs`
- Modify: `crates/core/src/session/turn.rs`
- Modify: `crates/core/src/tasks/mod.rs`
- Modify: files that construct `TurnResolvedConfigFact`, `CodeModeToolCallFact`, or `ControlToolCallFact`

- [x] **Step 1: Make `codex-core-plugins::ArtifactOperation` canonical**

  Keep the existing recognition type and its fields in `codex-core-plugins`. In `crates/core/src/tools/events.rs`, pass those fields directly to the existing OTEL metric tags and remove the analytics-only envelope, `TrackEventsContext`, lifecycle field, timestamp helper, and `track_artifact_operation` call. Do not remove `ARTIFACT_OPERATION_STARTED_METRIC` or `ARTIFACT_OPERATION_EXPECTED_OUTPUT_COUNT_METRIC`.

- [x] **Step 2: Remove analytics-only turn profile ownership**

  Remove the `codex_analytics::TurnProfile` import and the `TurnProfile`/`completed_profile` storage and return path from `TurnTimingState`. Preserve `started_at`, item timing, first-token/first-message timing, the existing TTFT/TTFM methods, and any active `codex_otel` calls. Remove only profile fields and calls whose sole consumer is `track_turn_profile`.

- [x] **Step 3: Remove the analytics metadata trait coupling**

  Delete the `impl codex_analytics::TurnAnalyticsMetadata for TurnMetadataState`. Remove `Arc<dyn TurnAnalyticsMetadata>` from code-mode/control-tool fact shapes and their guards. Keep `TurnMetadataState` and its Responses/MCP metadata behavior owned by core. If a caller still needs a value for a non-analytics operation, pass the concrete core-owned value or the existing turn context instead of introducing another analytics-named trait.

- [x] **Step 4: Remove analytics-only fact construction**

  Delete construction and emission of `TurnResolvedConfigFact`, `TurnTokenUsageFact`, `TurnProfileFact`, `TurnCodexErrorFact`, `CodeModeToolCallFact`, and `ControlToolCallFact` where no non-analytics consumer remains. Preserve actual runtime behavior around compaction, tool dispatch, errors, code-mode execution, and token accounting when it feeds rollout, OTEL, UI, or security paths.

- [x] **Step 5: Verify the type boundary**

  Run:

  ```bash
  rg -n 'codex_analytics::(ArtifactOperation|TurnProfile|TurnAnalyticsMetadata|TurnResolvedConfigFact|TurnTokenUsageFact|TurnProfileFact|TurnCodexErrorFact|CodeModeToolCallFact|ControlToolCallFact)' crates --glob '*.rs'
  cargo check -p codex-core-plugins -p codex-core
  ```

  Expected result: no core-owned execution state imports an analytics type, and the artifact metrics producer still compiles.

### Task 4: Remove analytics client injection and producer calls

**Files:**
- Modify: `crates/core/src/state/service.rs`, `session/mod.rs`, `session/session.rs`, `codex_thread.rs`, `thread_manager.rs`, and all remaining core call sites from Task 1
- Modify: `crates/app-server/src/analytics_utils.rs`, `message_processor.rs`, `outgoing_message.rs`, `request_processors.rs`, request processors, extensions, and test support
- Modify: `crates/core-plugins/src/manager.rs`
- Modify: `crates/ext/goal/`, `crates/ext/guardian-reviewer/`, `crates/ext/guardian-v2/`, `crates/ext/history-notes/`, `crates/ext/skills/`
- Modify: `crates/external-agent-migration/`
- Modify: `crates/core/tests/common/`, `crates/thread-manager-sample/`, and affected manifests

- [x] **Step 1: Remove client fields and constructors**

  Remove `AnalyticsEventsClient` fields from service/session/request processor structs, constructor parameters, extension data insertion, and `core-api` re-export. Replace `analytics_events_client.is_enabled()` branches with the owning behavior: remove analytics-only branches, retain unconditional provider-neutral behavior, and retain OTEL metric checks through `SessionTelemetry` where they already exist.

- [x] **Step 2: Remove producer calls by capability**

  Delete calls that only feed analytics for initialize/request/response/notification tracking, plugin-used/install events, skills, compaction, goals, Guardian analytics, code mode, MCP elicitation, image preparation, hook runs, external-agent import analytics, and turn analytics. Do not delete the underlying feature or security flow when it has another current consumer.

- [x] **Step 3: Remove plugin metrics sidecar only if its sole sink is analytics**

  Audit `crates/core/src/plugins/metrics.rs` and `crates/core-plugins/src/plugin_metrics_sidecar.rs`. If the sidecar has no OTEL, UI, plugin contract, or security consumer beyond `AnalyticsEventsClient`, remove the sidecar path and its dedicated types; otherwise retain the producer-owned measurement types and detach only the analytics call.

- [x] **Step 4: Remove extension-specific analytics modules**

  Delete analytics-only extension modules and fields, including the goal analytics wrapper and analytics-only Guardian reporting parameters. Preserve Guardian decisions, outcomes, retry behavior, and existing `SessionTelemetry` metrics.

- [x] **Step 5: Verify the injection removal**

  Run:

  ```bash
  rg -n 'codex-analytics|codex_analytics|AnalyticsEventsClient|track_[a-z_]+\(' crates --glob '*.{rs,toml}'
  cargo check -p codex-core -p codex-app-server -p codex-core-plugins -p codex-core-api -p codex-external-agent-migration
  ```

  Remaining `track_` matches must belong to non-analytics APIs, and any `codex_analytics` match must be in the not-yet-removed crate itself or be eliminated in the next task.

### Task 5: Delete the analytics crate and workspace edges

**Files:**
- Delete: `crates/analytics/`
- Modify: root `Cargo.toml`
- Modify: `Cargo.lock` if the workspace package disappears from the lockfile
- Modify: every manifest still declaring `codex-analytics`
- Modify: generated dependency graph artifacts when the repository task requires them
- Modify: `docs/openai-decoupling-plan.md` and `docs/pre-release-compatibility-inventory.md`

- [x] **Step 1: Remove manifests and workspace membership**

  Delete the workspace member and `codex-analytics` workspace dependency after all Rust consumers are gone. Remove direct manifest dependencies from core, app-server, core-api, core-plugins, extensions, tests, and external-agent migration.

- [x] **Step 2: Delete analytics-only source and tests**

  Delete the crate source, debug capture implementation, analytics client/reducer tests, and fixtures that have no remaining producer or consumer. Do not delete provider-neutral tests for OTEL, plugin recognition, Guardian behavior, MCP, or app-server protocol merely because their old setup used `AnalyticsEventsClient::disabled()`.

- [x] **Step 3: Regenerate and inspect artifacts**

  Run `just crate-dependency-graph` and inspect the dependency diff for the removal of `codex-analytics` without unrelated edge changes. Run `just generate` if config or protocol generated files were changed.

- [x] **Step 4: Run the repository verification gate**

  Run `just check` and `git diff --check`. Per `AGENTS.md`, do not run tests for this cleanup. Resolve compile errors by restoring the owning provider-neutral behavior or moving a type to its producer; do not reintroduce an analytics dependency to silence a missing type.

- [x] **Step 5: Close the documentation slice**

  Mark the inventory rows complete with the final producer/consumer audit, mark the Phase 3 plan item complete only after the full verification gate, and record which metrics, security, plugin, and provider-neutral paths were retained.

## Completion Criteria

- `codex-analytics` is no longer a workspace member or dependency.
- No production code imports `codex_analytics` or uses analytics-specific client/fact/reducer names.
- Core execution state and plugin domain types no longer depend on an analytics crate.
- OTEL metrics, provider-neutral plugin behavior, Guardian/security behavior, MCP behavior, and current app-server clients remain available.
- `metrics_enabled` is the only remaining configuration gate formerly named analytics, with the same explicit/default semantics.
- `just check`, required `just generate`, dependency graph regeneration, and `git diff --check` succeed.

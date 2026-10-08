# 更新機能削除 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Genex の CLI、daemon、TUI、doctor から更新機能と更新専用の外部アクセスを削除し、daemon の通常ライフサイクルと初回 bootstrap だけを残す。

**Architecture:** 更新専用の producer/consumer を削除し、初回 daemon package 準備と通常の pid-backed lifecycle を独立した責務として維持する。TUI/CLI/doctor は更新状態を保持・表示・実行せず、外部配布経路に責務を戻す。

**Tech Stack:** Rust workspace, clap, Tokio, ratatui, serde JSON, `just` verification recipes.

**Spec:** `docs/superpowers/specs/2026-10-08-remove-update-capability-design.md`

## Global Constraints

- Work directly on `main`; do not create a Git worktree.
- Do not add compatibility migrations for obsolete update cache, updater pid, or updater socket state.
- Preserve daemon `bootstrap`, `start`, `restart`, `stop`, `version`, initial package validation/copy, operation locks, recovery, and remote server-version warnings.
- Do not remove unrelated uses of the word `update`, such as thread metadata updates, patch updates, configuration updates, or marketplace upgrades.
- Rust check commands run through the repository `just` recipes and a user systemd scope with the memory limits from `AGENTS.md`.
- For this cleanup, use `just check --tests` and `just check` as verification substitutes; do not run standalone tests.

## Review Focus

- Bootstrap must still prepare a missing daemon package after updater code is removed; verify daemon package validation and lifecycle call paths.
- A remote app-server version warning must remain while update execution and update notification paths disappear; verify its remaining `update_versions` consumer.
- Doctor output must remain valid after removing its update check and desktop CDN probe; verify check ordering, grouping, and snapshots.
- TUI exit and startup state must not retain update actions or perform external version probes; verify all `AppExitInfo` and `AppEvent` constructors.
- Generated schemas/docs must not reintroduce the removed daemon update command or updater settings; inspect `just generate` diff and residual searches.

---

### Task 1: Remove daemon update execution while preserving bootstrap and lifecycle

**Files:**
- Modify: `crates/app-server-daemon/src/lib.rs`
- Modify: `crates/app-server-daemon/src/prepare_install.rs`
- Modify: `crates/app-server-daemon/src/settings.rs`
- Modify: `crates/app-server-daemon/src/telemetry.rs`
- Delete: `crates/app-server-daemon/src/update_loop.rs`
- Delete: `crates/app-server-daemon/src/manual_update.rs`
- Delete: `crates/app-server-daemon/src/migration.rs`
- Delete/update tests: `crates/app-server-daemon/src/update_loop_tests.rs`, `crates/app-server-daemon/src/prepare_install_tests.rs`, settings/telemetry tests as required by removed APIs

**Interfaces:**
- Remove `UpdateStatus`, `UpdateOutput`, `update`, `run_pid_update_loop`, `update_from_cli`, and all updater-specific daemon methods.
- Keep `Daemon::bootstrap`, lifecycle commands, `prepare_install::prepare` in missing-package mode, `InstallContext` validation, operation locking, and pid backend lifecycle.
- Remove `UpdaterSettings`, auto-update interval fields, updater pid/socket paths, and update telemetry tags only where they are update-only; retain daemon state paths needed by lifecycle.

- [x] **Step 1: Audit daemon consumers and mark the exact retained bootstrap path.**

  Run: `rg -n 'update_loop|manual_update|migration|UpdateOutput|UpdateStatus|update_from_cli|run_pid_update_loop|auto_update|update_interval|update_pid|manual_update_socket|ensure_managed_updater' crates/app-server-daemon crates/cli crates/tui`

  Expected: every match is classified as update-only removal or lifecycle/bootstrap retention before edits.

- [x] **Step 2: Remove updater modules and public APIs.**

  Delete the updater loop/manual/migration modules and remove their module declarations, public functions, result types, updater settings, updater process startup/replacement, and update-only telemetry. Simplify `prepare_install` to the missing-package path used by bootstrap/start, preserving package manifest/target/executable validation and safe staging.

- [x] **Step 3: Remove update-only tests and update retained daemon tests.**

  Remove tests whose only subject is scheduled/manual update, pin restoration, updater handoff, installer script execution, or `--from-cli`; retain or adjust bootstrap/package/lifecycle tests so they no longer mention updater state.

- [x] **Step 4: Run the daemon-targeted workspace type/test-target check.**

  Run: `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=4 just check --tests -p genex-app-server-daemon`

  Expected: command succeeds with no updater-related unresolved references.

### Task 2: Remove CLI update dispatch and doctor update diagnostics

**Files:**
- Modify: `crates/cli/src/main.rs`
- Modify: `crates/cli/src/doctor.rs`
- Modify: `crates/cli/src/doctor/output.rs`
- Delete: `crates/cli/src/doctor/updates.rs`
- Delete/update tests: `crates/cli/src/daemon_update_tests.rs` and update-only doctor tests/snapshots
- Modify: `crates/cli/src/daemon_telemetry.rs` if update command metrics are update-only

**Interfaces:**
- Remove `AppServerDaemonSubcommand::Update` and hidden `PidUpdateLoop` plus dispatch/help/name handling.
- Remove `handle_app_exit` update-action execution and daemon handoff execution from the CLI.
- Keep normal TUI exit handling, daemon lifecycle dispatch, doctor checks other than update diagnostics, and JSON output contracts for retained commands.

- [x] **Step 1: Remove CLI command and exit-action references.**

  Delete the daemon update variants, update dispatch branches, updater HTTP client factory, update telemetry call paths, `run_update_action`, and the now-unused imports/arguments. Ensure `app-server daemon start|restart|stop|version|bootstrap` still dispatches unchanged.

- [x] **Step 2: Remove doctor update check and desktop update CDN probing.**

  Remove the `updates` module, its check registration, desktop update augmentation, update-only notes/rendering, and any now-unused imports. Preserve the doctor report schema and all non-update checks.

- [x] **Step 3: Remove update-only CLI tests and snapshots.**

  Delete tests for package-manager update commands, daemon update handoff, update health rows, and update-only snapshots; adjust retained doctor fixtures only when their check set or order changes.

- [x] **Step 4: Compile CLI test targets.**

  Run: `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests -p genex-cli`

  Expected: command succeeds and no removed CLI update symbol remains.

### Task 3: Remove TUI update prompts, state, menu, and notices

**Files:**
- Modify: `crates/tui/src/lib.rs`
- Modify: `crates/tui/src/app.rs`
- Modify: `crates/tui/src/app_event.rs`
- Modify: `crates/tui/src/app/event_dispatch.rs`
- Modify: `crates/tui/src/app/exit_summary.rs`
- Modify: `crates/tui/src/app/startup.rs`
- Modify: `crates/tui/src/startup_orchestration.rs`
- Modify: `crates/tui/src/history_cell/mod.rs` and `crates/tui/src/history_cell/notices.rs`
- Delete: `crates/tui/src/update_action.rs`, `update_prompt.rs`, `updates.rs`, `updates_cache.rs`
- Modify or retain: `crates/tui/src/update_versions.rs` according to its remote-version consumer
- Delete/update tests: update prompt/cache/action tests and daemon-menu update tests

**Interfaces:**
- Remove `UpdateAction`, `DaemonUpdateSource`, `AppExitInfo.update_action`, `App.pending_update_action`, update app events, startup update probe/prompt, and update history cell.
- Keep remote server version comparison and warning behavior if still consumed by `status/remote_connection.rs`.
- Remove daemon menu update choices and the update-only daemon menu module; do not remove unrelated daemon connection/status UI.

- [x] **Step 1: Remove startup update probing and prompt flow.**

  Delete the release/package registry request path, `version.json` cache use, startup popup, and update action propagation. Ensure TUI initialization and normal exit continue to construct valid `AppExitInfo` values.

- [x] **Step 2: Remove daemon update menu and event routing.**

  Remove update choices, confirmation screens, update events, pending update state, immediate update exit path, and TUI handoff telemetry. Preserve any daemon status or connection handling that is not an update action.

- [x] **Step 3: Remove update history cell and retain only non-update version warnings.**

  Delete `UpdateAvailableHistoryCell` and its imports/tests. Keep the remote app-server compatibility warning path if `update_versions.rs` still supplies it; remove only functions with no remaining consumer.

- [x] **Step 4: Compile TUI test targets.**

  Run: `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests -p genex-tui`

  Expected: command succeeds with no TUI update prompt/action/cache references.

### Task 4: Update documentation, compatibility audit, generated artifacts, and final verification

**Files:**
- Modify: `crates/app-server-daemon/README.md`
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/provider-decoupling-plan.md` and affected update-removal plan docs
- Modify generated artifacts only if `just generate` identifies a producer/consumer change

**Interfaces:**
- Documentation must describe only retained daemon lifecycle/bootstrap behavior and must not advertise update commands/settings.
- The compatibility inventory must record explicit `remove` decisions and producer/consumer evidence for daemon updater, TUI updater, doctor update diagnostics, and package-manager update paths.

- [x] **Step 1: Remove obsolete update documentation and record audit decisions.**

  Remove update commands, automatic-update settings, installer URL instructions, and package-manager update instructions from active docs. Add a dated inventory entry with the audited producers/consumers and `remove` decision; keep historical external names only where they are required for legal/history or unrelated retained contracts.

- [x] **Step 2: Regenerate checked-in artifacts and inspect the diff.**

  Run: `just generate`

  Expected: generation succeeds; inspect every generated diff and retain only changes caused by removed producers.

- [x] **Step 3: Run repository-wide required checks.**

  Run: `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests`

  Expected: command succeeds.

  Run: `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check`

  Expected: command succeeds.

- [x] **Step 4: Perform residual audit and whitespace verification.**

  Run: `rg -n -i 'daemon update|pid-update-loop|autoUpdateEnabled|updateIntervalMinutes|GENEX_INSTALLER_URL|UpdateAvailableHistoryCell|UpdateAction|updates.status|@codex/codex|brew upgrade.*codex|releases/latest' crates apps docs --glob '!docs/superpowers/specs/2026-10-08-remove-update-capability-design.md' --glob '!docs/superpowers/plans/2026-10-08-remove-update-capability.md'`

  Expected: no active update producer/consumer remains; any retained match is explicitly justified as unrelated version compatibility or historical/external text in the inventory.

  Run: `git diff --check`

  Expected: no whitespace errors.

# Windows サポート削除 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Windows を supported build/runtime surface から除去し、Linux/macOS のみを維持して WSL を Windows 利用者向けに案内する。

**Architecture:** まず producer/consumer と compatibility decision を台帳化し、Windows 専用 crate・protocol・config surface を削除する。その後、core/sandboxing/CLI/TUI と低レベル共通 crate の `cfg(windows)` 分岐・直接依存を Linux/macOS 実装へ収束させ、最後にCI・配布・文書・生成物を更新する。unsandboxed fallback や Windows 用 stub は追加しない。

**Tech Stack:** Rust workspace/Cargo、生成 JSON/TypeScript schema、Python/Just build scripts、GitHub Actions、既存の `just generate` / `just check --tests` / `just check` 検証。

**Spec:** `docs/superpowers/specs/2026-10-06-remove-windows-support-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Keep Linux/macOS sandbox, filesystem, network, keyring, terminal, PTY, and provider-neutral behavior intact.
- Remove Windows implementation and supported build paths; do not replace them with unsandboxed execution or compatibility stubs.
- Before changing production code, record producer/consumer evidence and an explicit `remove`, `retain`, or `migrate-in-place` decision in the two compatibility audit documents.
- Run `just generate` whenever generated artifacts may be affected, `just check --tests` for every Rust change, and `just check` always; do not run standalone tests.
- Preserve only third-party/upstream, historical/legal, or non-Windows external-contract identifiers that the audit explicitly marks `retain`.

## Review Focus

- Removing Windows sandbox code must not remove Linux/macOS sandbox manager selection or security checks; verify supported targets still compile.
- Removing Windows protocol/config types must remove every producer and consumer, including generated JSON/TypeScript schema, without leaving stale app-server event handling.
- Simplifying shared `cfg` code must preserve Unix behavior for PTY, UDS, exec-server, HTTP proxy/TLS, keyring, and sleep inhibition.
- Deleting Windows-only dependencies must not remove a dependency still used by Linux/macOS or a third-party public contract; verify with Cargo metadata and lockfile audit.
- Removing Windows docs/CI/package paths must leave one clear WSL recommendation and no active Windows release job or installer path.

---

### Task 1: Establish the Windows compatibility audit ledger

**Files:**
- Modify: `docs/provider-decoupling-candidates.md`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Create: `.superpowers/sdd/2026-10-06-remove-windows-support/progress.md` (git-ignored execution ledger)

**Interfaces:**
- Consumes: current `rg`, `cargo metadata --no-deps`, workspace manifests, generated schema paths, CI/package manifests.
- Produces: a complete classification of each Windows production family as `remove`, `retain`, or `migrate-in-place`, used by Tasks 2–6.

- [ ] **Step 1: Capture the baseline inventory**

  Run tracked-path and production-reference searches for `cfg(windows)`, `target_os = "windows"`, Windows-only crate/module names, Windows sandbox protocol names, PowerShell parser use, CI/package paths, and direct Cargo dependencies. Record the paths and producer/consumer edges in the candidates document.

- [ ] **Step 2: Record explicit decisions**

  Add inventory entries for: dedicated Windows sandbox/MXC crates; Windows sandbox protocol/config/app-server surface; core/sandboxing/CLI/TUI Windows paths; low-level Windows platform modules; direct Windows dependencies; CI/package/setup paths; and active documentation. Mark Windows-only families `remove`, shared Unix code `migrate-in-place`, and third-party/historical/non-Windows contracts `retain` with evidence.

- [ ] **Step 3: Check the ledger for omissions**

  Compare the recorded families against `git ls-files` and `cargo metadata --no-deps`; expected result is that every production Windows match has a decision before code deletion begins.

- [ ] **Step 4: Commit the audit record**

  ```bash
  git add docs/provider-decoupling-candidates.md docs/pre-release-compatibility-inventory.md
  git commit -m "docs: audit Windows support removal boundaries"
  ```

### Task 2: Remove dedicated Windows crates and protocol/config surfaces

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`
- Delete: `crates/windows-sandbox-rs/**`
- Delete: `crates/mxc-sandbox/**`
- Modify/delete: `crates/app-server-protocol/src/protocol/v2/windows_sandbox.rs` and its generated JSON/TypeScript files under `crates/app-server-protocol/schema`
- Modify/delete: `crates/app-server/src/request_processors/windows_sandbox_processor.rs` and Windows-only tests
- Modify: `crates/app-server-protocol/src/protocol/v2/mod.rs`, app-server module declarations, and affected protocol/config exports
- Modify/delete: `crates/config/src/loader/windows.rs`, Windows sandbox config modules, feature registry, and generated config schema artifacts

**Interfaces:**
- Consumes: Task 1 decisions for dedicated Windows-only producers and consumers.
- Produces: a workspace with no Windows-only sandbox/MXC package, protocol types, schema, config feature, or app-server request path.

- [ ] **Step 1: Remove workspace members and direct dependencies**

  Remove the Windows sandbox and MXC workspace members, internal dependency aliases, Windows-only direct external dependencies, and lockfile packages that become unreachable. Preserve dependencies still required by Linux/macOS or retained upstream contracts.

- [ ] **Step 2: Remove Windows protocol and generated schema producers**

  Delete Windows sandbox protocol modules, generated JSON/TypeScript types, app-server request processor, event/request variants, and tests. Remove module declarations and re-exports from every consumer found by exact-reference search.

- [ ] **Step 3: Remove Windows config and feature surface**

  Delete Windows-only config loader/types, feature keys, config-read exposure, and generated config schema entries. Do not remove Linux/macOS sandbox configuration or shared config parsing.

- [ ] **Step 4: Regenerate and inspect generated artifacts**

  Run `just generate`. Expected: generated artifacts no longer contain Windows sandbox protocol/config entries; the command succeeds and unrelated schema output is unchanged.

- [ ] **Step 5: Verify crate/protocol removal**

  Run `cargo metadata --no-deps` and exact searches for `genex-windows-sandbox`, `genex-mxc-sandbox`, `windows_sandbox`, `WindowsSandbox`, and the removed feature/schema names. Expected: no active workspace or production references remain.

- [ ] **Step 6: Commit the dedicated-surface removal**

  ```bash
  git add Cargo.toml Cargo.lock crates docs
  git commit -m "Remove Windows sandbox and protocol surfaces"
  ```

### Task 3: Collapse core, sandboxing, CLI, and TUI to supported platforms

**Files:**
- Delete/modify: `crates/core/src/config/windows_sandbox_config.rs`, `crates/core/src/windows_sandbox.rs`, `crates/core/src/windows_sandbox_read_grants.rs`, `crates/core/src/windows_system_config.rs`, and their tests
- Modify/delete: `crates/sandboxing/src/**` Windows managers, Windows sandbox state, and tests
- Modify/delete: `crates/cli/src/doctor/**` Windows security/dev-drive modules and tests; CLI Windows-only branches
- Modify/delete: `crates/tui/src/**` Windows sandbox prompts, Windows console/screen-reader/terminal-probe/IDE-pipe paths, events, state, and tests
- Modify: affected `Cargo.toml` files and generated snapshots/config fixtures

**Interfaces:**
- Consumes: Task 2's supported workspace APIs and the ordinary Linux/macOS sandbox/config interfaces.
- Produces: core, sandboxing, CLI, and TUI code with no Windows-only state/events/imports while preserving supported-platform behavior.

- [ ] **Step 1: Remove Windows state and event producers**

  Delete Windows sandbox config/grant/system modules, app/TUI Windows state, prompt/event variants, readiness/setup branches, and tests whose only purpose is Windows behavior. Remove all imports, module declarations, re-exports, telemetry, and snapshots that become unreachable.

- [ ] **Step 2: Simplify shared platform selection**

  Replace `cfg(any(unix, windows))` with the appropriate supported-platform condition, retain Linux/macOS implementations, and remove Windows-only fallback arms without changing Unix sandbox error semantics.

- [ ] **Step 3: Remove Windows CLI diagnostics and package assumptions**

  Delete Windows desktop/security/dev-drive diagnostics and Windows-specific package identity, executable, or setup-helper assumptions. Update doctor/help output to describe only supported platforms.

- [ ] **Step 4: Compile affected test targets**

  Run `cargo check --tests -p genex-core -p genex-sandboxing -p genex-cli -p genex-tui`. Expected: all affected supported-platform targets compile without Windows sandbox types or events.

- [ ] **Step 5: Commit the high-level simplification**

  ```bash
  git add crates Cargo.lock
  git commit -m "Remove Windows paths from core CLI and TUI"
  ```

### Task 4: Remove Windows branches from low-level shared crates

**Files:**
- Modify/delete: `crates/exec-server/src/fs_sandbox_windows_tests.rs`, `src/no_follow/windows.rs`, `tests/file_system_windows.rs`, and Windows target dependencies
- Modify/delete: `crates/http-client/src/outbound_proxy/windows.rs`, `src/windows_tls.rs`, and target dependencies
- Modify/delete: `crates/network-proxy/src/windows_proxy_ingress.rs`, `src/windows_tcp_attribution.rs`, tests, and target dependencies
- Modify/delete: `crates/uds/src/windows_peer.rs`, `windows_security.rs`, `windows_socket_validation.rs`, and target dependencies
- Modify/delete: `crates/utils/pty/src/windows_input.rs`, Windows tests, and target dependencies
- Modify/delete: `crates/utils/sleep-inhibitor/src/windows_inhibitor.rs`, target dependencies, and platform selection
- Modify/delete: Windows branches in `crates/path-utils`, `crates/arg0`, `crates/hooks`, `crates/rmcp-client`, `crates/keyring-store`, `crates/shell-command`, `crates/exec`, and related manifests

**Interfaces:**
- Consumes: supported-platform call sites from Tasks 2–3.
- Produces: shared crates that compile for Linux/macOS without Windows-only modules, direct dependencies, target tables, or tests.

- [ ] **Step 1: Audit each low-level module's supported consumers**

  For every Windows file or `cfg(windows)` block, identify the Linux/macOS replacement or confirm the symbol is Windows-only and can be deleted. Record any `retain` exception in the inventory before editing.

- [ ] **Step 2: Remove Windows implementations and manifests**

  Delete Windows modules/tests and remove only the corresponding target dependencies and features. Change shared target conditions to Unix/macOS-supported conditions where needed; keep OS-independent code and supported platform dependencies.

- [ ] **Step 3: Remove PowerShell-only parser support**

  Delete the PowerShell tree-sitter dependency, parser branch, and Windows-only parser fixtures/tests. Keep Bash and other supported shell handling.

- [ ] **Step 4: Compile the affected workspace targets**

  Run `cargo check --tests` after the low-level edits. Expected: supported-platform workspace test targets compile with no unresolved Windows modules or dependencies.

- [ ] **Step 5: Commit low-level cleanup**

  ```bash
  git add Cargo.toml Cargo.lock crates
  git commit -m "Remove Windows platform implementations"
  ```

### Task 5: Remove Windows build, CI, package, and active documentation paths

**Files:**
- Modify: `justfile`, `.github/**`, `README.md`, `docs/install.md`, `docs/getting-started.md`, and platform/package documentation found by the audit
- Delete: `scripts/setup-windows.ps1` and other Windows-only setup/release scripts after consumer audit
- Modify: voice/build tooling and package manifests only where Windows-specific support is repository-owned and no supported consumer remains
- Modify: `docs/provider-decoupling-candidates.md`, `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: final supported-platform code and dependency graph from Tasks 2–4.
- Produces: Linux/macOS-only build and release instructions, no active Windows CI/package path, and a clear WSL recommendation.

- [ ] **Step 1: Remove Windows-only Just and CI recipes**

  Remove `[windows]` recipes, Windows shell selection, Windows matrix jobs, Windows artifact/upload targets, and Windows-only install commands. Preserve Unix/macOS recipes and generic cross-platform tooling.

- [ ] **Step 2: Remove Windows package/setup paths**

  Delete Windows setup/install scripts, manifests, helper packaging, and release metadata after checking all references. Update package inventories to list only Linux/macOS artifacts.

- [ ] **Step 3: Update user-facing guidance**

  Add concise wording to README/install/getting-started documentation: Windows is unsupported; use the Linux build under WSL. Remove instructions that promise a Windows executable or native Windows setup.

- [ ] **Step 4: Reconcile audit documents**

  Mark each completed deletion and each retained third-party/historical identifier with its final producer/consumer evidence. Ensure no active Windows support claim remains.

- [ ] **Step 5: Commit build/documentation cleanup**

  ```bash
  git add .github justfile README.md docs scripts third_party
  git commit -m "Remove Windows build and release paths"
  ```

### Task 6: Regenerate and complete repository-wide verification

**Files:**
- Verify all changed files from Tasks 1–5
- Modify generated artifacts only when required by generation output

**Interfaces:**
- Consumes: completed Windows removal and audit ledger.
- Produces: verified Linux/macOS-only repository with documented retain exceptions.

- [ ] **Step 1: Regenerate checked-in artifacts**

  Run `just generate`. Expected: success; inspect and classify every generated diff.

- [ ] **Step 2: Run required checks**

  Run `just check --tests`, then `just check`. Expected: both exit successfully with no errors.

- [ ] **Step 3: Run metadata and Windows-reference audits**

  Run `cargo metadata --no-deps`, `git diff --check`, and repository-wide `rg` audits for Windows target cfgs, Windows-only crate/module names, Windows sandbox protocol/config names, PowerShell parser dependencies, Windows CI/package paths, and native Windows support claims. Expected: no unclassified active production references; every retained match is documented.

- [ ] **Step 4: Review the final diff against the spec**

  Confirm Linux/macOS implementations remain reachable, no unsandboxed Windows fallback or stub was introduced, WSL guidance is present, and the audit documents cover every deletion family.

- [ ] **Step 5: Commit any verification-generated changes**

  ```bash
  git add -A
  git commit -m "Finalize Linux and macOS platform support"
  ```

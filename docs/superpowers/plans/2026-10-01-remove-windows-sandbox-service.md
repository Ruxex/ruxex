# Remove the Windows sandbox service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the Windows sandbox service and Registered Core mode while preserving the ordinary Windows CLI sandbox.

**Architecture:** Delete the service crate and its package/IPC lifecycle, then simplify `genex-windows-sandbox` and app-server to use the existing local/elevated helper path. Keep command-runner process isolation, ACLs, Restricted Tokens, Job Objects, Private Desktop, deny-read handling, and normal network controls intact.

**Tech Stack:** Rust workspace, Cargo features and generated config schema, Windows APIs, app-server protocol/config, `just generate`, `just check --tests`, `just check`.

**Spec:** `docs/superpowers/specs/2026-10-01-remove-windows-sandbox-service-design.md`

## Global Constraints

- Work directly on `main`; do not create a worktree.
- Preserve the ordinary Windows sandbox setup and command execution security boundary.
- Do not replace service absence with unsandboxed execution.
- Remove Registered Core together with the service because it has no valid fallback without service-owned provisioning.
- Follow the repository verification policy: `just generate` when generated artifacts are affected, `just check --tests` for Rust changes, and `just check` always; do not run standalone tests unless explicitly requested.
- Update `docs/pre-release-compatibility-inventory.md` with an explicit producer/consumer audit and decision before or alongside each production cleanup slice.

## Review Focus

- Ordinary CLI setup still requires and validates `genex-windows-sandbox-setup.exe` and `codex-command-runner.exe`; test package validation and helper materialization after service removal.
- Elevated setup no longer branches on service availability or Registered Core; test that setup failures remain explicit rather than falling back to unsandboxed execution.
- ACL/deny-read, Restricted Token, Job Object, Private Desktop, and proxy/firewall setup remain reachable; compile the affected Windows sandbox targets and inspect removed exports.
- App-server readiness/setup no longer performs registration refresh or service provisioning; compile app-server/core/sandboxing test targets and inspect config schema generation.
- Workspace metadata, generated schemas, manifests, and documentation contain no active service/Registered Core contracts; distinguish historical plan text from active documentation.

---

### Task 1: Remove the service crate and configuration/build surface

**Files:**
- Modify: `Cargo.toml`, `Cargo.lock`
- Delete: `crates/windows-sandbox-service/**`
- Modify: `crates/features/src/lib.rs`
- Modify: `crates/app-server/src/request_processors/config_processor.rs`
- Modify: generated config artifacts under `crates/core/config.schema.json` and any files changed by `just generate`
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: the workspace member and `Feature::WindowsSandboxService` references identified by the pre-flight search.
- Produces: a workspace with no service crate, no `windows_sandbox_service` feature key, and no config-read exposure for that key.

- [ ] **Step 1: Record the producer/consumer decision**

  Add or update the inventory entry to mark the service crate, feature key, config exposure, and package lifecycle as `remove`, recording that ordinary CLI setup/runner artifacts remain current consumers.

- [ ] **Step 2: Remove the workspace and feature surface**

  Delete the service crate, remove its workspace member and lockfile package, remove `Feature::WindowsSandboxService` and its `FeatureSpec`, and remove `windows_sandbox_service` from config-read exposure. Do not remove `WindowsSandbox`, `WindowsSandboxElevated`, or the ordinary Windows sandbox package entries.

- [ ] **Step 3: Regenerate and inspect generated configuration**

  Run `just generate`. Expected: generated config/schema artifacts update without the removed service feature, and no ordinary Windows sandbox entries disappear.

- [ ] **Step 4: Verify workspace removal**

  Run `cargo metadata --no-deps` and `rg -n 'genex-windows-sandbox-service|windows_sandbox_service' Cargo.toml Cargo.lock crates --glob '*.{rs,toml,json}'`. Expected: no active workspace, crate, feature, or generated-config references remain.

- [ ] **Step 5: Commit the service/config removal**

  ```bash
  git add Cargo.toml Cargo.lock crates/windows-sandbox-service crates/features crates/app-server docs/pre-release-compatibility-inventory.md
  git commit -m "Remove Windows sandbox service surface"
  ```

### Task 2: Remove Registered Core and service IPC from the Windows sandbox library

**Files:**
- Modify: `crates/windows-sandbox-rs/src/lib.rs`
- Delete when audit confirms service-only ownership: `app_package.rs`, `app_package_tests.rs`, `installation_record.rs`, `provisioning_protocol.rs`, `provisioning_client.rs`, `provisioning_client/**`, `runtime_ownership.rs`, `runtime_ownership_tests.rs`, `service_identity.rs`, `service_identity_tests.rs`
- Modify: `crates/windows-sandbox-rs/src/elevated/runner_client.rs`, `runner_metrics.rs`, `runner_pipe.rs`
- Modify: `crates/windows-sandbox-rs/src/helper_materialization.rs`, `identity.rs`, `process.rs`, `conpty/mod.rs`
- Modify: `crates/windows-sandbox-rs/src/setup.rs`, `setup_provisioning.rs`, `setup_provisioning/**`, and affected Windows sandbox tests
- Modify: `crates/windows-sandbox-rs/src/uninstall_windows/**` only where service-owned registration/cleanup is referenced
- Modify: `docs/pre-release-compatibility-inventory.md`

**Interfaces:**
- Consumes: ordinary setup, runner, process, ACL, deny-read, and network primitives in the library.
- Produces: a provider/service-independent Windows sandbox library whose public exports contain only ordinary setup, runner, process, ACL, and network capabilities.

- [ ] **Step 1: Audit each service/Registered Core consumer**

  For every `registered_core`, package-identity, runtime-ownership, provisioning, service-name, and service-pipe match, identify whether it is service-only or also supports ordinary CLI setup. Record decisions in the inventory before changing the consumer.

- [ ] **Step 2: Delete service-only protocol and ownership modules**

  Remove the named-pipe protocol/client, service identity, package registration, runtime ownership receipt, Registered Core package verification, and their tests. Remove corresponding module declarations and public re-exports from `lib.rs`.

- [ ] **Step 3: Simplify helper and runner selection**

  Remove Registered Core alias/image verification and package-context propagation. Make normal helper materialization and runner launch the only path, while preserving the command runner, elevation IPC, process creation, Job Object, console/ConPTY, and stdio behavior.

- [ ] **Step 4: Simplify setup provisioning**

  Remove service-owned provisioning branches and Registered Core-only errors. Keep the existing elevated helper, ACL refresh, sandbox account setup, deny-read ACL synchronization, firewall/proxy setup, and cleanup behavior needed by ordinary CLI sandboxing.

- [ ] **Step 5: Compile the Windows sandbox crate targets**

  Run `cargo check --tests -p genex-windows-sandbox`. Expected: the library and its test targets compile with no service/Registered Core API references, while ordinary Windows sandbox modules remain present.

- [ ] **Step 6: Commit the library cleanup**

  ```bash
  git add crates/windows-sandbox-rs docs/pre-release-compatibility-inventory.md
  git commit -m "Remove Registered Core sandbox provisioning"
  ```

### Task 3: Simplify app-server, core, and sandboxing callers

**Files:**
- Modify: `crates/app-server/src/request_processors/windows_sandbox_processor.rs`
- Modify: `crates/app-server/src/lib.rs`
- Modify: `crates/core/src/windows_sandbox.rs` only if service-specific setup wrappers remain
- Modify: `crates/sandboxing/src/manager.rs`, `crates/sandboxing/src/manager_tests.rs`
- Modify: affected app-server/core/sandboxing tests and fixtures
- Modify: generated protocol/config artifacts only if `just generate` reports them

**Interfaces:**
- Consumes: the ordinary setup APIs retained by Task 2.
- Produces: app-server and sandboxing callers that never set, refresh, or branch on Registered Core/service state.

- [ ] **Step 1: Remove startup registration refresh**

  Delete the app-server readiness `OnceCell`, Registered Core refresh task, service provisioning calls, and startup-only registration flag read. Preserve normal Windows sandbox readiness responses.

- [ ] **Step 2: Simplify setup request processing**

  Remove the `WindowsSandboxService` feature and Registered Core conditions from setup selection. Route elevated and unelevated setup through the existing `codex_core::windows_sandbox::run_windows_sandbox_setup` path, preserving explicit errors from helper setup.

- [ ] **Step 3: Remove environment propagation**

  Remove Registered Core environment injection/filtering from `crates/sandboxing/src/manager.rs` and migrate or delete tests that only assert the removed opt-in. Preserve environment sanitization unrelated to Registered Core.

- [ ] **Step 4: Compile affected callers**

  Run `cargo check --tests -p genex-app-server -p genex-core -p genex-sandboxing`. Expected: callers compile without service APIs and ordinary sandbox setup/session code remains available.

- [ ] **Step 5: Commit caller simplification**

  ```bash
  git add crates/app-server crates/core crates/sandboxing
  git commit -m "Use direct Windows sandbox setup"
  ```

### Task 4: Update active documentation and package audits

**Files:**
- Modify: `docs/hawk-entrypoints.md`
- Modify: `docs/provider-specific-crates.md`
- Modify: `docs/sandbox.md`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: active installer/package manifests only if the audit finds service-specific entries

**Interfaces:**
- Consumes: the final production implementation after Tasks 1–3.
- Produces: documentation that describes only the supported ordinary Windows sandbox and does not advertise the removed service.

- [ ] **Step 1: Remove active service documentation**

  Remove the service binary from active entrypoint/dependency/package documentation and explain that Windows CLI sandboxing uses the setup helper and command runner. Leave historical plan/spec references intact unless they describe current supported behavior.

- [ ] **Step 2: Reconcile the compatibility inventory**

  Change the existing Windows package/registry/pipe retention entry to reflect the removal of service-owned contracts, while retaining any platform identifiers still consumed by ordinary setup/security code.

- [ ] **Step 3: Verify active documentation references**

  Run `rg -n -i 'windows.?sandbox.?service|registered.?core|CODEX_WINDOWS_REGISTERED_CORE|OpenAI\.CodexSandbox' crates docs Cargo.toml`. Expected: only the new design/plan and explicitly historical records remain; no active production or supported-behavior reference remains.

- [ ] **Step 4: Commit documentation cleanup**

  ```bash
  git add docs
  git commit -m "Document direct Windows sandbox support"
  ```

### Task 5: Final generated-artifact and repository verification

**Files:**
- Verify: all files changed by Tasks 1–4
- Modify: generated artifacts only when required by the verification output

**Interfaces:**
- Consumes: the completed implementation and active documentation.
- Produces: a clean repository with no unaccounted service/Registered Core production references and passing required checks.

- [ ] **Step 1: Regenerate and inspect the diff**

  Run `just generate`. Expected: generation succeeds; inspect the diff and ensure only intended schemas/manifests changed.

- [ ] **Step 2: Run required Rust checks**

  Run `just check --tests`, then `just check`. Expected: both succeed with zero errors.

- [ ] **Step 3: Run final symbol and metadata audits**

  Run:

  ```bash
  rg -n -i 'windows.?sandbox.?service|registered.?core|CODEX_WINDOWS_REGISTERED_CORE|OpenAI\.CodexSandbox' crates Cargo.toml Cargo.lock --glob '*.{rs,toml,json}'
  rg -n 'genex-windows-sandbox-service|genex-windows-sandbox-setup|codex-command-runner' . --glob '!target/**'
  cargo metadata --no-deps
  git diff --check
  ```

  Expected: no active service/Registered Core production references; setup/runner references remain; metadata succeeds; diff check is clean.

- [ ] **Step 4: Review the final diff and commit**

  Confirm that ordinary Windows sandbox security and package validation remain in the diff, then create the final cleanup commit if verification introduced changes.

# Cloud Config Hosted Policy Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove ChatGPT-hosted cloud configuration and policy loading while retaining local configuration and managed requirements.

**Architecture:** Delete the cloud-bundle input from `codex-config`, so configuration loading is entirely local. Remove the resulting unused entrypoint plumbing and then remove `codex-cloud-config` and its workspace declarations.

**Tech Stack:** Rust workspace, Cargo, Just, TOML configuration loader.

**Spec:** `docs/superpowers/specs/2026-09-19-cloud-config-hosted-policy-removal-design.md`

## Global Constraints

- Preserve packaged, local system, locally managed, user, project, thread, and runtime configuration sources.
- Preserve locally managed requirements and their existing validation/precedence.
- Do not introduce a replacement remote-policy trait or provider interface.
- Do not change authentication, provider registry, or unrelated backend integration.
- This is crate-removal work: use `cargo check` and `just check` as primary verification; do not add deletion-specific tests first.
- Update `docs/provider-decoupling-plan.md` only after this implementation unit passes all verification.

## Review Focus

- Local managed `requirements.toml` continues to constrain configuration when no hosted bundle exists.
- Startup paths no longer authenticate or access the network merely to load configuration.
- Existing local system configuration retains its ordering ahead of user configuration.
- Strict config errors continue to refer to local configuration fields and sources.
- No workspace manifest, lockfile, or source import leaves a `codex-cloud-config` dependency behind.

### Task 1: Remove hosted-bundle support from `codex-config`

**Files:**
- Delete: `crates/config/src/cloud_config_bundle.rs`
- Delete: `crates/config/src/cloud_config_bundle_tests.rs`
- Delete: `crates/config/src/cloud_config_layers.rs`
- Delete: `crates/config/src/cloud_config_layers_tests.rs`
- Modify: `crates/config/src/lib.rs`
- Modify: `crates/config/src/state.rs`
- Modify: `crates/config/src/loader/mod.rs`
- Modify: `crates/config/src/loader/managed_requirements.rs`
- Modify: `crates/config/src/loader/tests.rs`
- Modify: `crates/config/src/application_requirements_tests.rs`
- Modify: `crates/config/src/test_support.rs`
- Modify: `crates/config/src/test_support_tests.rs`

**Interfaces:**
- Consumes: no preceding task output.
- Produces: `ConfigLoadOptions` with only `loader_overrides` and `strict_config`; `load_config_layers_state` never retrieves a remote bundle.

- [ ] **Step 1: Remove the hosted bundle public API and loader option.**

  In `lib.rs`, remove the two cloud module declarations and all `CloudConfig*` / `CloudRequirements*` re-exports. In `state.rs`, remove the `CloudConfigBundleLoader` import, the `cloud_config_bundle` field, and its default initialization.

- [ ] **Step 2: Make layer loading local-only.**

  In `loader/mod.rs`, remove the `CloudConfigBundleLayers` import, the `cloud_config_bundle` destructuring, the conditional network-bundle conversion block, and `layers.extend(cloud_config_layers)`. Keep `managed_requirements::load_requirements_from_sources` as the source of local requirements.

  In `loader/managed_requirements.rs`, remove its bundle-loader parameter and bundle-derived requirements processing; callers pass only the filesystem, home path, overrides, and strictness.

- [ ] **Step 3: Remove hosted-only implementation and test support.**

  Delete the four cloud bundle/layer implementation and unit-test files. Delete `CloudConfigBundleFixture` and its tests. Remove loader and application-requirements tests that construct a cloud bundle; retain equivalent assertions that use local managed requirement fixtures where the behavior is provider-neutral.

- [ ] **Step 4: Update local-source documentation and test expectations.**

  Remove `cloud` from the configuration/requirements precedence comments in `loader/mod.rs`. Remove assertions and imports that name cloud config from `loader/tests.rs`, `application_requirements_tests.rs`, and `test_support_tests.rs`.

- [ ] **Step 5: Verify the configuration crate.**

  Run: `cargo check -p codex-config`

  Expected: exit status 0 with no unresolved `CloudConfig*` symbols.

- [ ] **Step 6: Commit the configuration boundary.**

  ```bash
  git add crates/config
  git commit -m "refactor: remove hosted config loading"
  ```

### Task 2: Remove hosted-policy plumbing from consumers and their tests

**Files:**
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/core/src/config/config_loader_tests.rs`
- Modify: `crates/core/tests/common/lib.rs`
- Modify: `crates/core/tests/common/test_codex.rs`
- Modify: `crates/core/src/agent/role_tests.rs`
- Modify: `crates/core/src/config/config_tests.rs`
- Modify: `crates/core/src/connectors_tests.rs`
- Modify: `crates/core/tests/suite/{approvals,auto_review,client,cloud_config,current_time_reminder,exec_policy,guardian_context_budget,hooks,hooks_executor,mcp_ema_config,mcp_turn_metadata,model_provider_requirements_tests,network_approval,network_approval_completion,settings_constraints,tools,unified_exec,unified_exec_process_events,web_search}.rs`
- Modify: `crates/app-server-client/src/lib.rs`
- Modify: `crates/app-server/src/{config_manager,in_process,lib}.rs`
- Modify: `crates/app-server/src/request_processors/{account_processor,config_errors,thread_processor_tests}.rs`
- Modify: `crates/app-server/tests/suite/{conversation_summary,v2/mcp_resource,v2/model_provider_enforcement_tests,v2/remote_thread_store,v2/thread_fork,v2/thread_read,v2/thread_resume,v2/thread_start,v2/thread_unarchive,v2/turn_cost_otel}.rs`

**Interfaces:**
- Consumes: local-only `ConfigLoadOptions` from Task 1.
- Produces: core, app-server client, and app-server construction APIs with no `CloudConfigBundleLoader` fields or methods.

- [ ] **Step 1: Remove the loader from shared construction APIs.**

  Delete `cloud_config_bundle` fields, builder methods, and default values from core test/config builders and `InProcessClientStartArgs`. Update every constructor to omit the removed field rather than supplying `Default::default()`.

- [ ] **Step 2: Remove app-server refresh and error paths.**

  Delete `ConfigManager` storage and methods that replace or clear a cloud bundle loader. Remove account/login-triggered loader refreshes, cloud bundle preload during app-server startup, and error translation that identifies `CloudConfigBundleLoadError`.

- [ ] **Step 3: Remove cloud-policy-only tests.**

  Delete `crates/core/tests/suite/cloud_config.rs`. Delete or rewrite test cases whose only setup is `with_cloud_config_bundle` or `CloudConfigBundleFixture`; retain tests whose asserted behavior can be configured through the existing local managed configuration fixtures.

- [ ] **Step 4: Verify shared consumers compile.**

  Run: `cargo check -p codex-core -p codex-app-server-client -p codex-app-server`

  Expected: exit status 0 with no cloud-bundle construction or error handling remaining.

- [ ] **Step 5: Commit the consumer boundary.**

  ```bash
  git add crates/core crates/app-server-client crates/app-server
  git commit -m "refactor: remove hosted policy plumbing"
  ```

### Task 3: Remove entrypoint setup and the cloud config crate

**Files:**
- Delete: `crates/cloud-config/`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/cli/{Cargo.toml,src/cloud_config.rs,src/main.rs,src/mcp_cmd.rs,src/daemon_telemetry.rs,src/debug_sandbox.rs,src/debug_sandbox/cloud_config.rs,src/debug_sandbox/cloud_config_tests.rs}`
- Modify: `crates/tui/{Cargo.toml,src/lib.rs,src/startup_orchestration.rs,src/onboarding/auth.rs,src/session_archive_commands.rs,src/worktree_startup.rs,src/app.rs,src/app/config_persistence.rs,src/app/startup.rs,src/app/test_support.rs,src/app/tests.rs}`
- Modify: `crates/exec/{Cargo.toml,src/lib.rs,src/worktree.rs,tests/suite/worktree.rs}`
- Modify: `crates/windows-sandbox-service/{Cargo.toml,src/machine_policy.rs}`
- Modify: `crates/cli/tests/{cloud_config,features,sandbox_cloud_config,worktree}.rs`
- Modify: `docs/provider-decoupling-plan.md`

**Interfaces:**
- Consumes: consumer APIs without hosted loaders from Task 2.
- Produces: workspace with no `codex-cloud-config` member, dependency, source import, or cloud-bundle startup path.

- [ ] **Step 1: Simplify CLI, TUI, exec, and Windows setup.**

  Remove imports of `codex_cloud_config` and `CloudConfigBundleLoader`. Delete bootstrap/fetch/reload helpers and function parameters that only exist to pass a bundle loader. Change configuration calls to pass `ConfigLoadOptions { loader_overrides, strict_config }` or the equivalent builder API without a cloud field.

- [ ] **Step 2: Delete cloud-specific command and startup tests.**

  Remove tests that inspect bundle cache files, mock cloud configuration HTTP requests, or assert remote config loading errors. Keep normal configuration, MCP, worktree, and sandbox coverage configured through local files.

- [ ] **Step 3: Remove the crate and manifests.**

  Remove `crates/cloud-config` from workspace members and workspace dependencies in root `Cargo.toml`; remove it from each former direct consumer's `Cargo.toml`; then delete `crates/cloud-config/`. Run Cargo metadata resolution to regenerate `Cargo.lock` rather than editing it manually.

  Run: `cargo check -p codex-cli -p codex-tui -p codex-exec -p codex-windows-sandbox-service`

  Expected: exit status 0 and `rg -n 'codex_cloud_config|codex-cloud-config|CloudConfigBundle' Cargo.toml crates --glob 'Cargo.toml' --glob '*.rs'` returns no results.

- [ ] **Step 4: Regenerate the dependency graph and update phase progress.**

  Run: `just crate-dependency-graph`

  Expected: generated graph has no `codex-cloud-config` node or incoming edge.

  Update `docs/provider-decoupling-plan.md` to mark this first Phase 3 unit complete and state that hosted policy loading was removed while later backend integration work remains.

- [ ] **Step 5: Run repository verification.**

  Run: `just check`

  Expected: exit status 0.

- [ ] **Step 6: Commit the removal.**

  ```bash
  git add Cargo.toml Cargo.lock crates/cli crates/tui crates/exec crates/windows-sandbox-service crates/cloud-config docs/provider-decoupling-plan.md
  git commit -m "chore: remove cloud config integration"
  ```

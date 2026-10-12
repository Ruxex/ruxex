# Unify AppServer Runtime Provider Construction Implementation Plan

> **For implementation:** execute this plan directly on the repository `main` branch. Do not create a worktree.

**Goal:** Make daemon startup and in-process startup construct one authoritative AppServer runtime through the same builder, so the provider runtime and its provider-neutral contract cannot be supplied independently or diverge.

**Architecture:** Add a public `AppServerRuntimeBuilder` and an `AppServerRuntime` value in `genex-app-server`. The runtime owns one injected `SharedModelProvider`; configuration-based construction is limited to the host-boundary helper that creates the adapter once. Daemon startup and in-process startup both consume this value. `InProcessAppServerBuilder` remains a transport-specific launcher; it no longer accepts a contract-only provider override.

**Tech Stack:** Rust, Tokio, `genex-app-server`, `genex-app-server-client`, existing `genex-model-provider` runtime and contract adapters.

**Spec:** `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md` and the phased migration in `docs/superpowers/plans/2026-10-08-single-provider-genex-implementation.md`.

## Global Constraints

- Preserve the current main-branch work and do not rewrite unrelated provider-migration changes.
- Keep exactly one provider implementation per AppServer runtime. The contract adapter must be derived from the same runtime provider.
- Keep provider-specific transport/auth implementation behind the `ModelProvider` adapter. Core and AppServer consumers receive only the shared contract handle.
- Mount and validate the provider-scoped data root before opening credentials or SQLite, including when a provider runtime is injected.
- Do not add provider selection or credentials to threads, rollouts, or protocol payloads.
- Use the repository verification policy: Rust checks must run through the user systemd scope and `just`; do not run standalone test binaries.

## Review Focus

- Is `AppServerRuntime` the only source of provider handles passed to `MessageProcessor`?
- Can daemon and in-process startup accidentally create different provider implementations?
- Does the in-process compatibility path still mount the provider home before sensitive state is opened?
- Does the public API make it clear that `InProcessAppServerBuilder` launches a transport while `AppServerRuntimeBuilder` constructs shared runtime state?
- Are existing OpenAI endpoint contracts and current host behavior unchanged?

---

## Context and Decision

The current code had two provider-shaped values in `MessageProcessorArgs`: the old `SharedProviderRuntime` used by core chat/session code and a `SharedModelProvider` contract adapter used by provider-neutral app-server APIs. The migration now makes `SharedModelProvider` the single runtime-owned handle, so status/model metadata and turns cannot diverge.

This change makes the relationship explicit without attempting the larger migration of every core consumer in the same patch:

```rust
let provider = host_selected_model_provider(config, credentials);
let runtime = AppServerRuntimeBuilder::new(provider).build();
```

Both transports pass `runtime` onward. The runtime owns the injected contract handle, so there is one
construction point and no contract-only injection path.

## Task 1: Add the shared runtime value and builder

**Files:**

- Add `crates/app-server/src/runtime.rs`.
- Update `crates/app-server/src/lib.rs` to expose the runtime builder/value.
- Add focused unit coverage in `crates/app-server/src/runtime.rs` or its module tests.

**Interfaces:**

- Add `pub struct AppServerRuntime` containing one shared provider contract.
- Add `pub struct AppServerRuntimeBuilder` with `new(SharedModelProvider)` and `build()`.
- Add the shared provider accessor and provider descriptor helper.
- Add a configured-runtime helper for legacy startup paths so they also use the builder.

**TDD steps:**

1. Add a compile-time test/use that builds an `AppServerRuntime` from one `SharedModelProvider` and verifies that the exposed descriptor is the descriptor of that provider.
2. Run the focused `just check --tests -p genex-app-server` command through the required systemd scope and confirm the test/API fails before implementation.
3. Implement the value, builder, derived adapter construction, and configured-runtime helper.
4. Run the focused check again.

## Task 2: Make daemon startup use the shared runtime

**Files:**

- Update `crates/app-server/src/lib.rs`.
- Update `crates/app-server/src/message_processor.rs`.
- Update any app-server tests or support modules that construct `MessageProcessorArgs`.

**Changes:**

- Replace the separate `provider` and `provider_contract` fields in `MessageProcessorArgs` with one `AppServerRuntime`.
- Destructure/access the two handles only inside `MessageProcessor` and the existing core bridge points.
- Keep `run_main_with_transport_options` as the configuration-based convenience wrapper, and add `run_main_with_transport_options_and_runtime` for hosts that inject an already-built runtime. Both paths validate the provider-home marker before credentials and SQLite, then pass the single runtime through `MessageProcessor`.
- Remove the daemon-local duplicate `provider_contract(...)` construction.

**Verification:**

- Compile all affected app-server test targets with `just check --tests -p genex-app-server` in the memory-limited systemd scope.
- Use `rg` to confirm there is no daemon startup path that constructs an independent contract adapter after the shared runtime is built.

## Task 3: Make in-process startup consume the same runtime

**Files:**

- Update `crates/app-server/src/in_process.rs`.
- Update `crates/app-server-client/src/lib.rs` and its public start arguments.
- Update the in-process test support that currently injects a contract-only fake provider.

**Changes:**

- Replace `InProcessStartArgs.provider: Option<SharedModelProvider>` with `runtime: Option<AppServerRuntime>`.
- Remove `InProcessAppServerBuilder::model_provider(...)`; add a runtime-oriented setter only if the compatibility builder needs one, with a name such as `runtime(...)`.
- Make the explicit builder path require an `AppServerRuntime` and pass it unchanged to startup.
- Keep the legacy direct `start(...)` path working by creating a configured `SharedModelProvider` through `AppServerRuntimeBuilder::from_provider_info`.
- Select the descriptor for data-root mounting from the injected runtime when present, or from configuration before credentials when using the legacy path. Do not construct a provider merely to inspect the descriptor and then construct another provider for execution.
- Change the client crate to forward the runtime value without depending directly on `genex-model-provider`.

**TDD steps:**

1. Change the existing in-process injection test so it builds an `AppServerRuntime` and checks that provider status/model operations use the same runtime-derived descriptor.
2. Run `just check --tests -p genex-app-server` and `just check --tests -p genex-app-server-client` through the required scope to expose API mismatches.
3. Implement the in-process and client wiring.
4. Re-run both focused checks.

## Task 4: Update the design records and remove stale terminology

**Files:**

- Update `docs/superpowers/specs/2026-10-08-single-model-provider-library-design.md`.
- Update `docs/superpowers/plans/2026-10-08-single-provider-genex-implementation.md`.
- Update any nearby app-server client API comments or examples.

**Changes:**

- Document `AppServerRuntimeBuilder` as the common provider/runtime construction point.
- Describe `InProcessAppServerBuilder` as a transport launcher that consumes an already-built runtime.
- State that daemon startup uses the same builder internally; it does not receive a provider through the daemon protocol.
- State that embedding hosts may supply the runtime through `run_main_with_transport_options_and_runtime`; the daemon protocol still has no provider-selection operation.
- Remove examples showing a contract-only `model_provider(...)` override.
- Record that the old runtime/contract split remains an internal transitional adapter until the core provider-contract migration is complete.

## Task 5: Repository verification and final audit

**Checks:**

1. `git diff --check`.
2. Targeted Rust formatting check for changed Rust files.
3. `systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=4 just check --tests -p genex-app-server`.
4. The same command for `genex-app-server-client`.
5. Full `systemd-run ... --setenv=CARGO_BUILD_JOBS=2 just check --tests`.
6. Full `systemd-run ... --setenv=CARGO_BUILD_JOBS=2 just check`.
7. `rg` audit for `provider_contract`, `InProcessStartArgs`, `InProcessAppServerBuilder`, and `AppServerRuntimeBuilder` to confirm all producers and consumers use the new ownership boundary.

No checked-in generated schema or protocol artifact is expected to change. If implementation changes one, run `just generate` through the required systemd scope and inspect the complete diff before completion.

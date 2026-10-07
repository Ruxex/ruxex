set working-directory := "."
set positional-arguments
export CODEX_REPO_ROOT := justfile_directory()
export JUST_SHELL := justfile_directory() / "scripts/just-shell.py"
set shell := ["python3", "-c", 'import os, runpy; runpy.run_path(os.environ["JUST_SHELL"], run_name="__main__")']

rust_min_stack := "8388608" # 8 MiB
python := "python3"

# Display help
help:
    just -l

# `codex`
alias c := codex
codex *args:
    cargo run --bin genex -- {args}

# `genex exec`
exec *args:
    cargo run --bin genex -- exec {args}

# Build and stage the local runtime package used by the Tauri desktop app.
before-desktop:
    systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 {{ justfile_directory() }}/scripts/setup-gstreamer.sh -- {{ justfile_directory() }}/scripts/download-rusty-v8.sh -- rtk cargo build -p genex-cli -p genex-code-mode-host
    pnpm --dir {{ justfile_directory() }}/apps/desktop run stage:runtime --output {{ justfile_directory() }}/apps/desktop/src-tauri/resources/genex-package --genex-bin {{ justfile_directory() }}/target/debug/genex --code-mode-host {{ justfile_directory() }}/target/debug/genex-code-mode-host

# Start the local Tauri desktop application in development mode.
desktop: before-desktop
    pnpm --dir {{ justfile_directory() }}/apps/desktop tauri dev

# Analyze the configured production binaries with Hawk and save the JSON report.
hawk:
    {{ justfile_directory() }}/scripts/setup-gstreamer.sh -- {{ justfile_directory() }}/scripts/download-rusty-v8.sh -- cargo hawk check --output-format json > {{ justfile_directory() }}/hawk.json

# Start `genex exec-server` and run genex-tui.
[no-cd]
[positional-arguments]
[unix]
tui-with-exec-server *args:
    {{ justfile_directory() }}/scripts/run_tui_with_exec_server.sh "$@"

# Run the CLI version of the file-search crate.
file-search *args:
    cargo run --bin genex-file-search -- {args}

# Run the standalone code-mode host from source.
code-mode-host *args:
    cargo run --bin genex-code-mode-host -- {args}

# Build the CLI and run the app-server test client
app-server-test-client *args:
    cargo build -p genex-cli
    cargo run -p genex-app-server-test-client -- --codex-bin ./target/debug/genex {args}

# Format the justfile and Rust sources.
fmt:
    @{{ python }} scripts/format.py

# Check formatting without modifying files.
fmt-check:
    @{{ python }} scripts/format.py --check

fix *args:
    cargo clippy --fix --tests --allow-dirty {args}

clippy *args:
    {{ justfile_directory() }}/scripts/setup-gstreamer.sh -- {{ justfile_directory() }}/scripts/download-rusty-v8.sh -- rtk cargo clippy --tests "$@"

[unix]
install:
    rustup show active-toolchain
    cargo fetch

# Run nextest with --no-fail-fast so all tests are run.
#
# Run `cargo install --locked cargo-nextest` if you don't have it installed.
# Prefer this for routine local runs. Workspace crate features are banned, so
# there should be no need to add `--all-features`.
[unix]
test *args:
    RUST_MIN_STACK={{ rust_min_stack }} NEXTEST_PROFILE=local {{ justfile_directory() }}/scripts/setup-gstreamer.sh -- {{ justfile_directory() }}/scripts/download-rusty-v8.sh -- cargo nextest run --no-fail-fast "$@"

# Type-check with the same native V8 and GStreamer inputs used by `just test`.
[unix]
check *args:
    {{ justfile_directory() }}/scripts/setup-gstreamer.sh -- {{ justfile_directory() }}/scripts/download-rusty-v8.sh -- rtk cargo check "$@"

# Run explicit workspace benchmark targets.
bench *args:
    cargo bench --workspace --bench '*' {args}

# Run benchmark targets once to ensure they start successfully.
bench-smoke:
    just bench -- --test

# Regenerate all checked-in generated artifacts.
generate: write-config-schema write-app-server-schema write-hooks-schema

# Regenerate the json schema for config.toml from the current config types.
write-config-schema:
    cargo run -p genex-config-schema --bin genex-write-config-schema

# Regenerate app-server protocol schemas.
write-app-server-schema *args:
    {{ python }} scripts/write_app_server_schema.py {args}

# Generate JSON and HTML for workspace member crate dependencies.
crate-dependency-graph *args:
    {{ python }} scripts/generate_crate_dependency_graph.py {args}

[no-cd]
write-hooks-schema:
    cargo run --manifest-path {{ justfile_directory() }}/Cargo.toml -p genex-hooks --bin write_hooks_schema_fixtures

# Run argument-comment Dylint checks from source.
[no-cd]
argument-comment-lint *args:
    {{ python }} {{ justfile_directory() }}/tools/argument-comment-lint/run.py {args}

[no-cd]
argument-comment-lint-from-source *args:
    {{ python }} {{ justfile_directory() }}/tools/argument-comment-lint/run.py {args}

# Tail logs from the state SQLite database
[unix]
log *args:
    if [ "${1:-}" = "--" ]; then shift; fi; cargo run -p genex-cli --bin logs_client -- "$@"

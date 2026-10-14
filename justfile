set positional-arguments
export JUST_SHELL := justfile_directory() / "scripts/just-shell.py"
set shell := ["python3", "-c", 'import os, runpy; runpy.run_path(os.environ["JUST_SHELL"], run_name="__main__")']

rust_min_stack := "8388608" # 8 MiB
python := "python3"
cargo_runner := env_var_or_default("RUXEX_CARGO_RUNNER", "cargo")
rusty_v8_mirror := justfile_directory() / ".build-cache/rusty-v8"

# Display help
help:
    just -l

# Remove Cargo build outputs and Ruxex-managed rusty_v8 artifacts.
[unix]
clean:
    rm -rf -- {{ justfile_directory() }}/target {{ justfile_directory() }}/.build-cache/rusty-v8

# Download and verify the sandbox-enabled rusty_v8 artifacts used by Rust recipes.
[unix]
prepare-rusty-v8:
    {{ justfile_directory() }}/scripts/download-rusty-v8.sh

# Run the provider-owned sample host with the same rusty_v8 setup as `check`.
alias c := run
[unix]
run *args: prepare-rusty-v8
    RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} run -p ruxex-provider-host-example -- "$@"

# Analyze the configured production binaries with Hawk and save the JSON report.
[unix]
hawk: prepare-rusty-v8
    RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} hawk check --output-format json > {{ justfile_directory() }}/hawk.json

# Format the justfile and Rust sources.
fmt:
    @{{ python }} scripts/format.py

# Check formatting without modifying files.
fmt-check:
    @{{ python }} scripts/format.py --check

[unix]
fix *args: prepare-rusty-v8
    RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} clippy --fix --tests --allow-dirty "$@"

[unix]
clippy *args: prepare-rusty-v8
    RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} clippy --tests "$@"

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
test *args: prepare-rusty-v8
    RUST_MIN_STACK={{ rust_min_stack }} NEXTEST_PROFILE=local RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} nextest run --no-fail-fast "$@"

# Run the standard Cargo test harness when cargo-nextest is not available.
[unix]
test-cargo *args: prepare-rusty-v8
    RUST_MIN_STACK={{ rust_min_stack }} RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} test "$@"

# Build the workspace with the same native V8 input used by `check` and `test`.
[unix]
build *args: prepare-rusty-v8
    RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} build "$@"

# Type-check with the same native V8 input used by `just test`.
[unix]
check *args: prepare-rusty-v8
    RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} check "$@"

# Run explicit workspace benchmark targets.
[unix]
bench *args: prepare-rusty-v8
    RUSTY_V8_MIRROR={{ rusty_v8_mirror }} {{ cargo_runner }} bench --workspace --bench '*' "$@"

# Run benchmark targets once to ensure they start successfully.
[unix]
bench-smoke:
    just bench -- --test

# Regenerate all checked-in generated artifacts.
generate: write-config-schema write-app-server-schema write-app-server-schema-experimental write-hooks-schema

# Regenerate the json schema for config.toml from the current config types.
write-config-schema:
    cargo run -p ruxex-config-schema --bin ruxex-write-config-schema

# Regenerate app-server protocol schemas.
write-app-server-schema *args:
    {{ python }} scripts/write_app_server_schema.py {args}

write-app-server-schema-experimental:
    {{ python }} scripts/write_app_server_schema.py --experimental

[no-cd]
write-hooks-schema:
    cargo run --manifest-path {{ justfile_directory() }}/Cargo.toml -p ruxex-hooks --bin write_hooks_schema_fixtures

# Tail logs from the state SQLite database
[unix]
log *args:
    if [ "${1:-}" = "--" ]; then shift; fi; cargo run -p ruxex-cli --bin logs_client -- "$@"

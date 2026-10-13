## Installing & building

### System requirements

| Requirement                 | Details                                                         |
| --------------------------- | --------------------------------------------------------------- |
| Operating systems           | macOS 12+, Ubuntu 20.04+/Debian 10+ |
| Git (optional, recommended) | 2.23+ for built-in PR helpers                                   |
| RAM                         | 4-GB minimum (8-GB recommended)                                 |

### Build from source

Windows is not a supported host. Windows users should install and run Genex in
WSL2 using a supported Linux distribution.

```bash
# Clone the repository and navigate to the root of the Cargo workspace.
cd /path/to/your/checkout

# Install the Rust toolchain, if necessary.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup component add rustfmt
rustup component add clippy
# Install helper tools used by the workspace justfile:
cargo install --locked just
# Install nextest for the `just test` helper.
cargo install --locked cargo-nextest

# Build the provider-owned sample host.
cargo build -p genex-provider-host-example

# Launch the TUI with a sample prompt.
cargo run -p genex-provider-host-example -- "explain this codebase to me"

# After making changes, use the root justfile helpers (they run at the repository root):
just fmt
just fix -p <crate-you-touched>

# Run the relevant tests (project-specific is fastest), for example:
just test -p genex-tui
# `just test` runs the test suite via nextest:
just test
# Avoid `--all-features` for routine local runs because it increases build
# time and `target/` disk usage by compiling additional feature combinations.
```

## Tracing / verbose logging

Codex is written in Rust, so it honors the `RUST_LOG` environment variable to configure its logging behavior.

The TUI records diagnostics in bounded local stores by default. Set `log_dir` explicitly to enable a plaintext TUI log for a run:

```bash
genex-provider-host-example -c log_dir=./.genex-log
tail -F ./.genex-log/genex-tui.log
```

Provider-specific non-interactive command surfaces belong to the host application. A host selects its provider at compile time and invokes the provider-neutral library APIs directly.

See the Rust documentation on [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) for more information on the configuration options.

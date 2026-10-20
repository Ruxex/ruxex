# Rusty V8 setup

This document applies only to applications that enable JavaScript code
execution through `ruxex-code-mode-runtime`, `ruxex-code-mode-host`, or a
feature that includes them.

App Server, CLI/TUI, custom providers, tools, and MCP do not need Rusty V8
when code mode is disabled.

## Download the native artifacts

Ruxex `0.1.0` uses the sandbox-enabled Rusty V8 `150.4.0` artifacts. Download
the archive and generated binding for the application's target from the
[Rusty V8 release assets](https://github.com/openai/codex/releases/download/rusty-v8-v150.4.0/).
Verify the archive with the matching checksum file.

Place the files in a local mirror with this layout:

```text
~/.cache/ruxex/rusty-v8/
└── v150.4.0/
    ├── librusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.a.gz
    ├── src_binding_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.rs
    └── rusty_v8_ptrcomp_sandbox_release_x86_64-unknown-linux-gnu.sha256
```

Replace `x86_64-unknown-linux-gnu` with the application's target. The
supported targets are:

- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-musl`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

The archive and binding must come from the same sandbox-enabled release and
target. An ordinary non-sandbox Rusty V8 archive is not compatible with code
mode.

## Configure the build

Set `RUSTY_V8_MIRROR` to the directory containing `v150.4.0/` before building
your host:

```sh
export RUSTY_V8_MIRROR="$HOME/.cache/ruxex/rusty-v8"
cargo build
```

Library users do not need to clone the Ruxex repository or run `just` for this
setup.

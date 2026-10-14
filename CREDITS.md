# Third-party and fork provenance

Ruxex includes third-party Rust dependencies and currently uses three patched
repositories for terminal and WebSocket behavior. Their package names and
upstream license terms remain applicable. Ruxex must preserve each dependency's
license files and relevant upstream history when moving the active sources to
Ruxex-owned repositories.

## `crossterm`

- Upstream project: `https://github.com/crossterm-rs/crossterm`
- License at the pinned revision: MIT
- Current temporary source: `https://github.com/openai-oss-forks/crossterm`
- Current pinned revision: `45fecb9508105988f42fe6ff0441783ed3717f92`
- Ruxex migration target: `https://github.com/Ruxex/crossterm`

The pinned fork history contains terminal input/event and color-query changes.
The Ruxex repository must retain the upstream `LICENSE` and the patch commits;
the package remains named `crossterm` because that is the third-party package
identity consumed by Cargo.

## `tokio-tungstenite`

- Upstream project: `https://github.com/snapview/tokio-tungstenite`
- License at the pinned revision: MIT
- Current temporary source: `https://github.com/openai-oss-forks/tokio-tungstenite`
- Current pinned revision: `0e5b2d73aa18dd9f0a50ee9ff199d5aef7594186`
- Ruxex migration target: `https://github.com/Ruxex/tokio-tungstenite`

The pinned fork history contains opt-in proxy support, proxy integration
fixtures, and a dependency pin to the patched `tungstenite` repository. The
Ruxex copy must update that internal dependency URL to the Ruxex
`tungstenite-rs` repository without changing the package name or public API.

## `tungstenite`

- Upstream project: `https://github.com/snapview/tungstenite-rs`
- License at the pinned revision: MIT OR Apache-2.0
- Current temporary source: `https://github.com/openai-oss-forks/tungstenite-rs`
- Current pinned revision: `4fffad30fe373adbdcffab9545e9e9bf4f2fc19f`
- Ruxex migration target: `https://github.com/Ruxex/tungstenite-rs`

The pinned fork history contains environment-based proxy support and the
WASM32 deflate compatibility change. Both `LICENSE-MIT` and `LICENSE-APACHE`
from the upstream project must remain in the Ruxex copy.

## Migration policy

The active workspace declarations should stop using the temporary fork URLs
after the Ruxex repositories are created and verified. This file records
provenance; it does not authorize changing the external LLM endpoint contract
or rewriting third-party author history. Before changing `Cargo.toml` and
`Cargo.lock`, verify that each Ruxex repository contains the pinned revision,
the required license files, and the same package metadata.

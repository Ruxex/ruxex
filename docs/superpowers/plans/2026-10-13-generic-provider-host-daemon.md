# Generic provider host daemon

## Goal

Make the host binary the owner of the concrete provider type. The foreground
CLI starts the same executable in an AppServer daemon mode, and the daemon
constructs the provider with `P::new(...)`. No provider instance crosses the
process boundary; only startup configuration does.

## Boundary

1. `genex-app-server` exposes a provider-construction context and a
   `ModelProviderFactory` trait. Its generic transport entry point constructs
   `P` after loading configuration and before opening provider-scoped state.
2. The existing non-generic AppServer entry point remains available for the
   standalone AppServer binary and current tests during the transition.
3. The daemon launcher gains an explicit host executable/argument boundary so
   a host can launch its own binary in daemon mode instead of relying on the
   managed built-in executable.
4. CLI/library integration then supplies the concrete provider factory to both
   foreground and daemon startup paths. The default interactive flow remains
   daemon-first; `--no-daemon` remains in-process.

## Verification

- Add contract tests proving the generic AppServer entry point calls the
  factory and uses its descriptor for the provider home.
- Compile production and test targets with `just check` and
  `just check --tests` under the repository's memory-limited systemd scope.
- Run `just generate` only if checked-in generated artifacts are changed.

# ruxex-app-server-daemon

> `ruxex-app-server-daemon` is experimental and its lifecycle contract may
> change while the daemon lifecycle flow is still being developed.

`ruxex-app-server-daemon` provides the machine-readable lifecycle API used by
provider-owned hosts, desktop clients, and local app-server clients. It
supports Linux and macOS using Unix process and file-locking primitives.

There is no fixed-provider CLI command for this crate. A host selects its
`ModelProvider` and calls the Rust API; the host executable is re-entered with
serializable daemon arguments when a detached process is required.

Use `run_with_host_executable` for an already packaged host, or
`bootstrap_from_package_with_host_args` when the host package must first be
installed into the managed layout.

On success, every command writes exactly one JSON object to stdout. Consumers
should parse that JSON rather than relying on human-readable text. Lifecycle
responses report the resolved backend, socket path, local CLI version, and
running app-server version when applicable.

## Bootstrap flow

`bootstrap` can use any complete provider-owned host package. If no daemon package is installed,
it copies the invoking package into
`RUXEX_HOME/packages/app-server-daemon` and prints an installation message.
Existing daemon packages are reused, including legacy installations; a broken
selection is not silently replaced. A bare executable cannot supply a new
installation.

The daemon records its settings under `RUXEX_HOME/app-server-daemon/` and starts
app-server as a pidfile-backed detached process. The package contains the
executable and its runtime helpers; the user's CLI command and shell setup are
left unchanged.

Previously launched legacy daemons retain
`RUXEX_HOME/packages/standalone/current`, including its flat binary layout when
present. Starts and restarts continue using that location until the daemon is
explicitly stopped or a new package is bootstrapped.

## Lifecycle semantics

`start` is idempotent and returns after app-server is ready to answer the normal
JSON-RPC initialize handshake on the Unix control socket.

`restart` stops any managed daemon and starts it again.

`stop` sends a graceful termination request first, then force-terminates the
process after the configured grace window if it is still alive. The
`shutdownGraceSeconds` setting defaults to 60 and accepts an integer from 0
through 300.

All mutating lifecycle commands are serialized per `RUXEX_HOME`, so concurrent
operations do not race another in-flight lifecycle operation.

## State

The daemon stores its local state under `RUXEX_HOME/app-server-daemon/`:

- `settings.json` for lifecycle settings
- `app-server.pid` for the app-server process record
- `daemon.lock` for daemon-wide lifecycle serialization

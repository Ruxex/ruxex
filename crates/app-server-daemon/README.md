# genex-app-server-daemon

> `genex-app-server-daemon` is experimental and its lifecycle contract may
> change while the daemon lifecycle flow is still being developed.

`genex-app-server-daemon` backs the machine-readable `genex app-server`
lifecycle commands used by local and remote app-server clients.
It is intended for Genex instances launched over SSH, including fresh developer
machines that should expose app-server through the managed local socket.

## Platform support

The daemon supports Linux, macOS, and Windows using platform-specific process
and file-locking primitives. Windows startup requires a non-elevated terminal
whose host permits detached child processes.

Windows automatic attachment requires the canonical socket address to fit the
108-byte AF_UNIX limit (including its terminator). A short junction alias whose
resolved address exceeds that limit falls back to the embedded server. Use a
shorter `GENEX_HOME` to share the daemon; discovery does not trust a mutable alias.

Shared clients use the environment inherited when the daemon started. Opening a
new terminal or clearing variables there does not clear the running daemon's
environment; per-client environment isolation is not provided.
If an implicitly discovered daemon cannot initialize the connection, the TUI
starts an embedded local server instead. Explicit app-server `--remote`
endpoints remain authoritative and report connection failures.

## Commands

```sh
genex app-server daemon start
genex app-server daemon restart
genex app-server daemon update
genex app-server daemon stop
genex app-server daemon version
genex app-server daemon bootstrap
```

On success, every command writes exactly one JSON object to stdout. Consumers
should parse that JSON rather than relying on human-readable text. Lifecycle
responses report the resolved backend, socket path, local CLI version, and
running app-server version when applicable.

Eligible managed daemons check for updates after five minutes, then hourly by
default. Edit `GENEX_HOME/app-server-daemon/settings.json` to change this:

```json
{"shutdownGraceSeconds": 60,
 "updater": {"autoUpdateEnabled": false, "updateIntervalMinutes": 120}}
```

Positive minute intervals have no configured cap. `daemon restart` applies the
enabled state; the next updater wait reads a new interval. The preference does
not affect an explicit `daemon update`.

`daemon update` selects the latest stable release, even with automatic updates
disabled. It also returns pinned or local managed packages to production update
eligibility, preserving the automatic-update preference. Legacy installations
migrate to the dedicated root once the published installer and release support
migration. JSON reports `updated`, `noUpdate`, or `unsupported`, with installed
and running versions. A running daemon restarts, so active or queued work may be
interrupted; a stopped daemon stays stopped. Installer errors return nonzero.
The updater uses saved network settings; CLI `-c` overrides do not reach it.

For all managed app-server shutdowns, including explicit stop and restart and
updater-triggered restarts, `shutdownGraceSeconds` defaults to 60 and accepts
an integer from 0 through 300. Zero forces shutdown immediately after requesting
a graceful exit; the five-minute maximum bounds the wait even if a turn is still
running.

## Bootstrap flow

For a new Linux or macOS machine:

```sh
Set `CODEX_INSTALLER_URL` to a provider installer endpoint before enabling
standalone updates, then run the provider's documented installation command.
$HOME/.genex/packages/standalone/current/codex app-server daemon bootstrap
```

On Windows, use a non-elevated PowerShell terminal whose host allows breakaway:

```powershell
The installer endpoint is provider-specific; configure it through
`CODEX_INSTALLER_URL` rather than relying on a built-in hosted URL.
$genexHome = if ($env:GENEX_HOME) { $env:GENEX_HOME } else { Join-Path $HOME '.genex' }
& "$genexHome\packages\standalone\current\bin\codex.exe" app-server daemon bootstrap
```

`bootstrap` can use any complete CLI package. If no daemon package is installed,
it copies the invoking package into `GENEX_HOME/packages/app-server-daemon` and
prints an installation message without asking for confirmation. Existing daemon
packages are reused, including legacy installations; a broken selection is not
silently replaced. A bare executable cannot supply a new installation.

It records the daemon settings under `GENEX_HOME/app-server-daemon/`, starts app-server as a
pidfile-backed detached process. It launches a detached updater loop when
automatic updates are enabled, the installer selected the stable `latest`
channel, and the managed binary supports the updater command.

## Installation and update cases

New daemons use `GENEX_HOME/packages/app-server-daemon/current/bin/genex`
(`codex.exe` on Windows). The package contains the executable and its helpers.
Daemon-only installer updates leave the user's CLI command and shell setup alone.

Previously launched legacy daemons retain `GENEX_HOME/packages/standalone/current`,
including its flat binary layout when present. Starts and scheduled updates keep
using that location. An explicit production update prepares and validates a
compatible dedicated package before stopping the legacy updater and daemon,
selecting the new package, and restarting only a previously running daemon.
The old CLI package files and selection remain unchanged.

| Situation | What starts | Does this daemon fetch new binaries? | Does a running app-server eventually move to a newer binary on its own? |
| --- | --- | --- | --- |
| Latest-channel installer has run; `start` or `bootstrap` is used with automatic updates enabled | Managed binary and detached updater when supported | When supported, the platform's installer runs on the configured cadence. | When supported, the running server restarts with the new binary before the updater replaces itself. |
| Installer selected an explicit release; `bootstrap` is used | Managed binary only | No; the selected release stays pinned. | No; an explicit restart uses the selected binary. |
| Another tool updates the managed binary | A fresh start or explicit restart uses it; a running server is reused. | Yes, when a latest-channel updater is running, on the configured cadence. | An updater that was running through the change compares binary contents on its next successful installer pass and refreshes the server first. |

### Managed packages

For dedicated and retained legacy daemon installations:

- lifecycle commands use the selected daemon package, regardless of the invoking
  CLI version; they do not implicitly replace an existing package
- `bootstrap` is supported
- managed `start`, `restart`, and `bootstrap` ensure a single detached pid-backed
  updater loop only when automatic updates are enabled for a stable latest-channel
  release whose managed binary supports the updater command
- the installer records the latest-channel selection alongside `current`;
  selecting an explicit release clears it, even if that version is currently
  latest. The updater checks the selection again while holding the install lock
  so an in-flight update cannot override a new pin
- installs made before the installer recorded channel selections need one new
  `latest` installation to opt into automatic updates; until then the daemon
  continues to serve app-server without updating the selected release
- after a successful refresh, if app-server is running and the managed binary
  contents changed, the updater restarts app-server with that binary first and
  only then replaces its own process image
- the updater loop is not reboot-persistent; a managed start after reboot
  starts it again

### Out-of-band updates

This daemon does not watch arbitrary executable files for replacement. If some
other tool updates the managed binary path:

- an updater that was already running notices a changed managed
  binary on its next successful scheduled installer pass; if
  app-server is running, it refreshes app-server first and then refreshes itself
  once that replacement starts successfully
- if the updater was absent during a same-version binary replacement, a later
  managed start recovers it but cannot infer the running server's previous
  executable identity; use `genex app-server daemon restart` to refresh the server

## Lifecycle semantics

`start` is idempotent and returns after app-server is ready to answer the normal
JSON-RPC initialize handshake on the Unix control socket.

`restart` stops any managed daemon and starts it again.

`stop` sends a graceful termination request first, then force-terminates the
process after the configured grace window if it is still alive.

All mutating lifecycle commands are serialized per `GENEX_HOME`, so concurrent
operations do not race another in-flight lifecycle operation.

## State

The daemon stores its local state under `GENEX_HOME/app-server-daemon/`:

- `settings.json` for updater preferences
- `app-server.pid` for the app-server process record
- `app-server-updater.pid` for the pid-backed standalone updater loop
- `daemon.lock` for daemon-wide lifecycle serialization

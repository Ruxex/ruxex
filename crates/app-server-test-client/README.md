# App Server Test Client
Quickstart for running and hitting the standalone `genex-app-server` binary.

## Quickstart

Run from `<reporoot>/genex-rs`.

```bash
# 1) Build the standalone app-server binary
cargo build -p genex-app-server --bin genex-app-server

# 2) Start websocket app-server in background
cargo run -p genex-app-server-test-client -- \
  --app-server-bin ./target/debug/genex-app-server \
  serve --listen ws://127.0.0.1:4222 --kill

# 3) Call app-server (defaults to ws://127.0.0.1:4222)
cargo run -p genex-app-server-test-client -- model-list
```

`send-message` and `send-message-v2` handle `request_user_input` server requests interactively.
When Genex asks a question, choose a numbered option (or `o` for a free-form answer when offered)
and the client will send the response and continue streaming the same turn.

## Testing logout

`test-logout` initializes the app-server, sends an `account/logout` request, and waits for the
resulting `account/updated` notification. It uses the active `GENEX_HOME`, so point it at an
isolated directory when testing credential cleanup.

```bash
cargo run -p genex-app-server-test-client -- \
  --app-server-bin ./target/debug/genex-app-server \
  test-logout
```

## Watching Raw Inbound Traffic

Initialize a connection, then print every inbound JSON-RPC message until you stop it with
`Ctrl+C`:

```bash
cargo run -p genex-app-server-test-client -- watch
```

## Testing Thread Rejoin Behavior

Build and start an app server using commands above. The app-server log is written to `/tmp/genex-app-server-test-client/app-server.log`

### 1) Get a thread id

Create at least one thread, then list threads:

```bash
cargo run -p genex-app-server-test-client -- send-message-v2 "seed thread for rejoin test"
cargo run -p genex-app-server-test-client -- thread-list --limit 5
```

Copy a thread id from the `thread-list` output.

### 2) Rejoin while a turn is in progress (two terminals)

Terminal A:

```bash
cargo run --bin genex-app-server-test-client -- \
  resume-message-v2 <THREAD_ID> "respond with thorough docs on the rust core"
```

Terminal B (while Terminal A is still streaming):

```bash
cargo run --bin genex-app-server-test-client -- thread-resume <THREAD_ID>
```

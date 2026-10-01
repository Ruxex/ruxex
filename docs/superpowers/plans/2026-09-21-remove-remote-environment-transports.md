# リモート実行環境トランスポート削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Hosted環境登録・relayと、exec-serverへの直接WebSocket環境接続を削除し、app-serverのremote接続とローカル実行基盤を保持する。

**Architecture:** `EnvironmentManager`はlocal/stdio環境だけを扱い、`codex --remote`のapp-server clientは独立した機能として残す。Hosted registry、Noise/Direct rendezvous、forward、`CODEX_EXEC_SERVER_URL`、environment URLは公開面・protocol・依存グラフから取り除く。

**Tech Stack:** Rust, `genex-exec-server`, app-server protocol, CLI/TUI, Cargo workspace, `just check`, `just generate`。

**Spec:** `docs/superpowers/specs/2026-09-21-remove-remote-environment-transports-design.md`

## Global Constraints

- `codex --remote`、`RemoteAppServerClient`、app-server WebSocket/Unix socket transportは保持する。
- Process/Filesystem/HTTP capability、sandbox、capability roots、network policyは保持する。
- `environments.toml`のstdio `program`経路は保持する。
- Hosted remoteとexec-server環境の直接WebSocket経路は削除する。
- テストコマンドは実行せず、`just check`と必要時の`just generate`を使う。

## Review Focus

- app-server用`--remote`とexec-server用`--remote`を混同せず、前者だけ残ること。
- `EnvironmentManager`がlocal/stdioで初期化でき、remote URLなしで通常起動できること。
- Noise relayの暗号化・再接続コードを残骸として残さないこと。
- exec-serverのlocal/stdio RPCとsandboxを削除しないこと。
- `core-api`、CLI snapshot、schema、docsにHosted remote型が残らないこと。

### Task 1: producer/consumer監査とremote inventory更新

**Files:**
- Inspect: `crates/exec-server/src/environment.rs`, `environment_provider.rs`, `environment_toml.rs`, `client.rs`, `client_transport.rs`
- Inspect: `crates/exec-server/src/remote.rs`, `environment_registry.rs`, `noise_relay/`, `relay.rs`, `forward.rs`
- Inspect: `crates/cli/src/main.rs`, `crates/cli/src/exec_server_auth.rs`, `crates/core-api/src/lib.rs`
- Modify: `docs/pre-release-compatibility-inventory.md`

- [x] **Step 1: Hosted remote symbolsを分類する**

  `RemoteEnvironmentConfig`、`EnvironmentRegistry*`、`NoiseRendezvous*`、`ExecServerTransportParams::WebSocketUrl`、`CODEX_EXEC_SERVER_URL`、AWS remote argsのproduction producer/consumerを列挙する。

- [x] **Step 2: app-server remoteと区別する**

  `RemoteAppServerClient`、TUI/CLI `--remote`、`--remote-auth-token-env`を保持対象としてinventoryに記録する。

- [x] **Step 3: inventoryへremove/retain根拠を記録する**

  runtime sliceにHosted remote、直接WebSocket environment、保持するapp-server remote、local/stdio runtimeを別行で記録する。

### Task 2: Hosted registry、Noise/Direct、forward経路を削除する

**Files:**
- Delete: `crates/exec-server/src/remote.rs`
- Delete: `crates/exec-server/src/remote/`
- Delete: `crates/exec-server/src/environment_registry.rs`
- Delete: `crates/exec-server/src/noise_relay/`
- Delete: `crates/exec-server/src/relay.rs`
- Delete: `crates/exec-server/src/relay_proto.rs`
- Delete: `crates/exec-server/src/forward.rs`
- Modify: `crates/exec-server/src/lib.rs`
- Modify: `crates/exec-server/src/client_api.rs`, `client_transport.rs`, `client.rs`, `client_recovery.rs`, `client_refresh.rs`
- Modify/Delete: remote relay and registry tests

- [x] **Step 1: public exportsとtransport variantsを削除する**

  Noise bundle/provider、registry request/response、remote config/run functions、`NoiseRendezvous` transport variantをre-exportと内部型から削除する。

- [x] **Step 2: reconnect/error/telemetry分岐を整理する**

  NoiseRendezvous専用のreconnect strategy、registry error、remote registration telemetryを削除し、local/stdio clientのrecoveryとtelemetryを保持する。

- [x] **Step 3: relay protocolとremote-only testsを削除する**

  relay専用integration/unit tests、forward tests、registry testsを削除する。Process/Filesystem RPCの一般テストは残す。

### Task 3: 直接WebSocket環境接続と設定を削除する

**Files:**
- Modify: `crates/exec-server/src/environment.rs`
- Modify: `crates/exec-server/src/environment_provider.rs`
- Modify: `crates/exec-server/src/environment_toml.rs`
- Delete: `crates/exec-server/src/environment/connect_options.rs`
- Modify: `crates/exec-server/src/client_api.rs`, `client_transport.rs`, `client.rs`
- Modify: affected environment config tests

- [x] **Step 1: URL environment providerを削除する**

  `CODEX_EXEC_SERVER_URL`、environment `url` parsing、`RemoteEnvironmentOptions`、`WebSocketUrl` transportを削除し、`program` stdioとlocal environmentだけを残す。

- [x] **Step 2: WebSocket client branchを削除する**

  remote exec-server WebSocketの接続、認証header、reconnect branchを削除し、stdio/local clientの初期化とrecoveryを保持する。

- [x] **Step 3: standalone exec-server listen面を監査する**

  `codex exec-server --listen ws://...`のproduction consumerを確認する。環境接続以外のconsumerがない場合はWebSocket listenを削除し、stdio listenとlocal server RPCだけを残す。

### Task 4: CLI、依存、protocol re-exportを整理する

**Files:**
- Modify: `crates/cli/src/main.rs`
- Delete: `crates/cli/src/exec_server_auth.rs` when its only consumer is removed
- Modify: `crates/cli/Cargo.toml`, `crates/exec-server/Cargo.toml`
- Modify: `crates/core-api/src/lib.rs`
- Modify: CLI argument tests and checked-in snapshots

- [x] **Step 1: exec-server remote argumentsを削除する**

  `--remote`、`--remote-transport`、`--environment-id`、`--aws-sigv4`、AWS options、`forward` subcommand、remote起動分岐を削除する。TUI/app-serverの同名引数は残す。

- [x] **Step 2: unused dependenciesを削除する**

  `genex-aws-auth`などremote登録専用依存のworkspace reverse dependencyを確認し、他のconsumerがなければ削除する。

- [x] **Step 3: core-apiとsnapshotsを更新する**

  Hosted registry/Noise型のre-exportとexec-server help snapshotを削除し、app-server protocolの保持対象には触れない。

### Task 5: docs、生成物、checkを更新する

**Files:**
- Modify: `docs/provider-decoupling-plan.md`
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `crates/exec-server/README.md`
- Modify: generated artifacts discovered by `rg`

- [x] **Step 1: unsupported remote usageをdocsから削除する**

  Hosted registration、direct/noise transport、URL environment、forwardの説明を削除し、app-server `--remote`の説明は残す。

- [x] **Step 2: generated artifactsを更新する**（生成対象差分なし。`just generate` はテストを呼び出すため未実行）

  protocol/schema/CLI生成物に差分がある場合は`just generate`を実行する。

- [x] **Step 3: repository checkを実行する**

  `just check`を実行し、`rg`で削除対象のproduction symbolが残っていないことを確認する。テストコマンドは実行しない。

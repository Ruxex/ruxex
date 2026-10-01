# リモート実行環境トランスポート削除 設計

## 目的

provider Hostedの環境レジストリ経由のリモート実行環境と、既知のexec-serverへ直接接続するWebSocket環境経路を削除する。ローカル実行、stdioで起動するローカル補助プロセス、exec-serverのProcess/Filesystem RPC本体、app-server自体への接続機能は保持する。

## 背景

このリポジトリの「remote」には複数の独立した機能がある。

- `codex --remote` はTUI/CLIを別ホストのapp-serverへ接続するapp-server client機能。
- `CODEX_EXEC_SERVER_URL` と `environments.toml` の `url` は、app-serverから別のexec-serverへ接続するリモート実行環境機能。
- `codex exec-server --remote` は、exec-serverをHosted環境レジストリへ登録し、NoiseまたはDirect rendezvousでharnessへ接続する機能。

今回削除するのは後二者であり、app-server clientのremote transportは対象外とする。

## 設計方針

### 削除するもの

1. Hosted環境レジストリ・rendezvous経路
   - `EnvironmentRegistryClient`
   - `/cloud/environment/{id}/register`、`connect`、`validate` およびDirect登録API
   - `RemoteEnvironmentConfig`、`RemoteEnvironmentTransport`
   - Noise rendezvous、relay、forwarder、登録リトライ
   - ChatGPT account ID、remote exec-server向けChatGPT/API key認証、AWS SigV4登録経路
   - `codex exec-server --remote`、`--remote-transport`、`--environment-id`、`--aws-sigv4`、`forward`

2. 直接WebSocketによるexec-server環境接続
   - `CODEX_EXEC_SERVER_URL`
   - `environments.toml` の環境 `url`
   - `ExecServerTransportParams::WebSocketUrl`
   - `RemoteEnvironmentOptions`
   - WebSocket専用の環境manager/client接続とその再接続処理

3. 上記のみに使われる公開API、環境ID、エラー、テレメトリ、CLIヘルプ、スナップショット、テスト、README、生成物。

### 保持するもの

- app-serverそのものへ接続する `codex --remote`、`RemoteAppServerClient`、WebSocket/Unix socket transport。
- `EnvironmentManager` のローカル環境処理。
- `environments.toml` の `program` によるstdio起動経路は、ローカル補助プロセス経路として保持する。
- exec-serverのProcess/Filesystem/HTTP capability、sandbox、capability roots、security checks。
- exec-server protocolと、保持対象のstdio/local clientが必要とするRPC実装。
- provider-neutralな環境選択・環境情報の型。ただし削除したWebSocket/Hosted transport専用のフィールドは除く。

## 実行後の構成

```text
TUI/CLI ── app-server --remote ──> 別ホストのapp-server       (保持)

app-server ── local EnvironmentManager                    (保持)
           └─ stdioで起動したローカル補助exec-server         (利用箇所がある場合保持)

exec-server --remote / WebSocket環境 / Hosted relay          (削除)
```

## 主要な変更境界

### exec-server

`crates/exec-server/src/remote.rs`、`environment_registry.rs`、`noise_relay/`、`relay.rs`、`relay_proto.rs`、`forward.rs`、およびremote専用の`client_api`/reconnect分岐をproducer/consumer監査後に削除する。`client.rs`、`client_transport.rs`、`environment.rs`は、保持するstdio/local経路に必要な範囲だけ残す。

`environment_provider.rs` と `environment_toml.rs` は、WebSocket URLを受け付けない。stdio `program` エントリを保持する場合は、設定モデルをその最小形にする。

### CLI

`crates/cli/src/main.rs` のexec-server専用remote引数と起動分岐を削除する。app-server/TUI用のトップレベル `--remote` と `--remote-auth-token-env` は残すため、同名引数を一括削除しない。

`crates/cli/src/exec_server_auth.rs` とAWS認証依存は、他の本番利用がないことを確認してから削除する。exec-server help snapshotと引数テストは新しいCLI面に合わせて更新する。

### protocol / core-api / config

`core-api`からHosted環境レジストリ型とNoise接続型のre-exportを削除する。app-server protocolの環境選択・環境情報は、ローカル環境や保持するstdio経路が利用する範囲を残す。

`CODEX_EXEC_SERVER_URL`、`environments.toml`のURL設定、remote専用設定・環境変数を設定スキーマとドキュメントから削除する。既存設定を読む互換処理はpre-release forkのため追加しない。

### ドキュメントと棚卸し

`docs/provider-decoupling-plan.md` にHosted remote environmentと直接WebSocket environment transportの完了を反映する。`docs/pre-release-compatibility-inventory.md` には、producer/consumer、`remove`判断、保持したapp-server remoteとlocal/stdio runtimeの根拠を記録する。

## 検証方針

- 本プロジェクト指示に従い、テストコマンドは実行しない。
- 各削除段階で `cargo check` 相当のコンパイル確認ではなく、最終的にリポジトリ標準の `just check` を実行する。
- CLIやprotocolの生成物を変更した場合だけ `just generate` を実行し、生成差分を確認する。
- `rg` とCargo依存グラフで、Hosted remote型・URL接続型のproduction consumerが残っていないこと、app-server `--remote` とlocal/stdio実行が残っていることを確認する。

## 非目標

- app-serverへのremote接続機能の削除。
- Process/Filesystem RPC、sandbox、capability root、network policyの削除。
- ローカル実行機能やprovider-neutralなモデル・MCP機能の変更。

# ruxex-app-server-mock

`ruxex-app-server-mock` は、Ruxex の E2E テストで子プロセスとして起動するための
決定的な AppServer モックです。production 用の AppServer ではなく、外部依存や
認証なしで JSON-RPC の往復を検証するために使います。

## Binary

binary は newline-delimited JSON を stdin から読み、request に対する JSON-RPC
response だけを stdout に一行ずつ書きます。stdout は protocol 専用で、診断は
stderr に出ます。

```sh
printf '%s\n' '{"id":1,"method":"thread/start"}' \
  | ruxex-app-server-mock
```

未設定のメソッドには、次のような決定的な応答を返します。

```json
{"id":1,"result":{"mock":true,"method":"thread/start"}}
```

`initialize` には E2E クライアントが検証できる mock identity と platform 情報を
含む payload を返します。notification と受信済み response/error には応答しません。

## Scenario and isolated home

子プロセスから typed response fixture を読み込む場合は、scenario JSON を渡します。

```json
{
  "responses": {
    "thread/start": {"threadId": "e2e-thread"},
    "account/read": {"account": null}
  }
}
```

```sh
ruxex-app-server-mock \
  --scenario ./e2e/scenario.json \
  --ruxex-home ./tmp/e2e-home
```

`RUXEX_MOCK_SCENARIO`、`RUXEX_MOCK_HOME`、`RUXEX_MOCK_LISTEN` でも指定でき、
`RUXEX_HOME` は mock home のフォールバックとして利用されます。

## WebSocket mode

Tauri template の既存 WebSocket 接続経路を使う場合は `--listen` を指定します。
stdio と同じ protocol を WebSocket text frame で提供します。

```sh
ruxex-app-server-mock \
  --listen 127.0.0.1:43123 \
  --scenario ./e2e/scenario.json
```

Tauri 側では `ws://127.0.0.1:43123` を WebSocket endpoint として指定できます。
mock binary は production runtime package へ自動で混ぜず、E2E 用の build artifact
として配置してください。

## Library

同一 package の library API では、method 単位の fixture を設定できます。

```rust
use ruxex_app_server_mock::MockAppServer;
use serde_json::json;

let server = MockAppServer::builder()
    .with_response("thread/start", json!({"threadId": "e2e-thread"}))
    .build()?;
```

`MockAppServer::handle_message` は I/O なしのテストに使え、`run_stdio` は E2E の
子プロセス入口として使えます。tool 実行、streaming、approval、persistent thread
などの振る舞いは、現在のモックの対象外です。

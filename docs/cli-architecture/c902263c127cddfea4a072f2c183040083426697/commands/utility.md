# ユーティリティと内部コマンド

## `completion`

`codex completion [bash|zsh|fish|...]` はルートの clap コマンドを使ってシェル補完スクリプトを生成します。ローカルかつステートレスで、リモートモードを拒否します。パーサーは [`CompletionCommand`](../../../../crates/cli/src/main.rs#L197) で、[main.rs#L1420](../../../../crates/cli/src/main.rs#L1420) でディスパッチされます。

## `features`

`codex features list|enable|disable` は機能設定を読み込むか編集します。`list` は有効な設定を解決し、機能キー、段階、有効状態を出力します。enable/disable は永続設定を編集します。enum は [`main.rs#L897`](../../../../crates/cli/src/main.rs#L897) 付近にあり、ディスパッチは [L1575](../../../../crates/cli/src/main.rs#L1575) にあります。[機能フラグ](../common/feature-flags.md)も参照してください。

## `exec-server`

実験的な `codex exec-server` サービスは stdio 経由で実行機能を公開し、リクエストの並行性は `RequestDispatchMode` が制御します。CLI ラッパーは [`ExecServerCommand`](../../../../crates/cli/src/main.rs#L525) で、ランタイムの担当は `genex-exec-server` と app-server の実行サポートにあります。[トランスポート/サーバー](../common/transport-and-server.md) と [テレメトリ](../common/telemetry.md)も参照してください。

## 内部トランスポートヘルパー

- `stdio-to-uds` は `genex-stdio-to-uds` を使い、stdio のバイト列を Unix ドメインソケットに中継します。
- 非表示の `tcp-tunnel` は HTTP/3 CONNECT プロキシ経由でローカル TCP ソケットを転送します。

これらはトランスポートの配管であり、汎用エージェントコマンドではありません。どちらも `cli_main` でディスパッチされ、ローカルでの担当範囲が曖昧になるリモートモードの組み合わせを拒否します。

## 内部スキーマ/トレースユーティリティ

app-server のスキーマ生成とデバッグトレースの縮約は、通常のエンドユーザー向けランタイムモードではなく、生成されたプロトコル/状態の成果物を公開する機能であるため、それぞれのコマンドページに記載しています。

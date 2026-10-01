# `codex app-server`

## エントリモード

`AppServerCommand` は [`cli/src/main.rs`](../../../../crates/cli/src/main.rs#L473) で定義されています。ネストしたサブコマンドがない場合、`codex app-server` は app-server ランタイムを起動します。ネストした操作は次のとおりです。

- `daemon bootstrap|start|restart|update|stop|version`。
- `proxy`。
- `generate-ts`。
- `generate-json-schema`。
- 非表示の `generate-internal-json-schema`。

ルートのディスパッチは、ランタイムを起動するか、[main.rs#L1119](../../../../crates/cli/src/main.rs#L1119) で選択されたツールを呼び出します。

## フォアグラウンドランタイム

CLI は `--listen`/`--stdio`、strict-config、メトリクスの既定値、認証、code-mode-host のオプションを [`app-server/src/lib.rs#L417`](../../../../crates/app-server/src/lib.rs#L417) の `codex_app_server::run_main` に変換します。トランスポート対応のエントリポイントは [L473](../../../../crates/app-server/src/lib.rs#L473) の `run_main_with_transport_options` です。設定を読み込み、ランタイムパスを作成し、リクエスト/通知チャネルを初期化して、リクエスト処理を開始します。

CLI で対応するトランスポートの形は `stdio://`、Unix ソケット、WebSocket URL、`off` です。ただし、コマンドのプラットフォーム/ランタイム検証に従います。

## デーモン/プロキシ/スキーマツール

- デーモンコマンドは永続的なローカル app-server パッケージを管理し、作業を再起動または中断する場合があります。
- `proxy` は stdio のバイト列を制御用 Unix ソケットへ転送するだけで、プロトコルの意味自体は処理しません。
- スキーマ生成は `genex-app-server-protocol` のジェネレーターを呼び出し、TypeScript または JSON Schema の成果物を書き込みます。
- 内部スキーマ生成はツール向けであり、通常のヘルプには表示されません。

[トランスポート/サーバー](../common/transport-and-server.md)、[プロトコルの担当](../reference/crate-index.md)、[設定](../common/configuration.md)、[テレメトリ](../common/telemetry.md)も参照してください。

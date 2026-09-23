# トランスポートとサーバーの境界

## ローカルとリモートの形

CLI は次の経路でランタイム動作に到達できます。

- TUI/Core の直接起動。
- ローカル app-server デーモン。
- 明示的なリモート app-server エンドポイント。
- app-server の stdio、Unix ソケット、または WebSocket トランスポート。
- exec-server または内部 stdio-to-UDS 配管。

`codex-cli` はコマンドとエンドポイントを決めます。`codex-app-server` はリクエスト処理を、`codex-app-server-transport` は接続/トランスポートの仕組みを、`codex-app-server-protocol` はシリアライズ可能なプロトコル型を管理します。[app-server コマンド](../commands/app-server.md)も参照してください。

## プロトコル境界

プロトコルのリクエスト/通知は型付きの値です。トランスポートはバイト列/メッセージを移動し、サーバーのプロセッサーはリクエストを Core/セッション操作に変換します。プロトコル型があるクレートからランタイムの担当範囲を推測してはいけません。

## ライフサイクル

デーモンのライフサイクルコマンドは、管理対象パッケージのインストール、起動、再起動、停止、更新、照会を行えます。プロキシコマンドは既存の制御ソケットにバイト列を転送します。これらは運用経路であり、実行中の作業に影響する場合があります。単一のフォアグラウンドリクエストの接続ライフタイムとは異なります。

関連する Trait には、クライアント観測用の [`RequestTelemetry`](../reference/trait-index.md#requesttelemetry) と TUI 入力用の [`EventSource`](../reference/trait-index.md#eventsource) があります。一方、多くのサーバー境界は公開 Trait ではなく、具体的なリクエスト/プロセッサー型です。

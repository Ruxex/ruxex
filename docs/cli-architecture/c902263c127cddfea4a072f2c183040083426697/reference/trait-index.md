# Trait 索引

この索引には、CLI の機能境界または共有ランタイム契約を説明する Trait を掲載します。可視性は定義宣言に基づいて記録しており、公開 re-export が定義元のクレートを変えることはありません。

## API とプロバイダーの境界

### `AuthProvider`

- 定義: `codex-api`、[`src/auth.rs:30`](../../../../crates/codex-api/src/auth.rs#L30)
- 可視性: `pub`
- 役割: API クライアントが利用する、非同期のリクエスト時認証ヘッダープロバイダー。
- 関連する利用側: `codex-login`、`codex-model-provider`、`codex-api` のエンドポイント。

### `ModelProvider`

- 定義: `codex-model-provider`、[`src/provider.rs:123`](../../../../crates/model-provider/src/provider.rs#L123)
- 可視性: `pub`
- 役割: モデル選択と Core が利用する、プロバイダーの機能/アカウント状態/復旧の境界。

### `RequestTelemetry`

- 定義: `codex-client`、[`src/telemetry.rs:6`](../../../../crates/codex-client/src/telemetry.rs#L6)
- 可視性: `pub`
- 役割: リクエストごとのトランスポート観測フック。

### `SseTelemetry`

- 定義: `codex-api`、[`src/telemetry.rs:18`](../../../../crates/codex-api/src/telemetry.rs#L18)
- 可視性: `pub`
- 役割: SSE ポーリングの観測フック。

### `WebsocketTelemetry`

- 定義: `codex-api`、[`src/telemetry.rs:35`](../../../../crates/codex-api/src/telemetry.rs#L35)
- 可視性: `pub`
- 役割: WebSocket のリクエスト/イベント観測フック。

## 設定と永続化

### `ThreadConfigLoader`

- 定義: `codex-config`、[`src/thread_config.rs:86`](../../../../crates/config/src/thread_config.rs#L86)
- 可視性: `pub`
- 役割: app-server/Core を具体的な設定ソースから独立させる、スレッド単位の設定読み込み境界。

### `ThreadStore`

- 定義: `codex-thread-store`、[`src/store.rs:88`](../../../../crates/thread-store/src/store.rs#L88)
- 可視性: `pub`
- 役割: セッションのライフサイクル経路が利用する、永続化されたスレッドメタデータ/履歴/ストアの境界。

### `QueueStore`

- 定義: `codex-thread-store`、[`src/queue_store.rs:14`](../../../../crates/thread-store/src/queue_store.rs#L14)
- 可視性: `pub`
- 役割: 既存セッション向けにキューへ入れられたメッセージ。

### `SessionTask`

- 定義: `codex-core`、[`src/tasks/mod.rs:176`](../../../../crates/core/src/tasks/mod.rs#L176)
- 可視性: `pub(crate)`
- 役割: バックグラウンドの Tokio タスクにスケジュールされたセッション作業の内部ライフサイクル契約。

## ツールとポリシーの境界

### `ToolExecutor`

- 定義: `codex-tools`、[`src/tool_executor.rs:106`](../../../../crates/tools/src/tool_executor.rs#L106)
- 可視性: `pub`
- 役割: 汎用的な呼び出しからツール出力へのハンドラー境界。

### `CoreToolRuntime`

- 定義: `codex-core`、[`src/tools/registry.rs:54`](../../../../crates/core/src/tools/registry.rs#L54)
- 可視性: `pub(crate)`
- 役割: `ToolExecutor` 実装に対する Core レジストリのランタイムビュー。

### `Approvable`

- 定義: `codex-core`、[`src/tools/sandboxing.rs:309`](../../../../crates/core/src/tools/sandboxing.rs#L309)
- 可視性: `pub(crate)`
- 役割: 承認判断/レビューキーに関するツールランタイム契約。

### `Sandboxable`

- 定義: `codex-core`、[`src/tools/sandboxing.rs:343`](../../../../crates/core/src/tools/sandboxing.rs#L343)
- 可視性: `pub(crate)`
- 役割: サンドボックスの優先設定/構成に関するツールランタイム契約。

### `ToolRuntime`

- 定義: `codex-core`、[`src/tools/sandboxing.rs:364`](../../../../crates/core/src/tools/sandboxing.rs#L364)
- 可視性: `pub(crate)`
- 役割: 具体的なツールランタイム向けに承認契約とサンドボックス契約を組み合わせるもの。

### `Rule`

- 定義: `codex-execpolicy`、[`src/rule.rs:214`](../../../../crates/execpolicy/src/rule.rs#L214)
- 可視性: `pub`
- 役割: ポリシー評価が利用する実行可能ルールのマッチング抽象化。

## UI とコマンド出力の境界

### `EventSource`

- 定義: `codex-tui`、[`src/tui/event_stream.rs:45`](../../../../crates/tui/src/tui/event_stream.rs#L45)
- 可視性: `pub`
- 役割: TUI イベントブローカーが利用するターミナル/イベントソース抽象化。

### `EventProcessor`

- 定義: `codex-exec`、[`src/event_processor.rs:13`](../../../../crates/exec/src/event_processor.rs#L13)
- 可視性: `pub(crate)`
- 役割: 応答イベント/ステータスを非対話出力の動作へ変換する内部処理。

### `ToolOutput`

- 定義: `codex-tools`、[`src/tool_output.rs:12`](../../../../crates/tools/src/tool_output.rs#L12)
- 可視性: `pub`
- 役割: ツールの結果/出力の境界。

### `TurnItemEmitter`

- 定義: `codex-tools`、[`src/tool_call.rs:56`](../../../../crates/tools/src/tool_call.rs#L56)
- 可視性: `pub`
- 役割: ツール/ターン項目を会話/イベント処理へ戻すもの。

## プロトコルと認証の統合

### `ExperimentalApi`

- 定義: `codex-app-server-protocol`、[`src/experimental_api.rs:5`](../../../../crates/app-server-protocol/src/experimental_api.rs#L5)
- 可視性: `pub`
- 役割: スキーマ/API 生成時に実験的な利用を示すプロトコル型/メソッドのマーカー。

### `AuthManagerConfig`

- 定義: `codex-login`、[`src/auth/manager.rs:1223`](../../../../crates/login/src/auth/manager.rs#L1223)
- 可視性: `pub`
- 役割: login マネージャーの設定アクセスを `codex-core::Config` から独立させるもの。

## 除外

`Renderable` のような描画専用 Trait と、関係のないテスト専用ヘルパーは CLI の機能境界を説明しないため掲載していません。これらは有効な内部インターフェースとして残ります。ここでの除外は文書の範囲を決めるものであり、存在しないという主張ではありません。

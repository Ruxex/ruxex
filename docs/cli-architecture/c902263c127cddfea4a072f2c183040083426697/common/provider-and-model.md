# プロバイダーとモデルの選択

## フロー

```text
Config + AuthManager
  -> モデルプロバイダー設定
  -> codex-model-provider::create_model_provider
  -> ModelProvider の機能オブジェクト
  -> codex-api の Provider / クライアント
  -> Responses またはプロバイダー固有のストリーム
```

[`ModelProvider`](../reference/trait-index.md#modelprovider) は `codex-model-provider` で定義され、リクエスト/ストリーム型とトランスポート向けクライアントは `codex-api` と `codex-client` にあります。プロバイダーオブジェクトは機能、アカウント状態、復旧動作、リクエスト構築の判断を保持し、Core はターン中にその結果として得られるプロバイダー抽象化を利用します。

モデルカタログ/デバッグ経路とランタイムのモデル選択は関連していますが、同一ではありません。`codex debug models` はカタログデータを検査し、ターンは設定されたプロバイダー/モデル経路を作成します。

## プロバイダー非依存のルール

共通のエージェントフローでは、この境界を「モデルプロバイダー」と「応答ストリーム」として説明すべきです。プロバイダー固有の認証、URL/ヘッダー構築、再試行の詳細はこの下位に置きます。これにより、別のプロバイダー実装を選んだ場合も `codex-core` を再利用できます。

## 関連する Trait

- [`ModelProvider`](../reference/trait-index.md#modelprovider): プロバイダーの機能とライフサイクルに関係する動作。
- [`AuthProvider`](../reference/trait-index.md#authprovider): リクエスト時の認証ヘッダー。
- [`SseTelemetry`](../reference/trait-index.md#ssetelemetry) / `WebsocketTelemetry`: トランスポートの観測。

# CLI 向けクレート索引

| グループ | クレート/パッケージ | 担当 |
| --- | --- | --- |
| エントリポイント | `codex-cli` | ルートの clap 文法、ディスパッチ、コマンド固有の CLI ヘルパー |
| 対話 UI | `codex-tui` | TUI 引数、イベントループ、セッション起動、描画、対話出力 |
| 非対話 UI | `codex-exec` | exec/review 引数、リクエストループ、イベント処理、JSONL/最終出力 |
| エージェントランタイム | `codex-core` | 設定ランタイム、スレッド、ターン、ツール、承認、ポリシー統合 |
| 共有 CLI 引数 | `codex-utils-cli` | 共有オプション構造体と上書きの解析 |
| 設定モデル | `codex-config` | 設定層/型、診断、スレッド設定インターフェース |
| 認証 | `codex-login`、`codex-keyring-store` | 資格情報の読み込み、login/logout、更新、保存 |
| プロバイダー/モデル | `codex-model-provider`、`codex-models-manager` | プロバイダーの機能/設定とモデルカタログ |
| API 境界 | `codex-api`、`codex-client` | リクエスト/応答ストリーム、プロバイダーエンドポイント、再試行、トランスポートテレメトリ |
| プロトコル | `codex-protocol`、`codex-app-server-protocol` | ドメイン型とワイヤー型、スキーマ生成、実験的 API マーカー |
| サーバー | `codex-app-server`、`codex-app-server-transport`、`codex-app-server-client` | リクエスト処理、トランスポート、クライアント/サーバーのライフサイクル |
| ツール | `codex-tools` | 汎用ツール呼び出し/出力の Trait と型 |
| ポリシー/サンドボックス | `codex-execpolicy`、`codex-sandboxing`、プラットフォーム用サンドボックスクレート | ルール、承認/サンドボックスのプリミティブ、ホスト側の適用 |
| 永続化 | `codex-thread-store`、`codex-rollout`、`codex-history`、`codex-state` | スレッド/キューストア、ロールアウト/履歴、ローカル状態 |
| 拡張性 | `codex-mcp`、`codex-rmcp-client`、`codex-plugin`、`codex-core-plugins` | MCP とプラグインの設定/ランタイム統合 |
| プラットフォーム/プロセス | `codex-arg0`、`codex-install-context`、`codex-terminal-detection`、`codex-process-hardening` | 起動コンテキスト、インストール、ターミナル、プロセス安全性 |

パッケージ名と Rust クレート名は異なる場合があります。この対応関係については Cargo マニフェストが正式な情報源です。ランタイムの担当範囲は [システムマップ](../00-system-map.md) を、インターフェースレベルの担当範囲は [Trait 索引](trait-index.md) を参照してください。

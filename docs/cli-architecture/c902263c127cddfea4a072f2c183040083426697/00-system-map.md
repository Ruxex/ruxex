# システムマップ

## 正規化したパイプライン

```text
argv
  -> clap による解析とルート上書きの統合
  -> CODEX_HOME/プロファイル/設定の解決
  -> 認証とプロバイダー/モデルの選択
  -> ローカル、リモート、app-server、または直接トランスポートの選択
  -> スレッド/セッションの受理
  -> ターン、コマンド、ツールの実行
  -> 承認、execpolicy、サンドボックス、ユーザー検証
  -> ロールアウト/履歴/スレッドの永続化とテレメトリ
  -> TUI、JSONL、プロトコル、またはプロセス終了時の出力
```

これは概念上の正規化であり、単一の関数呼び出しを表すものではありません。コマンドによって段階を省略したり、順序を入れ替えたりします。たとえば `genex completion` は CLI 解析後に終了しますが、`genex exec` は app-server リクエストを利用でき、`genex` の対話モードはローカルデーモン経由で実行できます。

## 段階ごとの担当

| 段階 | 主な担当 | 担当するもの |
| --- | --- | --- |
| 解析とディスパッチ | [`genex-cli`](../../../crates/cli/src/main.rs) | `MultitoolCli`、`Subcommand`、ルート検証、ディスパッチ |
| 対話用引数モデル | [`codex-tui`](../../../crates/tui/src/cli.rs) | TUI 固有のフラグと、resume/fork/agents 用のラッパー状態 |
| 非対話用引数モデル | [`genex-exec`](../../../crates/exec/src/cli.rs) | `genex exec` のオプションと exec サブコマンド |
| 設定 | [`genex-core`](../../../crates/core/src/config/mod.rs) と [`genex-config`](../../../crates/config/src/lib.rs) | 読み込み、上書き、プロファイル、検証、実行時設定 |
| エージェントのオーケストレーション | [`genex-core`](../../../crates/core/src/lib.rs) | スレッド、ターン、ツール、承認、モデルリクエスト、共有状態 |
| UI 表示 | [`codex-tui`](../../../crates/tui/src/lib.rs) | ターミナルのイベントループ、履歴セル、入力、レンダリング |
| App-server の実行時処理 | [`genex-app-server`](../../../crates/app-server/src/lib.rs) | リクエスト処理、トランスポート、デーモン向けランタイム、ライフサイクル |
| 公開プロトコル | [`genex-app-server-protocol`](../../../crates/app-server-protocol/src/lib.rs) と [`genex-protocol`](../../../crates/protocol/src/lib.rs) | ワイヤー型、通知、リクエスト、共有ドメイン型 |
| プロバイダー/API 境界 | [`genex-api`](../../../crates/genex-api/src/lib.rs)、[`genex-client`](../../../crates/genex-client/src/lib.rs) | プロバイダーへのリクエスト、ストリーミング、再試行、認証ヘッダー、トランスポート |
| 実行/ポリシー | [`genex-tools`](../../../crates/tools/src/lib.rs)、[`genex-execpolicy`](../../../crates/execpolicy/src/lib.rs)、[`genex-sandboxing`](../../../crates/sandboxing/src/lib.rs) | ツールレジストリ、コマンドポリシー、ホストのサンドボックス構築 |
| 永続化 | [`genex-rollout`](../../../crates/rollout/src/lib.rs)、[`genex-thread-store`](../../../crates/thread-store/src/lib.rs)、[`genex-history`](../../../crates/history/src/lib.rs) | ロールアウトファイル、スレッドメタデータ、履歴、状態ストア |

## 重要な境界

### CLI とランタイム

`genex-cli` はユーザー向けのコマンド文法を管理し、経路を選択します。コマンドの動作全体を所有するわけではありません。ランタイム処理は `genex-tui`、`genex-exec`、`genex-core`、app-server、またはユーティリティクレートに委譲されます。

### プロトコルとサーバー

`genex-app-server-protocol` と `genex-protocol` は、シリアライズ可能な形とドメイン値を定義します。`genex-app-server` はリクエストを処理し、状態を調整します。プロトコル型がプロトコルクレートで定義されているからといって、そのクレートが動作を所有するとは限りません。

### プロバイダー境界とエージェントの動作

`genex-api` と `genex-client` は、リクエスト/ストリーム/トランスポートの境界を提供します。モデル応答をいつ要求するか、イベントをどう解釈するか、ツールや承認をいつ関与させるかは Core が決定します。この分離が、リポジトリの他の部分で利用するプロバイダー非依存の継ぎ目です。

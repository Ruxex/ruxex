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

これは概念上の正規化であり、単一の関数呼び出しを表すものではありません。コマンドによって段階を省略したり、順序を入れ替えたりします。たとえば `codex completion` は CLI 解析後に終了しますが、`codex exec` は app-server リクエストを利用でき、`codex` の対話モードはローカルデーモン経由で実行できます。

## 段階ごとの担当

| 段階 | 主な担当 | 担当するもの |
| --- | --- | --- |
| 解析とディスパッチ | [`codex-cli`](../../../crates/cli/src/main.rs) | `MultitoolCli`、`Subcommand`、ルート検証、ディスパッチ |
| 対話用引数モデル | [`codex-tui`](../../../crates/tui/src/cli.rs) | TUI 固有のフラグと、resume/fork/agents 用のラッパー状態 |
| 非対話用引数モデル | [`codex-exec`](../../../crates/exec/src/cli.rs) | `codex exec` のオプションと exec サブコマンド |
| 設定 | [`codex-core`](../../../crates/core/src/config/mod.rs) と [`codex-config`](../../../crates/config/src/lib.rs) | 読み込み、上書き、プロファイル、検証、実行時設定 |
| エージェントのオーケストレーション | [`codex-core`](../../../crates/core/src/lib.rs) | スレッド、ターン、ツール、承認、モデルリクエスト、共有状態 |
| UI 表示 | [`codex-tui`](../../../crates/tui/src/lib.rs) | ターミナルのイベントループ、履歴セル、入力、レンダリング |
| App-server の実行時処理 | [`codex-app-server`](../../../crates/app-server/src/lib.rs) | リクエスト処理、トランスポート、デーモン向けランタイム、ライフサイクル |
| 公開プロトコル | [`codex-app-server-protocol`](../../../crates/app-server-protocol/src/lib.rs) と [`codex-protocol`](../../../crates/protocol/src/lib.rs) | ワイヤー型、通知、リクエスト、共有ドメイン型 |
| プロバイダー/API 境界 | [`codex-api`](../../../crates/codex-api/src/lib.rs)、[`codex-client`](../../../crates/codex-client/src/lib.rs) | プロバイダーへのリクエスト、ストリーミング、再試行、認証ヘッダー、トランスポート |
| 実行/ポリシー | [`codex-tools`](../../../crates/tools/src/lib.rs)、[`codex-execpolicy`](../../../crates/execpolicy/src/lib.rs)、[`codex-sandboxing`](../../../crates/sandboxing/src/lib.rs) | ツールレジストリ、コマンドポリシー、ホストのサンドボックス構築 |
| 永続化 | [`codex-rollout`](../../../crates/rollout/src/lib.rs)、[`codex-thread-store`](../../../crates/thread-store/src/lib.rs)、[`codex-history`](../../../crates/history/src/lib.rs) | ロールアウトファイル、スレッドメタデータ、履歴、状態ストア |

## 重要な境界

### CLI とランタイム

`codex-cli` はユーザー向けのコマンド文法を管理し、経路を選択します。コマンドの動作全体を所有するわけではありません。ランタイム処理は `codex-tui`、`codex-exec`、`codex-core`、app-server、またはユーティリティクレートに委譲されます。

### プロトコルとサーバー

`codex-app-server-protocol` と `codex-protocol` は、シリアライズ可能な形とドメイン値を定義します。`codex-app-server` はリクエストを処理し、状態を調整します。プロトコル型がプロトコルクレートで定義されているからといって、そのクレートが動作を所有するとは限りません。

### プロバイダー境界とエージェントの動作

`codex-api` と `codex-client` は、リクエスト/ストリーム/トランスポートの境界を提供します。モデル応答をいつ要求するか、イベントをどう解釈するか、ツールや承認をいつ関与させるかは Core が決定します。この分離が、リポジトリの他の部分で利用するプロバイダー非依存の継ぎ目です。

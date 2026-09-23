# ツール実行

## 正規化した経路

```text
モデル応答のツール呼び出し
  -> ツール名/引数のルーティング
  -> ToolExecutor ハンドラーまたは CoreToolRuntime
  -> 承認判断
  -> サンドボックス構築
  -> プロセス/ネットワーク/ファイルシステム操作
  -> ToolOutput / プロトコルイベント
```

汎用ツール境界は [`codex_tools::ToolExecutor`](../reference/trait-index.md#toolexecutor) です。Core のレジストリは [`CoreToolRuntime`](../reference/trait-index.md#coretoolruntime) を追加し、具体的な実行ランタイムは承認とサンドボックスの機能とともに [`ToolRuntime`](../reference/trait-index.md#toolruntime) を実装します。

`TurnItemEmitter` と `ToolOutput` は、ツールの結果/イベントをターンと出力の層へ戻します。MCP、プラグイン、コードモード、シェル、パッチ、組み込みツールはこの経路の形を共有しますが、ハンドラーとポリシーの詳細が異なります。

## 重要な違い

`codex exec` CLI は、エージェントが発行する exec ツールと同じものではありません。前者はトップレベルの非対話コマンドで、後者はターン内のツールランタイムです。Core の実行プリミティブを共有することはできますが、入力、承認、出力の境界は異なります。

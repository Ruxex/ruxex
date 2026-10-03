# `genex review`

## エントリ

`genex review` は `genex_exec::ReviewArgs` をラップする [`ReviewCommand`](../../../../crates/cli/src/main.rs#L266) を使います。ルートのディスパッチは [main.rs#L1050](../../../../crates/cli/src/main.rs#L1050) で `exec` と同じ非対話実行ファミリーに入ります。

## ランタイムの形

review コマンドはレビュー固有の指示と対象選択を exec ランタイムに渡します。設定、プロバイダー/モデル選択、ツール実行、承認/ポリシー、出力処理は `genex exec` と共有します。違いはリクエストの構築とレビュー用プロンプトの意味論です。

```text
genex review [OPTIONS]
  -> ReviewCommand + ReviewArgs
  -> genex-exec review path
  -> repository/context collection
  -> model turn and tool calls
  -> non-interactive result output
```

共通の実行ライフサイクルについては [exec のページ](exec.md) を、共有される境界については [プロバイダー/モデル](../common/provider-and-model.md) と [ツール実行](../common/tool-execution.md) を参照してください。

## 境界に関する注意

CLI クレートはトップレベルの表記と strict-config ラッパーを管理します。`genex-exec` はレビュー引数の解釈と実行オーケストレーションを管理し、`genex-core` はリクエストが受理された後のエージェント/ランタイム動作を管理します。

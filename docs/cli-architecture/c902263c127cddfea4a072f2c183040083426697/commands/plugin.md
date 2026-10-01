# `codex plugin`

## エントリ

プラグインは [`PluginCli` と `PluginSubcommand`](../../../../crates/cli/src/plugin_cmd.rs#L39) で解析されます。ルートのディスパッチは [main.rs#L1083](../../../../crates/cli/src/main.rs#L1083) でプラグインランナーに入ります。

サブコマンドは `add`、`list`、`marketplace`、`remove` です。Marketplace には [`marketplace_cmd.rs`](../../../../crates/cli/src/marketplace_cmd.rs#L35) に独自の `add`、`list`、`upgrade`、`remove` コマンドがあります。

## フロー

```text
codex plugin <operation>
  -> プラグインセレクター / Marketplace 引数
  -> 設定とプラグイン入力の読み込み
  -> Marketplace の解決
  -> インストール、一覧表示、アップグレード、または削除
  -> 人間向けまたは JSON の出力
```

`add` は `PLUGIN@MARKETPLACE`、またはプラグインと `--marketplace` を解決し、ローカルのプラグイン/キャッシュ状態にインストールします。`list --available --json` には未インストールの Marketplace プラグインを含めることができます。これらの操作はローカルのプラグイン状態を変更しますが、それ自体がモデルのターンを開始することはありません。

## 境界

コマンド文法は `genex-cli` にあり、プラグインのライフサイクルと入力/設定の意味論は `genex-plugin` および app-server 統合にあります。[設定](../common/configuration.md)、[トランスポート/サーバー](../common/transport-and-server.md)、[出力/エラー](../common/output-and-errors.md)も参照してください。

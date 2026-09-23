# 共通の設定フロー

## 正規化した順序

```text
ルート CLI フラグ
  -> CliConfigOverrides / SharedCliOptions
  -> CODEX_HOME と現在のディレクトリ
  -> プロファイルと config.toml の層
  -> ローダーの上書きとハーネスの上書き
  -> ConfigBuilder
  -> 不変のランタイム Config
```

CLI ヘルパー [`cloud_config::config_builder`](../../../../crates/cli/src/cloud_config.rs#L20) は `-c` の上書きを解析し、`CODEX_HOME` と有効な CWD を解決して、[`codex_core::config::ConfigBuilder`](../../../../crates/core/src/config/mod.rs#L1290) を構築します。ビルダーはランタイムの利用側に `Config` が渡される前に、永続化済み/ユーザー/プロジェクトの層とコマンド固有の上書きを組み合わせます。

`codex-utils-cli` は [`SharedCliOptions`](../../../../crates/utils/cli/src/shared_options.rs#L9) のような共有引数の形を管理し、`codex-cli`、`codex-exec`、`codex-tui` は各コマンドでどの共有フラグを公開/グローバル化するかを決めます。

## 優先順位

コマンド固有の値と明示的な CLI 上書きは、読み込まれた既定値より優先されます。プロファイルの選択は読み込む設定層を変えるもので、任意の `-c` キー/値上書きと同じではありません。Strict-config は未知フィールドの動作を変え、設定読み込みに意味がないコマンドでは拒否されます。

## Trait 境界

[`ThreadConfigLoader`](../reference/trait-index.md#threadconfigloader) は設定クレートにおける、機能に関係する主要な Trait です。これにより app-server/Core は、ローダーを CLI 全体に依存させずに、スレッド単位の設定を読み込めます。

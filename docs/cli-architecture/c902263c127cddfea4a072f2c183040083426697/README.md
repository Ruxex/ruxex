# Codex CLI アーキテクチャのスナップショット

ソーススナップショット: `c902263c127cddfea4a072f2c183040083426697`

このディレクトリでは、`codex` から利用できる機能が Rust ワークスペースに入り、各段階をどのクレートが担当し、共有 Trait がどこで定義されているかを説明します。これはエンジニア向けの実装マップであり、ユーザーマニュアルではありません。また、このコミット後もソースの行番号が固定されることを保証するものではありません。

## 読む順番

1. [システムマップ](00-system-map.md) — 正規化したエンドツーエンドのパイプライン。
2. [CLI エントリポイント](01-cli-entrypoint.md) — ルートの解析、検証、ディスパッチ。
3. [コマンドのページ](commands/interactive.md) — 機能グループごとのページ。
4. [共通アーキテクチャ](common/configuration.md) — 共有動作を一度だけ説明。
5. [Trait 索引](reference/trait-index.md) と [クレート索引](reference/crate-index.md) — 所有範囲と拡張ポイント。
6. [ソースマップ](reference/source-map.md) — スナップショットのパスと行番号アンカー。

## コマンドグループ

| CLI の機能 | ページ |
| --- | --- |
| `codex` の対話 TUI | [対話モード](commands/interactive.md) |
| `codex agents` | [エージェント](commands/agents.md) |
| `codex exec` | [実行](commands/exec.md) |
| `codex review` | [レビュー](commands/review.md) |
| `resume`、`queue`、`archive`、`delete`、`unarchive`、`fork` | [セッション](commands/sessions.md) |
| `mcp` | [MCP](commands/mcp.md) |
| `plugin` | [プラグイン](commands/plugin.md) |
| `app-server` | [app-server](commands/app-server.md) |
| `login`、`logout` | [認証](commands/auth.md) |
| `doctor` | [診断](commands/diagnostics.md) |
| `sandbox` | [サンドボックス](commands/sandbox.md) |
| `debug` | [デバッグ](commands/debug.md) |
| `execpolicy check` | [実行ポリシー](commands/policy.md) |
| `completion`、`features`、`exec-server`、内部トランスポート | [ユーティリティ](commands/utility.md) |

## 共通アーキテクチャ

- [設定](common/configuration.md)
- [認証](common/authentication.md)
- [プロバイダーとモデル](common/provider-and-model.md)
- [セッションとスレッド](common/session-and-thread.md)
- [ツール実行](common/tool-execution.md)
- [サンドボックスと権限](common/sandbox-and-permissions.md)
- [トランスポートとサーバー](common/transport-and-server.md)
- [永続化とロールアウト](common/persistence-and-rollout.md)
- [出力とエラー](common/output-and-errors.md)
- [機能フラグ](common/feature-flags.md)
- [テレメトリ](common/telemetry.md)

## 参照

- [Trait 索引](reference/trait-index.md)
- [クレート索引](reference/crate-index.md)
- [ソースマップ](reference/source-map.md)

## スナップショットのルール

ソースのコミットはディレクトリ名に含まれています。CLI の動作または文書化した Trait 境界が変わった場合は、別の完全なハッシュ名のディレクトリを作成し、このディレクトリはそのまま残してください。ソース参照が正しいのは、固定したコミットに対してだけです。

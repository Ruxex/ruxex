# Hawk 解析のバイナリ起点

Level 3 の公開未使用コード調査では、単一のバイナリだけを起点に
到達性を判断しない。あるバイナリから到達不能でも、別の実行経路、OS 固有の
ヘルパー、開発ツール、または統合テストの fixture から利用されることがあるためである。

この一覧は 2026-09-23 時点の
`cargo metadata --no-deps --format-version 1` の `bin` target を基にした。
現在は合計31件で、通常実行16件、開発・補助7件、テスト fixture 8件である。
更新時は同コマンドの出力と照合する。パッケージの自動検出バイナリも含める。

ここでいう「バイナリ」は、配布用の実行ファイルだけでなく、Cargo が `bin`
target として認識する開発ツールおよびテスト用サーバーも含む。したがって、
Level 3 の削除判断では通常実行の16件を主起点としつつ、対象コードが開発ツールや
テスト fixture からだけ使われる可能性を別集合で確認する。

## 判定方法

- **通常実行** は、対応するプラットフォームを含め、すべて Hawk の起点にする。
- **開発・補助** は、開発用ツールとして提供される公開コードを評価する場合に起点にする。
- **テスト fixture** は、プロダクションコードの削除判断には混ぜず、テスト・fixture 専用コードを
  評価する場合に別集合として解析する。
- 公開コードを dead と判断するには、そのコードが必要とするすべての起点集合で到達不能であることを
  確認する。単一バイナリの結果だけで削除しない。

## 通常実行

| パッケージ | バイナリ | 用途 | 対象 |
| --- | --- | --- | --- |
| `genex-cli` | `codex` | 対話 TUI、非対話実行、MCP、app-server 管理などを提供する主 CLI。 | 全プラットフォーム |
| `codex-tui` | `codex-tui` | TUI を単独で起動するエントリーポイント。 | 全プラットフォーム |
| `codex-exec` | `codex-exec` | 非対話のエージェント実行を行う CLI。 | 全プラットフォーム |
| `codex-app-server` | `codex-app-server` | IDE・クライアント向け app-server を stdio、Unix socket、WebSocket で提供する。 | 全プラットフォーム |
| `genex-code-mode-host` | `genex-code-mode-host` | code mode 用の stdio/gRPC ホストと任意の OTEL trace transport を提供する。 | 全プラットフォーム |
| `genex-apply-patch` | `apply_patch` | パッチ適用ツールを起動する。 | 全プラットフォーム |
| `genex-file-search` | `genex-file-search` | ファイル検索結果を標準出力（必要に応じ JSON）へ出力する。 | 全プラットフォーム |
| `genex-execpolicy` | `genex-execpolicy` | Starlark ベースの実行ポリシーでコマンドを評価する。 | 全プラットフォーム |
| `genex-bwrap` | `bwrap` | 同梱した bubblewrap を実行するサンドボックス補助プログラム。 | Linux |
| `codex-linux-sandbox` | `codex-linux-sandbox` | Landlock と seccomp を使ってサンドボックス内でコマンドを実行する。 | Linux |
| `genex-shell-escalation` | `genex-execve-wrapper` | 権限昇格用の execve ラッパーを提供する。 | Unix |
| `genex-stdio-to-uds` | `genex-stdio-to-uds` | 標準入出力を Unix domain socket に中継する。 | Unix |
| `genex-windows-sandbox` | `genex-windows-sandbox-setup` | Windows sandbox のセットアップを行う。 | Windows |
| `genex-windows-sandbox` | `codex-command-runner` | Windows sandbox 内のコマンド実行を担う。 | Windows |
| `genex-windows-sandbox` | `codex-windows-managed-deny-probe` | managed 環境の ACL deny 設定を検査する。 | Windows |

## 開発・補助

| パッケージ | バイナリ | 用途 | Hawk での扱い |
| --- | --- | --- | --- |
| `genex-cli` | `logs_client` | 状態 DB のログを条件付きで表示・追跡する診断ツール。 | 開発・診断 API を評価する場合に含める。 |
| `genex-config-schema` | `genex-write-config-schema` | `config.toml` の JSON Schema fixture を生成する。 | スキーマ生成コードを評価する場合に含める。 |
| `genex-hooks` | `write_hooks_schema_fixtures` | hooks の schema fixture を生成する。 | hooks の生成コードを評価する場合に含める。 |
| `genex-http-client` | `custom_ca_probe` | カスタム CA を利用する HTTP 接続を検査する。 | HTTP クライアントの補助コードを評価する場合に含める。 |
| `codex-tui` | `md-events` | Markdown 入力を pulldown-cmark のイベント列として表示する診断ツール。 | TUI の Markdown 補助コードを評価する場合に含める。 |
| `codex-app-server-test-client` | `codex-app-server-test-client` | app-server を起動・操作する手動／統合テスト用クライアント。 | app-server テスト支援コードを評価する場合に含める。 |

## テスト fixture

| パッケージ | バイナリ | 用途 | Hawk での扱い |
| --- | --- | --- | --- |
| `codex-app-server` | `codex-app-server-test-notify-capture` | 通知 payload を指定ファイルへ原子的に保存する fixture。 | app-server 統合テスト専用集合。 |
| `codex-app-server` | `test_notify_capture` | UTF-8 の通知 payload を JSON ファイルとして保存する fixture。 | app-server 統合テスト専用集合。 |
| `codex-app-server` | `exec-server` | app-server 統合テスト向けの最小 exec-server fixture。 | app-server 統合テスト専用集合。 |
| `genex-rmcp-client` | `rmcp_test_server` | MCP クライアント検証用の stdio test server。 | MCP 統合テスト専用集合。 |
| `genex-rmcp-client` | `test_mcp_2026_discovery_stdio_server` | MCP 2026 discovery protocol を検証する stdio server。 | MCP 統合テスト専用集合。 |
| `genex-rmcp-client` | `test_mcp_2026_stdio_server` | MCP 2026 と legacy fallback を検証する stdio server。 | MCP 統合テスト専用集合。 |
| `genex-rmcp-client` | `test_stdio_server` | tools、resources、elicitation などを返す汎用 MCP stdio fixture。 | MCP 統合テスト専用集合。 |
| `genex-rmcp-client` | `test_streamable_http_server` | streamable HTTP transport を検証する MCP fixture server。 | MCP 統合テスト専用集合。 |

## 集約手順

1. 通常実行の各バイナリを、対応プラットフォームごとに Hawk の起点として解析する。
2. 同一 feature 構成の結果を合成し、いずれかの通常実行バイナリから到達するコードを live と扱う。
3. 開発・補助およびテスト fixture は、対象コードの用途に応じて別途合成する。
4. feature、動的 dispatch、外部公開 API を確認した後にだけ、到達不能コードを削除候補へ分類する。

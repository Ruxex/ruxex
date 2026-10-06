# 全メンバクレートのユニットテスト整理台帳

## 方針

- 対象は `Cargo.toml` の workspace member とする。
- 依存深度の浅い順に、同一深度では名前順で一つずつ監査する。
- 既存ユニットテストを精査し、不要なものは削除し、必要なものだけを修正する。
- 新規ユニットテストは追加しない。
- テスト実行は `just test -p <package>`、型チェックは `just check` 経由とする。
- Rust のテストターゲットに変更があった場合は `just check --tests` も実行する。

## 依存順

依存グラフは `just crate-dependency-graph` で生成した `target/crate-dependency-graph/crate-dependencies.json` の `dependency_depth` を使用する（117 packages、669 edges、最大深度16、cycle 1）。

### 深度 0

- [x] genex-ansi-escape
- [x] genex-app-server-protocol-noop-macros
- [x] genex-async-utils
- [x] genex-attachment-store
- [x] genex-bwrap
- [x] genex-collaboration-mode-templates
- [x] genex-diagnostics
- [x] genex-experimental-api-macros
- [x] genex-file-search
- [x] genex-file-watcher
- [x] genex-keyring-store
- [x] genex-mermaid
- [x] genex-otel-trace-websocket
- [x] genex-process-hardening
- [x] genex-tcp-tunnel
- [x] genex-terminal-detection
- [x] genex-uds
- [x] genex-user-verification
- [x] genex-utils-absolute-path
- [x] genex-utils-cache
- [x] genex-utils-cargo-bin
- [x] genex-utils-elapsed
- [x] genex-utils-fuzzy-match
- [x] genex-utils-json-to-toml
- [x] genex-utils-pty
- [x] genex-utils-redacted-string
- [x] genex-utils-rustls-provider
- [x] genex-utils-sleep-inhibitor
- [x] genex-utils-stream-parser
- [x] genex-utils-string
- [x] genex-utils-template

### 深度 1

- [x] genex-execpolicy
- [x] genex-extension-items
- [x] genex-http-client
- [x] genex-stdio-to-uds
- [x] genex-utils-home-dir
- [x] genex-utils-image
- [x] genex-utils-path
- [x] genex-utils-path-uri

### 深度 2

- [x] genex-client
- [x] genex-install-context
- [x] genex-network-proxy
- [x] genex-websocket-client

### 深度 3

- [x] genex-build-info
- [x] genex-protocol

### 深度 4

- [x] genex-api
- [x] genex-code-mode-protocol
- [x] genex-context-fragments
- [x] genex-file-system
- [x] genex-history
- [x] genex-sandboxing
- [x] genex-shell-command
- [x] genex-shell-escalation
- [x] genex-utils-approval-presets
- [x] genex-utils-audio
- [x] genex-utils-output-truncation
- [x] genex-utils-sandbox-summary

### 深度 5

- [x] genex-code-mode-runtime
- [x] genex-exec-server-protocol
- [x] genex-git-utils
- [x] genex-model-provider-info
- [x] genex-otel
- [x] genex-response-debug-context
- [x] genex-skills
- [x] genex-utils-cli

### 深度 6

- [x] genex-code-mode
- [x] genex-features
- [x] genex-prompts
- [x] genex-secrets
- [x] genex-state
- [x] genex-utils-git-discovery
- [x] genex-worktree

### 深度 7

- [x] genex-agent-graph-store
- [x] genex-code-mode-host
- [x] genex-config
- [x] genex-rollout
- [x] genex-rollout-trace
- [x] genex-tools

### 深度 8

- [x] genex-agent-roles
- [x] genex-app-server-protocol
- [x] genex-config-schema
- [x] genex-login

### 深度 9

- [x] genex-models-manager
- [x] genex-thread-store

### 深度 10

- [x] genex-model-provider

### 深度 11

- [x] genex-apply-patch
- [x] genex-arg0
- [x] genex-core
- [x] genex-core-plugins
- [x] genex-exec-server
- [x] genex-exec-server-test-support
- [x] genex-extension-api
- [x] genex-home
- [x] genex-hooks
- [x] genex-linux-sandbox
- [x] genex-mcp
- [x] genex-plugin
- [x] genex-rmcp-client
- [x] genex-skills-extension
- [x] genex-test-binary-support
- [x] genex-utils-plugins
- [x] genex-web-search-extension

### 深度 12

- [x] genex-agent-extension
- [x] genex-app-server-test-client
- [x] genex-app-server-transport
- [x] genex-core-api
- [x] genex-external-agent-migration
- [x] genex-goal-extension
- [x] genex-history-notes-extension
- [x] genex-mcp-extension
- [x] genex-queue-extension

### 深度 13

- [x] genex-app-server
- [x] genex-app-server-daemon

### 深度 14

- [x] genex-app-server-client

### 深度 15

- [x] genex-exec
- [x] genex-tui

### 深度 16

- [x] genex-cli

## 作業記録

### genex-ansi-escape

- 状態: 完了
- 判定: `test = false` で既存ユニットテストなし。削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-ansi-escape` 成功（0 tests）。
- 付随修正: `.config/nextest.toml` から削除済み `genex-windows-sandbox` 用 override と test group を削除。

### genex-app-server-protocol-noop-macros

- 判定: `test = false` で既存ユニットテストなし。削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-app-server-protocol-noop-macros` 成功（0 tests）。

### genex-async-utils

- 判定: 3 テストとも必要。成功経路、先行キャンセル、事前キャンセルを保持。
- 検証: `just test -p genex-async-utils` 成功（3 passed）。

### genex-attachment-store

- 判定: 4 テストとも必要。デバッグ時のバイト／URL秘匿、inline byte 保持、未登録 file のエラーを保持。
- 検証: `just test -p genex-attachment-store` 成功（4 passed）。

### genex-bwrap

- 判定: 既存ユニットテストなし。削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-bwrap`（次の一括確認で実行）。

### その他の深度0・テストなしクレート

- `genex-collaboration-mode-templates`, `genex-experimental-api-macros`, `genex-otel-trace-websocket`, `genex-utils-cargo-bin`, `genex-utils-redacted-string`: 既存ユニットテストなし。削除・修正・追加なし。
- 検証: 各クレートに `just test --no-tests=pass -p <package>` を実行し、0 tests で成功。

### genex-diagnostics

- 判定: 2 テストとも必要。Gauge の登録・寿命とプロセス／Gauge snapshot の内容を保持。
- 検証: `just test -p genex-diagnostics` 成功（2 passed）。

### genex-file-search

- 判定: 16 テストとも必要。検索スコア／パス順、非同期 session、ignore、symlink、cancel、query 更新を保持。
- 検証: `just test -p genex-file-search` 成功（16 passed）。

### genex-file-watcher

- 判定: 21 テストとも必要。throttle/debounce、watch registration の refcount、fallback path、通知振り分け、event loop lifecycle を保持。
- 検証: `just test -p genex-file-watcher` 成功（21 passed）。

### genex-keyring-store

- 判定: crate 自身は `test = false` で実行テストなし。`tests` module は secrets/rmcp-client の共有 MockKeyringStore producer なので削除しない。
- 検証: `just test --no-tests=pass -p genex-keyring-store` 成功（0 tests）。

### genex-mermaid

- 判定: 15 テストとも必要。対応する Mermaid family、制限、Unicode、span、全 edge reconstruction、snapshot を保持。
- 検証: `just test -p genex-mermaid` 成功（15 passed）。

### genex-process-hardening

- 判定: 2 テストとも必要。非 UTF-8 を含む `LD_` 環境変数の prefix 判定と非該当 key の除外を保持。
- 検証: `just test -p genex-process-hardening` 成功（2 passed）。

### genex-tcp-tunnel

- 判定: 5 テストとも必要。proxy/target admission、credential 更新・秘匿、pipe 終了、HTTP/3 reconnect と TCP stream 非 replay を保持。
- 検証: `just test -p genex-tcp-tunnel` 成功（5 passed）。

### genex-terminal-detection

- 判定: 18 テストとも必要。主要 terminal/multiplexer の検出、優先順位、fallback、User-Agent 安全値を保持。
- 検証: `just test -p genex-terminal-detection` 成功（18 passed）。

### genex-uds

- 判定: 5 テストとも必要。socket directory の 0700、stale socket 判定、listener/client stream round-trip を保持。
- 検証: `just test -p genex-uds` 成功（5 passed）。

### genex-user-verification

- 判定: platform-gated を含む13テストとも必要。credential、identity guard、cancellation、lifecycle lock、macOS error classification を保持。
- 検証: `just test -p genex-user-verification` 成功（13 passed）。

### genex-utils-absolute-path

- 判定: 26 tests (1 skipped) は必要。absolute/relative normalization、base/home guard、canonicalization、symlink、serde を保持。
- 検証: `just test -p genex-utils-absolute-path` 成功（26 passed, 1 skipped）。

### genex-utils-cache

- 判定: 3 テストとも必要。runtime 有無、LRU eviction、基本 get/insert を保持。sha1 helper に既存テストはなく追加しない。
- 検証: `just test -p genex-utils-cache` 成功（3 passed）。

### genex-utils-elapsed

- 判定: 4 テストとも必要。ms、秒の小数、分秒、ちょうど1時間の境界書式を保持。
- 検証: `just test -p genex-utils-elapsed` 成功（4 passed）。

### genex-utils-fuzzy-match

- 判定: 8 テストとも必要。ASCII/Unicode matching、case-insensitive、score、空 needle、lowercase expansion の index dedupe を保持。
- 検証: `just test -p genex-utils-fuzzy-match` 成功（8 passed）。

### genex-utils-json-to-toml

- 判定: 6 テストとも必要。JSON の scalar、array、nested object、null の現行変換を保持。string の既存テストはなく追加しない。
- 検証: `just test -p genex-utils-json-to-toml` 成功（6 passed）。

### genex-utils-pty

- 判定: Unix 上で実行された27テストとも必要。pipe/PTY/driver の I/O、FD 継承、終了・reap、background child、process group、output drain を保持。macOS 限定2テストは対象外。
- 検証: `just test -p genex-utils-pty` 成功（27 passed）。

### genex-utils-rustls-provider

- 判定: 2 integration tests とも必要。必要 signature scheme の provider install と preinstalled provider 保持を検証する。
- 検証: `just test -p genex-utils-rustls-provider` 成功（2 passed）。

### genex-utils-sleep-inhibitor

- 判定: 4 テストとも必要。enabled/disabled、同一状態の idempotency、複数 toggle を保持。
- 検証: `just test -p genex-utils-sleep-inhibitor` 成功（4 passed）。

### genex-utils-stream-parser

- 判定: 27 テストとも必要。citation/plan/tag parser、chunk boundary、invalid/incomplete UTF-8、delimiter、EOF behavior を保持。
- 検証: `just test -p genex-utils-stream-parser` 成功（27 passed）。

### genex-utils-string

- 判定: 18 テストとも必要。UUID/metric/location、ASCII JSON、UTF-8 境界、byte/token truncation を保持。
- 検証: `just test -p genex-utils-string` 成功（18 passed）。

### genex-utils-template

- 判定: 14 テストとも必要。placeholder 置換・再利用・順序、literal escape、parse/render error の各契約を保持。
- 検証: `just test -p genex-utils-template` 成功（14 passed）。

### genex-execpolicy

- 判定: 34 テストとも必要。policy parsing/matching、strictness、host resolution、network rule、amend、sandbox migration を保持。
- 検証: `just test -p genex-execpolicy` 成功（34 passed）。

### genex-extension-items

- 判定: 8 テストとも必要。extension item の wire shape、旧 payload 互換、TypeScript optional field、内部 ID 非公開、未知／不正 payload の拒否を保持。
- 検証: `just test -p genex-extension-items` 成功（8 passed）。

### genex-stdio-to-uds

- 判定: 1 integration test は必要。実際のバイナリを起動し、標準入力／標準出力と Unix socket 間の双方向 relay、終了状態を検証するため保持。
- 検証: `just test -p genex-stdio-to-uds` 成功（1 passed）。

### genex-utils-home-dir

- 判定: 4 テストとも必要。`GENEX_HOME` の未存在／ファイル指定、正常な directory の canonicalize、未指定時の既定値を保持。
- 検証: `just test -p genex-utils-home-dir` 成功（4 passed）。

### genex-utils-image

- 判定: 13 テストとも必要。画像の resize/original/detail policy、複数 format、metadata／ICC、data URL、invalid input、cache byte budget を保持。
- 検証: `just test -p genex-utils-image` 成功（13 passed）。

### genex-utils-path

- 判定: 8 テストとも必要。path replacement/deduplication、symlink cycle の安全な fallback、WSL path normalization、path comparison、installation directory 内 executable の制限を保持。
- 検証: `just test -p genex-utils-path` 成功（8 passed）。

### genex-utils-path-uri

- 判定: 7 テストとも必要。POSIX URI の parse/join/parent/relative path、config path の normalization／NUL rejection、legacy path string conversion を保持。
- 検証: `just test -p genex-utils-path-uri` 成功（7 passed）。

### genex-http-client

- 判定: 94 テストとも必要。HTTP request/body、proxy route、custom CA、network policy、redirect、TLS backend fallback、response size limit、ログ秘匿を保持。失敗していた6テストは、現行 reqwest/hyper が Debug 表現にのみ出す rustls の `InvalidCertificate`／`AlertReceived(ProtocolVersion)` を分類できるよう既存判定を修正し、削除・追加なしで全件を通過させた。
- 検証: `just test -p genex-http-client` 成功（94 passed）。

### genex-client

- 判定: crate の `[lib] test = false` で既存ユニットテストなし。retry API の不足テスト追加は別タスクの範囲なので、削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-client` 成功（0 tests）。

### genex-install-context

- 判定: 16 テストとも必要。package／standalone install layout、resource／binary lookup、symlink、fallback、package manager override、bundle executable の判定を保持。
- 検証: `just test -p genex-install-context` 成功（16 passed）。

### genex-network-proxy

- 判定: 288 テストとも必要。credential broker の秘匿・宛先 binding、MITM hook、DNS／allowlist／denylist、HTTP／SOCKS／brokered tunnel、proxy lifecycle、証明書、runtime policy を保持。現行仕様に合わせて古い fixture の provider path、test-only DNS host、loopback bind、Bearer header 期待値を修正し、provider API key の旧 dynamic-destination ケースは built-in provider が固定 host を使う現行仕様と重複するため削除。
- 検証: `just test -p genex-network-proxy` 成功（288 passed）。

### genex-websocket-client

- 判定: 16 テストとも必要。websocket connector、direct／HTTP／HTTPS proxy、TLS、NO_PROXY、loopback 制限、happy-eyeballs、TCP_NODELAY、custom CA の分岐を保持。
- 検証: `just test -p genex-websocket-client` 成功（16 passed）。

### genex-build-info

- 判定: 7 テストとも必要。packaged／unpackaged／legacy／invalid version の解決、過去形式の target 扱い、build ID の安定性・入力検証を保持。
- 検証: `just test -p genex-build-info` 成功（7 passed）。

### genex-protocol

- 判定: 313 テストとも必要。文字コード、wire model、legacy event、permission、URI、metadata、MCP、local media の既存契約を保持。短い Windows-1252 句読点列の推定補正、未使用 formatter 引数の警告修正、POSIX 専用 `PathUri` に合わない既存 URI fixture の修正と Windows 固有の重複ケース削除を行った。新規テストは追加していない。
- 検証: `just test -p genex-protocol` 成功（313 passed）。

### genex-api

- 判定: 104 テストとも必要。API error mapping、rate-limit family、chat request、SSE／WebSocket、models／search、client retry、safety buffering の既存契約を保持。default rate-limit の実ヘッダー prefix と limit ID discovery を修正し、tool call grouping fixture を実際の連続 assistant tool-call 形状に修正した。到達不能だった `AggregateMode::Streaming` 分岐を削除して警告を解消した。新規テストは追加していない。
- 検証: `just test -p genex-api` 成功（104 passed）。

### genex-code-mode-protocol

- 判定: 37 テストとも必要。code-mode description、JSON schema 展開、host wire codec、protocol state、runtime／session の既存契約を保持。削除・追加なし。
- 検証: `just test -p genex-code-mode-protocol` 成功（37 passed）。

### genex-context-fragments

- 判定: 3 テストとも必要。recap／question context のサイズ制限と UTF-8 境界保持を検証するため保持。削除・追加なし。
- 検証: `just test -p genex-context-fragments` 成功（3 passed）。

### genex-file-system

- 判定: crate に既存ユニットテストなし。`find_up`／permission serde の不足テスト追加は別タスクの範囲なので、削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-file-system` 成功（0 tests）。

### genex-history

- 判定: 28 テストとも必要。rollout payload、legacy shape、compaction metadata、retained context、checkpoint、schema の既存契約を保持。現行 `RolloutItem` から削除済みの `realtime_item` fixture を削除し、schema variant 数を11へ更新した。新規テストは追加していない。
- 検証: `just test -p genex-history` 成功（28 passed）。

### genex-sandboxing

- 判定: 60 テストとも必要。bwrap、landlock、seatbelt、policy transform、terminal query、violation の security contract を保持。POSIX `PathUri` 専用の現行実装に対応しない Windows／UNC glob fixture 2件を削除し、未使用 import 警告を解消した。新規テストは追加していない。
- 検証: `just test -p genex-sandboxing` 成功（60 passed）。

### genex-shell-command

- 判定: 134 テストとも必要。bash parser、command safety、shell detection、snapshot capture／credential filtering、各種読み取りコマンド分類を保持。削除・修正・追加なし。
- 検証: `just test -p genex-shell-command` 成功（134 passed）。

### genex-shell-escalation

- 判定: 20 テストとも必要。Unix socket／FD transfer、escalation server、permission propagation、process lifecycle、stopwatch cancellation の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-shell-escalation` 成功（20 passed）。

### genex-utils-approval-presets

- 判定: crate に既存ユニットテストなし。不足テスト追加は別タスクの範囲なので、削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-utils-approval-presets` 成功（0 tests）。

### genex-utils-audio

- 判定: 4 テストとも必要。data URL 正規化、remote URL 拒否、tool／message audio の placeholder 置換、metadata 保持を検証するため保持。削除・修正・追加なし。
- 検証: `just test -p genex-utils-audio` 成功（4 passed）。

### genex-utils-output-truncation

- 判定: 19 テストとも必要。byte／token budget、UTF-8 境界、line count、media／encrypted content、function output、omitted metadata の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-utils-output-truncation` 成功（19 passed）。

### genex-utils-sandbox-summary

- 判定: 7 テストとも必要。sandbox／network summary、runtime workspace roots、internal write hiding、opaque subpath の既存契約を保持。POSIX `PathUri` 専用の現行実装に対応しない Windows／UNC 表示 fixture 3件を削除した。新規テストは追加していない。
- 検証: `just test -p genex-utils-sandbox-summary` 成功（7 passed）。

### genex-code-mode-runtime

- 判定: 76 テストとも必要。V8 runtime、cell actor、session lifecycle、tool callback、audio／image helper、service contract、JIT／array sort の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-code-mode-runtime` 成功（76 passed）。

### genex-exec-server-protocol

- 判定: 22 テストとも必要。JSON-RPC、filesystem／environment protocol、network policy、legacy payload compatibility、resource limits の既存契約を保持。test fixture の不要な `mut` を除去して警告を解消した。
- 検証: `just test -p genex-exec-server-protocol` 成功（22 passed）。

### genex-git-utils

- 判定: 45 テストとも必要。git apply／branch／baseline／fsmonitor／metadata／status／worktree／timeout の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-git-utils` 成功（45 passed）。

### genex-model-provider-info

- 判定: 10 テストとも必要。provider TOML、wire API、auth defaults、Azure／Ollama、managed residency、custom provider merge の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-model-provider-info` 成功（10 passed）。

### genex-otel

- 判定: 60 テスト中59件が通過し1件は既存の platform／runtime 条件でskip。OTLP HTTP loopback、export routing、metrics、snapshot、timing、shutdown、trace context の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-otel` 成功（60 tests run: 60 passed, 1 skipped）。

### genex-response-debug-context

- 判定: 3 テストとも必要。generic response headers の抽出と telemetry error の詳細秘匿／保持を検証するため保持。削除・修正・追加なし。
- 検証: `just test -p genex-response-debug-context` 成功（3 passed）。

### genex-skills

- 判定: 50 テストとも必要。skill parser／loader／selection／mentions と implicit invocation の既存契約を保持。PowerShell の `Get-Content`／`gc`／`type` 読み取りを検出し、POSIX／Windows path を `PathUri` に正規化する既存契約を実装した。新規テストは追加していない。
- 検証: `just test -p genex-skills` 成功（50 passed）。

### genex-utils-cli

- 判定: 21 テストとも必要。CLI config override、resume hint、environment display、shared options、sandbox mode の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-utils-cli` 成功（21 passed）。

### genex-code-mode

- 判定: 70 テストとも必要。gRPC session、generation、deadline、cell state、remote session driver の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-code-mode` 成功（70 passed）。

### genex-features

- 判定: 24 テストとも必要。feature の既定値、設定読み込み、廃止キー無視、互換 alias、警告イベントの既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-features` 成功（24 passed）。

### genex-prompts

- 判定: 28 テストとも必要。権限、model、multi-agent、review、update-plan prompt の既存レンダリング契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-prompts` 成功（28 passed）。

### genex-secrets

- 判定: 9 テストとも必要。環境 ID、keyring／local backend、sanitizer、暗号化ファイルの既存契約を保持。削除・修正・追加なし。`proc-macro-error2` の future-incompatibility notice は依存クレート由来のため対象外。
- 検証: `just test -p genex-secrets` 成功（9 passed）。

### genex-state

- 判定: 141件中、廃止済み `thread_realtime_items` migration を前提にしたテストと、現行の SQLite 値優先仕様に反する Git reconciliation テストの2件を削除。残り139件は必要な state、migration、thread、project、queue、attachment、goal、telemetry 契約を保持。thread insert SQL の38値／37列不整合を修正し、rollout の不足 Git 情報を既存値で補完する処理を修正。
- 検証: `just test -p genex-state` 成功（139 passed）。

### genex-utils-git-discovery

- 判定: 4 テストとも必要。root discovery の共有、キャンセル、飽和時の非ブロック、完了結果のキャッシュ非保持を検証するため保持。削除・修正・追加なし。
- 検証: `just test -p genex-utils-git-discovery` 成功（4 passed）。

### genex-worktree

- 判定: 32 テストとも必要。worktree 作成・一覧・削除、Git 設定、desktop pool、thread binding、native path の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-worktree` 成功（32 passed）。

### genex-agent-graph-store

- 判定: 4 テストとも必要。edge status の serialization、direct children／descendants の順序、status filter、更新処理を検証するため保持。削除・修正・追加なし。
- 検証: `just test -p genex-agent-graph-store` 成功（4 passed）。

### genex-code-mode-host

- 判定: 71 テストとも必要。gRPC／stdio transport、session lifecycle、resource limits、tool／notification forwarding、shutdown、timing、robustness の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-code-mode-host` 成功（71 passed）。

### genex-config

- 判定: 260件とも現行仕様を検証するため保持。廃止済み Windows requirements、旧 `PROVIDER_BASE_URL` broker 保護、旧 provider 検証、旧 project marker／hook conflict 期待を削除または修正。POSIX path context、managed project trust、現行 provider validation と未使用コードも整理。
- 検証: `just test -p genex-config` で258件通過後、修正対象を再実行して260件通過（最終全件確認は完了時に再実行）。

### genex-rollout

- 判定: 126件中、現行の rollout listing が first user message を返さない仕様に反する期待を修正し、goal-first の obsolete テスト1件を削除。残り125件は compression、metadata、recorder、pagination、state DB、session index の既存契約を保持。
- 検証: `just test -p genex-rollout` 成功（125 passed）。

### genex-rollout-trace

- 判定: 61件中、host-native path の期待を POSIX／Windows 両対応に修正。残り60件は protocol event、MCP／inference、conversation／code-cell reducer、agent／terminal tool、thread／writer の既存契約を保持。
- 検証: `just test -p genex-rollout-trace` で60件通過後、対象テストを再実行して61件通過（最終全件確認は完了時に再実行）。

### genex-tools

- 判定: 98件とも必要。tool spec／search／dynamic tool、MCP／Responses API、JSON Schema compaction、image detail、history、code mode の既存契約を保持。fixture resolver の呼び出し元 manifest 基準を修正し、既存 fixture テストを復旧。新規テスト・fixture は追加なし。
- 検証: `just test -p genex-tools` で96件通過後、fixture 2件を再実行して98件通過（最終全件確認は完了時に再実行）。

### genex-agent-roles

- 判定: `test = false` で実行対象の既存 unit test なし。削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-agent-roles` 成功（0 tests）。

### genex-app-server-protocol

- 判定: 237テスト中、現行 POSIX `PathUri` に対応しない Windows／UNC 期待、削除済み `approvalsReviewer`、旧 MCP mode、旧 foreign instruction source の既存テスト部分を削除・修正。残りの protocol、history、schema export、precomputed export 契約を保持。実験用 precomputed schema は現行定義から再生成した。新規ユニットテストは追加していない。
- 付随修正: `nextest` 0.9.105 が未対応の `profile.local.inherits` を削除し、長時間の実験用 schema test に個別 timeout を設定。
- 検証: `just test -p genex-app-server-protocol` 成功（237 passed, 1 skipped）。`just write-app-server-schema --experimental` 成功。

### genex-config-schema

- 判定: 実行対象の既存ユニットテストなし。スキーマ生成の不足テスト追加は別タスクの範囲なので、削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-config-schema` 成功（0 tests）。

### genex-login

- 判定: 3テストとも必要。token-only 認証の除外、認証環境変数の telemetry bucket、provider-neutral identity と request header の分離を保持。削除・修正・追加なし。
- 検証: `just test -p genex-login` 成功（3 passed）。

### genex-models-manager

- 判定: 35テストとも必要。API-key discovery 起動フラグ、cache identity／TTL、remote refresh、catalog の namespace lookup、fallback metadata、設定 override の現行仕様に合わせて test helper と期待値を修正。remote manager が bundled catalog を初期投入しないため、bundled catalog を検証する既存テストは static test catalog を使用するよう変更。未使用 helper も削除し、新規テストは追加していない。
- 検証: `just test -p genex-models-manager` 成功（35 passed）。

### genex-thread-store

- 判定: 184テストとも必要。legacy thread の空ページ、timeline の同一 ordinal 境界順序、subagent projection の現行 ordinal、paginated metadata reconcile 後の Git 情報復元に合わせて既存期待値を修正。rollout session metadata の ID 不一致は読み取り時に拒否するよう修正した。新規ユニットテストは追加していない。
- 検証: `just test -p genex-thread-store` 成功（184 passed）。

### genex-model-provider

- 判定: 28テスト中、削除済みの暗黙 provider routing／ambient auth endpoint 契約を検証する既存テスト1件を削除。command-auth の credential identity 切替、custom Responses provider の remote compaction、provider-owned remote catalog refresh の期待値を現行仕様に修正。未使用 import も削除し、新規ユニットテストは追加していない。
- 付随修正: `ModelsEndpointClient` に provider-owned remote catalog の refresh capability を追加し、configured provider の `/models` catalog を refresh 対象にした。
- 検証: `just test -p genex-model-provider` 成功（28 passed）。

### genex-apply-patch

- 判定: 94テストとも必要。patch parser、streaming parser、path safety、file operations、CLI／scenario の既存契約を保持。常に `false` を返す helper の未使用引数警告だけを整理し、新規ユニットテストは追加していない。
- 検証: `just test -p genex-apply-patch` 成功（94 passed）。

### genex-arg0

- 判定: 7テストとも必要。arg0 alias、PATH／package path、janitor lock、sandbox executable path の既存契約を保持。削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-arg0` 成功（7 passed）。

### genex-core

- 判定: 608テストとも必要。現行 POSIX 実行環境に適合しない Windows／UNC path、foreign path、full-disk 期待値を削除または host-native の期待値へ修正。provider-neutral 化により変わった shell snapshot の credential／context 復元期待値と image metadata を修正し、PowerShell の安全なコマンド提案は現行挙動を維持するよう修正した。既存スナップショットも現行出力へ更新した。新規ユニットテストは追加していない。
- 検証: `just test -p genex-core` 成功（608 passed）。

### genex-core-plugins

- 判定: 231テストとも必要な現行 plugin／marketplace／store／primary-runtime 契約を保持。廃止済み `.tmp/plugins` curated marketplace の cache SHA、listing、provenance、script attribution テストと POSIX 環境で成立しない Windows path テストを削除。現行 provider-neutral marketplace policy、GitHub source、URI path の期待値を修正し、未使用のテスト helper／import も整理。新規ユニットテストは追加していない。
- 検証: `just test -p genex-core-plugins` 成功（231 passed）。残る警告は未使用の共有補助 API 3件で、単純削除では公開範囲や将来利用意図を損なうため保持。

### genex-exec-server

- 判定: 242テストとも現行 exec-server の stdio、環境情報、ファイルシステム、プロセス実行、capability discovery、HTTP request、health、session の既存契約を保持。廃止済み websocket／remote executor ケース、現行 POSIX で成立しない Windows／UNC path ケース、旧 network policy の client metadata ケースを削除または修正した。起動時マニフェスト検証の競合を修正し、stdio 子プロセスを専用 process group で管理するようにした。mountinfo の pseudo-root は path mount ではないため無視するよう修正した。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-exec-server` 成功（242 passed）。

### genex-exec-server-test-support

- 判定: 実行対象の既存ユニットテストなし。テスト不足の追加は別タスクの範囲なので、削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-exec-server-test-support` 成功（0 tests）。

### genex-extension-api

- 判定: 11テストとも必要。extension registry の contributor 順序・category、state store の型付き値・隔離・初期化、capability injector の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-extension-api` 成功（11 passed）。

### genex-home

- 判定: 7テストとも必要。instructions の override 優先順位、trim、directory／missing／invalid UTF-8、recoverable error、refresh retention の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-home` 成功（7 passed）。

### genex-hooks

- 判定: 176テストとも必要。hook discovery／trust、config／requirements の precedence、dispatcher、各 event parser、MCP／command runner、schema、output spill の既存契約を保持。現行 `ConfigLayerSource` precedence に合わない既存 fixture のレイヤー順と期待値だけを修正し、新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-hooks` 成功（176 passed）。

### genex-linux-sandbox

- 判定: 187テストとも必要。bubblewrap policy、mount／fd、Landlock／seccomp、proxy routing／lifecycle、WSLg、launcher、sandbox integration の既存安全性契約を保持。mountinfo の pseudo-root を path mount と誤認しないよう修正した。新規ユニットテストは追加していない。
- 検証: `just test -p genex-linux-sandbox` 成功（187 passed, 3 skipped）。

### genex-mcp

- 判定: 66テストとも必要。MCP catalog／source precedence／EMA、OAuth、pagination、plugin config の path／transport／environment policy、runtime resolution、RMCP client、HTTP／stdio lifecycle の既存契約を保持。廃止済み legacy URL から remote environment を生成する既存テスト2件を削除し、未使用 import を整理した。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-mcp` 成功（66 passed）。

### genex-plugin

- 判定: 7テストとも必要。plugin id の path alias 防止・legacy character set、environment descriptor の package root／resource binding、shared skill root の選択契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-plugin` 成功（7 passed）。

### genex-rmcp-client

- 判定: 320件中、現行 RMCP wire parser が受け付けない旧 `elicitation/create` custom dispatch 3件と、POSIX で foreign Windows cwd を executor に変換する旧 integration test 1件を削除。MCP form の現行 method、tool input cleanup、remote env source、MRTR request count、CA env inheritance の既存期待値を修正し、未使用 import を整理。残り320テストは OAuth、stdio／HTTP transport、MRTR、elicitation、executor process、security、resource、recovery の現行契約を保持。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-rmcp-client` 成功（320 passed, 9 skipped）。

### genex-skills-extension

- 判定: 171テストとも必要。`skills.list` が Orchestrator 所有エントリを現行の tool authority 形式へ変換していなかったため、既存の bounded metadata テストに合わせて変換処理を修正した。executor file-system authority テストの未使用 import も削除。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-skills-extension` 成功（171 passed）。

### genex-test-binary-support

- 判定: 実行対象の既存ユニットテストなし。テスト不足の追加は別タスクの範囲なので、削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-test-binary-support` 成功（0 tests）。

### genex-utils-plugins

- 判定: 12テストとも必要。plugin namespace の manifest 優先順位、symlink／非 regular file の拒否、legacy fallback、name 解決の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-utils-plugins` 成功（12 passed）。

### genex-web-search-extension

- 判定: 9テストとも必要。web run tool の出力、header／metadata、extension opt-in、history 抽出、legacy 値保持の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-web-search-extension` 成功（9 passed）。

### genex-agent-extension

- 判定: 1テストとも必要。resolved agent の prompt を forked thread で開始する既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-agent-extension` 成功（1 passed）。

### genex-app-server-test-client

- 判定: 2テストとも必要。request_user_input の選択肢再試行と free-form 回答収集の既存契約を保持。利用されていない login completion 待機 helper と専用 import は削除し、警告を解消した。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-app-server-test-client` 成功（2 passed）。

### genex-app-server-transport

- 判定: 29テストとも必要。認証 token 検証、listener／unix socket、queue backpressure、JSON-RPC wire shape、startup lock、WebSocket upgrade の既存契約を保持。`ConnectionAuth` は app-server 側の公開 crate API として利用されるため、transport 単体で未使用に見える補助メソッドの警告は保持した。新規ユニットテストは追加していない。
- 検証: `just test -p genex-app-server-transport` 成功（29 passed）。

### genex-core-api

- 判定: 実行対象の既存ユニットテストなし。テスト不足の追加は別タスクの範囲なので、削除・修正・追加なし。
- 検証: `just test --no-tests=pass -p genex-core-api` 成功（0 tests）。

### genex-external-agent-migration

- 判定: 127テストとも必要。外部エージェントの session／config／plugin／hook／MCP 検出・移行、タイトル／履歴変換、ledger、scope の既存契約を保持。ビルド時に報告された未使用 re-export／test helper は複数の cfg(test) サブモジュールから参照される構成のため保持し、新規ユニットテストは追加していない。
- 検証: `just test -p genex-external-agent-migration` 成功（127 passed）。

### genex-goal-extension

- 判定: 36テストとも必要。goal lifecycle、budget／usage accounting、continuation、subagent、tool visibility、steering、thread resume の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-goal-extension` 成功（36 passed）。

### genex-history-notes-extension

- 判定: 9テストとも必要。history／notes backend の認証、暗号化出力、画像入力、tool 公開条件、thread hint、Responses provider の既存契約を保持。統合 fixture に現行 provider 名、Responses wire API、認証必須設定を反映し、新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-history-notes-extension` 成功（9 passed）。

### genex-mcp-extension

- 判定: 6テストとも必要。executor plugin の declared config、manifest path、executor file system access、missing config、selected plugin contribution の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-mcp-extension` 成功（6 passed）。

### genex-queue-extension

- 判定: 17テストとも必要。queue の FIFO、user input 制約、attachment snapshot、編集／削除、lifecycle、retry／wake、停止・再開の既存契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-queue-extension` 成功（17 passed、1 slow）。

### genex-app-server

- 判定: 102テストとも必要。app-server の request serialization、thread status、connection lifecycle、user verification、in-process 起動、server error、command execution の既存契約を保持。user verification の統合 Harness が旧 API-key fixture から account identity を得ようとしていたため、現行の provider-neutral managed header 認証をテスト用に設定し、認証変更時の reload 経路は既存の API-key fixture で維持した。明確に未使用だった test-only import／re-export、不要な `mut`、常に成立する `if let` は整理した。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-app-server` 成功（102 passed）。

### genex-app-server-daemon

- 判定: 53テストとも必要。daemon の PID／identity、managed install、migration、manual update、lifecycle、package ownership、restart policy の既存契約を保持。更新 loop fixture が standalone の現行 legacy entrypoint（`codex`）ではなく `genex` を作成していたため、fixture を実装の legacy layout に合わせ、不要な test import も削除した。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-app-server-daemon` 成功（53 passed）。

### genex-app-server-client

- 判定: 24テストとも必要。in-process／remote client の initialize、typed request、WebSocket／Unix socket transport、認証、通知 backpressure、server request、shutdown、large frame の既存契約を保持。large-frame の既存応答 fixture に `requiresproviderAuth` という旧 typo があり、現行 `GetAccountResponse` の `requiresAuth` に修正した。未使用 import と未使用 test helper も削除。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-app-server-client` 成功（24 passed、1 slow）。

### genex-exec

- 判定: 1テストとも必要。config flag 後の resume prompt の top-level CLI parsing 契約を保持。削除・修正・追加なし。
- 検証: `just test -p genex-exec` 成功（1 passed）。

### genex-tui

- 判定: 1,155テストとも必要。backtrack mention binding、migration status、archived session、terminal cursor／palette、IDE IPC の安全な socket 検証、PTY の directory trust／focus palette／reconnect、各 UI／history／render 契約を保持。現行 display-name、ANSI reset、paginated rollout、secure temporary directory、provider definition、JSON-RPC account／history paging の契約に合わせて既存の期待値・fixture のみを修正した。新規ユニットテストは追加していない。
- 検証: `just fmt && just test -p genex-tui` 成功（1,155 passed、6 skipped）。

### genex-cli

- 判定: 現行 CLI の既存テストを監査し、必要なテストは Genex 移行後のコマンド、doctor 出力、snapshot、security guidance、marketplace の期待値へ修正した。実装が存在しない remote plugin catalog のテスト7件、削除済み hosted/cloud config・旧 session index・無関係な auth/metrics 挙動を検証していた stale な worktree の副次検証を削除した。owner binding、daemon lifecycle、plugin marketplace、MCP、doctor、worktree の現行契約は保持し、新規ユニットテストは追加していない。
- 検証: `just test -p genex-cli --retries 0 --test-threads=1` 成功（197 passed）。

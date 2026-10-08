# 対話 CLI の全体フロー（2往復の例）

この文書は、`codex` を起動してユーザーと2往復の対話を行う場合の流れを、設定・認証・スレッド・MCP・プラグイン・スキル・永続化の関係が見えるようにまとめたものです。

ここでいう「2往復」は、次の2つのユーザー入力を指します。

```text
ユーザー入力 A → モデル応答 A
ユーザー入力 B → モデル応答 B
```

実際には、1つのユーザー入力に対してモデルがツールを呼び出すと、ツール結果を渡すために同じターン内で複数回モデルへリクエストすることがあります。

## まず全体像

```text
CLI 起動
  │
  ├─ 引数を解析
  ├─ CODEX_HOME / プロファイル / config.toml を読み込む
  ├─ 認証状態を参照できる状態にする
  ├─ app-server を起動または接続する
  ├─ account / model / config requirements を取得する
  └─ スレッドを開始する
       │
       └─ Core の Session を作る
            ├─ Thread / rollout / SQLite の準備
            ├─ プラグイン、スキル、AGENTS.md を準備
            ├─ MCP ランタイムを構築し、必要なサーバーへ接続
            └─ 起動時の prewarm を実行

1回目の入力 A
  └─ Turn 1
       ├─ 現在の設定・環境・ツールをスナップショット
       ├─ 入力中で参照されたスキル／プラグイン／MCP を解決
       ├─ 必要なら認証付きでモデルへ /responses を送信
       ├─ ツール呼び出し、承認、sandbox 実行、結果の再送信
       └─ ユーザー入力・モデル応答・ツール結果を記録

2回目の入力 B
  └─ Turn 2
       ├─ 同じ Thread / Session を再利用
       ├─ 変更された設定や MCP だけを必要に応じて更新
       ├─ Turn 1 と同じ手順でモデル／ツールを実行
       └─ 追加の履歴を記録して応答を表示
```

大きく分けると、TUI は表示とユーザー入力を担当し、app-server はプロトコルの窓口、Core はセッション・ターン・ツール実行の本体、`genex-api` / provider はモデル通信を担当します。

関連資料: [システムマップ](00-system-map.md)、[CLI エントリポイント](01-cli-entrypoint.md)、[対話コマンド](commands/interactive.md)

## 1. 起動直後: 設定と通信経路を決める

### 1.1 CLI が引数を解析する

`codex` のエントリポイントは、CLI 引数、サブコマンド、環境変数由来の上書き値を解析します。サブコマンドが指定されていない通常の対話起動では、TUI が選ばれます。

主な流れは次のとおりです。

```text
argv
  → clap の解析
  → CLI 上書き値（モデル、profile、sandbox、承認など）
  → CODEX_HOME と作業ディレクトリの解決
```

ここではまだモデルリクエストは送信されません。

### 1.2 設定を段階的に読み込む

設定はおおむね次の順に有効値へ組み立てられます。

```text
組み込みのデフォルト
  → ユーザー設定（profile / config.toml）
  → 作業ディレクトリなどの環境に依存する設定
  → CLI 引数・環境変数による上書き
  → ConfigBuilder
  → 実行時に使う immutable な Config
```

起動時には、app-server 接続先や daemon の利用可否を判断するための bootstrap config が先に必要です。その後、TUI や app-server の初期化時に、より完全な設定が組み立てられます。

設定は1回だけ読まれるわけではありません。trust、プロフィール選択、設定変更、thread 開始などの境界で再ロードされるため、起動時に読んだ設定とターン実行時の設定が同一とは限りません。

詳細: [設定](common/configuration.md)

### 1.3 app-server を起動または接続する

TUI は設定に応じて、次のいずれかの app-server に接続します。

- ローカル daemon
- リモート app-server
- daemon を使わない場合の embedded / in-process app-server

いずれも TUI から見れば、`initialize` / `initialized` のプロトコルで接続する同じ窓口です。embedded の場合も、app-server の処理を経由するため、TUI が直接すべての Core API を呼ぶ構造ではありません。

app-server の初期化では、設定の妥当性を確認し、`CredentialManager`、`MessageProcessor`、`ThreadManager`、モデル管理、スキル監視、MCP・プラグイン関連のプロセッサを構築します。

## 2. 起動時の認証: 「ログイン」と「リクエスト認証」は別

認証は1つの処理ではなく、次の3層に分かれます。

### 2.1 ローカルの認証状態を読む

app-server / Core は `CredentialManager` を作り、ログイン済みか、どの認証方式を使うかを参照できる状態にします。この段階では、通常はまだモデル API を呼びません。

保存先は設定と環境に依存しますが、代表例は次のとおりです。

```text
CODEX_HOME/auth.json       ログイン情報のファイル保存
OS の keyring              利用可能な場合の安全な保存先
ephemeral                  永続化しない実行
```

`codex login` はこのローカル認証状態を作成・更新するコマンドです。認証状態の詳細は [認証](common/authentication.md) を参照してください。

### 2.2 アカウント情報を app-server 経由で取得する

TUI の初期表示では `account/read` が送られます。続いて `model/list`、`configRequirements/read`、利用可能な collaboration mode などを並行して取得します。

このとき、認証が必要な環境では保存済みの認証状態が使われます。ログインが必要、期限切れ、または利用可能な認証方式がない場合は、ここでログイン導線やエラーになります。

### 2.3 モデルリクエストの直前に認証ヘッダーを作る

実際のモデルリクエスト時には、`genex-api` の `AuthProvider` が現在の provider 設定と `CredentialManager` の状態から、リクエスト用の認証情報を作ります。

```text
保存済み認証状態
  → リクエスト直前の AuthProvider
  → Authorization / provider 固有ヘッダー
  → /responses または WebSocket のモデル接続
```

したがって、「起動時に認証処理が1回だけ走って、その結果を全ターンで固定利用する」という理解ではありません。起動時には状態確認、各モデルリクエスト時には送信可能な資格情報の解決が行われます。期限切れ時の更新や再試行が入る場合もあります。

なお、provider API の認証と MCP サーバー自身の OAuth 認証は別系統です。MCP OAuth の資格情報は、設定により `CODEX_HOME/.credentials.json` や keyring に保存されます。

## 3. Thread と Session を作る

用語を分けると、流れが見やすくなります。

| 用語 | 意味 |
| --- | --- |
| Thread | 会話の永続的な単位。ユーザー入力、応答、ツール結果の履歴を持つ |
| Session | Thread を実行するランタイム。設定、MCP、プラグイン、スキル、フックなどを束ねる |
| Turn | 1つのユーザー入力を起点に、モデル・ツール・再モデル呼び出しが完了するまでの単位 |

TUI が `thread/start` を送ると、app-server は必要に応じて設定を再ロードし、trust や実行環境を解決して `ThreadManager` に Thread 開始を依頼します。

Session 初期化では、おおむね次の処理が行われます。

1. Thread のメタデータ、rollout、状態 DB を準備する。
2. 認証、MCP、プラグイン、スキル、`AGENTS.md`、フックを初期化する。
3. 初期 MCP ランタイムを構築し、必要なサーバーを起動する。
4. 初期履歴を記録し、後続の Turn が実行できる状態にする。

通常の対話では、1回目と2回目の入力は同じ Thread / Session に入ります。`--ephemeral` の場合は Thread の永続化を抑制します。

## 4. プラグイン、スキル、MCP はいつ集めるか

似た概念に見えますが、情報が集まるタイミングと粒度が異なります。

### 4.1 プラグイン

app-server の構築時に、プラグイン管理機能と設定を扱うプロセッサが作られます。Session 初期化では有効なプラグインを解決し、プラグインが提供する次の情報をランタイムへ反映します。

- skill roots と skill snapshot
- MCP server 定義
- hooks
- capability summary

マーケットプレイスの設定によっては、プラグインの startup task や自動更新がバックグラウンドで開始されます。これは通常のモデル推論を待ってから行う処理ではありません。

一方、TUI に表示するプラグイン一覧は、画面や操作が必要としたときに `plugins/list` 相当の要求で取得されます。表示用一覧の取得と、実行用プラグインの解決は同じタイミングとは限りません。

### 4.2 スキル

スキルには「一覧・メタデータ」と「本文」があります。

```text
起動後
  → skills/list（メタデータを取得、通常はバックグラウンド）

各 Turn
  → 入力に明示的なスキル参照があるか判定
  → 参照されたスキルだけ SKILL.md 本文を読み込む
  → モデルへの追加コンテキストとして注入
```

起動時にはスキル root の監視も始まり、ファイル変更時にはキャッシュが無効化されます。ただし、すべての `SKILL.md` 本文を起動時にモデルコンテキストへ投入するわけではありません。入力で参照されたスキルを Turn 実行時に展開する、という理解が安全です。

### 4.3 MCP

MCP は、まず設定・プラグインから server 定義を集めてランタイムを作ります。初期ランタイムの接続ポリシーが eager のサーバーでは、MCP の `initialize` と `tools/list` が実行され、ツールカタログが保存されます。

```text
設定 / プラグイン
  → MCP server 定義をマージ
  → MCP runtime を publish
  → 必須サーバーを検証
  → initialize
  → tools/list
  → ツールルーターへ登録
```

すべてのサーバーが必ず起動直後に接続されるとは限りません。eager / deferred / cached のポリシー、明示的なスキル・プラグイン・MCP 参照、ランタイムの dirty 状態によって、Turn 開始時に接続・更新される場合があります。

MCP の OAuth 認証は、モデル API の認証とは別に MCP 接続時に必要になります。

## 5. 1回目の入力 A

ユーザーが最初のメッセージを送ると、TUI は app-server に Turn 開始要求を送ります。Core の `run_turn` は次のように動きます。

### 5.1 Turn の準備

1. 保留中の hook や前回処理を整理する。
2. 必要ならコンパクションを行う。
3. 現在の設定、Thread 状態、実行環境、MCP／プラグイン要求をスナップショットする。
4. 入力 A の中に、特定のスキル、プラグイン、MCP を明示的に参照する記述があるか判定する。
5. 必要なスキル本文を読み込み、プラグイン由来の追加情報を組み立てる。
6. MCP ツールと組み込みツールを含む、その Turn のツールセットを構築する。

入力中に明示的な参照がなければ、全スキル本文を読み込むとは限りません。既に作られたツールカタログや Session のキャッシュは再利用されますが、設定変更やファイル変更を検出すると更新されます。

### 5.2 モデルへリクエストする

Core は過去の履歴、入力 A、システム指示、スキル／プラグイン注入、利用可能ツールからモデル入力を作ります。

```text
Thread 履歴 + 入力 A + 実行時コンテキスト
  → Responses 用リクエスト
  → request-time authentication
  → provider の /responses または WebSocket
```

ストリーミングされたテキストは TUI に表示されます。

### 5.3 ツール呼び出しがある場合

モデルがツール呼び出しを返すと、Core は次を行います。

```text
モデルの tool call
  → ツールルーター
  → approval 判定
  → sandbox / process / filesystem / network 実行
  → tool output を履歴へ記録
  → tool output を含めて同じ Turn で再度モデルへ送信
```

そのため、ユーザーから見える「1回の回答」の裏側で、モデル API リクエストが複数回発生することがあります。

### 5.4 Turn 1 の永続化

ユーザー入力、モデルの response item、ツール呼び出し、ツール結果、エラーなどは履歴イベントとして記録されます。必要なタイミングで rollout が materialize され、Thread metadata とともに保存されます。

## 6. 2回目の入力 B

ユーザーが続けて入力 B を送ると、通常は新しい app-server や Thread を作りません。

```text
同じ app-server
  → 同じ Thread
  → 同じ Session
  → 新しい Turn 2
```

Turn 2 でも、入力 B の前に現在の設定やランタイム状態を確認します。Turn 1 の実行で MCP 接続、ツールカタログ、プラグイン解決結果、スキルキャッシュが有効なら再利用します。変更が検出された場合だけ、該当部分を更新します。

その後は Turn 1 と同じく、入力 B に必要なスキル／プラグインを展開し、履歴を含むモデルリクエストを送り、必要ならツールループを実行し、結果を永続化します。

つまり、2往復の境界は「モデル API の2リクエスト」ではなく、「ユーザー入力を起点とする2つの Turn」です。

## 7. 何がいつ保存されるか

| 対象 | 保存・更新のタイミング | 代表的な場所・形式 |
| --- | --- | --- |
| CLI 設定 | 起動時、設定変更時、必要な再ロード時 | `CODEX_HOME/config.toml`、profile、CLI 上書き |
| ログイン状態 | `login`、logout、更新、認証回復時 | `CODEX_HOME/auth.json`、OS keyring |
| MCP OAuth | MCP OAuth の認証・更新時 | `CODEX_HOME/.credentials.json`、OS keyring など |
| プラグイン状態 | プラグイン解決時、startup task / 更新時 | plugin cache / marketplace 関連の保存領域 |
| スキル一覧・キャッシュ | `skills/list`、ファイル変更検知時 | メモリ上の snapshot / watcher cache |
| MCP ツールカタログ | MCP `tools/list` 後、更新時 | Session の MCP runtime / catalog |
| Thread metadata | Thread 開始・更新時 | SQLite の thread history 系 DB |
| 会話とツールの履歴 | 入力・応答・ツール結果の記録時 | rollout JSONL、通常は `CODEX_HOME/sessions/` |
| 一時実行 | 実行中のみ | `--ephemeral` では Thread の永続化を抑制 |

SQLite のファイル名や細かい保存先はバージョン・設定で変わり得るため、利用時は [永続化と rollout](common/persistence-and-rollout.md) と現在の `CODEX_HOME` 設定を確認してください。

## 8. 誤解しやすい点

- `codex login` と、モデル API リクエスト直前の認証ヘッダー作成は別の処理です。
- app-server の起動と Thread の開始は別です。app-server は複数の Thread を扱えます。
- Thread と Session は同じものではありません。Thread は会話の単位、Session はその実行環境です。
- スキル一覧を取得することと、`SKILL.md` 本文をモデルへ注入することは別です。
- プラグイン一覧を TUI に表示することと、プラグイン由来の skill / MCP / hook をランタイムへ組み込むことは別です。
- MCP のサーバー認証と、モデル provider の認証は別です。
- 1 Turn は必ずしも1回のモデルリクエストではありません。ツール呼び出し、再試行、ストリーミング方式によって複数回になります。
- 起動時の WebSocket prewarm は、接続や認証を温める処理であり、ユーザー入力に対するモデル推論そのものではありません。

## 9. メモリーズ機能の扱い

このリポジトリでは、長期的な `memories` 機能は削除済みである。以下は削除範囲と、通常の会話履歴との境界を記録したものだ。

ここで削除対象にするのは、過去の Thread から長期メモリーを生成し、別の Thread の developer context へ注入する機能である。通常の Thread 会話履歴、rollout、Thread metadata、Session 内の ContextManager は対話の基本機能なので削除対象ではない。

削除対象の主な範囲は次のとおり。

- `[features].memories` と `[memories]` の設定・機能フラグ
- `codex-memories-read` / `codex-memories-write` と `codex-ext-memories`
- rollout から長期メモリーを抽出・統合する Phase 1 / Phase 2 のバックグラウンド処理
- `CODEX_HOME/memories/`、`memories_v2/` と専用 memories SQLite の保存領域
- `memory_summary.md`、`MEMORY.md`、`rollout_summaries/` の読み込み・注入
- 専用メモリーツール、memory reset / status API、TUI のメモリー設定画面
- Thread の `memory_mode`、`thread/memoryMode/set`、メモリー専用 telemetry と互換処理

一方、次は維持する。

- Thread の会話履歴と rollout JSONL
- Thread metadata と通常の SQLite state
- Session / Turn の ContextManager と compaction
- MCP、プラグイン、スキルなど、メモリーズ機能とは独立したコンテキスト拡張

この方針では、メモリーズ機能を無効化するだけでなく、生成・読み込み・保存・UI・プロトコル・永続化スキーマを一体として除去する。既存 Thread の会話履歴を壊さず、メモリーズ専用データだけを整理することが前提となる。

## 参考ソース

実装上の主な入口は次のファイルです。

- [`crates/cli/src/main.rs`](../../../crates/cli/src/main.rs): CLI エントリポイント
- [`crates/tui/src/startup_orchestration.rs`](../../../crates/tui/src/startup_orchestration.rs): 起動・設定・接続先の調整
- [`crates/tui/src/app_server_session.rs`](../../../crates/tui/src/app_server_session.rs): app-server との Thread / account 通信
- [`crates/app-server/src/in_process.rs`](../../../crates/app-server/src/in_process.rs): embedded app-server の初期化
- [`crates/app-server/src/message_processor.rs`](../../../crates/app-server/src/message_processor.rs): app-server の各サービス構築
- [`crates/core/src/session/session.rs`](../../../crates/core/src/session/session.rs): Session 初期化、MCP・プラグイン・スキル準備
- [`crates/core/src/session/turn.rs`](../../../crates/core/src/session/turn.rs): Turn 実行とモデル／ツールのループ
- [`crates/core/src/client.rs`](../../../crates/core/src/client.rs): provider 接続、認証、モデルリクエスト
- [`crates/genex-mcp/src/rmcp_client.rs`](../../../crates/genex-mcp/src/rmcp_client.rs): MCP initialize / tools/list

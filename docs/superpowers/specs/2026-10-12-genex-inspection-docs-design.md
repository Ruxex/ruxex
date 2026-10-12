# Genex Inspection Documentation Site Design

## 目的

Genexの現在の設計と実行時の振る舞いを、実装の細部を網羅するのではなく、システムを自分で設計・実装した人が持つべきメンタルモデルとして学べるドキュメントサイトを `docs/inspection` に作る。

読者は、まず起動経路と主要な実行単位を理解し、その後にチャットループ、コンテキスト収集、永続化、Provider境界、プロトコル、セキュリティを関連づけて学べるようにする。

## 設計方針

- ドキュメント本文はMarkdownを中心にし、コードの行単位の説明ではなく、責務・ライフサイクル・データの流れを説明する。
- 各ページは「役割」「呼び出し時点」「入力と出力」「状態」「境界」「主要ソース」「注意点」「関連ページ」の順で記述する。
- 具体的なコード上の根拠を示すため、主要なページからRustのソースファイルへ相対リンクする。
- チャットループ、コンテキスト収集、DBとRolloutの関係は、テキストだけでなくシーケンス図・構成図・データマップで説明する。
- 現在のGenexの実装を記述し、将来の単一Provider移行方針や過去の互換層を、現行挙動と混同しないように区別する。

## サイト技術

ReactベースのDocusaurusを使用する。

- `docs/inspection/package.json` にサイト専用のpnpm依存関係を置く。
- Docusaurusのclassic presetで、ドキュメントを `docs/`、設定とテーマを `.docusaurus/` 配下に置く。
- 日本語を主言語とし、検索・目次・前後ページナビゲーションを有効にする。
- MermaidはDocusaurusのMarkdown拡張として利用し、チャットループ・データ保存・実行境界を図示する。
- Reactコンポーネントは、図・表・コード参照などMarkdownだけでは表現しにくい箇所に限定する。
- リポジトリの既存Nodeプロジェクトとは独立して、`docs/inspection` から `pnpm start`、`pnpm build`、`pnpm serve` を実行できるようにする。

## 情報設計

### ガイド

- 読み方と学習順路
- 用語集
- Genexシステムマップ

### 実行モデル

- CLI・TUI・App Server・Desktopの関係
- 起動とRuntime構築
- AppServerとProviderの共有境界

### チャット処理

- チャットループ
- Context Assembly
- Streamingとイベント処理
- Tool Loop
- Compaction
- エラーとキャンセル

### 永続化

- 保存先全体図
- SQLiteの構成
- テーブル一覧
- Rollout JSONL
- MigrationとBackfill

### Provider

- Provider Contract
- 認証とCredential Snapshot
- Model Catalog
- Provider Home

### Capability

- Tools
- MCP
- Plugins
- Skills
- Sandboxと権限

### Protocol

- App Server Protocol
- TUIイベント処理
- Desktopクライアント

### リファレンス

- クレートマップ
- ソースマップ
- 変更時の読み順
- 障害調査と観測性

## 重要な説明対象

### チャットループ

`crates/core/src/session/turn.rs` の `run_turn` を中心に、次の順序を説明する。

1. Turn入力の受理とHook処理
2. 自動Compactionの判定
3. MCP・Plugin・Skillの要求解析
4. Environment、権限、Tool、AGENTS.md、履歴からのStepContext構築
5. ProviderへのStreaming Request
6. Assistant出力・Reasoning・Tool Callのイベント処理
7. Tool結果とSteer入力の履歴への追加
8. 次のSamplingが必要か、Turnを終了するかの判定
9. Token上限、Compaction、Stop Hook、キャンセル、エラーの扱い
10. Rollout、Thread Store、クライアントイベントへの反映

### 保存領域

現行コードでRuntimeが管理するSQLiteは、次の5ファイルとして説明する。

- `state_5.sqlite`: Thread、Project、Section、Attachment、Dynamic Tool、Spawn Edge、Backfill状態
- `logs_2.sqlite`: ログ
- `goals_1.sqlite`: Thread Goal
- `queue_1.sqlite`: 保留ユーザー入力とQueue Revision
- `thread_history_1.sqlite`: Thread Turn、Thread Item、履歴Projection状態

SQLite以外に、Rollout JSONL、Credential保存領域、添付ファイル、キャッシュがあることも明示し、どれが正規履歴でどれが検索・表示用Projectionかを説明する。

### チャット前の情報収集

設定、モデル、Thread履歴、AGENTS.md、作業ディレクトリ、Git情報、Environment、権限、Sandbox、MCP、Plugin、Skill、Tool Router、Shell Snapshot、添付、認証、時刻、Telemetryを、取得元・取得時点・モデル可視性・永続化の有無で比較できる表にする。

## 非目標

- 全Rust関数のリファレンス化
- 既存の一般ユーザー向け利用ガイドの置き換え
- Genexの実装そのものの変更
- サイトを外部ホスティングへ公開すること
- 過去のProvider互換層を詳細に再現すること

## 完了条件

- `docs/inspection` からpnpmでReactベースのDocusaurusサイトを起動・ビルドできる。
- 提案した学習順路で、全体像からチャットループ、コンテキスト、永続化、Provider、Protocolへ進める。
- チャットループ、DB構成、チャット前の情報収集がコード上の根拠とともに説明されている。
- 主要な図と表がサイト上で読める。
- サイト専用の検証コマンドでリンク切れ・ビルドエラーがない。

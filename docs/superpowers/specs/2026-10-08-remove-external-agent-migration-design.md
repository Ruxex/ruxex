# 外部エージェント移行機能削除 設計仕様

## 目的

Genex未リリースのため、Claude CodeやCursorなど別のAIエージェントから設定・Plugins・MCP・履歴を取り込む外部エージェント移行機能を、リポジトリから完全に削除する。

移行機能を削除しても、Genex自身の設定管理、MCP管理・実行、Plugins、Skills、Hooks、通常のセッション履歴、モデルプロバイダ認証には影響させない。

## 現状の到達範囲

対象機能は次の層で構成されている。

- `genex-external-agent-migration` crate
- app-serverの検出・インポート・履歴記録RPC
- app-serverの進捗・完了notification
- TUIの`/import` command、移行選択画面、app-server client wrapper
- app-server protocolのRust型、JSON schema、TypeScript schema
- `external_agent_config_imports` state tableとruntime API
- 外部エージェント移行専用のテスト、Cargo依存、ドキュメント記録

対応元は現在、デフォルトのClaude Codeと明示指定可能なCursorである。これは汎用的なエディタ設定インポートではなく、AIエージェント固有の設定・履歴変換機能である。

## 設計

### 1. 移行crateと専用処理の削除

`crates/external-agent-migration`をcrate単位で削除する。これに含まれるClaude/Cursorの設定変換、MCP変換、Plugins/marketplace変換、Hooks、Commands、Subagents、Skills、セッション検出・インポート、移行metricを削除する。

app-serverからは、移行processor/module、processor用state、thread managerやconfig processorへの初期化処理、request dispatch、移行notification処理を削除する。

### 2. TUI入口の削除

TUIから以下を削除する。

- `/import` slash commandのenum・説明・補完・dispatch・テスト
- 移行画面、選択モデル、source選択、flow、render、関連テスト
- `OpenExternalAgentConfigMigration` app event
- app-server clientの移行RPC wrapperと完了フラグ
- 移行notificationのevent target・event dispatch・表示処理

`/import`が他用途で使われていないことを確認し、command名だけを残す互換層は作らない。

### 3. Protocolと生成物の削除

app-server protocolから外部エージェント移行専用の以下を削除する。

- `externalAgentConfig/detect`
- `externalAgentConfig/import`
- `externalAgentConfig/import/recordHistory`
- `externalAgentConfig/import/readHistories`
- `externalAgentConfig/import/progress`
- `externalAgentConfig/import/completed`
- それらのRust型、Protocol enum variant、JSON schema、TypeScript schema

手編集で生成物だけを変更せず、producerの削除後に`just generate`で生成物を更新する。

### 4. Stateの削除

Genexは未リリースであり、過去に作成された利用者DBを維持する必要はない。そのため、以下をすべて削除する。

- `external_agent_config_imports` runtime API・record型・テスト
- state moduleのre-export
- `0038_external_agent_config_imports.sql`
- `0044_external_agent_config_imports_provider_id.sql`
- Cargo.lock上の移行crate依存

cleanup用の新しい`DROP TABLE` migrationは追加しない。

### 5. ドキュメントと監査記録

`docs/provider-decoupling-candidates.md`と`docs/pre-release-compatibility-inventory.md`に、producer/consumer監査結果と`remove`決定を記録する。移行機能をretainする既存記述は削除またはremove判断へ更新する。

## 非対象

- モデルプロバイダの認証、アカウント、Usage、モデル一覧
- MCPサーバの設定、OAuth、資格情報、実行
- GenexのPlugins、Skills、Hooksの通常機能
- Genex自身の設定・履歴・app-server protocol
- OpenAI endpoint固有の外部契約

## 検証方針

- `rg`で移行crate、RPC名、TUI入口、state table名のproduction残存がないことを確認する。
- `just generate`を実行し、checked-in生成schemaの差分を確認する。
- Rust削除・refactor後に`just check --tests`を実行する。
- 最終的に`just check`を実行する。
- standalone testは実行しない。これは本リポジトリのcrate削除方針に従う。

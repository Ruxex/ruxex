# LM Studio / Ollama 削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** LM StudioとOllamaをCodexの組み込みローカルプロバイダ機能から完全に削除し、汎用のユーザー定義model provider機能は保持する。

**Architecture:** `codex-lmstudio`、`codex-ollama`、`codex-utils-oss`を削除し、model-provider-infoの組み込みOSS登録、CLI/TUIの選択・ready check、`oss_provider`設定を除去する。任意の`model_providers`設定とResponses APIの汎用provider構成は変更しない。

**Tech Stack:** Rust, Cargo workspace, `codex-model-provider-info`, CLI/TUI config, generated JSON schema, `just check`, `just generate`。

**Spec:** `docs/openai-decoupling-plan.md`, `docs/pre-release-compatibility-inventory.md`

## Global Constraints

- `codex-model-provider-info`の汎用custom provider構成は保持する。
- `oss_provider`、`--local-provider`、LM Studio/Ollama自動検出とready checkは削除する。
- pre-release forkのため、`ollama-chat`からの移行・拒否だけを目的とする互換層も削除する。
- テストコマンドは実行せず、`just check`と必要時の`just generate`を使う。

## Review Focus

- 組み込みprovider一覧からLM Studio/Ollamaだけが消え、OpenAI、Bedrock、custom providerが残ること。
- TUI起動時にOSS provider選択や自動ポート検出が残らないこと。
- `model_providers`の任意URL・任意auth設定を壊さないこと。
- archive/session操作に残るOSS専用分岐がないこと。
- workspace dependency graphから3クレートと不要依存が消えること。

### Task 1: OSS providerのproducer/consumerと依存グラフを確定する

**Files:**
- Inspect: workspace `Cargo.toml`
- Inspect: `crates/lmstudio/Cargo.toml`, `crates/ollama/Cargo.toml`, `crates/utils/oss/Cargo.toml`
- Inspect: `crates/model-provider-info/src/lib.rs`
- Inspect: `crates/exec/src/lib.rs`, `crates/tui/src/oss_selection.rs`, `crates/tui/src/startup_orchestration.rs`
- Inspect: `crates/config/src/config_toml.rs`, `crates/core/src/config/mod.rs`

  - [x] **Step 1: workspace逆依存を確認する**

  Cargo metadata/treeで`codex-lmstudio`、`codex-ollama`、`codex-utils-oss`のproduction reverse dependencyを記録し、`exec`、`tui`、config、model-provider-infoの削除境界をinventoryに記録する。

  - [x] **Step 2: generic provider helperの扱いを決める**

  `create_oss_provider_with_base_url`のproduction consumerを確認する。OSS built-in専用なら削除し、custom provider構成に必要ならprovider-neutralな名前と実装へ移行する。

### Task 2: workspaceからLM Studio/Ollamaクレートを削除する

**Files:**
- Modify: workspace `Cargo.toml`
- Delete: `crates/lmstudio/`
- Delete: `crates/ollama/`
- Delete: `crates/utils/oss/`
- Modify: `crates/exec/Cargo.toml`
- Modify: `crates/tui/Cargo.toml`

  - [x] **Step 1: workspace membersと依存を削除する**

  3クレートのworkspace member、workspace dependency、直接依存を削除する。不要になった`which`、`wiremock`などは他のconsumerを確認してから削除する。

  - [x] **Step 2: execのOSS起動処理を削除する**

  `resolve_oss_provider`、ready check、default OSS model、`--local-provider`分岐、OSS専用archive処理を削除し、通常のprovider resolutionを残す。

  - [x] **Step 3: TUIのOSS選択処理を削除する**

  `oss_selection.rs`、起動時のポート検出・provider選択・model download/update処理を削除する。通常のstartup orchestrationとarchiveは維持する。

### Task 3: provider catalogとconfig/CLI surfaceを整理する

**Files:**
- Modify: `crates/model-provider-info/src/lib.rs`
- Modify: `crates/config/src/config_toml.rs`
- Modify: `crates/core/src/config/mod.rs`
- Modify: `crates/utils/cli/src/shared_options.rs`
- Modify: `crates/cli/src/main.rs`
- Modify: generated config schema files

  - [x] **Step 1: built-in provider entriesを削除する**

  `lmstudio`、`ollama`、専用port、OSS-only factoryを組み込みprovider mapから削除する。OpenAI、Bedrock、custom provider登録は残す。

  - [x] **Step 2: config fieldsとlegacy rejectionを削除する**

  `oss_provider`、`validate_oss_provider`、`ollama-chat`移行エラー、default provider setterを削除する。

  - [x] **Step 3: CLI optionとhelpを削除する**

  `--local-provider`および関連するcompletion/help/schema記述を削除する。app-server remoteなど無関係なCLI optionは変更しない。

### Task 4: fixture、docs、生成物を整理して検証する

**Files:**
- Modify/Delete: OSS-related tests and fixtures under `crates/core`, `crates/tui`, `crates/exec`, `crates/config`
- Modify: relevant docs and snapshots
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/openai-decoupling-plan.md`

  - [x] **Step 1: productionとfixtureの残存参照を削除する**

  `rg -n -i 'lmstudio|ollama|oss_provider|local-provider|ollama-chat' crates docs`で残存を分類し、custom provider例や一般名詞を誤削除しない。

  - [x] **Step 2: generated artifactsを更新する**

  config schemaやCLI snapshotに影響した場合は`just generate`を実行し、生成差分を確認する。

- [ ] **Step 3: repository checkを実行する**

  `just check`を実行する。テストコマンドは実行しない。

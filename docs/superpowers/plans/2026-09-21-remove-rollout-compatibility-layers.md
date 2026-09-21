# Rollout / Thread Store 互換レイヤー削除 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 現役の rollout/thread-store クレートを保持したまま、pre-release forkに不要な旧session index、非推奨alias、旧名前検索読み取り経路を削除する。

**Architecture:** SQLite/thread-storeをスレッドの正規メタデータ源とし、現行のJSONL replay、filesystem repair fallback、permission compatibilityは保持する。`session_index.jsonl`だけをproducer/consumer監査後に廃止し、CLIの名前解決と検索を現行のthread-store APIへ寄せる。

**Tech Stack:** Rust, `codex-rollout`, `codex-thread-store`, SQLite, JSONL rollout storage, `rg`, Cargo metadata, `just check`。

**Spec:** `docs/openai-decoupling-plan.md`, `docs/pre-release-compatibility-inventory.md`

## Global Constraints

- `codex-rollout`、`codex-thread-store`、`codex-rollout-trace` のクレート全体は削除しない。
- 現行のJSONL replay、SQLite projection、filesystem repair fallback、legacy sandbox policy parsingは個別根拠なしに削除しない。
- pre-release forkのため、旧persistedデータを読むためだけの互換readerは削除する。
- テストコマンドは実行せず、`just check`を使用する。

## Review Focus

- named session lookupがSQLiteのタイトル検索だけで同じ結果を返すことを、production consumer監査で確認する。
- rollout searchの名前フィルタがsession indexなしでthread-storeのcanonical metadataを使うことを確認する。
- delete pathが存在しないsession indexを前提にしないことを確認する。
- filesystem fallbackとsession-index互換readerを混同せず、前者を保持する。
- `ConversationItem` / `ConversationsPage` aliasと公開re-exportの削除後に生成APIが壊れていないことを確認する。

### Task 1: 互換レイヤーのproducer/consumer監査と棚卸し更新

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Inspect: `crates/rollout/src/session_index.rs`, `crates/rollout/src/list.rs`, `crates/rollout/src/lib.rs`
- Inspect: `crates/exec/src/lib.rs`, `crates/core/src/rollout.rs`, `crates/thread-store/src/local/delete_thread.rs`

- [ ] **Step 1: production symbol usageを列挙する**

  `append_thread_name`、`find_thread_meta_by_name_str`、`find_thread_names_by_ids`、`find_thread_name_by_id`、`remove_thread_name_entries`、`ConversationItem`、`ConversationsPage`、`INTERACTIVE_SESSION_SOURCES`を`--glob '!**/*test*'`で検索し、producerとconsumerを表にする。

- [ ] **Step 2: canonical replacementを確定する**

  named lookupは既存の`state_db.find_thread_by_exact_title`とthread-storeのcanonical metadata/searchを使用し、session indexの読み取りを代替する。現行writerがないことを確認したうえで、session indexのreader/writerを`remove`と記録する。

- [ ] **Step 3: inventoryにdecisionと根拠を記録する**

  persisted rollout/thread-history sliceの順序を保ち、各production matchについてproducer、consumer、`remove`または`retain`、検証コマンドを記録する。

### Task 2: 小さな非推奨aliasと未使用source定義を削除する

**Files:**
- Modify: `crates/rollout/src/list.rs`
- Modify: `crates/rollout/src/lib.rs`
- Modify: `crates/core/src/rollout.rs`
- Modify: `crates/core/src/lib.rs`
- Update: related compile fixtures and snapshots only when symbol references require it

- [ ] **Step 1: deprecated conversation aliasesを削除する**

  `ConversationItem`、`ConversationsPage`と、それらのre-exportを削除し、production参照を`ThreadItem`、`ThreadsPage`へ統一する。

- [ ] **Step 2: unused session source constantを削除する**

  production consumerがない`INTERACTIVE_SESSION_SOURCES`と古い`chatgpt`/`atlas` source値を削除する。provider-neutralなsession sourceは残す。

- [ ] **Step 3: stale exportsを削除する**

  production consumerがなくなった`find_thread_name_by_id`などのre-exportを削除し、delete pathで使う`remove_thread_name_entries`をTask 3まで保持する。

### Task 3: session_index互換readerとdelete/search fallbackをcanonical APIへ移行する

**Files:**
- Modify: `crates/rollout/src/session_index.rs`
- Modify: `crates/rollout/src/recorder.rs`
- Modify: `crates/rollout/src/list.rs`
- Modify: `crates/exec/src/lib.rs`
- Modify: `crates/thread-store/src/local/delete_thread.rs`
- Modify: `crates/thread-store/src/local/mod.rs`

- [ ] **Step 1: execの名前解決fallbackをcanonical lookupに置換する**

  `find_thread_meta_by_name_str`に依存するJSONL session-index fallbackを除去し、既存のSQLite exact-title lookupとthread-storeのcanonical list/searchだけで解決する。

- [ ] **Step 2: rollout searchのsession-index name filterをcanonical metadataへ置換する**

  `find_thread_names_by_ids`の読み取りを削除し、thread-storeのthread title/name metadataから同じ検索結果を作る。名前が存在しない場合は空のfilterとして扱う。

- [ ] **Step 3: session indexのdelete処理を削除する**

  `remove_thread_name_entries`と`session_index.jsonl`書き込み・削除処理を削除し、thread-store SQLiteのthread metadata削除だけを正規経路にする。

- [ ] **Step 4: historical readerだけを残していないことを確認する**

  `session_index`、`find_thread_meta_by_name_str`、`append_thread_name`をproduction範囲で再検索し、残存がないことを確認する。filesystem fallbackとpermission legacy parserの残存は想定どおりと記録する。

### Task 4: 生成物と文書を更新する

**Files:**
- Modify: `docs/pre-release-compatibility-inventory.md`
- Modify: `docs/openai-decoupling-plan.md`
- Modify: affected checked-in schemas or snapshots discovered by `rg`

- [ ] **Step 1: compatibility sliceの完了記録を更新する**

  producer/consumer audit結果と削除範囲をinventoryへ反映し、rollout/thread-store本体はretainと記録する。

- [ ] **Step 2: generated artifactsを確認する**

  Rust API/schema生成物が変更された場合は`just generate`を実行し、不要な差分を除去する。

- [ ] **Step 3: repository checkを実行する**

  `just check`を実行する。テストコマンドは実行しない。


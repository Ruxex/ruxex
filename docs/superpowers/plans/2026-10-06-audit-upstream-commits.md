# フォーク元追加コミット監査計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans when continuing this plan task-by-task.

**Goal:** フォーク点以後に公式 `openai/codex` へ追加されたコミットを取得し、Genex 化済みの本リポジトリへ取り込む価値があるものを日本語で整理する。

**Architecture:** 現在の `main` と共通祖先をフォーク点として固定し、公式リポジトリの `main` を別ディレクトリへクローンする。公式側の追加履歴を時系列・変更領域・本リポジトリとの差分で監査し、機械的な取り込み候補と、provider 固有・競合・不要な変更を分ける。

**Tech Stack:** Git、GitHub の公式 `openai/codex` リポジトリ、`git log` / `git diff` / `git range-diff`。

**Spec:** ユーザー依頼、および `AGENTS.md` の provider-decoupling / Genex 移行方針。

## Global Constraints

- 現在の `main` ブランチ上で作業し、Git worktree は作成しない。
- フォーク元の外部契約・OpenAI endpoint 固有名は保持し、Genex 移行と別の provider-specific removal は混ぜない。
- コード変更前に producer/consumer を監査し、必要なら互換性インベントリへ明示的な判断を記録する。
- Rust の検証が必要な変更を実施する場合は、リポジトリ指定の systemd scope 経由で `just check` 等を実行する。

## Review Focus

- フォーク点の同定が、`fork/main`・共通祖先・コミット作者の三つで一致しているか。
- 公式側の追加コミットが、本リポジトリの Genex 改変済み箇所へ依存していないか。
- OpenAI endpoint、第三者プロトコル、上流由来の法的・帰属情報を誤って変更候補にしないか。
- 公式側のセキュリティ修正・バグ修正・テスト修正を、provider 固有機能の復活と取り違えないか。
- 取り込み候補ごとに、変更対象、依存関係、競合見込み、検証方法を説明できるか。

### Task 1: フォーク点の確定

- [x] `main`、`fork/main`、共通祖先、作者・日時・件名を記録する。

### Task 2: フォーク元の取得

- [x] 公式 `https://github.com/openai/codex.git` を `../codex-upstream` にクローンする。
- [x] クローン先の `main` 先端、フォーク点の存在、取得日時を確認する。

### Task 3: 追加コミットの抽出

- [x] フォーク点から公式 `main` 先端までのコミットを時系列で抽出する。
- [x] 各コミットの変更ファイル・統計・主要な差分領域を確認する。

### Task 4: 取り込み価値の監査

- [x] 本リポジトリの既存変更との重複・競合・provider 固有性を確認する。
- [x] 「取り込む候補」「内容を移植検討」「取り込まない」に分類する。

### Task 5: 日本語一覧の作成

- [x] フォーク点、公式側の現在位置、追加コミット数を冒頭に示す。
- [x] 各コミットに SHA、日時、件名、一次トリアージ区分を付ける。
- [x] 調査範囲と未検証事項を明記する。

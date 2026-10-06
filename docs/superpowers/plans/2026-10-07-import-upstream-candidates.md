# upstream 取り込み実装計画（2026-10-07）

## Goal

公式 `openai/codex` のフォーク点以後、取得済み `main` の先端
`73178e7ca60fe8655c49727e60a467d7c03f896c` までについて、現行 Genex に
対応する安全性・バグ修正を一件ずつ監査し、必要な部分を移植する。各件は
producer/consumer の根拠と `remove` / `retain` / `migrate-in-place` を台帳に
記録し、個別コミットを作成して `main` へ push する。

## Constraints

- 現在の `main` へ直接実装する。worktree は作成しない。
- upstream SHA の直接 cherry-pick は行わず、`codex-rs` から `crates` への移動と Genex リネームを反映して移植する。
- OpenAI endpoint 固有の URL・request/response field・認証・model 名、第三者 protocol、帰属・法的記録は保持する。
- provider-hosted / remote-only / Windows-only / Guardian hosted の変更は、理由を台帳に記録して取り込まない。
- Rust 変更後は repository 指定の systemd scope で `just check` と `just check --tests` を実行する。

## Candidate order

1. `41f9084b30812db321a0b592def4f500d1e79cf4` — WebSocket/tool log redaction
2. `2c2a42e65de077c5518ea5b4c3999633ef6a12fc` — MCP OAuth HTTP(S) endpoint restriction
3. `7aa8f510496316b86a91a75ed0a71598ef03c640` — writable bubblewrap PATH protection
4. `a73898c249b3cc95849ae1ea91231a0c9a9e29e2` — sandboxed file-open parameters
5. `685270a56a96c76ae5b0853373a19d6ed5bc6fd4` — unconditional apply_patch line-ending preservation
6. `1d804e91b75454927fb4f51615b067ba6158c37c` — `.aws` protection under writable roots
7. `6f51c65958ebdd5a57708e012d045f172fc9aab4` — macOS system-alias permission matching
8. `18194bfd3534ca567d886eac454028dafaa68b6c` — capability roots bound to captured turns
9. `60713126ee0dbc483fba83fc032dfaa5998521ec` — runtime-only permission approval
10. `8f5387d104534d280a3c2b15ef9f5dbb3aa3a758` — execution-environment proxy requirements
11. `e4dba902abb7775b747b03f1e9056ec6cdad403a` — caller-provided MITM CAs
12. `f817e1690570aac60480babbd2243cc991ba2118` — macOS numeric ioctl value
13. `888e02db34bc6fe226e2131448413c051491f5b8` — local HTTP/WebSocket network policy propagation
14. `1f52d407041d933e490ca3033729bce6319d07fa` — turn-bound permission grants

`8f5387d104`、`60713126ee`、`1f52d40704` は現行構造との比較結果により、
部分移植または `retain` となり得る。`80e0b51c9e` の session pagination、
`4d15794336` の ranked tool discovery、`73178e7ca6` の Guardian v2 pool は、
それぞれ現行実装済み・機能拡張・provider-hosted 専用として別判断を台帳に残す。

## Verification

- 各候補: 対応テストを RED→GREEN で確認し、個別コミットを作成する。
- 各 Rust 変更: systemd scope 経由の `just check`、`just check --tests`。
- generated artifact に影響する場合: `just generate` と差分確認。
- 最終: 全候補の台帳状態、`git status`、`git log`、push 先 head を確認する。

# Hawk 結果の分析

調査日: 2026-09-23

## 結論

`codex-cli.json` は、`codex` バイナリから見た公開 API の縮小候補を絞り込む資料としては有用である。ただし、これだけを根拠に削除できるコードは少ない。`hawk.toml` の起点が `codex-cli` パッケージの `codex` だけであり、別バイナリ、テスト target、OS 固有バイナリ、外部利用者からの参照を完全には表していないためである。

今回の結果からの優先順位は次のとおりである。

1. `codex-core` の非推奨型エイリアス3件は、リポジトリ内の参照が定義自身しかなく、削除候補として最も明確である。
2. `codex-core::test_support` は実行時コードではないが、統合テストから使われている。削除ではなく、テスト支援クレートへの移動または公開範囲の縮小を検討する。
3. `dead_public` の残りは、まず全バイナリの結果を合成してから個別に producer/consumer を確認する。特に app-server、TUI、Windows sandbox、protocol は削除しない。
4. `unnecessary_public` と `unnecessary_restricted_visibility` は「未使用」ではなく、可視性を狭められる可能性を示す。公開 protocol、生成 schema、serde/動的 dispatch、外部 API を含むため、一括置換しない。

このレポートではコード自体は変更していない。また、指定どおり `pub` を外して `just check` する検証はまだ実施していない。

## 1. 解析対象と限界

対象はリポジトリ直下の `codex-cli.json` である。JSON の概要は以下のとおり。

| 項目 | 内容 |
| --- | --- |
| Hawk schema | 5 |
| feature | `all-features` |
| target | `x86_64-unknown-linux-gnu` |
| production 起点 | `codex-cli` / `codex` |
| diagnostics | 3,740件 |
| `includes_non_production_targets` | `true` |

`hawk.toml` には次の1起点だけが定義されている。

```toml
[[production]]
package = "codex-cli"
bin = "codex"
```

一方、`cargo metadata --no-deps --format-version 1` では bin target が31件ある。したがって、少なくとも `codex-exec`、`codex-app-server`、`codex-tui`、`codex-code-mode-host`、`logs_client`、schema生成ツール、Windows sandbox の各バイナリを別起点として確認する必要がある。OS 固有コードについては、今回の Linux 結果から Windows の到達性を判断してはいけない。

Hawk の診断種別は次のように読む。

- `dead_public`: 今回の `codex` 起点から参照されなかった `pub` 項目。別バイナリや外部利用まで不要とは限らない。
- `unnecessary_public`: 今回の起点で必要な項目だが、観測範囲では `pub` より狭い可視性にできる可能性がある項目。削除候補ではない。
- `unnecessary_restricted_visibility`: `pub(crate)` 等の restricted visibility をさらに狭められる可能性がある項目。
- `test_compiled_only`: テスト target のコンパイルで現れた診断。production runtime の削除候補とは分けて扱う。

## 2. 件数の集計

| 診断 | 合計 | test flag なし | test flag あり | 主な意味 |
| --- | ---: | ---: | ---: | --- |
| `dead_public` | 585 | 260 | 325 | 公開項目の到達性を再監査する候補 |
| `unnecessary_public` | 2,161 | 1,818 | 343 | 可視性を狭める候補 |
| `unnecessary_restricted_visibility` | 994 | 856 | 138 | `pub(crate)` 等を狭める候補 |
| 合計 | 3,740 | 2,934 | 806 |  |

`dead_public` の production 側260件は、次のクレートに集中している。

| クレート | 件数 |
| --- | ---: |
| `codex-core` | 53 |
| `codex-protocol` | 22 |
| `codex-tui` | 18 |
| `codex-http-client` | 17 |
| `codex-app-server-protocol` | 13 |
| `codex-mcp` | 12 |
| `codex-core-plugins` | 11 |

test flag ありの `dead_public` 325件は、`app_test_support` が159件、`core_test_support` が157件を占める。これは runtime の掃除量を示す数字ではなく、テスト支援 API の公開範囲と不要 helper を別に棚卸しするための数字である。

可視性診断は `codex-tui`（506件）、`codex-core`（381件）、`codex-config`（233件）、`codex-app-server-protocol`（224件）、`codex-rollout-trace`（202件）に多い。protocol/schema/trace 型の field が大量に含まれるため、件数だけで公開面を削るべきではない。

生成された `OUT_DIR` の protobuf ファイルにも14件の診断がある。生成ファイルを直接編集する対象ではなく、必要なら generator、prost の生成設定、または全バイナリ合成後の公開 API 方針を見直す。

## 3. producer/consumer 監査

### 削除候補

| 項目 | 観測結果 | 判定 |
| --- | --- | --- |
| `codex-core/src/lib.rs` の `ConversationManager`、`NewConversation`、`CodexConversation` | `rg` でリポジトリ内の参照は各型エイリアスの定義自身だけ。いずれも `ThreadManager`、`NewThread`、`CodexThread` への deprecated alias。 | `remove` 候補。コード変更時に `just check` で確認する。外部利用者ゼロというリポジトリ方針とも整合する。 |
| `codex-app-server-protocol` の `generate_json`、`generate_ts`、`generate_types` の再エクスポート | Hawk の `codex` 起点では未使用。ただし schema generator 群、テスト、CLI の `generate-*` サブコマンドが同クレートの別 API を使用している。 | `migrate-in-place`。wrapper 単位で全 bin・生成手順を確認してから `pub(crate)` 化または削除。 |
| `codex-core/src/test_support.rs` の Hawk dead 判定項目 | ファイル自身が「Test-only helpers」と明記されているが、`crates/core/tests`、`crates/app-server/tests` 等が `codex_core::test_support::*` を直接使用している。 | `migrate-in-place`。runtime API からテスト支援を分離する候補だが、単純削除は不可。 |

### Hawk の false positive または retain 寄りの項目

| Hawk の項目 | 現在の consumer | 判定 |
| --- | --- | --- |
| `codex-cli` の `run_command_under_seatbelt` / `run_command_under_windows_sandbox` 再エクスポート | `crates/cli/src/main.rs` の sandbox dispatch（それぞれ `1472`行、`1486`行付近）が使用する。同一 package の sibling bin/library 境界を Hawk の単一起点結果だけでは十分に表せていない。 | `retain` |
| `codex-app-server::run_main`、`InProcessClientHandle::respond_to_server_request`、`fail_server_request` | app-server の bin、CLI の app-server dispatch、`codex-app-server-client` の in-process client が使用する。 | `retain` |
| `codex-app-server-client` の `request` / `notify` 系 API | app-server client 内の in-process/remote handle、`codex-exec`、TUI の request handle 経路から使用される。 | `retain`。メソッド単位の可視性は全 client target を合成後に再評価。 |
| `codex-windows-sandbox` の cancellation token、capture、legacy preflight | Windows 実装・stub・`codex-core` の Windows execution path が使用する。今回の Linux 起点で未到達でも、現行 platform fallback と security behavior に該当する。 | `retain` |
| `codex-tui::ComposerInput` と `ComposerAction` | `codex-tui` が明示的に public wrapper として提供している。今回の `codex` 内部で全 method が呼ばれないことは、外部 crate/API 利用がないことを意味しない。 | `retain`。公開 API 方針を決めてから狭める。 |
| `codex-protocol::mcp_approval_meta` の metadata constants | MCP elicitation の test/wire payload が使用する。map key の生成・参照や JSON 境界は単純な Rust symbol 参照だけでは判定しづらい。 | `retain`。protocol compatibility/security の監査対象。 |
| protobuf 生成コードの `as_str_name` / `from_str_name` | `OUT_DIR` の生成物。Hawk の判定だけで生成 API を手編集・削除しない。 | `retain`。生成元と code-mode host の全起点を確認。 |

## 4. 掃除の進め方

### 第一段階: 低リスクの候補

まず deprecated alias 3件の削除可否を別変更として扱う。これらは production consumer が見つからず、provider-neutral capability や platform fallback に触れない。次に、`codex-core::test_support` のうち各 helper を test target ごとに分類し、使用されない helper だけを削除するか、`core_test_support` 側へ移す。

### 第二段階: 全バイナリ合成後の visibility cleanup

`dead_public` 260件をそのまま削除せず、次の順に確認する。

1. `docs/hawk-entrypoints.md` の31 bin targetを実際の Hawk 起点集合にする。
2. 同一 feature/profile の結果を合成し、いずれかの通常実行 bin から到達するものを live とする。
3. 開発・補助 bin と test fixture は production と別集合で合成する。
4. `pub` の変更は、Rust の参照だけでなく protocol、serde、macro、生成 schema、環境変数、文字列 dispatch、OS 条件を監査する。
5. Rust を変更した場合は、リポジトリ方針に従い `just check`、`just check --tests` を実行する。

### 第三段階: 候補の扱い

| 区分 | 扱い |
| --- | --- |
| `remove` | deprecated alias のように producer/consumer がなく、別起点・生成・動的参照もないものだけ。 |
| `migrate-in-place` | test support、public wrapper、同一 package の sibling bin など、consumer はあるが責務・公開範囲を整理できるもの。 |
| `retain` | app-server protocol、OS sandbox/security、現行 MCP、生成 schema、外部向け public wrapper。Hawk の未到達だけでは変更しない。 |

## 5. 未実施の検証

今回は JSON の集計、`cargo metadata` による bin target の棚卸し、対象 symbol の `rg` による producer/consumer 確認までを行った。次の検証はまだ行っていない。

- `pub` / `pub(crate)` を変更しての `just check`
- Rust の変更を伴う `just check --tests`
- 全31 bin 起点の Hawk 結果合成
- 生成物に影響する変更後の `just generate`

したがって、この文書の `remove` は実装済みの削除宣言ではなく、次の小さな変更単位へ渡せる分析上の候補である。

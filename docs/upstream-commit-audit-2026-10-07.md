# フォーク元追加コミット監査（2026-10-07）

## 基準点と取得結果

- 本リポジトリ: `main` の先端 `fadce565ccbc977b62dc875815e850e947820e42`
- フォーク点: `e269f2164cbb9f499e4f22301c393500e2a831f3`
- フォーク点の作者: `jif <jif@openai.com>`
- フォーク点の日時: 2026-09-17 09:54:59 UTC
- フォーク点の件名: `Include sender user messages in Guardian delegation reviews (#46179)`
- 根拠: ローカル `fork/main` の先端であり、本リポジトリ `main` との共通祖先。公式履歴にも同一 SHA が存在する。
- 公式リポジトリ: `https://github.com/openai/codex.git`
- クローン先: `../codex-upstream`
- 公式 `main` の取得時点の先端: `73178e7ca60fe8655c49727e60a467d7c03f896c`
- フォーク点以後の追加コミット: **997 件**
- 追加された Git merge commit: **0 件**（公式 `main` は squash 済みの通常コミット列）

完全な時系列一覧は [`upstream-commits-after-fork-2026-10-07.md`](upstream-commits-after-fork-2026-10-07.md) に記録した。そこに付けた区分はタイトルによる一次トリアージであり、最終的な採否ではない。

## 取り込み候補の一次判断

フォーク側は `codex-rs` から `crates` へ移動し、Genex リネーム、Windows・Bedrock・ChatGPT hosted・remote hosted surface の削除または中立化が進んでいる。そのため upstream の SHA をそのまま cherry-pick するのではなく、必要なロジックを Genex 側へ移植する前提で判定する。

| SHA | upstream 件名 | 判定 | producer / consumer の根拠 |
|---|---|---|---|
| `7aa8f51049` | Reject sandbox-writable bubblewrap executables from PATH (#51211) | **取り込み候補（高）** | Genex 側の `crates/sandboxing/src/bwrap.rs` は cwd 配下を避けるだけで、filesystem policy により置換可能な PATH の祖先を検査していない。`crates/linux-sandbox/src/launcher.rs` も権限文脈なしで launcher を一度選ぶ。upstream の writable-root / ancestor / `faccessat2` 検査を Genex の policy API に合わせて移植すべき。 |
| `41f9084b30` | Remove WebSocket headers and tool payloads from info logs (#48686) | **取り込み候補（高・小規模）** | Genex 側の `crates/genex-api` は Responses WebSocket を保持し、`crates/core/src/stream_events_utils.rs` と `crates/core/src/tools/registry.rs` は Tool payload を info log に出している。ヘッダー・tool payload の秘匿は provider 固有ではないため、ログ契約を確認して最小差分を移植する価値が高い。 |
| `2c2a42e65d` | Restrict MCP OAuth authorization endpoints to HTTP(S) (#47326) | **取り込み候補（高・要確認）** | Genex 側の `crates/rmcp-client/src/oauth/issuer_binding.rs` は origin/issuer を検証するが、authorization endpoint の scheme 制限は同ファイルにない。redirect 側には HTTP(S) 検査があるため、metadata 起点の `file:` 等が別経路で到達しないか監査してから移植する。 |
| `685270a56a` | Make apply_patch preserve line endings unconditionally (#51203) | **取り込み候補（中〜高）** | Genex 側の `crates/apply-patch` と core は `ApplyPatchFileUpdateMode`、設定キー、`GENEX_APPLY_PATCH_PRESERVE_LINE_ENDINGS` をまだ保持し、既定は LF 正規化。pre-release で不要な互換 flag を削除する方針と整合するが、設定 schema・standalone executable・app-server test を一体監査してから移植する。 |
| `8f5387d104` | Honor execution environment proxy requirements (#48198) | **移植監査（中）** | `crates/network-proxy/src/environment_policy.rs` と core の environment/network approval は現行到達経路として残る。一方 upstream patch は remote execution 文脈を含むため、削除済み remote hosted transport を復活させず、現行 local/provider-neutral network policy 部分だけを抽出する必要がある。 |
| `60713126ee` | Avoid stdin approval for runtime-only permission grants (#48073) | **移植監査（中）** | Genex core は stdin approval と runtime-only grant を保持している。承認を弱める変更ではなく、不要な再承認を避ける安全性・UX 修正だが、現在の `crates/core/src/unified_exec/stdin_approval.rs` の状態機械との対応を確認する。 |
| `1f52d40704` | Bind permission grants to the originating turn (#49880) | **移植監査（高だが大規模）** | turn-scoped permission state は安全性境界に直結する。ただし upstream は Guardian/session 構造を含む 22 ファイル変更で、Genex 側の Guardian hosted integration は同じ構造ではないため、概念とテストケースを抽出し、直接 cherry-pick はしない。 |
| `80e0b51c9e` | Make session lookup pagination stable and report listing failures (#51230) | **現行実装と照合** | Genex 側の `crates/tui/src/named_session_lookup.rs` は既にページング、曖昧性、state DB / rollout path の検証を実装している。upstream の差分を参考に未対応の cursor 境界だけ確認するが、重複実装としての取り込みは不要。 |
| `4d15794336` | Add ranked tool discovery to JavaScript code mode (#51209) | **機能拡張として保留** | Genex 側には `tool_search` と Code Mode の producer/consumer が既にある。23 ファイル・約 1,100 行の upstream 拡張は有用そうだが、今回の互換性・安全性 cleanup の必須修正ではなく、別の機能移植タスクに分離する。 |
| `73178e7ca6` | Keep Guardian v2 WebSocket pools warm with concurrent replenishment (#51378) | **今回は取り込まない** | upstream は Guardian v2 sampler の専用 connection pool を更新する。Genex 側に同じ Guardian v2 production crate/到達経路はなく、Responses WebSocket 自体だけを理由に provider hosted Guardian 機能を復活させない。 |

## 追加の取り込み候補（詳細監査待ち）

上表の最優先候補に加え、以下は現行 Genex に対応する安全性・実行境界が残っているため、追加で確認する価値がある。ただし upstream 側の変更には削除済みの Windows/MXC/remote hosted 経路も混在するため、いずれも部分移植が前提である。

| SHA | upstream 件名 | upstream 規模 | 現行 Genex との対応 | 暫定方針 |
|---|---|---:|---|---|
| `a73898c249` | Use dedicated parameters for sandboxed file opens (#49818) | 2 ファイル、+11/-8 | exec-server の sandboxed file open は現行の filesystem capability 境界に残る。汎用 path validation と file-descriptor 制御の変更として比較できる。 | **高・部分移植**。`crates/exec-server` の fs helper/open producer と capability consumer を対応付ける。 |
| `1d804e91b7` | Protect `.aws` directories under sandbox writable roots (#48176) | 11 ファイル、+281/-143 | writable root 配下の credential directory 保護は provider 固有ではなく sandbox の安全性境界に関係する。ただし upstream の変更は Windows/MXC/Bedrock を含む。 | **高・条件付き**。現行で AWS credential を読む producer を確認し、Linux/macOS の保護ロジックだけ抽出する。 |
| `6f51c65958` | Fix macOS system-alias matching in patch permission checks (#47879) | 17 ファイル、+792/-304 | apply_patch の permission check、system alias、macOS seatbelt が現行にも残る。path alias の誤判定は workspace 外への書き込み許可につながる。 | **中〜高・部分移植**。`apply-patch`、absolute-path、sandboxing の alias 判定を現行型へ写像する。 |
| `f817e16905` | Use the numeric ioctl value in the macOS sandbox policy (#49028) | 1 ファイル、+5/-1 | macOS sandbox policy の ioctl 判定に限定された platform fallback 修正。provider 依存ではない。 | **中・小規模移植**。macOS の現行 policy source と debug sandbox の生成結果だけ確認する。 |
| `18194bfd35` | Restrict capability roots to captured turn environments (#49280) | 4 ファイル、+171/-401 | capability roots、turn 単位の environment、exec-server protocol は現行 Genex でも保持対象。root の寿命・所有 turn の取り違え防止に関係する。 | **高・設計照合**。現行の `crates/exec-server-protocol` と `crates/core/src/session/mcp.rs` に対して、削除ではなく root binding の安全性だけ抽出する。 |
| `e4dba902ab` | Support caller-provided MITM CAs in the network proxy (#47132) | 13 ファイル、+470/-68 | 現行には custom CA と managed MITM CA の producer/consumer が既にあり、機能の一部は先行実装済みの可能性が高い。 | **中・重複確認**。`crates/network-proxy` と `crates/sandboxing` の CA 配布・信頼境界を比較し、未実装部分だけ移植する。 |
| `888e02db34` | Enforce network policy throughout HTTP and WebSocket requests (#47389) | 57 ファイル、+1660/-923 | 現行の HTTP/WebSocket と network policy は保持対象だが、upstream 変更には ChatGPT backend、remote、Guardian 経路が混在する。 | **中〜高・大規模部分移植**。local HTTP/WebSocket の policy propagation のみを追跡し、hosted transport は除外する。 |

## 取り込まない範囲

- Windows sandbox / registered Core / PowerShell / MXC 専用変更: 本リポジトリは Windows support を削除済み。
- ChatGPT backend / hosted config / cloud thread resume / remote control / provider Apps / image generation / Realtime voice / usage・billing: provider-specific または hosted surface として削除・中立化済み。
- Guardian v2 の hosted sampler、gateway OAuth、EMA/ID-JAG、Cloud Tasks など: 現行 Genex の汎用 MCP/OAuth/security surface と同一視しない。
- upstream の docs、snapshot、provider product SKU、OpenAI backend telemetry だけを変更するコミット: Genex リネームの対象外。endpoint 固有の値、外部 protocol、帰属・法的記録は保持する。

## 次の実装単位

1. `7aa8f51049` の bubblewrap PATH 安全性を Genex の `PermissionProfile` / `FileSystemSandboxPolicy` と launcher 初期化に移植する。
2. `41f9084b30` のログ秘匿を `genex-api` と core の Tool log producer に移植し、ログテストを更新する。
3. `2c2a42e65d` の MCP OAuth scheme 検査を、現在の issuer binding / redirect / network policy の producer-consumer と突き合わせる。
4. `685270a56a` は設定キーと standalone compatibility env の `remove` / `retain` 判断を `docs/pre-release-compatibility-inventory.md` に記録してから移植する。

この監査ではコード変更・cherry-pick はまだ実施していない。

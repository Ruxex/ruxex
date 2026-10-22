# Ruxex CLI／プロバイダ受入れテストマトリクス

## 目的

この文書は、`ModelProvider`／`ModelProviderFactory` を実装したホストが、
Ruxex CLI と TUI をリリース前に検証するための機能一覧、テスト方法、期待値を
定義する。単にプロバイダAPIへ接続できることではなく、CLIから見える機能、
App Serverとの境界、認証情報の隔離、セッション永続化、失敗時の終了コードと
出力までを受入れ対象とする。

対象は、プロバイダ固有のエンドポイント・認証・wire formatを除く共通CLI契約と、
プロバイダアダプタがその契約へ正しく変換する部分である。プロバイダ固有のHTTP
payload、資格情報の取得方法、実サービスのレート制限値は、別途プロバイダ自身の
integration testで検証する。

## 1. 変更不可の前提

| ID | 前提 | 受入れ条件 |
|---|---|---|
| INV-01 | プロバイダはruntime起動時に1つ選択する | thread、rollout、`resume`、`fork`、`queue`がプロバイダを選択・切替しない |
| INV-02 | プロバイダ境界は資格情報を所有する | thread/rolloutファイル、JSONL出力、エラー、ログにtoken・secret・Authorization値がない |
| INV-03 | 別プロバイダは別runtime／別data rootを使う | provider Aの認証状態・セッション・キャッシュがprovider Bから見えない |
| INV-04 | data rootを解決してから機密状態を開く | `RUXEX_HOME`／`ProductIdentity` と `ruxex-home.json` の検証前にcredential storeを開かない |
| INV-05 | provider identityはfactoryと実体で一致する | `provider_descriptor()` と `descriptor()` のname/versionが一致する |
| INV-06 | 未対応能力は明示的に扱う | `Unsupported` 相当の結果を、成功・空データ・panicとして扱わない |
| INV-07 | ストリームは順序を保つ | start → delta/item → usage/rate-limit（提供時）→ completed、または明示的なerrorになる |
| INV-08 | 非対話モードは機械可読である | `exec --json` は1 event/line、診断はstderr、最終終了コードは意味を持つ |

## 2. テスト層と共通fixture

### 2.1 テスト層

| 層 | 方法 | 必須対象 |
|---|---|---|
| L0 parser | Clapのparse testまたはCLI subprocess | help、必須引数、conflict、alias、exit code 2 |
| L1 provider contract | `ruxex-model-provider-contract-tests` とfake provider | identity、capability、catalog、snapshot、account、operation support |
| L2 runtime | fake provider + App Server in-process | stream、tool、approval、interrupt、resume、queue、永続化 |
| L3 CLI subprocess | 一時`RUXEX_HOME`、stdin/stdout/stderrを捕捉 | 実際の引数、終了コード、出力、ファイル副作用 |
| L4 provider HTTP | HTTP mock／recording server | method、URL、headers、body、retry、timeout、error mapping |
| L5 live smoke | 非機密の専用provider account | login、model list、最小prompt、stream、logout、実サービス制限 |
| L6 platform/security | Linux/macOS CI、secret scan、権限検査 | sandbox、worktree、socket、credentials、redaction |

リリース必須はL0〜L4とL6、L5はproviderのCI方針に従う。L5だけでL0〜L4を
代替してはならない。

### 2.2 fixture

各subprocessテストは、テストごとに次を用意する。

```text
fixture/
├── home/
│   ├── ruxex-home.json       # テスト用 ProductIdentity marker
│   ├── config.toml            # 必要な場合だけ作成
│   └── AGENTS.md
├── repo/
│   ├── .git/
│   ├── tracked.txt
│   └── untracked.txt
└── scripts/
    ├── fake-tool                # stdout/stderr/exit codeを制御
    ├── fake-http-headers-helper # secretを出力しないことを検証
    └── fake-mcp-server
```

推奨する環境変数と初期条件は次のとおり。

```text
RUXEX_HOME=<fixture/home>
RUXEX_TEST_PROVIDER_INFO=<provider info JSON>
RUXEX_TEST_CREDENTIAL=<テスト用の非本番値>
HOME=<テスト用home>              # 外部のcredential/configを混ぜない
TERM=dumb                        # 出力比較を安定させる場合
```

provider fakeは少なくとも次のscriptable stateを持つ。

1. authenticated／unauthenticated／managed auth。
2. catalogに2モデル（既定モデル1つ、reasoning非対応モデル1つ）。
3. 通常のtext delta、複数item、tool call、usage、rate-limit event。
4. HTTP 401、403、404、409、429、500、timeout、malformed JSON。
5. operationごとの`Unsupported`、retryable error、non-retryable error。
6. provider factoryの生成回数、runtimeのprovider descriptor、送信requestを記録する。

テストは実際のprovider secretをfixtureやsnapshotへ書き込まない。secretを
検査する場合は`TEST_SECRET_SENTINEL`のような無害な値を使い、出力には出ないことを
確認する。

### 2.3 共通の判定方法

各テストで最低限、次を記録する。

| 判定対象 | 期待値 |
|---|---|
| process exit | 成功`0`、ユーザー入力不正`2`、処理失敗`1`。サブコマンド固有の契約がある場合は表に明記 |
| stdout | ユーザー結果またはJSONL。secret・内部stack traceを含まない |
| stderr | 診断・進捗・警告。通常の結果を混ぜない |
| provider request | 想定operation、model、入力、認証route、retry回数。secretはredacted比較 |
| filesystem | 作成・更新・削除されるファイルの集合、権限、再実行結果 |
| state | session/thread/rolloutが再開・fork・queue可能か、credentialが復元されないか |

## 3. provider実装そのものの必須テスト

### 3.1 factoryとidentity

| ID | 方法 | 期待値 |
|---|---|---|
| PVD-01 | `construct_and_assert_model_provider_factory::<P>(init, expected)` | factoryが成功し、factory descriptorと実体descriptorが一致する |
| PVD-02 | factory生成を2回行い、それぞれのruntimeを比較 | provider stateが意図せずglobal共有されない。共有する場合もcredentialはruntime root内のみ |
| PVD-03 | descriptor name/versionをCLI、App Server、doctor、rollout metadataで確認 | すべて同じ値。OpenAI/Codex等の不要な名前をRuxex provider identityに混ぜない |
| PVD-04 | provider infoを欠損・不正・別wire APIで起動 | 機密状態を開く前に明確な設定エラー。panic、空の既定providerへのfallbackなし |

### 3.2 capabilityと操作

| ID | 方法 | 期待値 |
|---|---|---|
| PVD-10 | capabilitiesを全組合せで生成しcontract testを実行 | capabilityと実際のoperation supportが一致する |
| PVD-11 | `list_models`を成功、空、認証失敗、malformed responseで実行 | catalogのprovider identity、model id、表示名、visibility、reasoning/input modalityが保持される |
| PVD-12 | chat requestを固定しrequest snapshotを取得 | connection key、credential snapshot、request formatがprovider内で安定し、thread stateへsecretが漏れない |
| PVD-13 | account stateをauthenticated／unauthenticated／expiredで取得 | auth status、account id、plan等の公開可能情報だけを返す。tokenは返さない |
| PVD-14 | login／logout／deleteを実行 | 成功・未ログイン・再実行が冪等。deleteは対象providerのcredentialだけを消す |
| PVD-15 | usageを対応・非対応・provider errorで実行 | 対応時は単位と期間が正しく、非対応時は`Unsupported`またはCLIの明示的な未対応表示 |
| PVD-16 | streamにtext、tool、reasoning summary、usage、rate-limit、completedを混在 | event順序、空delta、unicode、複数messageを失わずに中継する |
| PVD-17 | 401/403/404/409/429/5xx/timeout/JSON decodeを各operationで実行 | provider-neutral errorへ分類され、retry可否と表示が一貫する。request/credentialはerror textに出ない |
| PVD-18 | 同一providerでthread resume／forkを実行 | providerはruntime起動時の1つだけで、thread metadataにcredentialを保存しない |

### 3.3 provider HTTP mockの検証

HTTP mockの期待値は「provider固有payloadの完全一致」と「Ruxex境界の意味」の2段階に
分ける。

- provider testではURL、HTTP method、認証header、wire body、response schemaを完全一致させる。
- CLI/runtime testでは、上記をprovider-specificな文字列としてassertせず、model、prompt、
  stream event、error class、secret redactionをassertする。
- retry testでは、同一idempotency keyまたは同じrequest identityを使い、意図しない二重turnを
  作らないことを確認する。
- timeout testではprocessが無期限に残らず、exit `1`、stderrに原因、rolloutが壊れた中途半端な
  secret状態を残さないことを確認する。

## 4. CLI共通機能

### 4.1 起動、help、config、環境

| ID | 実行 | 期待値 |
|---|---|---|
| CLI-001 | `ruxex --help` | exit `0`、主要subcommandとusageが表示される。実在しないprovider名を表示しない |
| CLI-002 | `ruxex --version` | exit `0`、package versionのみ。stderrにerrorなし |
| CLI-003 | `ruxex unknown-command` | exit `2`、候補とusageをstderrへ、provider factoryは未生成 |
| CLI-004 | `ruxex --bad-flag` | exit `2`、機密値をechoしない |
| CLI-005 | `ruxex -c model=test-model -c features.<known>=true` | config overrideがその起動だけに有効。永続configは意図せず書き換えない |
| CLI-006 | `ruxex -c unknown.key=value` と `--strict-config` | strict時はexit `1`、非strict時の警告／無視は既存契約どおり |
| CLI-007 | `RUXEX_HOME=<temp>`、markerあり／なし／不正 | marker検証後にだけstate/credentialを開く。失敗時にcredentialを作らない |
| CLI-008 | `--profile <profile>`、存在／不在／不正TOML | 正常時はprofileが優先。失敗時は対象pathを示し、base configを黙って使わない |
| CLI-009 | `--model <id>`、存在／未知 | providerへ指定modelを送る。未知modelはproviderまたはCLI契約に従う明示エラー |
| CLI-010 | `--cd <dir>`、存在／不存在／非directory | cwdが実行・config・sessionに反映。不存在はexit `1`でprovider callなし |
| CLI-011 | `--image <file>`、複数、不存在、非画像 | 対応時は入力itemとして1回だけ送る。invalid pathはprovider call前にexit `1` |
| CLI-012 | `--sandbox <mode>`、`--add-dir` | 解決済み絶対pathとmodeが実行policyに反映。許可外pathへ書けない |
| CLI-013 | `--dangerously-bypass-approvals-and-sandbox`／`--yolo` | aliasが同じ動作。警告または明示的な危険モード表示があり、通常のapprovalと併用不可 |
| CLI-014 | `--dangerously-bypass-hook-trust` | hook trustのみを変更し、sandbox/approvalを変更しない |
| CLI-015 | `--worktree` | 新しいmanaged worktreeで実行。source repoを破壊せず、失敗時の未使用checkoutを表示 |
| CLI-016 | `--remote <ws/wss/unix>`、auth token env | remote endpointへ接続し、token値ではなくenv名／redacted表示のみ |
| CLI-017 | `--remote-auth-token-env <missing>` | 接続前にexit `1`。tokenそのものをstderrに出さない |
| CLI-018 | `--enable <known>`／`--disable <known>` | feature overrideがその起動で有効 |
| CLI-019 | `--enable <unknown>`／`--disable <unknown>` | exit `1`、`Unknown feature flag`相当。provider callなし |
| CLI-020 | `--no-daemon` | daemonが起動中でもin-processで動作。通常起動との結果が一致 |
| CLI-021 | `--color always/never/auto` | stdoutのANSI有無が指定どおり。JSONLにはANSIを混ぜない |
| CLI-022 | `--search` | web search capabilityが有効なときだけprovider/tool requestへ反映し、未対応時は明示エラー |

### 4.2 新規インタラクティブセッション

| ID | 方法 | 期待値 |
|---|---|---|
| TUI-001 | `ruxex "hello"`、fake streamを接続 | exit `0`、deltaが順序どおり表示、provider factoryは1回、turn completedで終了 |
| TUI-002 | promptなしでTTY起動し入力 | startup draft、入力、送信、終了が成立。非TTYでは明示的な扱いになる |
| TUI-003 | promptをstdin、`-`、prompt+stdinで与える | stdin blockの境界と順序が保持され、二重送信されない |
| TUI-004 | tool callを発生させapproval yes/no/interrupt | yesはtool結果後に継続、noは拒否結果として継続、interruptは停止し再開可能 |
| TUI-005 | providerがstream途中でerror | exit `1`、利用者向け原因をstderrまたはTUI errorへ、processは終了しstateを破壊しない |
| TUI-006 | 429/rate-limit event | rate-limit表示が出る。無限retryせず、retry policyどおりに終了または再試行 |
| TUI-007 | Ctrl-C／turn interrupt | `turn/interrupt`相当が1回、二重送信なし。保存済みthreadはresume可能 |
| TUI-008 | 長文delta、unicode、空delta、複数item | 表示・保存・exportで欠落・文字化け・順序逆転なし |
| TUI-009 | provider未対応のusage/search/画像 | 未対応を明示し、他のsession操作は継続可能 |

## 5. 非対話CLI

### 5.1 `ruxex exec`

| ID | 実行 | 期待値 |
|---|---|---|
| EXEC-001 | `ruxex exec "hello"` | exit `0`、最終回答がstdout、credentialはstdout/stderr/rolloutにない |
| EXEC-002 | `printf 'hello' \| ruxex exec -` | stdinが初期指示。空stdin、stdin read errorを個別に扱う |
| EXEC-003 | `ruxex exec prompt --json` | stdoutがvalid JSONLのみ。各行parse可能、diagnosticはstderr |
| EXEC-004 | `--json --output-last-message <file>` | 最終messageがfileへ1回だけ書かれ、stdout JSONLも壊れない |
| EXEC-005 | `--output-last-message <existing>` | 上書き契約を確認し、permission errorはexit `1`。部分書込みを残さない |
| EXEC-006 | `--output-schema <valid.json>` | schemaに適合する最終結果を返す。schema不適合は成功扱いしない |
| EXEC-007 | malformed／unreadable output schema | provider call前にexit `1`、pathとparse errorを表示 |
| EXEC-008 | `--ephemeral` | session/rolloutを永続化しない。ただしそのturnは正常完了する |
| EXEC-009 | `--skip-git-repo-check` | 非Git cwdでも実行。通常は非Git cwdを規約どおり拒否 |
| EXEC-010 | `--ignore-user-config` | `config.toml`を読まず、`RUXEX_HOME`自体は使用する |
| EXEC-011 | `--ignore-rules` | execpolicy rulesを読まない。設定全体を無視しない |
| EXEC-012 | `--thread-source <valid/invalid>` | valid sourceはmetadataへ反映、invalidはexit `2` |
| EXEC-013 | `--sandbox`、`--ask-for-approval`、`--yolo` | execは既定でheadless approval policy。相互conflictと実際のpolicyを検証 |
| EXEC-014 | `--worktree` | forkではなく新規worktreeを作成し、providerは同じruntime identity |
| EXEC-015 | provider 401/429/500/timeout | 各error classでexit `1`、JSONモードのerror eventもvalid JSON |

### 5.2 `ruxex exec resume` / `fork`

| ID | 実行 | 期待値 |
|---|---|---|
| EXEC-020 | `ruxex exec resume <UUID> "next"` | 既存threadを1回だけ読み込み、同じproviderで続行 |
| EXEC-021 | `ruxex exec resume --last "next"` | 最新の対象sessionを選び、`--last`のpositionalをpromptとして扱う |
| EXEC-022 | `--last <SESSION_ID> <PROMPT>` | positional解釈の不正をexit `2`。別sessionへ誤送信しない |
| EXEC-023 | `--all` | cwd filterを解除し、別cwdのsessionも候補に含む |
| EXEC-024 | resumeに画像 | image itemが続きのpromptに添付される |
| EXEC-025 | resume不存在／非interactive除外 | exit `1`またはpickerで明示表示。新規sessionへ黙ってfallbackしない |
| EXEC-026 | `ruxex exec fork <UUID> [PROMPT]` | 新UUIDのthreadが作られ、元threadは変更されない。fork先へpromptが送られる |
| EXEC-027 | forkのimage、worktree | 新thread／新worktreeへだけ反映。元のcwdとrolloutを変更しない |
| EXEC-028 | credentialsを含む元rolloutをfork | prompt/historyは引き継ぐが、credentialは引き継がない |

### 5.3 `ruxex review` と review mode

| ID | 実行 | 期待値 |
|---|---|---|
| REV-001 | `ruxex review --uncommitted` | staged/unstaged/untrackedの差分をproviderへ1 review requestとして渡す |
| REV-002 | `ruxex review --base main` | baseとの差分だけ。base解決失敗はprovider callなしでexit `1` |
| REV-003 | `ruxex review --commit <SHA> --title "T"` | commit差分とtitleが渡る。title単独はexit `2` |
| REV-004 | prompt付きreview | custom instructionが差分と混ざらず、順序が契約どおり |
| REV-005 | `--uncommitted --base main`、各組合せ | conflictはexit `2`、曖昧なreviewを実行しない |
| REV-006 | `--worktree` | unsupportedとしてexit `1`／`2`、sourceを変更しない |
| REV-007 | provider未対応review | 未対応表示と終了コードが明確。通常chatに黙ってfallbackしない |

## 6. セッション管理

| ID | 実行 | 期待値 |
|---|---|---|
| SES-001 | `ruxex resume <UUID>` | 対象UUIDのみを開き、保存済みmessageを表示して続行 |
| SES-002 | `ruxex resume <name>` | exact name優先。UUIDとしてparse可能な文字列はUUID優先 |
| SES-003 | `ruxex resume --last`、`--all`、`--include-non-interactive` | 各filterの組合せが一覧と選択に正しく反映 |
| SES-004 | `ruxex fork <UUID>`／`--last` | 新session id、元session不変、provider identity不変 |
| SES-005 | `ruxex archive <UUID|name>` | archived stateが保存され、resume既定一覧から除外 |
| SES-006 | `ruxex unarchive <UUID|name>` | 再び通常一覧に出る。別sessionを変更しない |
| SES-007 | `ruxex delete <UUID>` | 確認あり。confirm yesで関連session stateを削除 |
| SES-008 | `ruxex delete <UUID> --force` | UUID限定、確認なし。name + forceはexit `1` |
| SES-009 | delete name、confirm no、stdin EOF | 削除しない。終了結果とstateが一貫する |
| SES-010 | `ruxex queue --thread <id> --message "hi"` | 対象threadへ1 messageをqueue。image optionは拒否 |
| SES-011 | queue中にprovider error／既存turn | messageが二重queueされない。retry後の順序を確認 |
| SES-012 | `ruxex agents` | shared daemon上のactive sessionsを表示。`--no-daemon`は拒否 |
| SES-013 | state DB corrupt／lock | 自動backupまたは診断ガイダンス。backup対象がfailed DBだけである |

## 7. 認証・provider lifecycle

| ID | 実行 | 期待値 |
|---|---|---|
| AUTH-001 | `ruxex login status` authenticated | exit `0`、provider idとauthenticatedだけを表示 |
| AUTH-002 | `ruxex login status` unauthenticated | exit `1`、not authenticated。secretなし |
| AUTH-003 | `ruxex login delete` credentialあり／なし | ありは削除、なしはno-op。対象provider root以外を変更しない |
| AUTH-004 | `ruxex logout` authenticated | provider logoutを1回、exit `0`、成功表示 |
| AUTH-005 | `ruxex logout` unauthenticated | exit `0`、not logged in。不要なnetwork callをしない |
| AUTH-006 | logout error／delete error | exit `1`、errorはredacted。credential DBを壊さない |
| AUTH-007 | provider Aでlogin後provider B rootを起動 | BからA credentialが見えない。Aのroot pathをBの出力に出さない |
| AUTH-008 | auth state取得後にthread/rollout保存を検査 | token、credential snapshot、Authorization headerがない |
| AUTH-009 | provider factory descriptorとconfig provider info不一致 | 起動を拒否し、credential storeを開かない |

## 8. MCP

### 8.1 CLI

| ID | 実行 | 期待値 |
|---|---|---|
| MCP-001 | `ruxex mcp list` | configured server一覧。secretはenv名または`<redacted>` |
| MCP-002 | `ruxex mcp list --json` | valid JSON、server name/transport/statusがstable。token値なし |
| MCP-003 | `ruxex mcp get <name>`／`--json` | 対象serverだけ。不存在はexit `1` |
| MCP-004 | `ruxex mcp add <name> -- <command> args` | config.tomlへ正しく追加、command argsとenvを保持 |
| MCP-005 | `mcp add <name> --url <URL>` | streamable HTTP serverとして保存 |
| MCP-006 | URL + `--bearer-token-env-var` | env名だけ保存。実tokenをconfigへ保存しない |
| MCP-007 | commandとurl同時／どちらもなし | Clap validation exit `2` |
| MCP-008 | `mcp remove <name>` | 対象entryだけを削除、他server不変 |
| MCP-009 | add/list/get/removeの再実行 | idempotencyまたは明示的なduplicate error。設定が壊れない |
| MCP-010 | `mcp login <name> --no-browser` | authorization URLを表示しcallbackを受ける。state/issuer検証あり |
| MCP-011 | scopes、AUTO/DCR registration | 指定scopeとregistration strategyを1回のloginに反映 |
| MCP-012 | invalid callback、wrong issuer、expired state | credentialを保存せずexit `1` |
| MCP-013 | `mcp logout <name>` | OAuth tokenだけを削除し、server configを残す |

### 8.2 runtime

| ID | 方法 | 期待値 |
|---|---|---|
| MCP-020 | stdio server起動、tool list、tool call | startup status、tool schema、result/errorがTUI/App Serverへ伝播 |
| MCP-021 | HTTP server + bearer env | header helperが一度だけ実行され、tokenはログ・rolloutに出ない |
| MCP-022 | MCP server timeout／malformed result | turnをhungにせず、対象tool errorとして継続可能 |
| MCP-023 | approval拒否 | toolを実行せず、modelに拒否結果を返す |
| MCP-024 | server name/pathにshell metacharacter | shell injectionなし。設定値をそのままargv/env境界で扱う |

## 9. Plugin／marketplace

| ID | 実行 | 期待値 |
|---|---|---|
| PLG-001 | `ruxex plugin marketplace add <local>` | sourceとsnapshotが保存され、marketplace nameが安定 |
| PLG-002 | Git source、`--ref`、複数`--sparse` | 指定ref/pathだけ取得。外部repoの任意実行を許可しない |
| PLG-003 | `marketplace list`／`--json` | configured source、root、snapshot状態を表示。secretなし |
| PLG-004 | `marketplace upgrade`／`<name>` | allまたは対象のみ更新。失敗したmarketplaceの状態を明示 |
| PLG-005 | `marketplace remove <name>` | config、local cache、snapshotの削除範囲が契約どおり |
| PLG-006 | `plugin list`、`--marketplace`、`--json` | installed pluginが正しい。`--available`はJSON時だけ有効 |
| PLG-007 | `plugin add name@marketplace` | plugin manifest、version、installed pathを表示／JSON化 |
| PLG-008 | `plugin add name --marketplace m` | selector解決が同じ結果。曖昧なselectorは拒否 |
| PLG-009 | `plugin remove` | pluginとlocal cacheの範囲だけ削除、他marketplace不変 |
| PLG-010 | invalid manifest、duplicate、snapshot欠損 | exit `1`、partial installを残さないまたはcleanupを表示 |
| PLG-011 | `marketplace publish <path> --registry <url>` | registryへ正しいbundle、token headerはredacted。HTTP errorはexit `1` |
| PLG-012 | `--token-env-var` missing／malformed URL | publish前にexit `1`、secret値を表示しない |
| PLG-013 | `/plugins` | TUIからbrowseができ、CLI plugin commandと状態が一致 |

## 10. App Server、daemon、生成物

| ID | 実行 | 期待値 |
|---|---|---|
| APP-001 | `ruxex app-server`（stdio既定） | protocol initialize後、指定provider runtimeでrequestを処理 |
| APP-002 | `--listen stdio://`／`--stdio` | 同じtransport。相互conflictはexit `2` |
| APP-003 | `--listen unix://PATH` | socket pathで起動し、path権限を適切にする |
| APP-004 | `--listen ws://IP:PORT`、`off` | endpointのbind／無効化が期待どおり。bind失敗はexit `1` |
| APP-005 | websocket auth enabled／missing token／invalid token | validのみ接続。tokenをログ・errorへ出さない |
| APP-006 | `app-server daemon bootstrap/start/restart/stop` | lifecycle state、pid/socket、再実行が冪等 |
| APP-007 | `app-server daemon version` | CLIとrunning server versionをJSONで表示。server不在を明示 |
| APP-008 | daemonを起動して`agents`, `queue`, `resume` | shared stateが見える。providerはdaemon側の1 runtimeだけ |
| APP-009 | `app-server proxy --sock PATH` | socketへstdioを中継。接続失敗はstderr、exit `1` |
| APP-010 | `generate-ts --out DIR` | checked-in schemaから生成し、実験的要素の既定除外を確認 |
| APP-011 | `generate-json-schema`、`--experimental` | JSON Schemaが生成され、default/experimentalの差分が期待どおり |
| APP-012 | internal schema generator | internal artifactのみ生成し、public schemaへ混入しない |
| APP-013 | daemon shutdown／crash／stale socket | thread保存、stale socket cleanup、再起動が安全 |
| APP-014 | `ruxex exec-server --listen stdio://` | stdio RPCを受け付け、`--concurrent-requests 1`の既定でrequest順序を保つ |
| APP-015 | `exec-server --concurrent-requests <n>`、invalid値、strict config | 指定並列数だけ処理し、0・不正URL・不正configは起動前にexit `1` |

## 11. sandbox、execpolicy、filesystem

| ID | 実行 | 期待値 |
|---|---|---|
| SBOX-001 | Linux `ruxex sandbox ...` | Landlock/bwrap policyが適用され、許可rootだけread/write可能 |
| SBOX-002 | macOS `ruxex sandbox ...` | seatbelt profileが適用され、denyが期待どおり |
| SBOX-003 | unsupported platformのsandbox | compile/runtime契約どおりのunsupported error。危険な無制限実行へfallbackしない |
| SBOX-004 | `--permission-profile`／`--profile` | named profileとconfig layerが正しい順序で解決 |
| SBOX-005 | `--cd`、`--add-dir`、`--allow-unix-socket` | 絶対path化、重複、不存在、外部socketの扱いを検証 |
| SBOX-006 | `--sandbox-state-json` | JSON stateがvalid時だけ適用し、conflicting optionsをexit `2` |
| SBOX-007 | sandbox commandのexit 0/1/127/signal | 子processの終了状態をCLIが壊さず伝える |
| SBOX-008 | `execpolicy check` valid/invalid rules | command match、syntax error、missing fileの判定とexit codeが安定 |
| SBOX-009 | commandにsecret、newline、unicode | argv/env境界を保ち、shell injectionとsecret echoがない |
| SBOX-010 | sandbox内からprovider call | provider HTTP routeが許可policyどおりで、禁止networkは拒否 |

## 12. doctor、debug、補助コマンド

| ID | 実行 | 期待値 |
|---|---|---|
| AUX-001 | `ruxex doctor` | config、auth、runtime、network、sandbox、git、diskの診断が完了 |
| AUX-002 | `doctor --summary` | 短いhuman report。exit codeはfail有無を反映 |
| AUX-003 | `doctor --json` | valid JSON、redacted report。secret、credential pathの過剰な露出なし |
| AUX-004 | missing config／bad config／missing helper | 各checkが独立してfailし、他checkの診断を継続 |
| AUX-005 | `doctor`でprovider request snapshotを取得 | configured providerだけを使う。snapshotにcredentialがない |
| AUX-006 | `debug models` | raw model catalog JSONがvalid。provider descriptorとmodel identityが一致 |
| AUX-007 | `debug prompt-input [PROMPT] --image FILE` | model-visible input listがJSONで再現され、画像path/bytesの扱いが契約どおり |
| AUX-008 | `debug app-server send-message-v2 MSG` | test client経由で1 messageを送り、response/exitを返す |
| AUX-009 | hidden `debug trace-reduce BUNDLE [-o FILE]` | valid traceのみreduced stateを書き、malformed bundleは壊れたfileを残さない |
| AUX-010 | `completion bash/zsh/fish/powershell` | 各shell scriptが生成され、command/optionの主要集合を含む |
| AUX-011 | `features list` | feature key、stage、effective stateが表示／JSON契約があればvalid |
| AUX-012 | `features enable/disable <key>` | configの対象keyだけ変更。unknown featureはexit `1` |
| AUX-013 | hidden `stdio-to-uds SOCKET_PATH` | stdio bytesをsocketへ中継し、socket失敗を明示 |
| AUX-014 | hidden `tcp-tunnel` | proxy条件、接続失敗、shutdownが安全。通常helpには露出しない |
| AUX-015 | `ruxex-state-logs` | home/db、level、時間、module/file/thread/search、backfill、compactを検証。secretを表示しない |

## 13. TUIスラッシュコマンド一覧

各コマンドは、(a) popupに出る、(b) 完全一致でdispatchされる、(c) inline argsの有無が
契約どおり、(d) task中／side conversation中の可否、(e) error時にTUIが壊れない、を
最低1回検証する。`strum`のaliasも個別に実行する。

| コマンド | 主なテストと期待値 |
|---|---|
| `/model` | catalog表示、model/reasoning選択、次turnだけまたはsession設定への反映 |
| `/ide` | IDE contextの有無、malformed context、provider requestへの混入位置 |
| `/permissions` | approval/sandbox選択が実際のtool実行policyへ反映 |
| `/keymap` | valid/invalid binding、再起動後の保存、重複bindingの扱い |
| `/vim` | insert/normal mode切替、入力とexitの復帰 |
| `/setup-default-sandbox`／`/skills` | skills表示・選択、存在しないskill、trust境界 |
| `/hooks` | lifecycle hook一覧・管理、未trusted hookは実行されない |
| `/review` | inline instructionのreview実行、review errorの復帰 |
| `/rename` | thread名変更、空名・重複名、resumeで同じ名前 |
| `/new`／`/clear` | 新thread生成、旧thread保存、provider factory再生成なし |
| `/archive`／`/delete` | confirm、state変更、cancel、削除後resume拒否 |
| `/resume`／`/fork`／`/worktree` | picker、last、UUID/name、fork元不変、worktree cleanup |
| `/init` | AGENTS.md作成、既存ファイル上書き確認、cwd反映 |
| `/compact`／`/recap` | summary生成、stream error、context縮小後の継続 |
| `/plan`／`/goal` | mode/goal表示・更新、goal stateのpersist、providerへ不要なsecretなし |
| `/agents`／`/subagents` | daemon-wide／session subagentsの一覧・切替、target誤りなし |
| `/side`／`/btw` | ephemeral fork、side中の使用可能コマンド制限、元turn非破壊 |
| `/copy` | last response／selectionのcopy。未対応端末では明示エラー |
| `/export` | markdownの順序、画像表現、secret redaction、file error |
| `/raw` | raw scrollback toggle、ANSIとcopy結果の契約 |
| `/diff` | tracked/unstaged/untracked差分、巨大diff、git error |
| `/mention` | file picker、binary/large/missing file、prompt入力の境界 |
| `/status` | cwd、model、provider identity、token usage。secretなし |
| `/daemon` | local daemonのstatus/start/stop、stale socket |
| `/cd`／`/pwd`（alias `/cwd`） | cwd変更と表示、invalid dir、次turnへの反映 |
| `/usage` | usage表示、unsupported、reset操作を誤って自動実行しない |
| `/title`／`/statusline` | item設定、invalid item、再起動後の保存 |
| `/theme` | theme一覧、選択、invalid theme、render fallback |
| `/mcp` | server/tool/status表示、verbose、server error |
| `/plugins` | plugin browse、marketplace error、auth policy表示 |
| `/logout` | provider logout、未ログイン、sessionは壊さない |
| `/quit`／`/exit` | 保存処理を待ち、exit。実行中turnの扱いを確認 |
| `/ps` | background terminal一覧、終了済みprocessのcleanup |
| `/stop`（alias `/clean`） | background terminalを全停止、対象外processは停止しない |

popupのfilter testでは、未知prefix、prefixだけ、alias、case-sensitive／insensitiveの
契約を確認する。また、feature flagで `/plan`、`/plugins`、`/usage`、`/goal`、
`/worktree` が表示・dispatchから消える条件も検証する。

## 14. 出力・エラー・セキュリティの横断テスト

| ID | 方法 | 期待値 |
|---|---|---|
| X-001 | 全subcommandをinvalid argで実行 | parse errorはexit `2`、stack traceなし |
| X-002 | providerに`TEST_SECRET_SENTINEL`を返させる | stdout、stderr、JSONL、rollout、sqlite、snapshot、doctor reportに出ない |
| X-003 | network errorへURL/header/bodyを含める | credential/header値はredact。必要なrequest idだけ公開 |
| X-004 | stdoutをpipe、stderrをpipe、TTY/non-TTYを切替 | user resultとdiagnosticがstream間で混ざらない |
| X-005 | `--json`で全成功・失敗・interrupt | 全stdout行がparse可能なJSON。最後のeventでsuccess/errorが分かる |
| X-006 | `--color never`でsnapshot | ANSIなし。`--color always`以外でterm依存の色を出さない |
| X-007 | 同じfixtureで2回実行 | 意図しないduplicate session、credential再保存、cache破損なし |
| X-008 | 並列で2 process起動 | lock contentionを明示し、DB corruptと誤判定しない |
| X-009 | 途中kill後に再起動 | backup/recoveryまたはresumeができ、secretのpartial fileが残らない |
| X-010 | path、model id、thread nameに空白・unicode・長大値 | shell injection、path traversal、表示崩壊なし |
| X-011 | provider A rootとprovider B rootを交互に利用 | cross-provider state leakなし |
| X-012 | Linux/macOSで同じfixtureを実行 | platform差分はsandbox／terminal capabilityに限定され、CLI契約は一致 |

## 15. 実行順序とリリースゲート

### PRごと

```sh
just fmt-check
just check
just check --tests
git diff --check
```

provider／CLIのcrateを絞る場合も、repositoryの`just` recipeを使う。テスト実行の
例は次のとおり。

```sh
just test-cargo -p ruxex-model-provider-contract-tests
just test-cargo -p ruxex-cli
just test-cargo -p ruxex-app-server
```

fixtureを使うCLI E2Eは、各テストが一時directoryを作り、テスト終了時にそのdirectory
だけを削除する。共有の`~/.ruxex`や作業treeを使わない。

### provider実装完了時

次をすべて満たすまで「CLI対応済み」としない。

- [ ] PVD-01〜PVD-18のうち、providerが対応すると宣言したoperationが全てgreen。
- [ ] `ruxex --help`からTUI、exec、auth、MCP、plugin、App Serverの入口を確認した。
- [ ] TUI-001〜TUI-009をfake providerで実行した。
- [ ] EXEC-001〜EXEC-015、EXEC-020〜EXEC-028、REV-001〜REV-007を実行した。
- [ ] SES-001〜SES-013で保存・resume・fork・削除を検証した。
- [ ] AUTH-001〜AUTH-009でcredential隔離とredactionを検証した。
- [ ] MCP-001〜MCP-013、MCP-020〜MCP-024を、stdioとHTTPの両方で実行した。
- [ ] PLG-001〜PLG-013をlocal fixtureとHTTP mockで実行した。
- [ ] APP-001〜APP-015をstdioとUnix socketで実行した。
- [ ] 対応OSのSBOX、doctor、debug、completion、featuresを実行した。
- [ ] TUIスラッシュコマンド表の全行を、少なくともpopup/dispatch/errorの3観点で確認した。
- [ ] X-001〜X-012のsecret、JSONL、parallel、recovery、cross-providerテストがgreen。
- [ ] LinuxとmacOSで、provider-specificな差分を記録した。
- [ ] 失敗したケースは「未対応」と「不具合」を区別し、capabilityまたはissueに記録した。

## 16. 実装との対応箇所

実装を変更したときは、機能表のIDだけでなく次の実装・テスト箇所も更新する。

| 領域 | 主な実装 | 既存の確認材料 |
|---|---|---|
| CLI dispatch | `crates/cli/src/cli_main.rs` | subcommand dispatch、unsupported worktree snapshot |
| auth | `crates/cli/src/login.rs` | key formatting unit test、provider factory経由のlifecycle |
| MCP | `crates/cli/src/mcp_cmd.rs`、`crates/cli/src/mcp_login.rs` | MCP login/config、App Server v2 MCP suite |
| plugin | `crates/cli/src/plugin_cmd.rs`、`crates/cli/src/marketplace_cmd.rs` | marketplace/plugin App Server suite |
| queue/session | `crates/cli/src/queue_cmd.rs`、`crates/tui/src/session_*` | thread queue/resume/fork App Server suite |
| provider facade | `src/lib.rs`、`crates/model-provider`、`crates/providers` | `tests/provider_facade.rs`、contract-tests |
| provider host | `examples/provider-host` | fake/test provider host、factory wiring |
| TUI | `crates/tui/src/cli.rs`、`crates/tui/src/slash_command.rs` | rendering、keymap、streaming、session tests |
| sandbox | `crates/cli/src/debug_sandbox.rs`、`crates/sandboxing` | Linux/macOS policy tests |
| recovery | `crates/cli/src/state_db_recovery.rs` | corruption/lock/backup unit tests |

この表の既存テストは、CLI全体の受入れ完了を意味しない。特に、providerを差し替えた
ときのL3 subprocess、credential redaction、cross-provider root isolation、live smokeは、
provider host側の専用テストとして追加する。

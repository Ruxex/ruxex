# Provider-Free Client Boundary Design

## 目的

TUIおよびAppServerクライアントを、接続先AppServerが所有するproviderから分離する。
クライアントは接続後にproviderの状態・capability・model catalogを取得し、別の
providerを持つAppServerへ接続しても、前のproviderの設定・キャッシュ・認証状態を
再利用しない。

対象はTUI、`RemoteAppServerClient`、および同じAppServer protocolを利用するクライアント
である。AppServer内部のprovider注入、provider home marker、外部OpenAI wire契約は既存の
単一provider runtime設計に従い、本設計では変更しない。

## 現状監査

### 既にAppServerを正としているもの

- TUIのmodel catalogは `model/list` から取得する。
- provider statusは `modelProvider/read` から取得する。
- provider capabilityは `modelProvider/capabilities/read` で提供される。
- Remote clientはcredentialsを保持せず、AppServer protocolのtransportを担当する。
- AppServer側がprovider、credentials、thread/rollout、provider-scoped SQLiteを所有する。

### TUI側に残るprovider依存

- `Config.model_provider` / `model_provider_id` はTUIの設定・status表示に残る。
- `Config.model_provider.wire_api` によってreasoning表示などのUI分岐を行う。
- custom providerの `base_url` をTUIのmodel pickerが直接表示する。
- bootstrap時の初期modelはローカル `config.model` がサーバー既定値より優先される。
- Remote TUIは既存のローカル state runtime を任意で開き、provider markerを検証しない。
- reconnectは同じ `AppServerTarget` に戻る経路であり、別AppServerへの切替時に全ての
  provider依存状態を更新する契約はまだ明示されていない。

## 設計

### 1. AppServer identity

AppServerは初期化応答で、秘密情報を含まないidentityを返す。

```text
AppServerIdentity {
    signature: opaque stable identity,
    version: runtime/provider contract compatibility version,
    build_version: optional diagnostic implementation version,
}
```

`signature` は同じprovider-scoped data rootを使うAppServer再起動では安定し、別provider
または別data rootでは変わる。provider markerと同じ所有境界で生成・保存する。endpoint URL、
filesystem path、credentialsはidentityに含めない。

`version` は単なるGenex製品のbuild versionではなく、provider contract、protocol schema、
model catalog/capabilityの互換性を含むcache compatibility versionとする。したがって、
provider descriptorをidentityに重複して持たせない。provider種別そのものは、mount時の
`genex-home.json` による不一致検出のために保持するが、クライアントのidentity／cache keyには
含めない。

暗号学的な相手認証はidentityの目的ではない。WebSocketのTLS/bearer token、Unix socketの
権限、その他のtransport securityで別途扱う。

### 2. 設定所有権

TUI設定を次の3層に分ける。

#### Client-local

テーマ、キーバインド、端末表示、editor、session pickerの表示方法、TUI固有の通知や
scrollback設定など。これらはAppServer切替時にも保持する。

#### AppServer-owned

provider descriptor、auth state、credentials、provider capability、model catalog、usage、
server-side model defaults、wire/API mode、provider endpoint、provider-specific config。
TUIは接続後に取得し、ローカルの同名設定をsource of truthにしない。

#### Session request

model、reasoning effort、service tier、approval、sandbox、cwdなど、TUIがセッションや
turnの要求として送る値。provider固有の意味を持つ値はTUIで解釈せず、AppServerが検証する。
modelやreasoningのローカル既定値を保持する場合は、現在のAppServer identityに紐付け、
model catalogに存在しなければ破棄してserver defaultへフォールバックする。

### 3. キャッシュ境界

model catalog、capability、account/usage表示、server defaultsなどの接続先由来のキャッシュは、
次の複合キーで管理する。

```text
(signature, version)
```

endpoint URLだけをキーにしない。同じendpointのAppServerが別provider homeで再起動される
可能性があるためである。identityまたはversionが変わった場合、model、
reasoning、capability、account/usageのキャッシュを破棄し、bootstrapをやり直す。

同一AppServerを利用する複数クライアントは、各自の一時キャッシュを持てるが、providerの
権威状態は常にAppServerから取得する。

### 4. Remote TUIのローカル状態

Remote TUIのprovider-scoped state DBをclient UI stateと同一視しない。Remote時のローカル
state runtimeは、以下のいずれかに移行する。

1. remote thread/rolloutを必要としないUI専用DBへ縮小する。
2. provider-scoped homeとは別のclient-local homeへ分離する。
3. client側で不要なstate DBを開かず、必要な情報をAppServer protocolから取得する。

既存の `state_5.sqlite`、`logs_2.sqlite`、`goals_1.sqlite`、`queue_1.sqlite`、
`thread_history_1.sqlite` の各利用者を監査してから選択する。少なくともRemote時に
provider-owned thread/rollout DBとして扱わない。

### 5. 接続切替

初期実装では、別AppServerへの切替を同一TUI内の暗黙の操作にしない。接続確立時にidentityを
確定し、切断後の再接続ではidentityを再検証する。別AppServerへ切り替えるAPIを追加する場合は、
以下を必ず破棄・再構築する。

- request handleとイベント購読
- model catalogとcapability cache
- account/usage表示
- selected model、reasoning、service tierの検証状態
- provider固有UI分岐
- remote threadの一時表示・pending request

## 変更対象の初期リスト

- `crates/tui/src/app_server_session.rs`: bootstrap時のserver identity、provider status、
  capability、model catalog取得と切替境界。
- `crates/tui/src/lib.rs`: remote/local state DB初期化と接続targetのライフサイクル。
- `crates/tui/src/app/reconnect.rs`: reconnect後のserver-derived state再構築。
- `crates/tui/src/status/card.rs`: ローカルprovider設定ではなくserver statusを表示。
- `crates/tui/src/chatwidget/model_popups.rs`: local provider base URL参照の除去。
- `crates/tui/src/app/startup.rs`: server default modelとlocal model overrideの優先順位。
- `crates/app-server-client/src/remote.rs`: 初期化応答からidentityを保持。
- `crates/app-server-protocol/src/protocol/v1.rs` および `v2/`: identityのwire型と生成schema。
- `crates/model-provider/src/home_marker.rs` / `crates/app-server/src/runtime.rs`:
  stable signatureとcompatibility versionの生成・保存境界。provider markerのprovider種別は
  mount mismatch検出用として維持する。

## 検証方針

- TUIの全設定キーについて、client-local / AppServer-owned / session requestを一覧化する。
- Provider AのAppServerへ接続後、Provider BのAppServerへ接続して、Aのmodel ID、capability、
  auth state、cache、local DBがBへ漏れないことを検証する。
- 同じAppServerの再起動ではsignatureが維持され、compatibility version変更時はcacheが無効化されることを検証する。
- Remote TUIがprovider-owned SQLiteをmarkerなしに開かないことを検証する。
- `just generate`、`just check --tests`、`just check`をリポジトリのmemory guard付きで実行する。

## 非目標

- provider adapter内部のHTTP/WebSocket wire実装変更。
- OpenAI endpoint URL、request/response fields、auth headers/variables、model identifiersの変更。
- AppServer自体のprovider切替。別providerは別runtime/data rootを使う。
- TUIの全設定をAppServerへ移すこと。画面表示・操作設定はclient-localに残す。

# Complete Single-Provider Boundary Design

## 状態

設計済み。既存の単一Provider runtime移行を、provider-neutral契約、認証ライフサイクル、TUI残存挙動、永続化・文書監査まで完了させるための実装仕様。

## 目的

単一Provider runtime/AppServerの実装を、移行途中の互換adapter境界を含めて完了状態にする。Coreと拡張はProvider固有のHTTP、wire、credential、内部設定型を知らず、Providerはruntime構築時に1つだけ注入される。

## 完了条件

- Coreのproduction codeが`ModelProviderInfo`、`CredentialManager`、`GenexAuth`、`genex_api::Provider`、`SharedAuthProvider`をProvider契約経由の内部型として要求しない。
- Provider契約の検索・backend・chat requestが`HttpClientFactory`、`HeaderMap`などHTTP実装型を公開境界に持たない。外部OpenAI endpointのURL、field、header、model identifierはadapter内で保持する。
- logout後に開始するturnはProviderの未認証状態として拒否され、実行中turnは開始時のcredential snapshotを使用して完了できる。認証状態の共有と変更通知は維持する。
- TUIにProvider IDのハードコードによる機能分岐がなく、usage/auth/billing表示はProvider-neutralな状態・capabilityで表現される。Provider固有のHosted/Daybreak表示は削除または明示的なProvider capabilityに移行する。
- thread、rollout、thread history、session logにProvider routing selectorやcredentialを保存しない。mounted-providerのstatus responseは読み取り専用として明示分類する。
- 互換性台帳、候補キュー、実装計画、生成schemaが実装状態と一致する。

## 設計

### 1. Provider-neutral execution contract

`SharedModelProvider`をCoreの唯一のProvider handleとして維持する。Coreは認証値やHTTP clientを解決せず、ProviderにProvider-neutralな操作を渡す。

- `chat`はrequest-scoped metadata、session/thread identity、transport policyを中立型で受け取り、wire payload・endpoint・credential・SSE/WebSocket transportをProvider adapter内で生成する。
- `search`と`backend`はfeature operation、JSON payload、session context、timeout、generic metadataを受け取り、HTTP header mapやHTTP client factoryを受け取らない。adapterは構築時に必要なHTTP transport policyを所有する。
- `api_provider`、`api_auth`、`api_auth_for_scope`、`model_provider_info`など既存adapter互換用のCore向けメソッドを削除する。managed-authやrouting hintが必要な場合は、Provider-neutralなrequest preparation/result型で表現する。
- `credential_manager`や`GenexAuth`をCoreへ返さない。Providerがauth snapshot、auth status、unauthorized recovery、credential lifecycleを所有する。

既存OpenAI互換・Bedrock・Chat/Responses wire契約はProvider adapter内に残し、外部契約の名称と値は変更しない。

### 2. Authentication lifecycle

turn admissionまたはturn開始直前にProviderのauth stateを確認し、未認証・無効状態の新規turnをProvider-neutralな認証エラーで拒否する。turn開始後はProvider operationが捕捉したcredential snapshotを使用し、後続のlogin/logoutでそのturnのcredentialを差し替えない。

login、credential update、logoutはruntimeが共有する同一Provider handleへ送る。auth revisionの通知とWebSocket/cache invalidationは維持し、logoutで新規turnだけを拒否する回帰テストを追加する。

### 3. TUI and provider-dependent cleanup

現在のmounted-provider statusは表示できるが、Provider IDの文字列比較で機能を有効化しない。Daybreak/cyber-policyのProvider固有通知は、実際のProvider capabilityが存在しない限り一般的な制限通知へ統一する。

usage/auth/rate-limit stateはProvider-neutralな`ProviderAuthStatus`、`ProviderCapabilities`、`ProviderUsage`を入力とする。Hosted billing専用の状態やGenex固有の名前は削除するか、外部契約として残す根拠を台帳に記録する。

### 4. Persistence and documentation

SQLiteのProvider routing column削除済み状態を維持する。APIの`model_provider`/`provider_id`はmounted-provider status responseに限り保持し、request selector・thread settings mutation・永続routingには使わない。TUI session logはProvider routing fieldsを出力しない。

互換性台帳には各残存項目の`remove`、`retain`、`migrate-in-place`とproducer/consumer根拠を記録する。完了した実装計画のcheckbox、候補キュー、仕様書の状態を同期する。

## 検証

- Provider contract、Core client、auth lifecycle、TUIにRED→GREENの単体またはcompile-only test targetを追加する。
- `just check`を必ず実行する。
- Rust production code変更後はsystemd user scope付き`just check --tests`を実行する。
- protocol/schemaに変更がある場合は`just generate`を実行し、生成差分を確認する。
- 最終監査で`rg`によりCoreからのProvider固有型参照、Provider ID hard-code、thread/rollout/session logのrouting persistenceを確認する。

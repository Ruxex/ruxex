# 単一モデルプロバイダ注入型Genexライブラリ設計

## 状態

実装進行中。単一Providerの設定・実行・データルート境界を段階的に移行する。

## 目的

Genexを、複数のモデルプロバイダを実行時に切り替えるアプリケーションではなく、
ホストバイナリが1つのモデルプロバイダ実装を注入して利用するエージェント基盤として
提供する。

Genexのライブラリ利用者は、自身のモデルバックエンドに対応する
`ModelProvider` trait 実装を渡して、モデル実行、モデル列挙、認証状態、使用量などの
プロバイダ機能を提供する。Genex本体は、特定のプロバイダの認証方式やAPI endpointを
前提にしない。

## 解決する問題

現在の実装は、`model_providers` の複数エントリ、AppServer共有の
`active_model_provider_id`、スレッドごとの `model_provider` を組み合わせている。
これは複数プロバイダを同一AppServerで扱うための構造だが、次の複雑さを生む。

- 複数クライアント間でアクティブプロバイダを同期する必要がある。
- プロバイダを切り替えたとき、既存スレッドをどの経路で再開するか決める必要がある。
- Providerごとの認証、モデル一覧、使用量、capabilityの差を共通protocolで表現する必要がある。
- スレッドDBの `model_provider` が実行経路の固定情報なのか、表示用メタデータなのかが曖昧になる。

Genexでは、プロバイダの実行時切替をコアの責務から外す。複数プロバイダを同時に使う
必要があるホストは、別のGenex/AppServerインスタンスを起動する。

## 用語

- **ModelProvider**: 1つのモデルバックエンドに対する実行・制御・認証の契約。
- **ProviderDescriptor**: Providerの種類と互換性を識別する非秘密メタデータ。
- **ProviderCapabilities**: モデル列挙、使用量、ストリーミングなどの対応機能。
- **Genex runtime**: `ModelProvider` を1つ受け取ってスレッドとturnを実行するライブラリのランタイム。
- **ホスト**: Genex runtimeを組み込むCLI、AppServer、デスクトップアプリ、または利用者のバイナリ。
- **認証状態**: Providerが現在のリクエストに利用できるcredentialを持つかどうか。

## 設計判断

### 1. ProviderはGenex runtimeの構築時に1つだけ注入する

ライブラリの公開境界は、概念的に次の形とする。

```rust
let runtime = Genex::builder()
    .model_provider(Arc::new(MyModelProvider::new(provider_config)))
    .build()?;
```

1つのGenex runtime/AppServerは、1つの `ModelProvider` だけを所有する。
Providerの種類を変更するRPC、Provider一覧からの選択、Provider間の自動fallbackは
Genexコアには持たせない。

Providerの実装型をバイナリの依存関係またはCargo featureで選択する場合、ビルドされた
バイナリは1種類のProvider実装を持つ。Provider endpoint、モデル、credential値などの
設定は実行時に渡せるが、Providerの実装種類そのものは実行時に差し替えない。

ライブラリ利用者が複数Providerを必要とする場合は、Providerごとに別のruntime/AppServer
を構築する。複数のruntimeを同一プロセスに置くことは可能だが、それぞれの共有状態は
独立させる。

### 2. Traitは実行と管理の両方を表す広い契約にする

Providerはチャット送信だけでなく、AppServerがユーザーへ提供する次の機能も所有する。

- チャットまたはResponses実行
- ストリーミング応答
- モデル列挙・モデルメタデータ取得
- 認証状態の取得
- ログイン、credential更新、ログアウト
- 使用量、quota、billing相当情報の取得
- Provider固有のcapability報告
- Provider固有のエラー分類と認証回復

これは「HTTPクライアントを差し替えるtrait」ではなく、ProviderをGenexから見た外部
サービス境界として扱うためである。

### 3. 非対応機能はtraitを分割しすぎず、capabilityで表す

すべてのProviderが使用量APIや対話的ログインを持つとは限らない。基本traitを機能ごとに
分割してホストが多数のtrait boundを合成するのではなく、広いtraitに capability と
明示的なunsupported結果を持たせる。

```rust
pub struct ProviderCapabilities {
    pub model_listing: bool,
    pub streaming: bool,
    pub usage: bool,
    pub interactive_login: bool,
    pub web_search: bool,
    pub standalone_web_search: bool,
}
```

capabilityがfalseの操作は、エラー文言で曖昧に表現せず、`ProviderError::Unsupported`
または対応する `Unavailable` 結果を返す。UIとprotocolは「非対応」と「認証がない」
「一時的に取得できない」を区別する。

### 4. CoreはProviderの内部型を知らない

Provider traitの引数と戻り値は、Genexが定義するprovider-neutralな型にする。
Provider固有のHTTP request、SSE payload、credential形式、モデル一覧JSONはProvider
実装内に閉じ込める。

```text
Genex core request
    -> ModelProvider::chat
    -> provider-specific request/transport
    -> provider-neutral response stream
    -> Genex turn/session state
```

現在の `ModelProviderInfo`、`CredentialManager`、`GenexAuth`、`Provider` など、既存の
内部型をそのまま公開traitの契約にしない。必要な場合は、内部adapterが公開traitと
既存実装の間を変換する。

### 5. Providerの識別はデータルートのマウント契約に置く

Providerの種類はスレッドではなく、Genex runtimeが使用するデータルートに紐付ける。
データルートの既定値は `.genex` とするが、ホストはビルド時の既定値を変更でき、
起動時にも `genex_home` 相当のパスを指定できる。これにより、Providerごとのバイナリや
複数のruntimeが同じ既定ディレクトリを誤って共有することを防ぐ。

データルートのルートには、Providerが生成する非秘密のマーカーを置く。

```text
<genex_home>/
  genex-home.json
  state.sqlite
  sessions/
  credentials/
```

`genex-home.json` は少なくとも次を含む。

```json
{
  "formatVersion": 1,
  "providerKind": "example-provider",
  "providerProtocolVersion": "1"
}
```

runtimeのマウント時には、次の順序で検査する。

1. Provider traitから現在の `ProviderDescriptor` を取得する。
2. データルートが空または新規なら、現在のdescriptorでマーカーを作成する。
3. 既存マーカーがある場合は、現在のdescriptorと比較する。
4. Provider種類または互換性バージョンが不一致なら、SQLiteやcredentialを読み込む前に失敗する。
5. マーカーがない既存データは、暗黙に別Providerとして開かず、明示的なmigrationまたは初期化を要求する。

この検査は「別Providerで同じスレッドを続行する」ためではなく、Providerの異なるruntimeが
同じ認証情報、履歴、キャッシュを誤って使用することを防ぐためのものである。

## Provider traitの契約

最終的なメソッド名は実装前のAPIレビューで確定するが、責務は次のように固定する。

```rust
pub trait ModelProvider: Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    fn descriptor(&self) -> ProviderDescriptor;
    fn capabilities(&self) -> ProviderCapabilities;

    fn chat(
        &self,
        request: ChatRequest,
    ) -> ProviderFuture<Result<ChatStream, Self::Error>>;

    fn list_models(
        &self,
        request: ListModelsRequest,
    ) -> ProviderFuture<Result<ModelCatalog, Self::Error>>;

    fn auth_state(&self) -> ProviderAuthState;
    fn login(
        &self,
        request: LoginRequest,
    ) -> ProviderFuture<Result<(), Self::Error>>;
    fn logout(&self) -> ProviderFuture<Result<(), Self::Error>>;

    fn usage(
        &self,
        request: UsageRequest,
    ) -> ProviderFuture<Result<UsageResult, Self::Error>>;
}
```

### Chat

`chat` は、モデル、指示、履歴、tool定義、service tier等を含む
provider-neutralな `ChatRequest` を受け取り、Genexが消費できるストリームを返す。
Responses、Chat Completions、WebSocketなどのwire差分はProvider実装が変換する。

Provider固有の応答イベントは、テキスト、tool call、usage、完了、認証エラーなどの
共通イベントへ変換する。未対応の高度なイベントは、汎用イベントに影響を与えない形で
Provider拡張metadataへ格納できる。

### モデル列挙

`list_models` はProviderの現在のモデル一覧を返す。モデル一覧が静的なProviderは
キャッシュ可能な実装を使える。モデル一覧が存在しないProviderは
`ProviderError::Unsupported` を返し、Genexが設定済みモデルだけを表示できるようにする。

モデル一覧の結果には、少なくとも次を含める。

- 安定したモデルID
- 表示名
- 対応するreasoning、context、tool等のcapability
- Providerが提供する場合のモデル制限・利用可能状態

### 使用量

`usage` は任意機能だが、traitの契約には含める。戻り値はProviderごとに異なる課金・
quota表現を共通のスナップショットへ変換する。

```rust
pub enum UsageResult {
    Unsupported,
    Available(UsageSnapshot),
}
```

使用量がProviderに存在しない場合、ゼロや空のquotaを返して「使用量ゼロ」と誤認させない。

### 認証

認証はGenexアカウントではなくProviderのcredential lifecycleとして扱う。

- `auth_state` は未認証、認証済み、期限切れ、更新中などを返す。
- `login` はcredentialを設定または更新する。
- `logout` は現在のProviderを未使用状態にする。
- credential値はthreadやrolloutに保存しない。
- ProviderがOAuth、API key、AWS credential、環境変数などのどれを使うかはProvider実装が所有する。

credential変更はruntime全体に反映される。スレッドごとに別credentialを持たせない。

## RuntimeとAppServerの状態モデル

```text
Genex runtime
  └── one ModelProvider
        ├── descriptor
        ├── capabilities
        ├── authentication state
        └── model/chat/usage operations
```

同じAppServerに接続する複数クライアントは、同じProvider、認証状態、モデル一覧、使用量
を共有する。クライアント側は状態を所有せず、Providerのスナップショットを表示する。

### Providerの交換

Provider交換はruntime中の操作として提供しない。OpenAI用と別Provider用で切り替える
場合は、別のruntime/AppServerを起動する。

Provider交換を将来導入する場合は、単一Provider traitの拡張ではなく、複数runtimeを
束ねる上位のRouterまたはHost機能として設計する。

### credentialの更新

同じProvider内でAPI keyやアカウントを変更する場合は、Providerのcredential stateを
更新する。

- 変更後に開始するturnは新しいcredentialを使う。
- 実行中のturnは、開始時に解決したcredential snapshotを使い続ける。
- logout後は新しいturnを拒否する。
- 実行中turnが認証エラーになった場合は、認証要求として終了する。
- 他のクライアントには認証状態変更をbroadcastする。

これにより、Providerの種類は固定したまま、同じProvider内のアカウント切替をサポートできる。

## スレッド永続化

スレッドにProviderの選択情報やdescriptorを保存しない。スレッドは、マウント済みの
データルートに対応する唯一のruntime Providerを使う。

Providerの種類や互換性はデータルートの `genex-home.json` が保証するため、
`threads.model_provider` はroutingのために不要になる。既存の `model_provider` 列を
変更または削除する場合は、persisted reader、履歴一覧、resume、監査、protocol schemaの
producer/consumerを先に確認する。

この設計では、同じProviderの認証情報やAPI keyを変更しても、スレッドは同じデータルートの
中で継続できる。別Providerのruntimeで同じデータルートを開こうとした場合だけ、マウント時に
不一致エラーを返す。

## ProtocolとUI

単一Provider化後のapp-server protocolは、複数Providerを前提にした選択APIを持たない。

### 残すAPI

- 現在のProvider descriptor/status取得
- Providerのモデル一覧取得
- Providerの使用量取得（非対応状態を含む）
- Providerの認証状態取得
- login / credential更新 / logout
- Provider capability取得

### 原則として削除または再設計するAPI

- Provider一覧からの選択
- `activeProviderId` のクライアント選択
- Provider間のswitch
- スレッドごとに異なるProviderを指定するAPI

既存APIを変更する場合は、production producer/consumerを監査し、
`docs/pre-release-compatibility-inventory.md` に `remove`、`retain`、または
`migrate-in-place` の判断を記録してから変更する。

UIはProvider pickerではなく、現在のProviderの状態、モデル選択、認証状態、使用量を表示する。
別Providerを利用する場合は、ホストまたは起動設定で別runtimeを選ぶ。

## Configurationとbuild

Providerの実装種類は、次のいずれかでバイナリへ組み込む。

- バイナリcrateが具体的なProviderを直接構築する。
- Cargo featureでProvider実装を選択する。
- ライブラリ利用者のバイナリが自身のProvider実装を注入する。

データルートの既定パスはビルド時に変更可能にする。ただし、同じバイナリで複数の
Provider用データを安全に扱えるよう、ホストは起動時の明示的なデータルート指定も受け付ける。
実効パスの優先順位は、明示的なbuilder引数、起動時の環境変数またはCLI設定、ビルド時の既定値、
`.genex` の順とする。

実行時設定は、選択済みProviderのendpoint、model、timeout、retry、credential参照、
capability overrideなどに限定する。Provider registryとactive provider選択は、Genex
runtimeの必須設定にしない。

## エラー処理

Providerエラーは少なくとも次の分類を持つ。

- `Unsupported`: Providerが機能自体を提供しない。
- `Unauthenticated`: credentialが未設定またはlogout済み。
- `Unauthorized`: credentialが拒否された。
- `InvalidRequest`: モデルや入力がProviderの制約に合わない。
- `RateLimited`: Providerのquotaまたはrate limitに達した。
- `Transient`: 再試行可能な一時エラー。
- `Transport`: 接続、TLS、ストリームなどのエラー。
- `Provider`: Provider固有だが共通分類できないエラー。

AppServerは分類をもとに共通のprotocol errorへ変換するが、Provider固有の詳細と安全な
表示文はProviderが提供できる。認証情報、API key、Authorization headerはエラー表示・
履歴・telemetryから除去する。

## テスト方針

ライブラリのprovider contractを検証するため、最小のfake Providerを用意する。

- Provider descriptorとcapabilityを取得できる。
- モデル一覧を取得できる。
- text/toolを含むchat streamを処理できる。
- 使用量対応、非対応、取得失敗を区別できる。
- auth state、login、credential更新、logoutを処理できる。
- 同じProviderを複数クライアントが共有する。
- credential変更後の新規turnが新しいcredentialを使う。
- 実行中turnのcredential snapshotが不意に変わらない。
- データルートのマーカーが初期化・再利用・不一致検出を正しく処理する。
- descriptor不一致時にDBやcredentialを開かず失敗する。
- Provider固有のstream、model catalog、usage、auth errorが共通型へ変換される。

Repository方針に従い、Rustコードを変更する実装段階では `just check` を常に実行し、
test targetに影響する変更では systemd scope付きの `just check --tests` も実行する。
protocolまたはchecked-in generated artifactに影響する場合は `just generate` を実行し、
生成差分を確認する。

## 移行方針

1. 外部公開を想定するprovider-neutralなrequest、response、usage、auth、capability型を定義する。
2. 現在の `ModelProvider` 実装から、Provider固有adapterとGenex共通処理を分離する。
3. `CredentialManager` を単一Providerのcredential lifecycleへ接続する。
4. `ConfigManager` の複数Provider registryとactive-provider runtime stateを削減する。
5. Coreのthread/turn生成が注入されたProviderを直接参照するようにする。
6. app-serverのProvider list/switch protocolをstatus/capability protocolへ移行する。
7. データルートのProvider markerを導入し、既存Provider情報のproducer/consumerを監査する。
8. スレッドDBの既存Provider情報を削除またはmarker由来の状態へ移行する。
9. CLI、TUI、app-server client、テストfixture、schema、生成物を一括監査する。

各段階で、既存の認証・使用量・モデル一覧・チャット・MCP・security behaviorを別々に確認し、
単一Provider化に不要なものだけを変更する。外部OpenAI endpointが要求するURL、request/response
field、authentication header、model identifierはこの設計によって変更しない。

## 非目標

- 1つのGenex runtimeで複数Providerを同時に実行すること。
- Provider間の自動fallbackや負荷分散。
- スレッドごとのcredentialまたはProvider選択。
- Providerごとの永続credentialをthreadやrolloutに保存すること。
- Providerの種類を実行時にpluginとして動的ロードすること。
- 使用量や認証を、すべてのProviderに必ず実装させること。
- 1つのデータルートを異なるProvider runtime間で共有すること。
- OpenAI endpoint固有の外部契約名をGenex独自名へ変更すること。

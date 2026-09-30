# `openai_base_url` の provider-neutral 化設計

## 背景

`ConfigToml::openai_base_url` は、組み込み OpenAI credential source の接続先をローカル設定から credential broker に渡すためだけの特例である。これにより、モデルプロバイダーの接続先設定と、credential broker が資格情報を注入してよい接続先の信頼境界が暗黙に結び付いている。

一方、credential broker には既に、資格情報ごとに許可する接続先を明示する汎用設定がある。

```toml
[features.network_proxy.credentials.my_gateway]
env = ["MY_GATEWAY_API_KEY"]
url_prefixes = ["https://gateway.example/v1"]
```

この仕組みをカスタム接続先の標準経路とし、OpenAI 固有の URL ブリッジを削除する。

## 目的

- `openai_base_url` の TOML 設定項目と、それを credential broker に渡す特例配線を削除する。
- `OPENAI_BASE_URL` による OpenAI 組み込み credential source の接続先拡張も削除し、カスタム接続先を汎用 credential 設定へ移行する。
- credential broker の資格情報取得、ホスト制限、ダミー値による秘密情報保護、リモート実行時のローカル秘密情報非転送は維持する。
- モデルプロバイダーの `base_url` を credential broker の信頼済み接続先へ自動変換しない。

## 非目的

- credential broker 自体の削除や、組み込み GitHub/OpenAI credential source の削除。
- `my_provider` の認証方式・既定 URL・モデルプロバイダー登録の変更。
- 既存の汎用 `CredentialProviderConfig` のスキーマや URL 検証を再設計すること。
- `model_providers.<name>.base_url` を credential broker の許可先として暗黙に扱うこと。

## 現状の問題となる経路

現在は概ね次の経路で OpenAI 固有の値が流れる。

```text
ConfigToml.openai_base_url
  -> NetworkConfigInputs.credential_broker_base_url
  -> NetworkProxyConfig.credential_broker_openai_host
  -> CredentialBrokerState.openai_api_host
  -> OpenAI CredentialSource.host_binding(..., configured_host)
```

さらに、`OPENAI_BASE_URL` 環境変数も OpenAI credential source の host binding に入り、組み込み `OPENAI_API_KEY` を追加の接続先へ注入できる。

## 変更後の設計

### 1. カスタム接続先は汎用 credential provider で明示する

カスタムゲートウェイで資格情報を注入したい場合は、`features.network_proxy.credentials.<name>` に資格情報の環境変数と `url_prefixes` を指定する。`url_prefixes` は資格情報を注入できる接続先の明示的な許可リストとして扱う。

モデルプロバイダーの設定例は次のようになる。

```toml
[model_providers.my_gateway]
name = "my_gateway"
base_url = "https://gateway.example/v1"
wire_api = "responses"

[features.network_proxy.credentials.my_gateway]
env = ["MY_GATEWAY_API_KEY"]
url_prefixes = ["https://gateway.example/v1"]
```

`model_providers.my_gateway.base_url` だけでは資格情報注入は有効にならない。これにより、接続先の設定と秘密情報を送信できる接続先の承認を分離する。

### 2. 組み込み OpenAI source は標準接続先だけを扱う

組み込み OpenAI credential source の既定接続先 `https://api.openai.com` と既存の API キー保護は維持する。`OPENAI_BASE_URL` と `ConfigToml::openai_base_url` はこの source の接続先を拡張する手段として扱わない。

OpenAI 互換の別エンドポイントが必要な場合は、上記の汎用 credential provider を使用する。これにより、接続先ごとに資格情報を分離でき、OpenAI の API キーを別プロバイダーへ誤って注入しない。

### 3. credential broker の内部状態を provider-neutral にする

- `NetworkProxyConfig::credential_broker_openai_host` を削除する。
- `CredentialBrokerState::openai_api_host` を削除する。
- `CredentialSource::host_binding` の引数から、OpenAI 固有の任意ホスト引数を削除する。
- 組み込み source の registry 呼び出しは、環境変数だけから host binding を生成する。
- リモート executor に送る proxy 設定から、ローカル由来の OpenAI ホスト値を削除する。

組み込み OpenAI source の host binding は `api.openai.com` のみを返す。汎用 provider の host binding は既存の `url_prefixes` 検証・マッチングを利用する。

### 4. セキュリティ境界を維持する

- 資格情報の注入先は、組み込み source または汎用 provider の明示的な許可先だけに限定する。
- モデルプロバイダーの `base_url` や任意の環境変数から、許可先を暗黙生成しない。
- 既存の HTTPS、ユーザー情報を含む URL、ホストの正規化、URL prefix の検証を弱めない。
- 資格情報の変更時に既存の brokered credential が再利用されない既存の無効化・再取得動作を維持する。
- リモート実行時にローカルの credential broker 設定や秘密情報を転送しない。

## 実装範囲

### 設定と配線

- `crates/config/src/config_toml.rs` から `openai_base_url` を削除する。
- `crates/core/src/config/mod.rs` の `NetworkConfigInputs` と設定反映処理から、credential broker 用 URL 引数と setter 呼び出しを削除する。
- `crates/network-proxy/src/config.rs` から OpenAI 固有の trusted host フィールド・setter を削除する。OpenAI 専用でしか使われない trusted-host helper も削除し、汎用 URL 検証は既存の configured provider 実装に委ねる。
- `crates/network-proxy/src/proxy.rs`、remote config、関連 protocol 型から OpenAI 固有の host フィールドを削除する。

### credential broker

- `CredentialBrokerState` から OpenAI host state を削除する。
- built-in source の host binding callback を provider-neutral なシグネチャへ変更する。
- OpenAI source から `OPENAI_BASE_URL` の context/binding/invalidation 処理を削除し、既定の `api.openai.com` のみを残す。
- GitHub source と registry の callback 呼び出しを新しいシグネチャへ更新する。
- 汎用 configured provider の実装・設定形式は変更しない。

### ドキュメントとインベントリ

- `docs/pre-release-compatibility-inventory.md` に、`openai_base_url` と OpenAI 固有 host bridge の producer/consumer audit、および `remove` / `migrate-in-place` の完了判断を記録する。
- `docs/openai-decoupling-plan.md` の Phase 3 の該当項目を、実装済みの状態へ更新する。
- カスタム接続先の移行方法として、汎用 credential provider の TOML 例を記録する。

## 検証方針

リポジトリの検証方針に従い、standalone test は実行せず、次を使用する。

- 設定スキーマ・既存 config テストに `openai_base_url` が残っていないことを確認する。
- 汎用 credential provider の許可 prefix に対する credential injection と、未許可先での非注入をテスト対象にする。
- 組み込み OpenAI source が標準接続先で動作し、任意の `OPENAI_BASE_URL` や model provider の `base_url` によって許可先を増やさないことを確認する。
- remote config のシリアライズ対象から OpenAI 固有 host が消えていることを確認する。
- `just check` を実行する。
- Rust の callback、設定、protocol、test target に変更があるため `just check --tests` を実行する。
- checked-in generated artifact への影響を確認し、影響があれば `just generate` を実行して差分を確認する。

## 互換性と移行

この fork は pre-release で外部利用者がいないため、`openai_base_url` は後方互換の読み替えを追加せず削除する。従来その項目で OpenAI 互換ゲートウェイを設定していた利用者は、同じ接続先を汎用 credential provider の `url_prefixes` と専用の環境変数へ移行する。

移行後も、モデルプロバイダーの `base_url` はリクエストの送信先を決めるだけであり、資格情報の注入許可を与えない。この分離を仕様・テスト・ドキュメントで明示する。

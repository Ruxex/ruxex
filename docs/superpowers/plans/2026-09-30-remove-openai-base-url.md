# `openai_base_url` の provider-neutral 化実装計画

## 概要

`ConfigToml::openai_base_url` と、OpenAI 固有の credential broker trusted-host bridge を削除する。カスタム接続先で資格情報を注入する経路は、既存の `features.network_proxy.credentials.<name>.url_prefixes` に統一する。組み込み OpenAI credential source は `api.openai.com` の保護と既存の秘密情報処理を維持する。

作業は現在の `main` ブランチで行う。pre-release fork で外部利用者がいないため、旧設定を読み替える互換レイヤーは追加しない。

## 変更方針

- `model_providers.<name>.base_url` は HTTP リクエスト先だけを決め、credential broker の許可先にはしない。
- カスタム接続先と資格情報は、同じ provider 名の汎用 credential provider 設定で明示的に関連付ける。
- `OPENAI_BASE_URL` は組み込み OpenAI source の接続先拡張として扱わない。
- credential broker、秘密情報のダミー化、host binding、リモート実行へのローカル状態非転送は保持する。

## 実装タスク

### 1. 監査結果をインベントリへ反映する

対象: `docs/pre-release-compatibility-inventory.md`

- `ConfigToml::openai_base_url` の producer（TOML loader/core config）と consumer（network-proxy broker trusted-host state、shell snapshot、テスト fixture）を確認する。
- `OPENAI_BASE_URL` の producer/consumer と、組み込み OpenAI source の標準 `api.openai.com` fallback を分けて記録する。
- 旧 OpenAI 固有 bridge は `remove`、credential broker の汎用 security behavior は `retain`、カスタム接続先は既存 generic credential provider へ `migrate-in-place` と明記する。
- コード変更前にこの判断を記録し、実装完了後に affected symbols が消えたことを追記する。

### 2. 期待する挙動をテストへ先に反映する

production code を変更する前に、既存テストを新しい契約へ更新・追加する。standalone test は実行せず、後続の `just check --tests` で test target をコンパイルする。

対象:

- `crates/config/src/config_toml.rs` と関連 schema/config tests: `openai_base_url` が TOML contract に存在しないことを確認する。
- `crates/network-proxy/src/credential_broker_tests.rs`、`crates/network-proxy/src/credential_broker/configured_tests.rs`: `url_prefixes` を持つ configured provider が許可先でのみ資格情報を注入し、未許可先では注入しないことを確認する。
- `crates/network-proxy/src/credential_broker_tests.rs`: built-in OpenAI source が標準 host のみを binding とし、`OPENAI_BASE_URL` で追加 host を得ないことを確認する。
- `crates/network-proxy/src/remote_config_tests.rs`: remote config が credential broker のローカル状態を含めないことを確認する。
- 既存の `openai_base_url` を使う統合テスト fixture は、明示的な model provider と generic credential provider の設定へ移行するか、旧 override の挙動だけを検証している場合は削除する。

### 3. 設定から network proxy への OpenAI 固有配線を削除する

対象:

- `crates/config/src/config_toml.rs`
- `crates/config/src/loader/mod.rs`
- `crates/core/src/config/mod.rs`
- `crates/core/src/config/network_config.rs`
- `crates/network-proxy/src/config.rs`

変更内容:

- `ConfigToml::openai_base_url` と project-local config denylist の obsolete entry を削除する。
- `NetworkConfigInputs::credential_broker_base_url` を削除し、`PreparedNetworkConfig` が setter を呼ばないようにする。
- `NetworkProxyConfig::credential_broker_openai_host`、default 初期化、setter、OpenAI 専用 trusted-host helper と専用テストを削除する。
- 汎用 configured provider の URL 検証・host binding は変更しない。

### 4. credential broker の状態と callback を provider-neutral にする

対象:

- `crates/network-proxy/src/credential_broker.rs`
- `crates/network-proxy/src/credential_broker/providers.rs`
- `crates/network-proxy/src/credential_broker/providers/openai.rs`
- `crates/network-proxy/src/credential_broker/providers/github.rs`
- `crates/network-proxy/src/credential_broker/registry.rs`
- `crates/network-proxy/src/proxy.rs`

変更内容:

- `CredentialBrokerState::openai_api_host` と configure 時の専用比較・再設定処理を削除する。
- `CredentialSource::host_binding` を `fn(&HashMap<String, String>)` に変更し、registry の built-in 呼び出しから OpenAI host 引数を除去する。
- OpenAI source の `context_env_vars`、`binding_env_vars`、invalidation、host binding から `OPENAI_BASE_URL` と trusted-host helper を削除する。
- OpenAI source の host binding は `api.openai.com` のみを維持する。
- GitHub source の callback と built-in source の registry 呼び出しを新しい signature に合わせる。
- proxy が broker-only config を作る箇所から OpenAI host field を削除する。
- configured provider の host binding path は現状のまま利用する。

### 5. 既存の統合テスト・fixture を新しい設定経路へ移行する

対象はまず次の検索結果を再確認し、各テストの意図ごとに移行する。

- `crates/tui/tests/suite/resize_reflow.rs`
- `crates/exec/tests/suite/resume.rs`
- `crates/core/tests/common/test_codex_exec.rs`
- `crates/core/tests/suite/cli_stream.rs`
- `crates/core/tests/suite/multi_agent_resume.rs`
- `crates/core/src/shell_snapshot_tests.rs`
- `crates/app-server/tests/common/config_tests.rs`
- `crates/app-server/tests/common/config.rs`
- `crates/app-server/tests/suite/v2/cyber_access_program.rs`
- `crates/app-server/tests/suite/v2/model_list.rs`
- `crates/app-server/tests/suite/v2/turn_start.rs`
- `crates/config/src/loader/tests.rs`

移行ルール:

- provider routing のテストは `[model_providers.<name>]` の `base_url` を使う。
- credential injection のテストは `[features.network_proxy.credentials.<name>]` の専用 env と `url_prefixes` を使う。
- `openai_base_url` の存在・旧 override の挙動だけを確認するテストは、新しい契約の schema rejection/absence テストへ置き換える。
- `my_provider` の認証・既定 URLを変更するような fixture 変更は行わない。

### 6. ドキュメントと生成物を更新する

対象:

- `docs/openai-decoupling-plan.md`
- `docs/pre-release-compatibility-inventory.md`
- 必要に応じて設定スキーマ・checked-in generated artifacts

変更内容:

- Phase 3 の該当項目を完了状態へ更新する。
- 旧 `openai_base_url` から generic credential provider への移行例を記録する。
- `just generate` の差分を確認し、生成物が変わった場合だけ関連差分を含める。

## 検証

実装完了後、次の順で実行する。

1. `rg -n "openai_base_url|credential_broker_openai_host|set_credential_broker_openai_base_url|OPENAI_BASE_URL|openai_api_host" crates docs --glob '*.rs' --glob '*.md'` で意図したドキュメント言及を除き旧経路が残っていないことを確認する。
2. `git diff --check`。
3. `just check --tests`。
4. `just check`。
5. generated artifacts に影響する差分があれば `just generate` を実行し、再度 `git diff --check` と差分確認を行う。

standalone test コマンドはリポジトリの cleanup 方針に従い実行しない。

## 完了条件

- `openai_base_url` が production config/runtime/protocol/test fixture の旧経路として残っていない。
- `OPENAI_BASE_URL` が built-in OpenAI credential source の許可先を増やさない。
- generic credential provider の明示的 URL prefix でのみ custom credential injection が動作する。
- credential broker の秘密情報保護と標準 OpenAI host binding が維持される。
- `just check --tests` と `just check` が成功する。
- inventory と decoupling plan が実装結果を反映している。

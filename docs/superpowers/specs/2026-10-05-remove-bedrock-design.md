# Amazon Bedrock 対応削除 設計

## 背景

Amazon Bedrock 対応は、モデルプロバイダーの実装だけでなく、AWS SigV4
認証、credential export/auth refresh、login、app-server protocol、設定、CLI、
TUI onboarding、生成 schema、テストにまたがっている。Genex は未リリースで、これらの
repository-owned surface に外部利用者向けの互換性要件はない。

現行監査では `genex-aws-auth` の実行時利用は Bedrock のモデルプロバイダー経路に
限定され、app-server と model-provider の依存宣言も Bedrock 関連の到達性による。

## 目的

- Amazon Bedrock の Mantle/Runtime provider、動的モデル取得に付随する Bedrock 特例、
  bearer/API key/AWS credential の認証経路を削除する。
- Bedrock 用の login、app-server protocol/API、config、CLI、TUI onboarding、テスト、
  README、生成 schema を削除する。
- Bedrock 専用コードしか利用していない `genex-aws-auth` クレートと AWS SDK/SigV4
  依存を workspace から削除する。
- provider-neutral な custom provider、API key/headers/external bearer 認証、汎用の
  `/models` 取得、既存の security/platform fallback は維持する。

## 非目的と保持する契約

- OpenAI endpoint 自体の URL、wire field、認証環境変数・HTTP header、model identifier は
  変更しない。
- Bedrock の外部 endpoint 値を、サポート継続を示す active code/documentation としては
  残さない。ただし、互換性インベントリには削除判断と監査根拠を記録する。
- AWS という文字列だけを理由に無関係な platform/security 処理を削除しない。追加の AWS
  consumer が見つかった場合は、Bedrock 削除とは別の判断として記録する。
- pre-release fork のため、旧 Bedrock protocol/config/persisted-data reader 用の互換 alias
  は追加しない。

## 現状の producer / consumer 監査

| 境界 | producer | consumer | 判断 |
|---|---|---|---|
| Model provider | `model-provider/src/amazon_bedrock/**`、`ModelProviderInfo` の Bedrock registry、AWS auth recovery/export | model-provider、core external-auth tests、model-provider-info tests | `remove` |
| AWS signing | `crates/aws-auth` の credential loading と SigV4 API | Bedrock adapter とその tests のみ | `remove` |
| Login/auth | `GenexAuth` の Bedrock API key/access keys、keyring/storage、auth manager | model-provider、app-server、CLI/TUI、auth fixtures | `remove` |
| Protocol/schema | `AuthMode`、login params/response、account variant、Bedrock discover/setup requests | app-server、TUI、generated JSON/TypeScript、protocol tests | `remove` |
| Configuration | `model_providers.*.aws`、Bedrock validation/overrides、config schema/edit UI | config loader、core、CLI doctor、model-provider-info | `remove` |
| UI / test clients | TUI onboarding、app-server test client、app-server integration suites | user-facing CLI/TUI and test binaries | `remove` |
| Documentation / audit | Bedrock support instructions and current capability descriptions | README/docs and compatibility audit | active support text は `remove`、削除根拠は記録として `retain` |

## 変更後の設計

### 1. Protocol と生成 schema

`AuthMode` から Bedrock API key/access keys variant、v2 account/login の Bedrock
variant、`account/bedrock/discover` と `account/bedrock/setup` の request/response を
削除する。protocol の export/module、シリアライズテスト、checked-in JSON/TypeScript
schema は Rust producer から再生成し、API key と headers など残存する auth mode の
形だけを保持する。

### 2. Runtime surface

app-server の Bedrock auth/setup processor、config manager/message processor の分岐、
CLI doctor/login の Bedrock 分岐、TUI onboarding の Bedrock module・画面遷移、app-server
test client と専用 integration test を削除する。Bedrock 以外の account/auth status と
provider-neutral login は既存の dispatch を維持する。

### 3. Model provider と設定

`model-provider/src/amazon_bedrock/` と専用 tests、model-provider の module/export、
shared state の AWS credential export/auth recovery を削除する。`ModelProviderInfo` の
Bedrock constants、AWS auth config structs/fields、built-in provider registry、Bedrock
override validation と関連 tests/config schema を削除する。

モデル provider の一般的な `base_url`、wire API、custom provider の登録、provider-owned
dynamic `/models` retrieval は残す。Bedrock を暗黙に代替する fallback は追加しない。

### 4. AWS auth crate と依存関係

`crates/aws-auth` を削除し、workspace member/dependency、`Cargo.lock` の到達不能な
AWS SDK/SigV4 依存、app-server/model-provider の直接依存を整理する。削除前後に
workspace-wide producer/consumer search を行い、Bedrock 以外の AWS 利用を誤って消して
いないことを確認する。

### 5. 記録と生成物

`pre-release-compatibility-inventory.md` にこの slice の producer/consumer audit と
`remove` / `retain` 判断を追加し、`provider-decoupling-candidates.md` の Bedrock 候補を
実施済みへ更新する。現行サポートを示す README、設定例、テスト fixture、line-count
generated artifact は削除・更新する。履歴的な監査行は、削除対象を説明するために必要な
範囲だけ残す。

## 検証方針

- Bedrock の active Rust/config/schema/README/test producer・consumer が残っていないことを
  exact search で確認する。
- `genex-aws-auth` が workspace metadata、manifest、lockfile、source のいずれにも到達
  しないことを確認する。
- `just generate` を実行し、checked-in schema と生成物の差分を確認する。
- Rust の削除・移動・refactor があるため `just check --tests` と `just check` を実行する。
- 生成後の `git diff --check` と、OpenAI endpoint/wire/auth/model identifier の retain-only
  audit を行う。

## 完了条件

1. Bedrock の runtime/provider/login/protocol/config/UI/test surface が削除されている。
2. `genex-aws-auth` と Bedrock 専用 AWS SDK/SigV4 到達性が削除されている。
3. 残存する provider-neutral capability と OpenAI endpoint 契約が変更されていない。
4. 互換性インベントリと候補一覧に producer/consumer 根拠と判断が記録されている。
5. `just generate`、`just check --tests`、`just check` が成功し、生成物の意図しない差分がない。

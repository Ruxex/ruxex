# Remove residual `ChatGPT` references from Rust sources

作成日: 2026-10-01

## 概要

Rust source と test source に残る `chatgpt` 表記を、大文字小文字を区別せず全件監査し、不要な ChatGPT 専用実装・fixture・文言を削除する。現役の provider-neutral な機能は汎用名へ移行し、永続化形式・外部 wire contract・有効なプライバシー境界のために文字列を変更できない場合だけ、明示的な保留としてインベントリに記録する。

単純な文字列置換や grep 回避のための分割・エンコードは行わない。各 match は producer と consumer を追跡してから、`remove`、`migrate-in-place`、`retain`、または `hold` のいずれかを決める。

## 目的

- Rust source、test source、doc comment、string literal に残る不要な `ChatGPT` 表記をなくす。
- ChatGPT 専用の認証、固定 URL、response metadata、hosted capability の残骸を削除する。
- 現役の認証・セキュリティ・永続化読者・provider-neutral capability を壊さず、必要なら汎用境界へ移行する。
- すべての production match を `docs/pre-release-compatibility-inventory.md` に producer/consumer、canonical replacement、decision、verification 付きで記録する。

## 非目的

- Rust 以外のファイルを、今回の検索結果だけを理由に一括変更すること。生成物や設定は Rust の変更で実際に影響を受ける場合だけ更新する。
- 文字列を難読化して検索結果だけを減らすこと。
- 現在の platform fallback、security behavior、provider-neutral capability を、名前に `legacy`、`fallback`、または外部 product 名が含まれるという理由だけで削除すること。
- 既存の persisted data を削除する migration を追加すること。

## 完了条件

1. `rg -ni 'chatgpt' --glob '*.rs' .` で得られる全 match が監査済みである。
2. 不要な match は削除または provider-neutral な名称・値へ移行されている。
3. 残る match がある場合、各 match が inventory の保留行と一対一で対応し、変更不能な理由、現行 consumer、将来の解除条件が記載されている。保留理由が成立しない match は残さない。
4. 検索スクリプトと手動の Rust-only 検索結果に未説明の差分がない。
5. Rust 変更について `just check --tests` と `just check` が成功し、checked-in generated artifact に影響した場合は `just generate` 後の diff を確認している。
6. 作業は監査・実装・fixture 整理・生成物/最終検証のまとまりごとにコミットされている。

## 分類基準

### `remove`

現行の producer がなく、consumer も不要な ChatGPT 専用機能、dead code、test fixture、snapshot、専用の説明文は削除する。削除により generic consumer が失われる場合は、consumer とそのテストも同じ slice で整理する。

### `migrate-in-place`

現行 consumer が存在するが、ChatGPT という product boundary は不要な場合に使う。型、関数、変数、テスト fixture、protocol field を provider-neutral な名称へ変更し、挙動と security boundary を維持する。外部から受け取る値は、内部型を汎用化したうえで境界 adapter に閉じ込める。

### `retain`

`chatgpt` という単語自体を残さずに現行機能を維持できる場合に使う。例えば generic な security predicate や product restriction は保持するが、ChatGPT 固有の検出・分岐・名称は削除または移行する。

### `hold`

コード中の文字列を変更すると、現行の persisted reader、外部 wire protocol、または active privacy/security contract が壊れ、同じ意味を持つ provider-neutral replacement が存在しない場合だけ使う。hold には exact path/symbol、producer、consumer、変更不能性、代替案を採用しない理由、解除条件、verification command を記録する。テスト専用の値、古い fixture、コメント、表示文は hold の根拠にならない。

## 監査対象と順序

### 1. Persisted rollout/thread-history readers and migrations

`SessionSource`、product restriction、thread resume の client name、rollout/history conversion を調査する。既存履歴の reader を維持する必要があるか、producer が現存するかを確認する。過去値のために本番 reader が必要なら、最小限の境界に隔離して hold する。テストだけが値を使う場合は generic fixture へ移行するか削除する。

### 2. Protocol/app-server aliases and schemas

認証 header、response metadata、request/response schema、生成 JSON/TypeScript との producer/consumer を調査する。専用 hosted contract が消滅していれば protocol と generated artifact を削除する。外部 wire contract が active の場合は adapter に限定し、inventory に hold を記録する。

### 3. CLI/config/import compatibility

`auth_mode`、login state、provider routing、config requirement、import fixture を調査する。ChatGPT 専用 auth/config path は削除し、provider-neutral な auth/provider path は維持する。既存 persisted config を読む必要がない pre-release boundary では、新たな no-op compatibility reader を作らない。

### 4. UI/observability/runtime shims

TUI、doctor、network policy、response debug、privacy redaction、telemetry、コメント、表示文を調査する。表示・テスト名・diagnostic metadata は汎用化または削除する。redaction やその他の security behavior は、同じ境界を provider-neutral に置き換えられるか確認してから変更する。

### 5. Build-system/experimental scaffolding

Rust source から参照される生成物、workspace edge、未登録 test source、fixture directory、build helper を確認する。ChatGPT 専用 capability の残骸だけを削除し、現役の platform/build fallback は保持する。

### 6. Final dependency/documentation audit

全 Rust match、workspace metadata、generated artifact、関連 docs、検索スクリプト出力を再確認する。未説明の match が一つでもあれば完了にしない。

## 実施単位とコミット方針

1. **監査・inventory**: 現在の全 match を分類し、production match を inventory に追加または既存行へ統合する。ここでは Rust code を変更しない。
2. **production cleanup**: persisted/protocol/config/runtime の順に、各 capability の producer と consumer を一体で削除・汎用化する。
3. **test/fixture cleanup**: production の決定に合わせて obsolete fixture、test helper、snapshot、test-only import を削除・汎用化する。
4. **generated/final audit**: `just generate`、`just check --tests`、`just check`、`git diff --check` を実行し、全検索結果と inventory を照合する。

各単位は独立したコミットにする。コミット前に対象範囲の検索と必要な検証を実行し、途中のコミットでも意図と検証結果が追跡できるようにする。

## 安全境界

- `ChatGPT-Account-ID` のような header は、まず現行 producer/consumer と外部 contract を確認する。単に名前を置換して別 provider に誤送信しない。
- 固定 `chatgpt.com` endpoint は、設定済み provider の endpoint や generic transport が現行 consumer を満たすか確認してから削除する。
- remote client 名による response-only redaction は、producer がリポジトリ外でも active privacy boundary なら、同等の provider-neutral contract がない限り hold 候補とする。
- persisted data の reader を削除・変更する場合、既存履歴を壊す証拠と、pre-release 方針との整合を inventory に明記する。
- `openai_chatgpt_moderation_metadata` などの metadata は、consumer が実際に UI/security/telemetry へ到達するかを確認し、不要なら parser と fixture をまとめて削除する。


# 全クレート Genex 命名移行 設計書

作成日: 2026-10-03

## 概要

`crates/` 配下の全 workspace member を対象に、依存の少ないクレートから順番に
`codex` の出現箇所を監査し、リポジトリ所有の名称を `Genex` / `genex` へ移行する。
Rust の内部シンボルだけでなく、crate/package、ファイル・ディレクトリ、protocol/schema、
永続化データ、CLI/config、UI、生成物、build/test fixture も対象に含める。

この作業は単純な文字列置換ではない。各 match について producer/consumer、実行時の
reader/writer、生成物、外部境界を確認し、変更前に compatibility inventory へ
`remove`、`migrate-in-place`、または `retain` の判断を記録する。クレート間に参照が
またがる場合は、consumer 側まで同じ移行 wave で更新する。

## 目的

- 全 workspace member を一度ずつ監査し、`no-match` を含めて未分類クレートを残さない。
- リポジトリ所有の `Codex` / `codex_*` 名称を、依存関係と consumer map に沿って Genex へ移行する。
- protocol、schema、永続化形式、CLI/config、UI、package/build 名を、監査済みなら同じ移行に含める。
- 保持した `codex` match について、変更できない理由と producer/consumer を台帳で追跡可能にする。
- provider 固有機能の削除や機能変更を、命名移行へ黙って混ぜ込まない。

## 非目的

- OpenAI endpoint 自体が要求する URL、request/response field、authentication variable/header、model identifier の変更。
- 外部 OpenAI fork/upstream、第三者 protocol/package、歴史的帰属・法的記録の名称変更。
- `codex` が別単語の一部分にすぎない識別子の変更。
- 外部リポジトリ URL、外部から取得する参照先、既存の security boundary の変更を名前だけで行うこと。
- producer/consumer 監査なしの repository-wide 機械的置換。

## 成功条件

1. `cargo metadata --no-deps` に含まれる全 workspace member が、依存順の監査台帳に一度だけ現れる。
2. 各クレートの `rg -ni 'codex'` 結果が、`migrate-in-place`、`remove`、`retain` のいずれかに分類され、未説明の match がない。
3. リポジトリ所有の API、protocol/schema、永続化、CLI/config、UI、package/build 名は、監査で保持理由が確認できない限り Genex へ移行される。
4. 保持対象には exact path/symbol/string、producer、consumer、保持理由、将来変更できる条件、検証方法を `docs/pre-release-compatibility-inventory.md` に記録する。
5. provider capability、endpoint、auth、telemetry、hosted assumption を発見した場合、変更を命名移行から分離し、候補台帳にも記録する。
6. Rust 変更後に `just check --tests` と `just check` が成功する。生成物に影響する wave では `just generate` を実行し、意図した差分だけが残る。
7. 最終的に、保持理由のない内部 `codex` 名称が残らず、保持された文字列は除外条件に対応する台帳行を持つ。

## 分類と判断基準

### `migrate-in-place`

producer と consumer がリポジトリ内にあり、名前を変えても OpenAI endpoint 固有契約、
第三者契約、歴史・法的記録を壊さない場合に使う。型名、関数名、module、crate名、
protocol/schema名、serialized field、CLI/config、UI文言、fixture、生成物を含む。
既存データや reader がある場合も、pre-release fork の方針と全 reader/writer の同時更新を
確認できれば移行する。

### `remove`

旧名に対応する dead code、不要な compatibility shim、未使用 fixture、生成物、または
不要な provider 固有経路で、producer と consumer を監査した結果、残す必要がない場合に使う。

### `retain`

次のいずれかに該当し、Genex への置換が外部契約の変更または意味の破壊になる場合だけ使う。

- OpenAI endpoint 必須の URL、wire field、auth variable/header、model identifier
- 外部 OpenAI fork/upstream または第三者 protocol/package の識別子
- 外部リポジトリ URL・参照先
- 歴史的な attribution、copyright、license、法的記録
- `codex` が別の単語の内部に現れるだけで、Genex の identity を表さない文字列
- 現行 security boundary、OS keychain/lock namespace、または外部状態を変更すると既存利用状態を壊す識別子

単に「既存名だから」「外部との互換性がありそうだから」「高 fan-out だから」は保持理由にしない。
高 fan-out は consumer を列挙してから横断移行する理由であり、恒久的な defer ではない。

## 監査順序

### 1. 全体 inventory と依存順位の生成

`cargo metadata --no-deps --format-version 1` から workspace member と内部依存 graph を取得する。
依存される側の数が少ない leaf から始め、topological layer を優先する。layer 内では、
transitive dependent 数、direct dependent 数、match 数の少ない順、最後に package 名順で処理する。
各 member を `rank`、依存先、依存元、Rust match 数、path match 数、decision、wave とともに
一時 audit artifact に記録する。

### 2. クレート単位の `rg` 監査

各クレートについて、Rust source、Cargo manifest、build script、test、fixture、README、
生成入力を対象に case-insensitive `rg` を実行する。match は次の境界に分類する。

- path/crate/package/module
- Rust symbol/import/re-export
- protocol/schema/serialized field
- persisted rollout/thread-history/SQLite/JSON
- CLI/config/env/import
- UI/telemetry/diagnostic
- build/generated/test fixture
- 外部契約・provider capability・security boundary

### 3. producer/consumer 監査と台帳更新

定義側だけでなく、workspace 全体の参照、別 binary、生成器、reader/writer、snapshot、
外部境界を確認する。変更前に inventory の対象 section へ exact path、canonical replacement、
producer/consumer、decision、verification を追加する。監査中に OpenAI 依存の機能を見つけたら、
`provider-decoupling-candidates.md` に独立した follow-up として記録する。

### 4. 横断リネーム・削除

台帳に記録した一つの symbol/name family を producer と全 consumer で同時に変更する。
高 fan-out 型、schema、persisted reader/writer、CLI snapshot、generated artifact は、同じ
family の全 producer/consumer を列挙してから変更する。別 family の provider capability
削除や挙動変更は同じ commit に混ぜない。

### 5. wave 検証と次クレートへの移行

各 wave で old/new exact-match audit、`git diff --check`、必要な `just generate`、
`just check --tests`、`just check` を実行する。失敗した場合は原因を特定して同じ wave 内で直し、
検証済みの状態から次の依存順位へ進む。

### 6. 最終監査

workspace metadata、全 crate の `rg -ni 'codex'`、生成物、docs、Cargo.lock、package/path
参照を再確認する。残った match はすべて台帳の `retain` 行と一対一に対応させる。

## 安全境界

- 外部 URL は URL 全体を置換せず、URL 内の repository/provider 契約か、リポジトリ所有の表示名かを判定する。
- protocol/schema/persisted 名は、リポジトリ内の producer と reader/writer を同時に更新できる場合に移行する。外部 reader が必要な場合のみ retain とする。
- CLI/config/env 名は、pre-release fork の repository-owned surface なら移行する。OpenAI endpoint/auth に必要な変数だけは保持する。
- security path、sandbox、keyring、OS namespace は、名前と挙動を分離して監査する。名前が外部状態の識別子なら retain、内部名なら migrate-in-place とする。
- 既存の `legacy` / `fallback` という語は、現行 platform fallback や provider-neutral capability を表す場合、`codex` でない限り名前だけを理由に削除しない。

## 成果物

- `docs/pre-release-compatibility-inventory.md`: 各 match の decision と producer/consumer 根拠
- `docs/provider-decoupling-candidates.md`: 命名移行中に発見した provider-specific follow-up
- `target/genex-symbol-audit/`: workspace coverage、依存順位、crateごとの match と wave の一時成果物
- 更新された source、Cargo metadata、protocol/schema、persisted reader/writer、CLI/UI、fixture、生成物

## 検証方針

standalone test 実行は既定にしない。Rust target のコンパイルと workspace の整合性には、
プロジェクト指定の `just check --tests` と `just check` を使う。checked-in generated artifact、
schema、snapshot、Cargo metadata に影響する場合は `just generate` を実行し、結果を目視で確認する。

# OpenAI 固有機能の段階的な分離計画

## 目的

このリポジトリから OpenAI 固有の機能と前提を段階的に取り除き、共有アーキテクチャをプロバイダー非依存の AI エージェント基盤にする。

クレートや機能を一度に削除すると、依存関係を通じて変更範囲が急拡大し、長期間ビルドできない状態になるおそれがある。そのため、依存関係の可視化、影響範囲の小さいクレートの削除、CLI 表面の縮小、未使用コードの調査、という順序で進める。

## 共通方針

- 各 Level と Phase は順番に実施し、前段階の成果物と検証結果を次段階の判断根拠にする。
- 一度の変更範囲を小さく保ち、各 Phase の完了時点でビルドと関連テストが通る状態に戻す。
- OpenAI 固有かどうかは、名前だけでなく、認証、API、ホスト型サービス、クラウド機能、プロトコル、設定、UI からの到達経路を含めて判断する。現行の capability が OpenAI 固有の外部契約なしに成立しないなら、その capability と、それだけを支える補助クレートは OpenAI 固有機能として扱う。
- 実装が一般的な WebRTC、音声入出力、HTTP transport、tool schema などを含むことだけを理由に残さない。現行の非 OpenAI provider 到達経路またはローカル利用がある部分だけを provider-neutral な基盤として残す。
- 一つのクレートに固有 adapter と汎用処理が混在する場合は、capability 単位で判定する。非 OpenAI 経路がなければクレートを含めて削除し、ある場合は固有 adapter を隔離・削除して汎用部分を残す。
- 削除対象と、その対象に依存する側を区別して記録する。依存されているという理由だけで同時に削除しない。
- 各変更では、削除した機能、残した現行互換性、意図的に破壊した旧形式、既知の未対応事項を記録する。

## Level 0: クレート依存関係の可視化

ルート `Cargo.toml` の `workspace.members` に列挙されたクレートについて、各クレートの `Cargo.toml` を直接読み取る Python スクリプトを作成し、機械可読な JSON と、人が確認できる HTML を生成する。Cargo コマンドや外部依存の解決には依存しない。

生成コマンドは次のとおり。

```shell
just crate-dependency-graph
```

Python 3.11 以降を使用する。既定では `target/crate-dependency-graph/crate-dependencies.json` と `target/crate-dependency-graph/crate-dependencies.html` を生成する。`--output-dir` で出力先を変更できる。

### 成果物

- クレートごとの直接依存関係を収録した JSON
- 依存元と依存先をたどれる可視化 HTML
- JSON と HTML を再生成する Python スクリプト
- 実行方法、出力先、前提ツールを記した説明

### JSON に含める情報

- クレート名とマニフェストのパス
- 通常依存、開発依存、ビルド依存、および target 条件の区分
- optional dependency と feature による有効化条件
- 各クレートの直接の依存先と依存元

外部クレートは対象に含めず、メンバークレート間の依存だけを収録する。`workspace = true` の依存は、ルートの `workspace.dependencies` を参照して内部依存かどうかを判定する。

可能であれば、後続の分析に利用できるよう、推移的な依存先、推移的な依存元、循環、依存の深さも算出する。

### 完了条件

- 現在のワークスペースに対してスクリプトを再実行できる。
- 出力が決定的で、差分としてレビューできる。
- HTML から、任意のクレートの依存先と影響先を確認できる。
- Level 1 で削除候補の影響範囲を比較できる。

## Level 1: OpenAI 固有クレートの特定と小規模な削除

### Phase 1: 候補クレートと影響範囲の整理

OpenAI 固有の機能を担うクレートを列挙し、Level 0 の依存グラフを使って影響範囲を整理する。

調査結果は [OpenAI 固有クレート候補と影響範囲](openai-specific-crates.md) に記録する。

各候補について、少なくとも次を記録する。

- OpenAI 固有と判断した根拠
- 提供している機能
- 直接依存している内部クレート
- そのクレートへ直接または推移的に依存する内部クレート
- CLI、TUI、app-server などからの到達経路
- feature flag、設定、テスト、ドキュメントへの影響
- 削除、汎用化、隔離、保留の推奨区分
- 想定される削除難易度とリスク

候補は影響範囲の小さい順に並べ、Phase 2 の実施単位を決める。OpenAI 固有に見えても、汎用的な責務を含むクレートは即時削除せず、責務分割の候補として扱う。

### Phase 2: 影響範囲の小さいクレートの削除

**進捗:** 完了。`codex-responses-api-proxy`、Cloud Tasks 3 クレート、`codex-chatgpt` は削除済み。

Phase 1 で整理した候補から、依存元が少なく、代替や責務移動が不要または限定的なクレートを削除する。

削除は小さな単位で行い、クレート本体に加えて次を処理する。

- ワークスペースメンバーと依存宣言
- feature flag と設定項目
- 呼び出し箇所と登録処理
- 対象クレート専用のテスト、fixture、ドキュメント
- 削除によって新たに未使用となった private code

### 完了条件

- 対象クレートと専用の到達経路が削除されている。
- ワークスペース全体がビルドでき、関連テストが通る。
- 削除対象外のプロバイダーや汎用機能に回帰がない。
- 依存関係 JSON と HTML が更新されている。
- 影響が大きい候補は無理に同じ Phase へ含めず、理由とともに保留されている。

### Phase 3: 残存 integration の段階的な隔離・削除

**進捗:** 進行中。`codex-cloud-config`、backend の user-settings による commit/PR attribution を担う `codex-git-attribution`、および remote-memory の backend rate-limit guard は削除済みである。workspace message、`/accounts/check` による workspace account routing、account usage/rate-limit 読み取り、リセット credit、workspace owner nudge、hosted analytics dashboard、backend turn-cost collection の API・UI・テスト到達経路を削除した。依存元がなくなったため、`codex-backend-client` と `codex-backend-openapi-models` も削除済みである。OpenAI Images adapter、専用 Images API endpoint、Realtime voice capability の API・transport・protocol・履歴・core・app-server・TUI 到達経路、OpenAI hosted web-search adapter の自動 `web.run` 到達経路、および analytics の OpenAI hosted sink も削除済みである。`codex-core-plugins` の OpenAI curated plugin startup sync（GitHub Git/HTTP/export archive producer、同期スレッド、専用 telemetry）と、同期結果だけを読むローカル curated marketplace キャッシュ経路も削除済みである。feedback capability についても、設定・protocol・app-server upload・TUI/exec UI・Guardian 失敗レビュー保持・専用クレート・Sentry/backend sink を削除し、provider-neutral なログ保存と hook feedback は保持した。外部ユーザー0のpre-release forkであるため、`docs/pre-release-compatibility-inventory.md` の互換性棚卸しと producer/consumer 根拠の記録を継続する。現行 runtime に必要な platform fallback、active client、security behavior、provider-neutral capability は個別根拠を記録して残す。

**機能単位の再監査:** Feedback の削除で確認した基準を、残存する Hosted capability にも適用する。クレート名ではなく、認証、固定 endpoint、hosted protocol、設定、feature、protocol、UI までを一つの到達可能な機能として監査する。現時点では次の機能を、Phase 3 の独立した実施単位として追加する。

- ChatGPT Remote Control は、ChatGPT 専用 URL・認証・enrollment 永続化・WebSocket・CLI・app-server protocol を含むため、機能全体を削除する。汎用 remote transport は現行 capability として存在しないため、この機能のためだけに抽象化を残さない。
- Hosted Plugin Marketplace / Sharing / Runtime は、OpenAI curated catalog、ChatGPT backend の検索・共有・mutation・bundle・remote runtime を削除する。ローカル、bundled、Git/npm などの provider-neutral な plugin loader、hook、tool、marketplace root は保持する。
- ChatGPT Apps / Connectors / host-owned Apps MCP は、connector directory、ChatGPT account/auth、Apps protocol、Apps UI、OpenAI 固有 MCP extension を削除する。通常の MCP server、汎用 OAuth/elicitation、provider-neutral な tool metadata は保持する。
- ChatGPT 認証と Agent Identity は、`codex-agent-identity` を全体削除候補とし、`codex-login` から ChatGPT OAuth/device/access-token とそれに依存する protocol/UI を削除する。Bedrock、workload identity、汎用 credential storage/keyring は保持する。

`codex-model-provider-info`、`codex-api`、`codex-http-client`、`codex-mcp` は、上記 capability を外した後に OpenAI 固有 adapter と現行の互換 transport を再監査する。Responses-compatible な非 OpenAI provider、標準 MCP、汎用 HTTP transport を利用する経路があるため、クレート全体の削除は先行して決めない。

**方針:** 全バイナリを起点にした Hawk の全面解析は、現時点では実行時間に対して得られる判断材料が少ないため後続へ延期する。まず依存グラフと参照検索を用いて、直接依存元が少ない hosted integration から削除可能な境界を作る。各変更では、対象クレートの公開 API と利用箇所を列挙し、OpenAI 固有の利用機能を一つだけ外す。依存元がなくなったクレートだけを削除し、汎用責務を含むクレートは先に provider-neutral な境界へ分離する。

**実施順序:**

1. `codex-cloud-config` の hosted policy source をローカル設定読み込みから切り離す。**完了**
2. `codex-backend-client` の account/usage/workspace、remote memory、git attribution などを機能単位で外し、依存元がなくなった後に `codex-backend-openapi-models` とともに削除する。**完了**
3. image generation、web search、analytics hosted sink、OpenAI curated plugin startup sync/cache、feedback の各 capability を整理する。OpenAI 固有部分を削除し、provider-neutral なログ、tool、plugin、media 表現を保持する。**完了**
4. Realtime voice capability を削除する。OpenAI Realtime の WebSocket / WebRTC call 契約、core の会話管理、app-server / TUI の voice UI・protocol・設定・履歴、および専用の `codex-realtime-webrtc` と `codex-voice-host` を削除した。通常のテキスト会話、汎用 audio utility、provider-neutral な media 処理は保持する。**完了**
5. ChatGPT Remote Control capability を削除する。transport、app-server protocol、daemon、CLI、state persistence、設定、ドキュメントを一単位として扱う。**完了**
6. Hosted Plugin Marketplace / Sharing / Runtime を削除する。`codex-core-plugins` の remote service、hosted search/share/install RPC、CLI/TUI hosted paths、remote analytics smoke client、protocol/schema surface を削除し、local/bundled/Git/npm plugin foundation は保持する。**完了（working tree）**
7. ChatGPT Apps / Connectors / host-owned Apps MCP を削除する。`codex-connectors` と専用 extension の依存元・汎用 MCP 利用を監査し、不要になったクレートは同じ単位で削除する。
8. ChatGPT 認証と Agent Identity を削除する。`codex-agent-identity` は全体削除を先に検証し、`codex-login` は ChatGPT 専用 branch と protocol/UI だけを外して汎用 credential 経路を残す。
9. 上記を反映した依存グラフを再生成し、`codex-model-provider-info`、`codex-api`、`codex-http-client`、`codex-mcp` の OpenAI adapter と provider-neutral な共有部分を再評価する。

### Phase 3 の完了ゲート

- `Feature::RemotePlugin`、`Feature::Apps` などの stable/default feature と、それらを有効にする設定・protocol・UI・CLI の producer/consumer 監査が完了している。
- ChatGPT/OpenAI が所有する hosted service へ到達する本番経路が、明示的に保持した provider-compatible transport と区別されている。
- `codex-agent-identity`、`codex-login`、`codex-connectors`、`codex-core-plugins`、`codex-mcp` の削除/分割判断が、クレート名ではなく capability ごとに記録されている。
- `docs/pre-release-compatibility-inventory.md` の各行に producer、consumer、`remove`/`retain`/`migrate-in-place`、検証コマンドがある。
- 非 OpenAI provider、ローカル実行、標準 MCP、platform fallback、security behavior を意図せず削除していない。

各実施単位では、`just crate-dependency-graph` による依存辺の確認、対象 API と利用箇所の参照検索、`just check` を行う。この cleanup ではテストを追加・実行せず、`just check` と生成物・参照差分を主な検証手段とする。Hawk は、対象クレートの直接依存元に関係する少数のバイナリだけを補助的に解析し、全バイナリ起点の結果合成は残存候補を十分に減らした後に実施する。

`codex-git-attribution` の削除では、app-server と CLI の extension 登録、backend user-settings の取得、専用テスト、workspace member と依存宣言を取り除いた。保存済み rollout から旧 attribution state を除去する互換処理は、現行 consumer の棚卸し後に pre-release compatibility inventory の対象として扱う。

## Level 2: `codex-cli` の OpenAI 固有コマンドの削除

### Phase 1: CLI 表面だけを縮小

**進捗:** 完了。`codex app` とトップレベルの `codex update`、および `codex login` の ChatGPT OAuth、device auth、OpenAI API key、access token の各経路は削除済み。旧トップレベルコマンド名は対話プロンプトとして誤実行されないよう、CLI 入口で明示的に拒否する。provider-neutral な認証状態確認に利用できる `codex login status` と、Amazon Bedrock の設定解除にも利用する `codex logout` は残している。TUI 終了時の更新処理と app-server daemon の更新処理は、この Phase の対象外として残している。

**後続対応:** `codex-cli` 外に残っていた TUI の Desktop app tooltip と status card、app-server daemon README、exec-server README の旧 `codex app`、`codex update`、`codex login` への案内も削除済み。

`codex-cli` クレートが公開するコマンドのうち、OpenAI 固有機能に関わるものを削除する。この Level では変更範囲を `codex-cli` クレート内に限定し、依存クレートの実装は削除・変更しない。

対象コマンドごとに、コマンド定義、引数、dispatch、help、補完、CLI 専用テストを確認する。コマンド削除により `codex-cli` 内で未使用になった import、関数、型、モジュールなどの dead code は同じ Phase で処理する。

依存クレート側に未使用コードが残っても、この Level では削除しない。後続の Level 3 で評価できるよう、到達不能になった機能と関係クレートを記録する。

### 完了条件

- OpenAI 固有コマンドが CLI の help と dispatch から消えている。
- `codex-cli` クレート内に、削除したコマンド由来の dead code や警告が残っていない。
- 依存クレートには意図しない変更がない。
- CLI のビルドと関連テストが通る。
- Level 3 で調査する到達不能コードの候補が記録されている。

## Level 3: 追加削除候補と公開未使用コードの調査

Level 1 Phase 3 と Level 2 の完了後に依存グラフを再生成し、さらに削除可能なクレートがないか検討する。

あわせて Hawk を使用し、依存グラフの最上位クレート（現時点では `codex-cli` を想定）から到達性を解析して、Dead and Public なコードを列挙する。最上位クレートは固定せず、Level 0 の結果と実際のエントリーポイントに基づいて確定する。
Hawk の起点となるバイナリ一覧と、複数起点の結果を合成する判断基準は
[Hawk 解析のバイナリ起点](hawk-entrypoints.md) に記録する。

調査結果は、少なくとも次の区分で整理する。

- 到達不能で、安全に削除できる公開コード
- feature、別バイナリ、テスト、外部利用からのみ到達するコード
- 公開 API 互換性のために残すコード
- 汎用化または責務移動を先に行うコード
- 追加で削除可能なクレート

Hawk の結果だけで自動的に削除を決めず、feature 構成、複数バイナリ、ライブラリとしての外部利用、動的な登録や dispatch を確認する。

### 完了条件

- 解析対象となる最上位クレートと feature 構成が明記されている。
- Dead and Public なコードの一覧に、到達不能と判断した根拠が付いている。
- 追加の削除候補クレートについて、依存元と影響範囲が整理されている。
- 削除可能、要分割、互換性維持、保留の判断が記録されている。
- 次の削除作業を、小さく独立した変更単位へ分解できている。

## 実施順序と判断ゲート

1. Level 0 の依存関係 JSON と HTML を生成する。
2. Level 1 Phase 1 で OpenAI 固有クレートと影響範囲を一覧化する。
3. 一覧をレビューし、影響範囲の小さい候補だけを Level 1 Phase 2 で削除する。
4. 依存グラフを更新し、削除による変化を確認する。
5. Level 2 で `codex-cli` 内の OpenAI 固有コマンドと同クレート内の dead code を削除する。
6. 再度依存グラフを更新する。
7. Level 1 Phase 3 で capability 単位の hosted integration を監査し、小単位で隔離・削除する。
8. Level 3 で追加のクレート削除候補と Dead and Public なコードを調査する。
9. 調査結果を、次の段階的な削除計画へ反映する。

各ゲートでは、ビルドが通らない状態のまま次の Level へ進めない。また、想定より影響範囲が広い場合は対象を分割または保留し、一括修正へ拡大しない。

## 検証の基本単位

各変更では、変更対象に応じて次を実施する。pre-release compatibility cleanup と crate removal では `just check` を主検証とし、テスト実行は行わない。

- Rust のフォーマットと静的検査
- 変更対象クレートの `cargo check` または `just check`
- ワークスペース全体のビルドまたはチェック
- CLI の help と代表的な非 OpenAI コマンドのスモークテスト
- 依存関係 JSON と HTML の再生成、および差分確認

時間や環境の制約で全体検証を省略する場合は、未実施項目と理由を明記する。

# OpenAI 固有クレート候補と影響範囲

## 目的と調査方法

この文書は、[OpenAI 固有機能の段階的な分離計画](openai-decoupling-plan.md) の Level 1 Phase 1 の成果物である。OpenAI または ChatGPT の認証、API、ホスト型サービス、クラウド機能を直接実装するクレートと、汎用責務の中にそれらを含むクレートを分け、Phase 2 で安全に削除できる単位を決める。

調査はコミット `f7affd3a0` を基準に、次の情報を照合した。

- `just crate-dependency-graph` で生成した Level 0 の JSON（150 クレート、894 依存辺）
- 各候補の `Cargo.toml`、README、公開 API、テスト
- `codex-cli`、`codex-tui`、`codex-app-server` からの import と dispatch
- OpenAI/ChatGPT の URL、認証方式、設定名、機能名を参照するコード

## OpenAI 固有機能の判定基準

削除判断の単位はクレート名や内部実装の一般性ではなく、現行で利用可能な **capability** とする。capability が OpenAI/ChatGPT の認証、固定 endpoint・wire protocol・モデル名・専用 header・hosted service 契約のいずれかなしには成立しない場合、OpenAI 固有機能である。

- 非 OpenAI provider またはローカルだけで同じ capability を利用できる経路がある場合、その共通処理は残す。OpenAI 固有 adapter のみを隔離・削除する。
- 非 OpenAI 経路がない場合、WebRTC、音声 device、codec、HTTP、tool schema などに一般性があっても、当該 capability の実装と専用補助クレートを削除対象に含める。将来の provider のためだけにコードは温存しない。
- 一つのクレートに両者が混在する場合、クレート全体のラベルで結論を出さない。呼び出し元、設定、wire contract、テストを capability ごとに追い、削除・隔離・保持を決める。

この基準により、ImageGen は OpenAI Images adapter を削除する隔離対象、Web Search は hosted OpenAI search だけを外す汎用化対象、Realtime voice は capability 全体を削除する対象である。

表中の「直接 / 推移」は、そのクレートへ依存する内部クレート数であり、通常依存、開発依存、ビルド依存、target 条件付き依存を含む。推移依存元の全一覧は Level 0 の JSON を正とし、ここでは判断に必要な直接依存元と主要な到達経路を記載する。

## Workspace member の扱い

`codex-chatgpt` はルートの `[workspace.dependencies]` にある path dependency を通じ、Cargo からは暗黙の workspace member として扱われていた。一方、Level 0 の生成スクリプトは `workspace.members` に明記されたクレートだけを対象とするため、当初の JSON には含まれていなかった。

分析基準を揃えるため、`crates/chatgpt` を `workspace.members` に明示し、依存グラフを再生成した。この変更は Cargo の build 対象を増やすものではなく、Cargo と Level 0 スクリプトの member 集合を一致させるものである。

## 候補一覧

影響範囲が小さく、独立して判断しやすい順に並べる。同じ実施単位に含める必要があるクレートは連続して記載した。

| 順位 | クレート | 直接 / 推移 | 固有性 | 推奨 | 難易度 | 主なリスク |
| ---: | --- | ---: | --- | --- | --- | --- |
| 1 | `codex-responses-api-proxy` | 1 / 1 | OpenAI Responses API 専用 | 削除 | 低 | 隠し CLI と専用バイナリの削除漏れ |
| 2 | `codex-cloud-tasks` | 1 / 1 | Codex Cloud タスク UI/CLI | 削除済み | 中 | `codex cloud` の help、dispatch、補完への残存 |
| 3 | `codex-cloud-tasks-mock-client` | 1 / 2 | Cloud Tasks 専用 mock | 削除済み | 低 | 通常依存に置かれており単独では削除不能 |
| 4 | `codex-cloud-tasks-client` | 2 / 3 | Codex Cloud Tasks API 専用 | 削除済み | 中 | mock と CLI の同時処理が必要 |
| 5 | `codex-chatgpt` | 2 / 5 | ChatGPT backend 専用 | 削除済み | 中 | apply と connectors を別の削除単位として処理 |
| 6 | `codex-cloud-config` | 5 / 7 | ChatGPT 配信設定と組織要件 | 隔離 | 高 | 起動時設定ロードと enterprise 制約に影響 |
| 7 | `codex-backend-client` | 6 / 13 | ChatGPT/Codex backend API | 削除済み | 高 | account/usage、workspace、hosted analytics、turn-cost の同時撤去が必要 |
| 8 | `codex-backend-openapi-models` | 1 / 14 | Codex backend の生成モデル | 削除済み | 中 | backend client の削除後に同時撤去 |
| 9 | `codex-image-generation-extension` | 3 / 49 | OpenAI Images API と固定モデル専用 | 削除済み | 高 | core の tool/event/history と app-server に波及 |
| 10 | `codex-web-search-extension` | 2 / 49 | OpenAI web search と互換 provider が混在 | 汎用化 | 高 | hosted/standalone の二経路を分ける必要がある |
| 11 | `codex-core-plugins` | 7 / 49 | 汎用 plugin 管理と OpenAI marketplace が混在 | 汎用化 | 高 | local plugin と third-party marketplace を維持する必要がある |
| 12 | `codex-analytics` | 10 / 49 | 汎用収集と OpenAI backend 送信が混在 | 汎用化 | 高 | 多数の拡張機能と core に波及 |
| 13 | `codex-feedback` | 7 / 54 | 汎用診断収集と OpenAI/Sentry 送信が混在 | 汎用化 | 高 | ログ layer、app-server API、TUI に波及 |
| 14 | `codex-response-debug-context` | 2 / 54 | 汎用デバッグ情報と OpenAI header が混在 | 汎用化 | 中 | model error の診断情報欠落 |
| 15 | `codex-login` | 26 / 56 | ChatGPT OAuth と汎用 credential 管理が混在 | 汎用化 | 非常に高 | ほぼ全エントリーポイントの認証に波及 |
| 16 | `codex-agent-identity` | 3 / 57 | ChatGPT issuer/JWKS と agent identity が一体 | 汎用化 | 高 | login、model provider、cloud config の認証境界 |
| 17 | `codex-model-provider-info` | 16 / 60 | 汎用 provider 設定に OpenAI 既定値が混在 | 汎用化 | 非常に高 | 非 OpenAI provider も同じ型を利用 |
| 18 | `codex-api` | 16 / 71 | Responses/WebSocket wire API と汎用 transport 境界 | 汎用化 | 非常に高 | core を含む最大の推移影響範囲 |
| 19 | Realtime voice capability（`codex-realtime-webrtc`、`codex-voice-host` を含む） | なし（削除済み） | OpenAI Realtime API のみが現行の起動経路 | 削除済み | 高 | API、protocol、履歴、core、app-server、TUI、専用補助クレートを一単位で削除した |

### 新基準による再評価

| 対象 | 現行の非 OpenAI capability | 判断 |
| --- | --- | --- |
| `codex-responses-api-proxy`、Cloud Tasks 3 クレート、`codex-chatgpt` | なし | 削除済み。専用到達経路を含めて削除する判断を維持する。 |
| `codex-cloud-config`、`codex-git-attribution`、remote memory rate-limit guard | なし | 削除済み。hosted policy、backend attribution、backend rate-limit guard を残さない。 |
| `codex-backend-client`、`codex-backend-openapi-models` | なし | workspace message、`/accounts/check`、account usage/rate-limit 読み取り、リセット credit、workspace owner nudge、hosted analytics dashboard、backend turn-cost collection を capability 単位で削除し、両クレートも削除済み。 |
| ImageGen | image artifact、tool invocation、履歴表現 | OpenAI Images API、固定モデル、OpenAI auth の adapter を削除し、共通表現だけを残す。 |
| Web Search | standalone search を実行できる互換 provider の経路 | OpenAI hosted search を削除する。共通 tool と standalone executor は残す。 |
| Realtime voice | なし（削除済み） | Realtime API adapter に限らず、`codex-realtime-webrtc`、`codex-voice-host`、voice UI、Realtime 履歴を含む capability 全体を削除した。通常のテキスト会話、汎用 audio utility、provider-neutral な media 処理は保持する。 |
| `codex-core-plugins` | local / third-party plugin の load と管理 | OpenAI curated marketplace と ChatGPT remote service だけを削除する。 |
| `codex-analytics`、`codex-feedback` | local event/reducer、diagnostic bundle | OpenAI/ChatGPT backend sink とそれに専用の metadata を削除する。 |
| `codex-response-debug-context` | 標準 HTTP/API の診断情報 | OpenAI 固有 header・URL の extractor を削除する。 |
| `codex-login`、`codex-agent-identity` | credential storage、鍵・署名 primitive、他 provider の auth | ChatGPT/OpenAI login、issuer/JWKS policy、専用 token flow を隔離後に削除する。 |
| `codex-model-provider-info`、`codex-api` | provider registry、互換 provider transport、共通 HTTP client | OpenAI built-in 定義と OpenAI wire adapter を隔離後に削除する。 |

## 削除候補

### 1. `codex-responses-api-proxy`

- **根拠:** `/v1/responses` だけを `https://api.openai.com` へ転送し、`OPENAI_API_KEY` を Bearer token として注入する専用 proxy である。
- **機能:** loopback HTTP proxy、API key の保護、request/response dump、proxy 用バイナリ。
- **内部依存先:** `codex-process-hardening`。
- **依存元:** 直接・推移ともに `codex-cli` のみ。
- **到達経路:** `codex-cli` の隠し `responses-api-proxy` subcommand から `run_main` を呼ぶ。TUI と app-server からの直接経路はない。
- **影響:** workspace member/dependency、CLI enum・dispatch・分類、クレート内テスト、README、専用バイナリ。専用 feature flag や設定項目はない。
- **推奨:** **削除**。汎用化するなら別の provider-neutral proxy として新設すべきで、現実装を共有層に残す理由はない。
- **難易度・リスク:** 低。主なリスクは隠し command とバイナリ alias の削除漏れである。

### 2–4. Cloud Tasks 3 クレート

対象は `codex-cloud-tasks`、`codex-cloud-tasks-client`、`codex-cloud-tasks-mock-client` である。

- **根拠:** ChatGPT 認証を要求し、Codex Cloud の environment、task、diff を ChatGPT backend の `/wham` API で操作する。
- **機能:** `codex cloud` の TUI/CLI、task submit/list/status/diff/apply、backend client、テスト用 mock client。
- **内部依存先:** `codex-cloud-tasks-client` は `codex-api`、`codex-backend-client`、`codex-git-utils`、`codex-http-client`、mock は client のみに依存する。`codex-cloud-tasks` は client、mock、`codex-core`、`codex-git-utils`、`codex-http-client`、`codex-login`、`codex-model-provider`、`codex-tui`、`codex-utils-cli` に依存する。
- **依存元:** UI クレートは `codex-cli` のみ。client は UI と mock、mock は UI のみ。3 クレート全体の外部到達点は `codex-cli` に閉じる。
- **到達経路:** `codex-cli` の公開 `cloud` / `cloud-tasks` subcommand と dispatch。既存 TUI クレートを表示部品として使うが、通常の対話 TUI から Cloud Tasks へ入る経路ではない。
- **影響:** CLI help・補完・dispatch、ChatGPT login/base URL、task URL、環境選択 UI、3 クレートのテスト。専用 Cargo feature flag はない。
- **推奨:** **3 クレートを一つの削除単位として削除**。mock client は `codex-cloud-tasks` の通常依存に置かれているため、現状のまま単独削除しない。
- **難易度・リスク:** 中。CLI 表面を Level 2 まで維持する方針と競合するため、Phase 2 で実施する場合は、Level 1 のクレート削除に伴う最小限の CLI 到達経路削除として明記する。

### 5. `codex-chatgpt`

**進捗:** 削除済み。`codex apply` と ChatGPT backend の connector directory / metadata fetch を削除した。app-server の provider-neutral な app protocol、plugin 宣言、MCP tool から得られる connector 情報は共有クレート側に残した。

- **根拠:** README が first-party ChatGPT APIs/products 専用と明記し、実装も ChatGPT backend auth、cookie、`/wham` API を前提とする。
- **機能:** hosted task の diff を取得する `apply`、ChatGPT connector directory、ChatGPT 用 HTTP client。
- **内部依存先:** `codex-connectors`、`codex-core`、`codex-git-utils`、`codex-http-client`、`codex-login`、`codex-model-provider`、`codex-plugin`、`codex-utils-cargo-bin`、`codex-utils-cli`。
- **依存元:** 直接は `codex-app-server` と `codex-cli`。推移は app-server client、Cloud Tasks、exec、TUI を加えた 6 クレート。
- **到達経路:** `codex apply`、app-server の connector 一覧・metadata 更新。connector の結果は TUI/app client から間接利用される。
- **影響:** `apply` command、app-server connector API、`apps` feature/config、ChatGPT auth/cookie、クレート内テストと README。
- **推奨:** **分割後に削除**。`apply` と connectors は独立した到達経路なので、一括削除ではなく二つの実施単位に分ける。
- **難易度・リスク:** 中。汎用の git patch 適用処理と connector model は残し、ChatGPT backend fetch だけを削除する必要がある。

## 隔離候補

### 6. `codex-cloud-config`

- **根拠:** ChatGPT backend から配布される remote config bundle と組織要件を取得・検証する。
- **機能:** bundle の取得、署名・有効期限検証、storage、設定レイヤーへの組み込み。
- **内部依存先:** `codex-agent-identity`、`codex-backend-client`、`codex-config`、`codex-core`、`codex-http-client`、`codex-login`、`codex-otel`、`codex-protocol`。
- **依存元:** 直接は app-server、CLI、exec、TUI、Windows sandbox service。推移は 7 クレート。
- **到達経路:** 各エントリーポイントの起動時 config load と app-server の config manager。
- **影響:** `chatgpt_base_url`、managed requirements、remote config storage、再ログイン判定、起動テスト。専用 README と Cargo feature flag はない。
- **推奨:** **隔離**。ローカル設定の読み込みと混ぜず、optional な hosted policy source として切り離した後に削除する。
- **難易度・リスク:** 高。無効化時にローカル設定や非 OpenAI provider の起動を壊さない検証が必要である。

### 7. `codex-backend-client`

- **根拠:** ChatGPT/Codex backend の `/wham` API、workspace message、usage、credit、profile などを実装する。
- **機能:** backend URL/path の構築、認証付き request、rate limit/credit/workspace API。
- **内部依存先:** `codex-api`、`codex-backend-openapi-models`、`codex-http-client`、`codex-login`、`codex-model-provider`、`codex-protocol`。
- **依存元:** 直接は app-server、cloud config、Cloud Tasks client、git attribution、memories write、TUI。推移は 13 クレート。
- **到達経路:** app-server の account/usage/workspace 系 request、TUI の backend banner/status、Cloud Tasks、remote memory と git attribution。
- **影響:** `chatgpt_base_url`、ChatGPT auth、usage/credit UI、workspace message、wiremock テスト。専用 README、Cargo feature flag はない。
- **推奨:** **隔離**。OpenAI backend integration 層として境界を明示し、利用機能ごとに依存を外してから削除する。
- **難易度・リスク:** 高。複数の製品機能を一つの client が担い、一括削除すると Phase の変更範囲を超える。

### 8. `codex-backend-openapi-models`

- **根拠:** Codex backend の OpenAPI 生成型だけを収録する。
- **機能:** rate limit、workspace message、usage、credit など backend response の serde model。
- **内部依存先:** なし。外部の serde 系だけに依存する。
- **依存元:** 直接は `codex-backend-client` のみ、推移は 14 クレート。
- **到達経路:** backend client を介して app-server、TUI、Cloud Tasks、cloud config、memories、git attribution へ到達する。
- **影響:** 生成モデルと、それを公開型として再 export する backend client の API。feature flag、独立設定、専用 README、crate 単体 test はない。
- **推奨:** **隔離**。直接依存数だけを見て先に削除せず、backend client と同じ境界に置く。backend 機能の縮小後に未使用モデルを削る。
- **難易度・リスク:** 中。生成元との同期と公開型互換性が主なリスクである。

## 汎用化・責務分割候補

### 9. `codex-image-generation-extension`

**進捗:** adapter は削除済み。専用 crate、Images API endpoint/types、extension 登録、image-generation feature/capability の live advertisement、専用テストを削除した。保存済み `ImageGenerationItem`、Responses item、rollout migration、app-server/TUI の履歴表示は、pre-release fork の全般的な互換性クリーンアップ計画 `docs/superpowers/plans/2026-09-19-remove-pre-release-compatibility-layers.md` の一部として削除する。

- **根拠:** OpenAI auth を利用可能条件とし、`gpt-image-2` と Images API の request/response に固定されている。
- **機能:** `image_gen` tool、画像生成・編集 request、data URL と artifact の変換。
- **内部依存先:** `codex-api`、`codex-core`、`codex-exec-server`、`codex-extension-api`、`codex-extension-items`、`codex-login`、`codex-model-provider`、`codex-model-provider-info`、`codex-protocol`、`codex-tools`、absolute-path/image/path-uri utility。
- **依存元:** 直接は app-server、core、core-api、推移は 49 クレート。
- **到達経路:** core/app-server の extension install、`features.image_generation`、model tool call、TUI/app-server の履歴表示。
- **影響:** image generation feature と capability、固定モデル名、Images API 型、tool/event/history、extension tests。専用 README はない。
- **推奨:** **削除済み（互換性層は全般 cleanup の一部として削除）**。OpenAI Images backend、固定モデル、live tool 到達経路に続き、pre-release では不要な旧 tool/event/history/protocol/migration/UI 到達経路も削除する。汎用画像入力・表示処理は残す。
- **難易度・リスク:** 高。クレート削除だけでは protocol と UI に image item が残るため、到達不能コードの扱いを Level 3 と調整する必要がある。

互換性層の削除対象は、専用 adapter の削除とは別に
`docs/pre-release-compatibility-inventory.md` の image-generation history and
protocol slice として扱う。generic image input、local media、`view_image` は
削除対象に含めない。

### 10. `codex-web-search-extension`

**進捗:** OpenAI provider と actor-authorized hosted request を自動的に
`web.run` standalone executor へ流す経路を削除済み。`supports_standalone_web_search`
を明示的に設定した custom provider の executor、共通の Search API 型、hosted
`web_search` tool 定義は保持する。

- **根拠:** OpenAI auth/actor authorization の hosted search を実装する一方、`supports_standalone_web_search` を持つ互換 provider も利用できる。
- **機能:** search/open/find の tool、検索 event の history/output 変換、provider API 呼び出し。
- **内部依存先:** `codex-api`、`codex-core`、`codex-extension-api`、`codex-extension-items`、`codex-login`、`codex-model-provider`、`codex-model-provider-info`、`codex-otel`、`codex-protocol`、`codex-tools`。
- **依存元:** 直接は app-server と core、推移は 49 クレート。
- **到達経路:** core/app-server の extension install、web search feature/model metadata、通常の model tool call。
- **影響:** hosted/standalone web search の選択、OpenAI provider 判定、search event/tool schema、extension tests。専用 README はない。
- **推奨:** **汎用化（OpenAI hosted 経路は削除済み）**。クレート自体は削除せず、OpenAI hosted search adapter と provider capability に基づく standalone executor を分離する。
- **難易度・リスク:** 高。OpenAI 経路だけを除き、互換 provider の standalone search を維持する必要がある。

### 11. `codex-core-plugins`

- **根拠:** local/third-party plugin 管理に加え、`openai-curated` 系 marketplace、ChatGPT backend catalog、remote install/uninstall を直接扱う。
- **機能:** plugin discovery/load、marketplace policy、remote catalog cache/mutation、skills/tools の組み込み。
- **内部依存先:** `codex-analytics`、`codex-app-server-protocol`、`codex-config`、`codex-connectors`、`codex-exec-server` と test support、`codex-git-utils`、`codex-hooks`、`codex-http-client`、`codex-login`、`codex-mcp`、`codex-model-provider`、`codex-otel`、`codex-plugin`、`codex-protocol`、`codex-shell-command`、`codex-skills`、`codex-tools`、absolute-path/path/path-uri/plugins utility。
- **依存元:** 直接 7、推移 49。app-server、CLI、core、TUI と connector/MCP extension が利用する。
- **到達経路:** 起動時 plugin load、CLI plugin command、app-server plugin API、TUI と extension registration。
- **影響:** OpenAI marketplace 名、startup sync URL、`chatgpt_base_url`、remote plugin API、marketplace/cache/policy tests。汎用 plugin 文書と設定は残す必要がある。
- **推奨:** **汎用化**。local plugin loader と一般 marketplace interface を残し、OpenAI curated policy と ChatGPT remote service を別 integration に移す。
- **難易度・リスク:** 高。クレート全体を削除すると provider-neutral な plugin foundation まで失う。

### 12. `codex-analytics`

- **根拠:** event/reducer 自体は汎用だが、ChatGPT backend の analytics endpoint、ChatGPT account、Codex product metadata を扱う。
- **機能:** thread/turn/tool event の集約、永続化、backend 送信。
- **内部依存先:** `codex-app-server-protocol`、`codex-git-utils`、`codex-login`、`codex-model-provider`、`codex-plugin`、`codex-protocol`、`codex-state`、absolute-path utility。
- **依存元:** 直接 10、推移 49。core、app-server、複数 extension が利用する。
- **到達経路:** core と app-server から全 UI へ間接到達する。独立した CLI command はない。
- **影響:** analytics 設定、login/model provider、state、plugin、protocol event、広範な reducer/client テスト。専用 README はない。
- **推奨:** **汎用化**。event model/reducer と OpenAI backend sink を分割し、sink だけを削除可能にする。
- **難易度・リスク:** 高。収集停止とローカル event 処理削除を混同すると、汎用 observability まで失う。

### 13. `codex-feedback`

- **根拠:** ログ・rollout・診断情報の収集は汎用だが、OpenAI の auth metadata と Sentry/backend upload が一体化している。
- **機能:** tracing layer、snapshot、attachment 圧縮、feedback upload、retry。
- **内部依存先:** `codex-http-client`、`codex-login`、`codex-protocol`、`codex-rollout`。
- **依存元:** 直接 7、推移 54。app-server、client、core、exec、guardian、model provider、TUI が利用する。
- **到達経路:** app-server の `feedback/upload`、TUI/exec の feedback UI・logging 初期化。
- **影響:** `feedback.enabled`、Sentry、ChatGPT account metadata、doctor report、attachment、関連 API/テスト。専用 README はない。
- **推奨:** **汎用化**。診断 bundle 作成と upload transport を分割し、OpenAI sink を optional integration として隔離する。
- **難易度・リスク:** 高。tracing subscriber の組み立てに関与するため、単純な dependency 削除では起動経路が壊れる。

### 14. `codex-response-debug-context`

- **根拠:** response failure の情報抽出は汎用だが、現在は `x-openai-authorization-error` と ChatGPT/Codex URL の診断を直接扱う。
- **機能:** API error から request ID、URL、status、authorization error を安全に抽出する。
- **内部依存先:** `codex-api` のみ。
- **依存元:** 直接は core と model provider、推移は 54 クレート。
- **到達経路:** model provider の API error を介して core、CLI/TUI/app-server の error reporting へ到達する。
- **影響:** OpenAI 固有 header、error debug context、model failure tests。feature flag、設定、専用 README はない。
- **推奨:** **汎用化**。標準的な HTTP/API debug fields と provider 固有 header extractor を分離する。
- **難易度・リスク:** 中。削除より分割が必要だが、状態や認証フロー自体は変更しない。

### 15. `codex-login`

- **根拠:** ChatGPT OAuth/device code、OpenAI API key、Codex backend token を実装する一方、credential storage、keyring、認証状態管理は汎用責務である。
- **機能:** login server/device flow、token refresh、API key、keyring/storage、AuthManager。
- **内部依存先:** `codex-agent-identity`、`codex-config`、`codex-http-client`、`codex-keyring-store`、`codex-model-provider-info`、`codex-otel`、`codex-protocol`、`codex-secrets`、`codex-terminal-detection`、`codex-utils-template`、`codex-workload-identity`。
- **依存元:** 直接 26、推移 56。CLI/TUI/app-server/core と大半の hosted integration が直接利用する。
- **到達経路:** `codex login status` と `codex logout`、TUI onboarding、app-server account API、model request auth。CLI からの ChatGPT OAuth、device auth、OpenAI API key、access token の各 login 経路は削除済み。
- **影響:** `OPENAI_API_KEY`、forced login method、ChatGPT auth modes、credential files/keyring、非常に多数の fixture とテスト、認証文書。
- **推奨:** **汎用化**。provider-neutral credential interface、API-key provider、ChatGPT login provider に責務分割する。即時削除はしない。
- **難易度・リスク:** 非常に高。認証、保存形式、後方互換性、非 OpenAI provider の利用を同時に扱う必要がある。

### 16. `codex-agent-identity`

- **根拠:** agent key/JWT の一般的な処理を含む一方、issuer、JWKS、environment 判定が `chatgpt.com` と Codex backend に固定されている。
- **機能:** agent identity key、JWT/JWKS 検証、署名、environment/URL 解決。
- **内部依存先:** `codex-http-client`、`codex-protocol`。
- **依存元:** 直接は cloud config、login、model provider、推移は 57。
- **到達経路:** login と model provider を介して CLI/TUI/app-server/core へ到達する。
- **影響:** ChatGPT environment、backend URL、identity cache/credential、認証テスト。feature flag、専用 README はない。
- **推奨:** **汎用化**。鍵管理・署名 primitive と ChatGPT issuer policy を分離する。
- **難易度・リスク:** 高。認証境界の変更であり、fail-open を起こさない設計とテストが必要である。

### 17. `codex-model-provider-info`

- **根拠:** provider の型と設定解決は汎用だが、OpenAI/ChatGPT の既定 URL、header、環境変数、wire API が組み込まれている。
- **機能:** provider metadata、base URL、headers、auth env、retry/stream settings、built-in provider。
- **内部依存先:** `codex-api`、`codex-protocol`、absolute-path/redacted-string utility。
- **依存元:** 直接 16、推移 60。OpenAI 以外の LM Studio/Ollama も利用する。
- **到達経路:** config と model provider を介して全 model request と各 UI に到達する。
- **影響:** `model_provider`、`model_providers.*`、`OPENAI_API_KEY`、OpenAI organization/project headers、Responses API transport、設定/単体テスト。専用 README はない。
- **推奨:** **汎用化**。provider registry と OpenAI built-in definition を分離する。クレート自体は削除対象にしない。
- **難易度・リスク:** 非常に高。OpenAI 固有値だけを外し、他 provider の設定互換性を保つ必要がある。

### 18. `codex-api`

- **根拠:** README が Codex/OpenAI API client と明記し、Responses、Realtime、memory summarize などの wire contract を実装する。一方、任意 base URL の provider や互換 server からも利用される。
- **機能:** typed request/response、SSE/WebSocket、retry、API error、provider/auth header injection。
- **内部依存先:** `codex-client`、`codex-http-client`、`codex-protocol`、rustls-provider utility、`codex-websocket-client`。
- **依存元:** 直接 16、推移 71 で候補中最大。core、model provider、backend client、MCP、各 extension が利用する。
- **到達経路:** core の model call を介して CLI/TUI/app-server/exec の主要実行経路すべてに到達する。
- **影響:** Responses API wire type、stream event、memory API、model/provider tests、crate README。専用 Cargo feature flag はない。Realtime URL/call adapter は削除済みである。
- **推奨:** **汎用化**。汎用 transport/client は `codex-client` / `codex-http-client` に維持し、OpenAI wire protocol を provider adapter として隔離する。互換 provider が使う protocol を「OpenAI 固有」という理由だけで削除しない。
- **難易度・リスク:** 非常に高。core protocol と多数の extension が wire type を直接共有している。

### 19. Realtime voice capability

- **根拠:** 現行の realtime conversation は OpenAI/ChatGPT の Realtime WebSocket と `realtime/calls` の WebRTC 契約、`gpt-realtime-*` モデル、専用 session event を前提とする。`codex-realtime-webrtc` と `codex-voice-host` の実装は WebRTC や音声 device を扱うが、実行時の利用元はこの TUI voice capability だけである。
- **対象:** `codex-api` の Realtime call / WebSocket adapter、core の realtime conversation・context・prompt・history、app-server protocol と request processor、TUI の voice UI・音声 session・設定、`codex-realtime-webrtc`、`codex-voice-host`、専用設定・prompt・テスト・snapshot。
- **非対象:** 通常のテキスト会話、thread history、一般の audio utility は、Realtime voice 専用の到達経路でない限り残す。
- **推奨:** **削除済み**。将来の provider のためだけに WebRTC / audio helper は維持しない。将来 provider-neutral な voice capability を導入する場合は、その provider の契約と共に新設する。
- **実績:** Realtime voice の public protocol、設定、UI、worker、保存済み履歴の reader/writer/projection を除去した。既存の Realtime rollout ファイル自体は削除せず、旧形式は現行 reader ではサポートしない。

## 実施単位の履歴と Phase 3 の後続候補

Phase 2 で完了した削除と、Phase 3 で扱う後続候補を次の順序で記録する。一単位ごとに依存グラフを再生成する。

1. **`codex-responses-api-proxy` の削除（完了）**
   - 依存元が `codex-cli` だけで、代替や責務移動を必要としない。
2. **Cloud Tasks 3 クレートの削除（完了）**
   - `codex-cloud-tasks`、client、mock をまとめ、`codex cloud` の専用到達経路だけを外す。
3. **`codex-chatgpt` の削除（完了）**
   - `codex apply` と ChatGPT backend connector 経路を分けて処理し、共有の plugin/MCP connector 表現を残してクレート本体を削除した。
4. **backend integration の隔離**
   - backend models/client、cloud config の利用機能を列挙して個別に外す。Level 1 Phase 3 で一括削除せずに進める。
5. **Realtime voice capability の削除（完了）**
   - OpenAI Realtime API に依存する会話・WebRTC・音声 host・UI、protocol、履歴、専用設定を一つの実施単位で削除した。`codex-realtime-webrtc` と `codex-voice-host` も削除済みである。
6. **Web Search hosted adapter の削除（完了）**
   - OpenAI provider と actor-authorized hosted request の自動 `web.run` 経路を削除し、明示的に opt-in した custom provider の standalone search と共通の hosted tool 定義を残した。
7. **共有クレートの provider 境界設計**
   - image generation、web search、plugins、analytics、feedback、response debug、login、agent identity、model-provider-info、codex-api は後続の汎用化作業として扱う。

Phase 2 の削除対象 1〜3 は完了した。4 以降は影響範囲または汎用責務が大きいため、Level 1 Phase 3 で capability ごとに到達経路を縮小してから扱う。

## 調査したがクレート削除候補から除外したもの

- `codex-http-client` と `codex-client`: ChatGPT cookie pool や request policy の一部に固有処理はあるが、共有 HTTP transport が主責務である。固有 module を adapter 側へ移す対象であり、クレート削除候補ではない。
- `codex-protocol`、`codex-core`、`codex-tui`、`codex-app-server`: 多数の OpenAI/ChatGPT 到達経路を含むが、エージェント基盤と主要 entrypoint そのものである。Level 1 のクレート削除ではなく、上記 integration の依存を外した後に Level 2/3 で到達不能コードを調べる。

## Phase 2 の検証観点

- `just crate-dependency-graph` の再生成結果から対象クレートと依存辺が消えること。
- `codex --help` と shell completion から公開 Cloud command が消え、隠し proxy command が dispatch 不能になること。
- `codex-cli` の build/test と workspace check が通ること。
- 通常の local exec、TUI 起動、非 OpenAI provider の設定解決に回帰がないこと。
- 削除しない候補の設定、テスト、文書を「名前が似ている」という理由だけで巻き込まないこと。

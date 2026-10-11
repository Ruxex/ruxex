# 設定上書き監査

最終更新: 2026-10-12

この文書は、`config.toml`、プロファイル、プロジェクト設定、CLI の
`-c/--config` 上書きについて、ユーザーが変更できる必要があるかを監査するための
作業記録である。監査は項目単位で継続し、判定が終わっていない項目は削除・変更しない。

## 判定

- `保持`: ユーザーが変更できる必要がある。現時点では変更しない。
- `管理設定へ移動`: ユーザー設定ではなく、ホスト・管理者・`requirements.toml` だけで決める候補。
- `ランタイム専用`: ファイルからは変更させず、ホストまたは実行時 API だけで決める。
- `削除対象`: 監査でユーザーが不要と判断し、設定から削除する項目。
- `削除候補`: 未リリース方針により、互換性のためだけの設定を削除する候補。ただしユーザー判断前は未監査とする。
- `未監査`: producer/consumer の確認と判定がまだ完了していない。

## 設定レイヤー

低い順の概念的な優先順位は次のとおり。後の層が前の層を上書きする。

```text
パッケージ既定値
  -> system / enterprise 設定
  -> ユーザー config.toml
  -> 選択中の profile
  -> プロジェクト .genex/config.toml
  -> CLI / セッション上書き
```

MDM 等の管理設定は別途、値の許可範囲・強制値として適用される。信頼されていない
プロジェクトの設定は無効化される。`requirements.toml` は通常のユーザー設定ではなく、
管理者が認証、権限、ネットワーク、機能などを制約するための層である。

## 監査対象一覧

### 実行・安全性

今回の方針: **保持**。このカテゴリはユーザーによる変更を残す。

| 設定 | 役割 | 判定 |
|---|---|---|
| `approval_policy` | コマンド実行時の承認方針 | 保持 |
| `sandbox_mode` | read-only / workspace-write / danger-full-access | 保持 |
| `default_permissions` | 既定の権限プロファイル | 保持 |
| `[permissions.<name>]` | ファイル・ネットワーク・workspace root の権限 | 保持 |
| `[sandbox_workspace_write]` | 書き込みルート、ネットワーク、`/tmp` の扱い | 保持 |
| `allow_login_shell` | ログインシェルの許可 | 保持 |
| `[shell_environment_policy]` | 子プロセスへ渡す環境変数 | 保持 |
| `allow_symlinked_genex_home` | macOS の symlink 経由ホーム領域の扱い | 保持 |
| `[projects.<path>]` | プロジェクトの trust level | 保持 |
| `notify` | ターン完了時に起動する外部コマンド | 保持 |

### 指示・コンテキスト

| 設定 | 役割 | 判定 |
|---|---|---|
| `instructions` | base/system 相当の指示を置換 | 保持 |
| `developer_instructions` | 全ターンに追加する developer 指示 | 削除対象 |
| `model_instructions_file` | モデル組み込み指示をファイルで置換 | 保持 |
| `compact_prompt` | 履歴圧縮時のプロンプト | 保持 |
| `include_permissions_instructions` | 権限説明の注入 | 保持 |
| `include_collaboration_mode_instructions` | コラボレーション説明の注入 | 保持 |
| `include_environment_context` | 環境情報の注入 | 保持 |
| `project_doc_max_bytes` | `AGENTS.md` 等の最大読込量 | 保持 |
| `project_doc_fallback_filenames` | 代替プロジェクト指示ファイル名 | 削除対象 |
| `project_root_markers` | プロジェクトルート判定マーカー | 削除対象 |
| `tool_output_token_limit` | ツール出力保存量のトークン上限 | 削除対象 |

### モデル・リクエスト

| 設定 | 役割 | 判定 |
|---|---|---|
| `model_verbosity` | Responses API の出力詳細度 | 保持 |
| `service_tier` | `default` / `priority` / `flex` 等 | 保持 |
| `web_search` | Web 検索モード | 保持 |
| `[tools.web_search]` | 検索ドメイン、地域、コンテキストサイズ | 保持 |
| `hide_agent_reasoning` | 推論イベントを隠す | 保持 |
| `show_raw_agent_reasoning` | 生の推論イベントを表示 | 保持 |

### 認証・プロバイダー境界

| 設定 | 役割 | 判定 |
|---|---|---|
| `cli_auth_credentials_store` | CLI 資格情報の file/keyring/auto | 保持 |
| `forced_login_method` | 許可するログイン方式の強制 | 保持 |
| `forced_managed_workspace_id` | managed auth の workspace 制限 | 保持 |
| `managed_auth_base_url` | host-managed auth の接続先 | 保持 |
| `responses_api_metadata` | 全 Responses API リクエストの製品メタデータ | 保持 |

### MCP・外部連携

| 設定 | 役割 | 判定 |
|---|---|---|
| `[mcp_servers.<name>]` | MCP の command/URL、環境変数、OAuth、timeout、tool allow/deny | 保持 |
| `mcp_enterprise_managed_auth` | MCP 用の企業 IdP | 保持 |
| `mcp_oauth_credentials_store` | MCP OAuth 資格情報の保存先 | 保持 |
| `mcp_oauth_callback_port` | OAuth callback の固定ポート | 保持 |
| `mcp_oauth_callback_url` | OAuth redirect URL | 保持 |
| `mcp_optional_startup_grace_ms` | MCP 起動待ち時間 | 保持 |
| `browser_use` | ブラウザ操作・origin 制限 | 保持 |
| `computer_use` | デスクトップ操作・macOS bundle 制限 | 保持 |
| `[plugins.<name>]` | プラグインの有効化と MCP policy overlay | 保持 |
| `[marketplaces.<name>]` | marketplace の source と取得状態 | 保持 |
| `skills` | スキルの有効化、指示注入、コンテキスト上限 | 保持 |
| `hooks` | lifecycle hook の matcher、command、trust state | 保持 |

### エージェント・機能

| 設定 | 役割 | 判定 |
|---|---|---|
| `agents.enabled` | multi-agent の有効化 | 保持 |
| `agents.max_concurrent_threads_per_session` | agent 同時実行数 | 保持 |
| `agents.max_depth` | V1 agent のネスト深度 | 保持 |
| `agents.interrupt_message` | 中断時メッセージを残すか | 保持 |
| `[agents.<role>]` | agent role の説明・設定ファイル・nickname | 保持 |
| `goals.max_goal_token_budget` | goal のトークン上限 | 保持 |
| `[features]` | 機能フラグと機能固有設定 | 保持 |
| `[tools.experimental_request_user_input]` | `request_user_input` の有効化 | 保持 |
| `[tools.update_plan]` | `update_plan` の有効化 | 保持 |
| `orchestrator` | orchestrator 所有の skills/MCP toggle | 保持 |

`[features]` の既知キーは、stable/実験機能、MCP、network、画像、multi-agent、
code mode、TUI、token/context/rollout budget、hooks、plugins などを含む。特に
`sqlite`、`remote_models`、`tool_search`、`item_ids`、`steer`、`collaboration_modes`、
`terminal_resize_reflow` などは Removed/Deprecated または互換性維持用であり、別途
削除監査を行う。

#### Feature 設定の読み方と解決順

`[features]` には、単純な boolean と、`enabled` に加えて詳細項目を持つ構造化設定の
2 種類がある。

```toml
[features]
multi_agent = true
code_mode = false

[features.code_mode]
enabled = true
default_exec_yield_time_ms = 10000
excluded_tool_namespaces = ["mcp"]
```

実効値は概念的に次の順で決まる。

```text
組み込み既定値
  -> config.toml / profile の [features]
  -> runtime の feature override
  -> requirements.toml / MDM の強制要件
  -> 依存関係の正規化
```

`requirements.toml` の feature 要件はユーザー設定より強く、指定された feature を固定
する。代表的な依存関係として、`code_mode_only = true` は `code_mode` も有効にし、
管理要件で明示されない限り `unified_exec` は有効化される。

主な構造化 Feature は次のとおり。今回 V2 と判定した項目以外は保持する。

| 設定 | 役割 | 判定 |
|---|---|---|
| `[features.code_mode]` | JavaScript code mode の有効化、実行待ち時間、公開する tool namespace | 保持 |
| `[features.code_mode_host]` | standalone code-mode host と in-process fallback の扱い | 保持 |
| `[features.non_prefixed_mcp_tool_names]` | MCP の旧 `mcp__` prefix を省略する server 指定 | 保持（MCP・外部連携方針） |
| `[features.multi_agent_v2]` | V2 agent backend とその詳細設定 | 削除対象 |
| `[features.token_budget]` | 履歴・context の token budget、compact 前の reminder、fallback prompt | 保持 |
| `[features.context_management]` | 実験的 context management | 保持 |
| `[features.rollout_budget]` | session/rollout の weighted token budget | 保持 |
| `[features.current_time_reminder]` | モデル可視の時刻 reminder と `clock.sleep` | 保持 |
| `[features.sleep_tool]` | sleep tool の model-driven / always-on 選択 | 保持（実行・安全性方針） |
| `[features.network_proxy]` | proxy、domain、Unix socket、credential broker の詳細 | 保持 |
| `[features.tool_registry]` | tool collision の扱いと request metadata | 保持 |

`Stage::Deprecated` はまだ動作を持つ廃止予定項目、`Stage::Removed` はレジストリ上は
廃止済みの項目を意味する。ただし本リポジトリでは `Stage::Removed` でも実装が残る項目が
あるため、producer/consumer を確認してから削除する。

#### Removed かつ互換専用の Feature（削除対象）

次は registry のコメントまたは `Features::apply_map` の処理から、現在の機能を切り替えず
古い設定を受理するためだけに残っていると確認できるもの。今回の方針により、設定キーと
設定からの上書き経路を削除対象とする。

| 設定キー | 現在の挙動 | 判定 |
|---|---|---|
| `undo` | GhostCommit は no-op | 削除対象 |
| `terminal_resize_reflow` | terminal resize reflow は常時有効 | 削除対象 |
| `apply_patch_preserve_line_endings` | patch は常に既存 line ending を保持 | 削除対象 |
| `use_linux_sandbox_bwrap` | 旧 Linux bubblewrap opt-in | 削除対象 |
| `remote_models` | legacy remote models flag | 削除対象 |
| `tool_search` | tool search は常時有効 | 削除対象 |
| `tool_search_always_defer_mcp_tools` | MCP tool の defer は常時適用 | 削除対象 |
| `resize_all_images` | centralized image preparation は常時適用 | 削除対象 |
| `item_ids` | response item IDs は常時有効 | 削除対象 |
| `steer` | Enter の steer 動作は常時有効 | 削除対象 |
| `send_async_message` | model-enabled async user messaging の旧 flag | 削除対象 |
| `collaboration_modes` | collaboration modes は常時有効 | 削除対象 |
| `remote_compaction_v2` | config 値は切り替えに使わず、beta header は常時広告 | 削除対象（設定キーのみ） |

`remote_compaction_v2` は provider request の beta header という別の producer があるため、
設定キーを削除しても、その外部契約に関する runtime 挙動は別途監査する。

#### Removed 表示だが保持するもの

`sqlite` は registry の stage が Removed でも SQLite state store の実行経路で参照されているため、
今回の判断では保持する。SQLite state store自体の削除は別途監査対象とする。

`shell_zsh_fork` と `unified_exec_zsh_fork` は registry の stage が Removed であり、今回の判断に
より設定キーだけでなく zsh fork backendそのものを削除対象とする。実装削除の前に、feature
registry、config schema、session/runtime、shell escalation wrapper、配布用zsh、app-serverと
approval/sandboxのテストおよび外部protocolのproducer/consumerを確認する。

`web_search_request`、`web_search_cached`、`use_legacy_landlock` は Deprecated であり、まだ
動作または明示的な legacy fallback を持つため、保持する。Removed と同じ扱いにはしない。

#### zsh fork系の意味（削除対象）

ここでいう「zsh fork」は、単に通常のzshをforkして起動する設定ではない。Genex用に
`EXEC_WRAPPER`対応パッチを当てたzshと、`genex-execve-wrapper`を組み合わせるUnix向けの
shell escalation backendである。

通常のshell実行では、Genexはzshに`-c`または`-lc`でスクリプトを渡す。zsh fork backendを
使うと、zshが内部で外部コマンドを`execve`する際に、環境変数で指定されたwrapperを経由させる。
wrapperは共有file descriptor上のGenex escalation serverへ、対象ファイル・引数・環境・作業
ディレクトリを送り、serverの判定を受ける。

判定結果は概ね次の3つである。

- `Run`: sandbox内で元のコマンドを実行
- `Escalate`: 必要なfile descriptorを引き渡し、sandbox外の実行へ切り替え
- `Deny`: 実行を拒否してエラーを返す

このため、shell全体だけでなく、shell scriptやその内部プロセスが起動する外部コマンド単位で
policy・approval・sandbox escalationを扱える。zshのbuiltinだけで完結する処理は`execve`を
通らないため、この仕組みの対象ではない。

設定の関係は次のとおり。

| 設定 | 役割 | 判定 |
|---|---|---|
| `shell_zsh_fork` | shell toolをzsh fork backendへ向ける主ゲート。既定値はfalse。 | 削除対象 |
| `unified_exec_zsh_fork` | unified execとzsh bridgeを組み合わせるcomposition gate。単独では他の機能を有効化しない。既定値はtrue。 | 削除対象 |

実際にzsh forkになるには、Unix、`shell_tool`、`unified_exec`、両方のzsh fork flag、ユーザー
shellがzshであること、パッチ済みzshとwrapper executableが利用可能であることが必要である。
remote environmentではDirect実行へ切り替わる。現在も実行コード、approval処理、sandbox処理、
app-server統合テストが残っているが、これらは削除時に同時に整理する。

#### Code Mode の意味（保持）

Code Mode は、モデルが毎回個別の tool call を直接発行する代わりに、JavaScript の
`exec` cell を一つ実行し、その JavaScript から複数の Genex tool を呼び出して結果を
組み合わせる tool 実行面である。単なるローカル shell 実行ではない。

```text
モデル
  -> exec(JavaScript)
       -> tools.exec_command(...)
       -> tools.mcp__...(...)
       -> Promise.all([...])
  <- cell の結果
```

実行環境は fresh V8 isolate で、Code Mode の JavaScript 自体には Node.js、直接の
filesystem、直接の network、console はない。必要な外部操作は `tools.*` に公開された
既存 tool を通して行い、その tool の sandbox、approval、MCP policy を受ける。
同じ Code Mode session の cell 間では保存した JavaScript 値を共有でき、長い処理は
`yield` して `wait` で再開できる。

Code Mode の代表的な目的は、複数の独立した tool 呼び出しを一回の JavaScript にまとめる、
並列化する、tool 結果を中間データとして次の呼び出しへ渡す、という orchestration である。
それにより、モデルが大量の個別 tool call と中間結果を直接コンテキストへ積む負担を減らす。

設定上は次の3つが関係する。

| 設定 | 役割 |
|---|---|
| `features.code_mode` | Code Mode の有効化。実行 cell の既定 yield 時間、nested tool から除外する namespace、直接公開だけにする namespace を設定する。 |
| `features.code_mode_host` | standalone `genex-code-mode-host` の利用と、host unavailable 時に in-process 実行へ fallback するかを設定する。 |
| `features.code_mode_only` | 通常の直接 tool を隠し、原則として `exec` / `wait` 経由にする。`direct_only_tool_namespaces` の例外は残せる。 |

モデルの metadata が `code_mode` / `code_mode_only` を指定している場合はそれが優先され、
指定がないモデルでは Feature 設定が既定の tool mode になる。Code Mode が使えない場合、
通常の `code_mode` は in-process fallback が許可されていれば direct tool に戻せるが、
`code_mode_only` は fail closed になる。

2026-10-12 JST に確認した本家 `openai/codex` の先頭コミット
`27065a91ccbf5ba8532844be3777fcf83b77820d` では、Feature の既定値は
`code_mode = false`、`code_mode_only = false`、`code_mode_host = true` である。一方、
同梱モデルカタログは 11 モデル中 10 モデルが `tool_mode = "code_mode_only"`、
`gpt-5.5` だけが `tool_mode = null` である。したがって本家の実際の既定挙動は、
設定だけ見れば Direct だが、選択するモデルによっては Code Mode Only になる。
これは全モデルが必ず Code Mode を実行できるという意味ではなく、モデル metadata が
Code Mode 向けの tool surface を選択しているという意味である。

Code Mode は独立した実行経路・V8 host・gRPC/stdio protocol・tool exposure policy を持つため、
単なる `exec` の別名ではない。設定を削除するかどうかは、通常の実行安全性と外部 tool の
組み合わせを含めて別途監査する。

#### Anthropic系・Llama系モデルの扱い（外部互換性は未検証）

Code Mode の可否は、モデル名に `claude`、`llama` などが含まれるかでは判定されない。
現在の選択経路は、モデル metadata の `ModelInfo.tool_mode` を最優先し、未指定の場合に
Feature 設定を fallback として使う。したがって、Anthropic系・Llama系でも provider adapter
またはモデルカタログが `code_mode` / `code_mode_only` を明示すれば、その mode を選択できる。
逆に未指定なら、Feature 設定が無効な通常時は Direct になる。

ここでいう「選択できる」は、API上で `exec` tool を渡せることと同義ではない。モデルが
JavaScript の `exec` 呼び出しを正しく生成し、実行結果を読んで次の cell を組み立てられるか、
さらに provider adapter が tool schema・stream・エラー形式を正しく変換できるかは、モデルと
adapter ごとの互換性である。Code Mode 専用の独立した `supports_code_mode` capability は、
今回確認した選択経路には存在しない。

2026-10-12 JST に確認した本家の同梱モデルカタログには、Anthropic/Claude または一般的な
Llama モデルの登録はない。そのため、本家についてそれらのモデルの既定値を「対応」または
「非対応」と断定できる状態ではなく、外部 provider を追加する場合はモデルカタログ、tool
呼び出し形式、streaming、認証・エラー変換を個別に監査する必要がある。この項目は設定削除の
決定ではなく、外部モデル対応の未検証事項として残す。

#### 外部調査：Anthropic系・Llama系のCode Mode相当機能（2026-10-12 JST）

ここでいうCode Mode相当を「モデルがコードまたは構造化tool callを生成し、ホストが実行し、
結果をモデルへ戻す反復実行」と定義して、公式資料を確認した。

| 系統 | 確認できた機能 | 判定 |
|---|---|---|
| Anthropic / Claude API | `tool_use` によるclient/server tools、sandbox内のPython/Bash実行、REPL状態保持、sandbox内コードから自前toolを呼ぶ `programmatic tool calling` がある。 | Code Mode相当あり |
| Anthropic / Claude Code | コードベースの読取・編集、コマンド実行、MCP連携を行うagentic coding tool。GenexのJavaScript `exec` と同一実装ではないが、製品レベルの相当機能がある。 | 相当機能あり |
| Meta / Llama 3.1 | tool callの構造化出力と、`Environment: ipython` によるPython code interpreter形式を公式prompt formatで定義。ただしモデル自身はtoolを実行せず、外部executorが必要。 | モデル機能として部分的にあり |
| Meta / Llama 3.3 | function/tool callingとbuilt-in code interpreter形式を公式prompt formatで定義。 | モデル機能として部分的にあり |
| Llama Stack等 | Llamaモデルの推論、tool実行、安全性、agent loopを組み合わせる外部のagent基盤。 | ホスト／基盤側で相当機能を構成可能 |

したがって、Code Modeはモデルのブランドに付随する単一のboolean capabilityではない。少なくとも
「tool calling」「モデル自身によるコード生成」「コード実行環境」「コードから別toolを呼ぶ機能」
「結果を戻して反復するagent loop」を分けて扱う必要がある。Claudeは公式APIにこの組み合わせを
比較的直接的に提供しているが、Llamaはモデル・prompt template・推論サーバー・executorの組合せに
依存する。Llama系の全モデルが同じ対応を持つわけではない。

主な一次資料：Anthropicの[Claude Code overview](https://code.claude.com/docs/en/overview)、
[Code execution tool](https://platform.claude.com/docs/en/agents-and-tools/tool-use/code-execution-tool)、
Metaの[Llama 3.1 tool-use prompt format](https://github.com/meta-llama/llama-models/blob/main/models/llama3_1/prompt_format.md)、
[Llama 3.3 tool-calling prompt format](https://github.com/meta-llama/llama-models/blob/main/models/llama3_3/prompt_format.md)。

#### GenexのCode ModeでClaudeを使う場合（未検証）

AnthropicのClaudeをGenexのCode Modeで使える可能性は高いが、現時点で相互運用が保証されている
とは判断しない。GenexのCode Modeは、Genex側が定義したJavaScript `exec` toolをモデルに提示し、
モデルがその引数としてコードを生成し、Genex側のV8 hostが実行する方式である。Anthropicの
`programmatic tool calling` はAnthropicのcode execution sandbox内からtoolを呼ぶ別機能なので、
Claude APIにその機能があることだけではGenex互換性の証明にならない。

必要条件は次の二つに分かれる。

1. **provider adapter側**：Anthropic Messages APIの`tool_use` / `tool_result`、tool schema、role、
   streaming、stop reason、エラーをGenexのprovider-neutral形式へ正しく変換すること。
2. **モデル側**：ClaudeがGenexの`exec` toolを選び、要求されたJavaScript形式を生成し、cell結果を
   読んで次のtool呼び出しを組み立てられること。これはモデルの学習・instruction tuning・promptに
   依存する。

つまり、Code Modeは「provider機能とは無関係で、モデルの学習だけの問題」ではない。実行hostと
   agent loopはGenex側、tool呼び出しプロトコルの接続はprovider adapter側、toolを適切に使う傾向は
   モデル側の責任である。同じClaudeモデルでも、Anthropic API、Bedrock、Vertex、互換OpenAI gateway
   などの経路が変われば、adapterの変換差で結果が変わり得る。

現時点の判定は「技術的には有望、実用上はClaudeモデル・Anthropic adapter・Genex prompt/tool schemaの
統合テストが必要」。外部モデル対応の採否はこの実測後に行う。

#### エージェント系の実装上の整理

`[agents]` と `[features]` の multi-agent 設定は、同じ「agent 機能」に見えても、
旧 V1 の制御と V2 のバックエンド選択・詳細設定が混在している。今回ここに記載するのは
意味と producer/consumer の整理である。V2 専用設定は今回の指定により削除対象、
V1/V2 共通設定は保持として記録する。

| 設定 | 実際の役割 | 判定 |
|---|---|---|
| `agents.enabled` | multi-agent を無効化すると、V1/V2 の agent tool を無効化する。`features.multi_agent_v2.enabled = true` があれば V2 が優先される。 | 保持 |
| `features.multi_agent` | 旧 V1 の collaboration backend。現在の feature 定義では stable、既定値は有効。 | 保持 |
| `features.multi_agent_v2.enabled` | V2 backend を選択する。既定値は無効で、`agents.enabled` より優先される。 | 削除対象 |
| `agents.max_concurrent_threads_per_session` | 主に V1 の同時 agent 数。V2 で専用値がない場合の互換フォールバックにも使われ、その際は内部で `+1` される。 | 保持 |
| `features.multi_agent_v2.max_concurrent_threads_per_session` | V2 のセッション全体の同時実行上限。現在の実装では root 分を含む上限として扱い、spawn 枠の計算時に 1 を引く。既定値は 4。 | 削除対象 |
| `agents.max_depth` | V1 の agent の入れ子深度。V2 では無視される。既定値は 1。 | 保持 |
| `agents.default_subagent_reasoning_effort` | spawn 時に指定がない場合の子 agent の reasoning effort。モデルそのものの既定値ではない。 | 保持 |
| `agents.interrupt_message` | agent の中断をモデル可視メッセージとして記録するか。既定値は有効。 | 保持 |
| `[agents.<role>]` | spawn 時に使う role の説明、role 専用 config file、nickname 候補。`config_file` は子 agent 用の追加設定レイヤーになる。 | 保持 |
| `features.multi_agent_v2.*_wait_timeout_ms` | V2 の `wait_agent` の最小・既定・最大待ち時間。既定は 10 秒 / 30 秒 / 1 時間。 | 削除対象 |
| `features.multi_agent_v2.wait_agent_enabled` | V2 の `wait_agent` tool を公開するか。 | 削除対象 |
| `features.multi_agent_v2.expose_spawn_agent_model_overrides` | spawn tool に子 agent の `model` と `reasoning_effort` 指定を公開するか。 | 削除対象 |
| `features.multi_agent_v2.tool_namespace` | V2 の agent tools に付ける namespace。 | 削除対象 |
| `features.multi_agent_v2.hide_spawn_agent_metadata` | spawn tool の model/reasoning などのメタデータを隠すか。 | 削除対象 |
| `features.multi_agent_v2.non_code_mode_only` | V2 の agent tools を non-code mode に限定するか。 | 削除対象 |
| `features.multi_agent_v2.*_usage_hint_text` | root agent / subagent に注入する利用案内テキスト。 | 削除対象 |
| `features.multi_agent_v2.subagent_developer_instructions` | 子 agent に継承する developer 指示を置き換える。トップレベルの `developer_instructions` とは別の設定項目だが、プロンプトへ影響する。 | 削除対象 |
| `features.multi_agent_v2.usage_hint_enabled` | 旧 V2 利用案内制御。現在は deprecated で値を無視する。 | 削除対象 |

現時点の組み込み既定値では、`features.multi_agent` は有効、
`features.multi_agent_v2` は無効、`agents.enabled` も有効であるため、選択される backend は
V1 である。V2 は設定または host 側の feature override で明示的に有効化された場合に選択される。

#### 本家 `openai/codex` との確認

2026-10-12 JST に確認した本家 `main` の先頭コミットは
`72f999edc7848961b99c4b98bf1e62d12fa35798`（2026-10-11 UTC）であり、
`multi_agent` は `default_enabled = true`、`multi_agent_v2` は
`default_enabled = false` のままである。したがって、少なくともこの時点では本家も
V1 を標準、V2 を opt-in としている。

コード上、V1 は単なる読み込み互換層ではなく、既定 backend として現在も tool 契約と
実行経路を持つ。一方 V2 は task-path routing、独自のセッション同時実行上限、異なる
agent tool 群を使う。さらに本家では V2 専用の動的 tool、subagent context limit、message
board などが引き続き個別の under-development feature として存在する。これは「V2 の
方が柔軟でも、挙動・プロトコル・運用負荷の変更を避けるため、標準化を保留している」
という推測を支持するが、本家の明示的な理由説明までは確認できていない。

今回の監査指定により、V2 専用の `[features.multi_agent_v2]` 設定は削除対象とする。
一方、`agents.enabled`、`agents.max_concurrent_threads_per_session`、
`agents.default_subagent_reasoning_effort`、`agents.interrupt_message`、`[agents.<role>]` は
V1/V2 の両方または agent 共通で使われるため、今回の指定だけでは削除対象に変更しない。

特に `agents.default_subagent_model` は型には存在するが `serde(skip)` であり、設定ファイルからは
変更できない runtime 専用値である。また、`features.multi_agent_v2.usage_hint_enabled` は
deprecated compatibility field で、読み込まれても値は無視される。

### TUI・保存・観測

| 設定 | 役割 | 判定 |
|---|---|---|
| `[tui]` | theme、通知、keymap、status line、terminal title、Vim、resume 等 | 保持 |
| `history` | history.jsonl の保存と最大サイズ | 保持 |
| `sqlite_home` | SQLite 保存先 | 保持 |
| `log_dir` | ログ保存先 | 保持 |
| `thread_unload_delay_secs` | app-server の thread unload 待ち時間 | 保持 |
| `background_terminal_max_timeout` | `write_stdin` のバックグラウンド出力待ち時間 | 保持 |
| `file_opener` | ファイル引用リンクの URI 形式 | 保持 |
| `profile` / `[profiles.<name>]` | 設定プロファイル | 保持 |
| `desktop` | デスクトップ側の不透明な設定領域 | 保持 |
| `notice` | 警告・移行案内の既読状態 | 保持 |
| `metrics.enabled` | メトリクスの有効化 | 保持 |
| `otel` | OTEL exporter、trace、prompt 記録 | 保持 |
| `experimental_thread_store` | 実験的 thread 保存方式 | 保持 |
| `suppress_unstable_features_warning` | 実験機能警告を抑制 | 保持 |

## ファイルから既に変更できない項目

`ConfigToml` の `serde(skip)` により、次は `config.toml` からは読み込まれない。

`model`、`model_provider`、`model_context_window`、`model_auto_compact_token_limit`、
`model_auto_compact_token_limit_scope`、`model_reasoning_effort`、
`plan_mode_reasoning_effort`、`model_reasoning_summary`、`model_catalog_json`、
`agents.default_subagent_model`。

これらは runtime、host、または `requirements.toml` 側の責務である。

## `developer_instructions` の挙動

`developer_instructions` は、システム指示文字列を置換する設定ではない。設定から
セッションへ引き継がれ、各ターンの入力に `role = "developer"` の別メッセージとして
追加される。実装上は `generic.developer_instructions` という content kind で扱われる。

一方、`instructions`、モデルカタログの instruction template、または
`model_instructions_file` から得た base instructions は、provider request の
`instructions` フィールドに渡される。Responses API ではこのフィールドが
system/base 相当の指示であり、Chat Completions では adapter の `instructions` として
渡される。したがって概念的には次の関係になる。

```text
base/system instructions
  + developer_instructions  (追加の developer メッセージ)
  + Genex が生成する権限・ツール・環境・機能の developer sections
  + user messages
```

`developer_instructions` は user message より上位の developer 文脈としてモデルに届くが、
Genex の組み込み base instructions を消すものではない。明示的な runtime/per-turn の
developer 指示がある場合は、設定ファイルの値よりそちらが優先される。

実装上の確認箇所:

- 設定ファイル値の統合: [`Config::load_config_with_layer_stack`](../crates/core/src/config/mod.rs#L3475)
- ターン先頭の developer message 化: [`build_initial_context_with_world_state`](../crates/core/src/session/mod.rs#L3750)
- Responses/Chat provider request の `instructions` と input の組み立て: [`client.rs`](../crates/core/src/client.rs#L603)

監査上は、ユーザーが自由に指定すると組み込み指示との衝突や安全指示の弱体化を起こし得る
ため、`instructions`、`developer_instructions`、`model_instructions_file`、および
`include_*_instructions` を同じ「プロンプト上書き」群として判断する。

## 次回の再開位置

次は「指示・コンテキスト」カテゴリから、各項目の producer/consumer、プロジェクト層で
の許可要否、`-c` での上書き経路を確認する。判断が済んだ項目の `判定` と理由を更新し、
その後「認証・プロバイダー境界」へ進む。

## 今回の監査対応完了 (2026-10-12)

監査判断に基づく削除を実装した。削除対象の設定・registry・zsh fork backend と互換テストを
除去し、Code Mode、実行安全系、保持指定された prompt/context、model/response、MCP/外部連携、
runtime の multi-agent 実装は維持した。`just generate`、`just check`、`just check --tests`、
`git diff --check` はすべて成功した。

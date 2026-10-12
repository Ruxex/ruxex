---
sidebar_position: 2
title: 用語集
---

# 用語集

| 用語 | このリポジトリでの意味 |
| --- | --- |
| Process | CLIやApp ServerのOSプロセス。RuntimeとDB接続を保持する。 |
| App Server | クライアントからのProtocol Requestを受け、ThreadとTurnを管理するサーバー。 |
| Runtime | 1つのApp Serverが共有するProvider、Credential Manager、設定、DB、各種サービスのまとまり。 |
| Provider | モデルへのChat、Streaming、認証、Model Catalog、Usageを提供する実装境界。 |
| Client | TUI、Desktop、別プロセスなど、App Server Protocolを利用する側。 |
| Thread | 継続する会話・作業単位。履歴、設定スナップショット、Rolloutへの参照を持つ。 |
| Turn | ユーザー入力から1つの最終的な応答までの、ユーザーから見た1回の処理。 |
| Step | 1回のモデルSampling Requestに使う、固定された設定・環境・Toolのスナップショット。 |
| Sampling | Providerへ履歴とコンテキストを送り、ストリーム応答を受け取る処理。 |
| Response Item | Assistant message、Reasoning、Function Callなど、会話履歴を構成する項目。 |
| Tool Call | モデルがToolを実行するために生成する要求。結果は次のSamplingの入力になる。 |
| MCP | 外部MCP Serverが提供するToolやResourceをGenexに接続する仕組み。 |
| Plugin | MCP、Skill、追加Capabilityをまとめて提供する構成単位。 |
| Skill | モデルに注入する指示と、必要に応じた依存Toolを定義する構成単位。 |
| StepContext | 1回のSamplingに必要なモデル設定、Environment、MCP、Tool Router、AGENTS.mdなどを束ねた不変スナップショット。 |
| World State | Environmentやファイル差分など、モデルが現在の作業世界を理解するための状態。 |
| Rollout | Threadのイベント・Response Itemを追記するJSONL記録。再開・再構築の基礎になる。 |
| Projection | Rolloutから抽出したThread一覧・検索・ページング用のSQLite状態。 |
| State DB | ThreadメタデータやProjectなど、一覧・関係・状態を保存するSQLite。 |
| Provider Home | Provider単位でSQLite・Rollout・Credentialなどを隔離するデータルート。 |
| Credential Snapshot | Turn開始時点の認証情報をProvider内部だけで扱う、不透明なスナップショット。 |
| Turn Input | ユーザー入力、Tool結果、他Agent通信など、Turn中に履歴へ取り込む入力。 |

## TurnとStepの違い

「チャットを1回行う」をコードに対応づけるとき、TurnとStepを混同しないことが重要です。

- Turnは、ユーザー入力を受けて最終応答またはエラーまで進む大きなライフサイクルです。
- Stepは、実際にモデルへ1回リクエストし、そのストリームとTool処理を完了する単位です。
- モデルがToolを要求すると、同じTurnの中でTool結果を追加して次のStepへ進みます。
- したがって、1 Turn = 1 Requestとは限りません。

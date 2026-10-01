# 共通の認証フロー

## 層

認証には 3 つの異なる層があります。

1. `codex login` / `logout` は `genex-login::AuthManager` を通じて、ローカルの資格情報状態を変更または参照します。
2. `genex-login` は保存済み認証と更新/復旧の動作を表します。
3. `genex-api::AuthProvider` は API クライアントにリクエスト時の認証ヘッダーを提供します。

CLI コマンドの境界は [`main.rs`](../../../../crates/cli/src/main.rs#L1392) にあります。保存済み認証の型とマネージャーの動作は [`login/src/auth/manager.rs`](../../../../crates/login/src/auth/manager.rs#L48) にあります。リクエスト側の Trait は [`AuthProvider`](../reference/trait-index.md#authprovider) です。

## 選択

`AuthManager` は `CODEX_HOME`、環境変数の資格情報、または設定された認証モードを使ってローカル状態を読み込みます。`model-provider` は選択された認証を、具体的なモデルプロバイダー向けの共有認証プロバイダーに変換します。したがって認証の選択はプロバイダー/モデルとトランスポートの構築より上流にありますが、エージェントのスレッド/ターンのライフサイクルからは独立しています。

## セキュリティ境界

資格情報の保存とトークン更新は、汎用的なプロバイダー非依存エージェント機能ではなく、統合境界です。Core は抽象化されたリクエスト/認証インターフェースを利用し、CLI のプロンプトや資格情報ファイルの詳細に依存してはいけません。

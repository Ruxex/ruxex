# 認証コマンド

## エントリ

`codex login` と `codex logout` は [`LoginCommand` と `LogoutCommand`](../../../../crates/cli/src/main.rs#L452) で解析されます。`login status` と logout は [main.rs#L1392](../../../../crates/cli/src/main.rs#L1392) で `codex-login` のヘルパーにディスパッチされます。

## 動作

- `codex login status` は現在選択されているローカル認証状態を報告します。
- `codex logout` は保存済みの認証資格情報を削除します。
- これらのコマンドはローカルの資格情報状態を変更または参照するため、リモートモードを拒否します。
- 選択した認証モードは後続のプロバイダー/クライアント構築に影響しますが、login コマンド自体はエージェントのターンを実行しません。

認証マネージャー、資格情報の形式、キーチェーン、環境の選択については[共通の認証](../common/authentication.md)に記載しています。プロバイダーの選択は、[プロバイダーとモデル](../common/provider-and-model.md)で説明する別の段階です。

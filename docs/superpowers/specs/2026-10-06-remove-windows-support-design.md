# Windows サポート削除 設計仕様

作成日: 2026-10-06

## 目的

本リポジトリのサポート対象を Linux と macOS に限定する。Windows 用の実装、ビルド対象、配布経路、プロトコル、設定、テスト、CI、ドキュメントを producer/consumer 監査後に削除し、Windows 利用者には WSL を推奨する。

## 成功条件

- workspace の production crate、manifest、protocol/schema、app-server request、CLI/TUI、sandbox、PTY、UDS、HTTP/proxy、keyring、sleep inhibitor に Windows 専用の実装または `cfg(windows)` 分岐が残らない。
- Windows 専用 crate、直接依存、feature、生成物、CI job、配布・セットアップスクリプトが workspace の supported path から除去される。
- Linux/macOS の既存の sandbox、filesystem、network、keyring、terminal、PTY 経路は維持される。
- README とインストール文書に「Windows は未サポートであり、WSL を推奨する」ことを記載する。
- `docs/provider-decoupling-candidates.md` に producer/consumer 監査結果を追記し、`docs/pre-release-compatibility-inventory.md` に各対象の `remove`、`retain`、または `migrate-in-place` を明記する。
- `just generate`、`just check --tests`、`just check`、metadata 検証、Windows 専用参照監査、`git diff --check` が成功する。

## 対象範囲

### 削除するもの

1. Windows sandbox とそのサービス、MXC/AppContainer、Windows helper、Windows 専用 package identity、named pipe、WFP/firewall、ACL、Restricted Token、Job Object、Private Desktop などの Windows 実装。
2. Windows sandbox に依存する app-server protocol/schema、request processor、config/feature、core/sandboxing の状態・prompt・telemetry。
3. Windows 専用の CLI doctor、TUI prompt/console/screen reader、PTY/terminal probe/IDE pipe、UDS peer validation、HTTP TLS/proxy、sleep inhibitor、keyring、path helper、exec-server filesystem 実装。
4. Windows 専用の Cargo target dependency、workspace member、lockfile の直接参照、CI matrix、配布・セットアップスクリプト、Windows 専用テストと fixture。
5. Windows 専用の PowerShell parser/feature。一般的なコードブロック例や第三者の公式用語は、Windows 実装を意味しない限り個別監査のうえで保持できる。

### 保持するもの

- Linux/macOS の実装と、それらのセキュリティ境界、fallback、provider-neutral capability。
- Linux/macOS でも使われる共通コード。Windows 分岐だけを取り除き、必要なら Unix または OS 非依存の実装へ簡素化する。
- 依存先が内部的に Windows target を持つだけで、supported path から直接利用していない upstream/third-party の lockfile エントリ。
- 歴史的・法的記録、外部プロトコルや外部 repository の識別子。ただし current production path の Windows support を表すものは保持しない。

## アーキテクチャと実装方針

サポート対象を実行時の OS 判定で切り替えるのではなく、workspace のコンパイル境界から Windows を除去する。Windows 専用 crate と機能を先に削除し、その後に共有 crate の条件分岐と依存を Linux/macOS の実装へ収束させる。これにより、Windows 向けの unsandboxed fallback や未検証の stub を残さず、Windows target 自体を supported build から外す。

変更は次の順序で行う。

1. 現在の Windows producer/consumer、生成物、CI、配布経路を台帳化する。
2. 専用 crate と Windows protocol/config/app-server surface を削除する。
3. core、sandboxing、CLI、TUI の共有状態と分岐を整理する。
4. 低レベル共通 crate の Windows 実装と直接依存を削除する。
5. build/CI/package/docs を Linux/macOS + WSL guidance に更新する。
6. 生成物を再生成し、production source と metadata を再監査する。

## エラー処理と利用者向け動作

Windows 用 binary を提供しない。Windows 上で無理に実行して unsandboxed に移行する経路や、Windows でだけ成功する互換 stub は追加しない。利用者向け文書・インストール案内では、Windows はサポート対象外であり WSL 上で Linux 版を使用するよう案内する。

## 監査ルール

各 production match について、定義側 producer、利用側 consumer、生成物・保存形式・外部境界の有無を確認してから変更する。判断は次のいずれかを台帳に記録する。

- `remove`: Windows 専用であり、Linux/macOS の consumer がない。
- `migrate-in-place`: 共有実装の Windows 分岐だけを削除し、Linux/macOS の既存 API・動作を維持する。
- `retain`: upstream/third-party、歴史・法的記録、または現行の非Windows外部契約であり、削除すると別の契約を壊す。

単なる単語検索の結果だけで削除を決めない。特に `windows` を含む一般的な fixture、公式 protocol terminology、transitive dependency は production reachability と照合する。

## 検証

- 生成物に影響する変更後は `just generate` を実行し、差分を確認する。
- Rust の追加・削除・移動・リファクタリング後は `just check --tests` を実行する。
- 常に `just check` を実行する。
- `cargo metadata --no-deps` で workspace member と直接依存を確認する。
- `rg` で `cfg(windows)`、`target_os = "windows"`、Windows 専用 crate/module/feature/protocol 名の残存を監査し、許容した retain を台帳と照合する。
- `git diff --check` を実行する。standalone test は実行しない。

## 非目標

- Linux/macOS に存在する一般的な `legacy` / `fallback` 実装や provider-neutral capability の削除。
- upstream/third-party crate 自体の Windows 対応を改変すること。
- WSL 専用の新機能を追加すること。WSL は Linux 版を利用するための推奨環境として文書化するだけにする。

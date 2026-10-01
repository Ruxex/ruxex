# Remove the Windows sandbox service design

作成日: 2026-10-01

## 概要

通常のCodex CLIが使用するWindows sandboxは維持し、Windows sandbox serviceと、それを必要とするRegistered Core実行形態だけを削除する。通常CLIは、ローカルのセットアップヘルパー、command runner、ACL、Restricted Token、Job Object、Private Desktop、既存のネットワーク制御を使って動作し続ける。

## 目的

- `genex-windows-sandbox-service`のビルド・配布・実行・IPC経路を削除する。
- Registered CoreのパッケージID検証、登録済みrunner alias、サービス所有のsandboxアカウント／runtime receipt、refresh処理を削除する。
- app-server、core、CLI、Windows sandbox libraryの分岐を通常sandbox経路へ簡素化する。
- 通常のWindows sandboxのセキュリティ境界と実行能力を変更しない。

## 非目的

- `genex-windows-sandbox-rs`そのものを削除すること。
- `genex-windows-sandbox-setup.exe`や`codex-command-runner.exe`を削除すること。
- Windows ACL、Restricted Token、Job Object、Private Desktop、deny-read、通常のネットワーク制御を削除すること。
- LinuxのbwrapやmacOS sandboxの実装を変更すること。

## 用語と前提

Registered Coreは通常CLIではなく、WindowsのMSIX/AppX package identityを持つデスクトップアプリ内のCore runtimeである。`CODEX_WINDOWS_REGISTERED_CORE=1`で要求され、staged package内のrunner image、サービスの登録receipt、サービス管理下のsandbox accountを使う。今回はこの実行形態自体を廃止し、通常CLIの非パッケージ／helper経路だけを残す。

## 設計

### 1. Workspaceと配布

- workspace member、依存関係、`genex-windows-sandbox-service` crate、service binaryを削除する。
- Windows CLI packageの必須ファイルは既存どおり`codex-command-runner.exe`と`genex-windows-sandbox-setup.exe`に限定する。
- installer、manifest、package lifecycle、service登録・アンインストール処理にサービスbinaryを要求する箇所があれば削除する。

### 2. Windows sandbox library

- Named pipe provisioning protocol/clientとservice用の設定・refresh APIを削除する。
- `CODEX_WINDOWS_REGISTERED_CORE`、AppX package identity検証、staged package path検証、Registered Core alias、runtime ownership registry receiptを削除する。
- 通常の`run_elevated_setup`、`run_legacy_setup_preflight`、ACL refresh、command runner起動、stdio転送、deny-read解決は保持する。
- 通常経路ではサービス不在を判定する必要がなくなるため、helper provisioningを唯一の昇格セットアップ経路にする。

### 3. app-serverとcore

- Windows sandbox readinessのRegistered Core refreshを削除する。
- setup request processorのservice feature／Registered Core分岐を削除し、通常の`run_windows_sandbox_setup`だけを呼び出す。
- `WindowsSandboxService` feature、関連config/schema、feature gateを削除する。
- Registered Core専用の起動時フラグ読み取りやログ・telemetryを削除する。

### 4. CLI実行

- CLIのWindows sandbox session生成は現行の`spawn_windows_sandbox_session_for_level`経路を維持する。
- setup helperが必要な場合は、既存の昇格helperを使う。
- package identityを前提にしたrunner選択やサービス所有アカウント選択を削除し、通常runner選択に統一する。

### 5. Documentationとinventory

- service、Registered Core、named pipe、registry receipt、service featureに関するドキュメントと不要なcompatibility inventory行を更新する。
- 通常Windows sandboxのセキュリティ構成と、サービス削除後も残る制約を`docs/sandbox.md`等に反映する。

## エラー処理

- 通常の昇格セットアップが失敗した場合は、従来どおりsetup helperのエラーを返す。
- Registered Core専用の「service unavailable」「service-compatible policy」エラーは削除する。
- serviceなしを理由に通常CLIをunsandboxedへフォールバックさせない。sandbox setup失敗時は明示的に失敗させる。

## 検証

- `rg -n -i 'windows.?sandbox.?service|registered.?core|CODEX_WINDOWS_REGISTERED_CORE|OpenAI\.CodexSandbox' crates docs Cargo.toml`で、通常sandboxに不要な本番参照が残っていないことを確認する。
- `rg -n 'genex-windows-sandbox-service|genex-windows-sandbox-setup|codex-command-runner' .`で、setup／runnerの参照だけが残ることを確認する。
- `just generate`を実行し、生成物の差分を確認する。
- `just check --tests`を実行する。
- `just check`を実行する。
- Windows固有コードについては、少なくともworkspace metadataと条件付き参照を監査し、サービス専用のfeature、crate、protocolが残っていないことを確認する。

## 受け入れ条件

1. `genex-windows-sandbox-service` crateとサービスbinaryがworkspace／配布対象から消えている。
2. Registered Coreの要求・検証・refresh・service provisioningがproduction codeから消えている。
3. 通常CLIのWindows sandbox setupとcommand execution経路が残っている。
4. サービスなしを理由に通常sandboxが無効化されたり、unsandboxed実行へフォールバックしたりしない。
5. 必須の生成・型チェックが成功する。

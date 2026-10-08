# 更新機能削除 設計仕様

## 目的

Genex の更新機能を削除し、CLI、daemon、TUI、doctor、更新用の外部 HTTP
アクセスおよび package-manager 起動経路を残さない。利用者は Genex を
インストールした外部の配布経路で更新する。

## 対象範囲

### 削除する機能

- `genex app-server daemon update` と `--from-cli` 更新経路
- daemon の detached updater process、手動更新 socket、定期更新 loop、
  installer script の取得・実行、legacy installation migration 専用更新経路
- TUI 起動時の latest-version HTTP probe、更新キャッシュ、更新 popup、
  更新可能通知 history cell、更新後に package manager を起動する処理
- TUI の daemon menu にある daemon 更新・確認・pin 操作
- `doctor` の `updates.status` check と desktop update CDN の診断
- 更新機能専用の telemetry、CLI/TUI の update state、更新専用テスト、
  更新専用ドキュメント

### 維持する機能

- `genex app-server daemon bootstrap`
- `genex app-server daemon start|restart|stop|version`
- 初回 daemon package の準備に必要な package 検証・コピー処理
- daemon の operation lock、pid backend、graceful shutdown、recovery
- remote app-server の server-version warning。これは更新を実行せず、接続先との
  互換性を知らせるだけのため維持する。
- `update_versions` に含まれる remote server version 比較など、更新実行や
  更新通知に使われない consumer。不要な関数だけを分離・削除する。

## 実装方針

1. app-server-daemon から公開更新 API、更新結果型、更新 loop、manual update、
   migration 専用コードを削除する。
2. `prepare_install` は初回 `Missing` package 準備に縮小し、`Replace`、pin、
   release selection、updater 復旧など更新専用分岐を削除する。
3. daemon 起動時・再起動時・bootstrap 時に updater を起動または停止する処理と、
   update 設定の読み書き・telemetry を削除する。daemon の通常設定と shutdown grace
   は維持する。
4. CLI の daemon `Update` / hidden `PidUpdateLoop` subcommand、dispatch、関連 telemetry、
   update tests を削除する。
5. TUI の update action/prompt/cache/update notice と、それに伴う `AppExitInfo`、
   `AppEvent`、pending state、startup handoff を削除する。daemon menu は更新項目を
   提示しないため、更新専用なら module ごと削除する。
6. doctor の update check と desktop update CDN probe を削除し、doctor の他の診断は
   変更しない。
7. 更新専用の設定キー、README、plan/inventory の記録を更新する。既存の
   `version.json` や updater pid/socket は削除 migration を追加せず、以後読まない。

## 動作上の完了条件

- `genex app-server daemon update` が CLI help、dispatch、completion、内部起動経路から
  消える。
- daemon 起動が installer URL、update socket、update pid loop、auto-update 設定を
  読み込まない。
- TUI が起動時に外部 release/package registry へ更新確認を行わず、更新 popup/history
  を表示しない。
- `doctor` が update status や desktop update CDN の到達性を報告しない。
- bootstrap/start/restart/stop/version と app-server 接続、既存 package 検証が維持される。
- 更新専用の producer/consumer がなくなり、更新専用ファイル・依存・テストが残らない。

## 検証

- `rg` による update command、updater socket/process、外部 release/package-manager
  更新経路の残存監査
- `just generate` と生成差分の確認
- `CARGO_BUILD_JOBS=2 systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check --tests`
- `CARGO_BUILD_JOBS=2 systemd-run --user --scope --collect --quiet -p MemoryHigh=16G -p MemoryMax=18G -p MemorySwapMax=4G --setenv=CARGO_BUILD_JOBS=2 just check`
- `git diff --check`

# `codex debug`

## エントリポイント

[`DebugSubcommand`](../../../../crates/cli/src/main.rs#L210) には次があります。

- `models [--bundled]`: モデルカタログを JSON として出力します。
- `app-server send-message-v2`: 診断用 app-server メッセージを送信します。
- `prompt-input [PROMPT] --image FILE`: モデルから見えるプロンプト入力を JSON として出力します。
- 非表示の `trace-reduce TRACE_BUNDLE`: ロールアウトトレースを再生し、縮約した状態 JSON を書き込みます。

ディスパッチは [main.rs#L1498](../../../../crates/cli/src/main.rs#L1498) から始まります。これらのコマンドはリモートモードを拒否し、通常のターン実行ではなく診断/保守用のエントリポイントです。

## 担当範囲と副作用

`debug models` はモデルマネージャー/カタログの経路を使い、カタログを更新するか、同梱カタログを利用できます。`prompt-input` は画像添付を含む入力正規化を検証します。`trace-reduce` はトレースバンドルを読み、縮約した状態を書き込みます。

[永続化/ロールアウト](../common/persistence-and-rollout.md)、[プロバイダー/モデル](../common/provider-and-model.md)、[出力/エラー](../common/output-and-errors.md)も参照してください。

# Provider host example

通常起動は既存のCloudflare Providerを使います。固定応答のダミーProviderを試す場合だけ、専用featureを有効にします。

```sh
just run-dummy app-server --stdio
```

ダミーProviderはモデル一覧として `A`、`B`、`C` を返し、APIキーでログインした後のチャットには常に同じ固定文を返します。データホームは既定で一時ディレクトリの `ruxex-dummy-provider` を使い、`RUXEX_DUMMY_HOME` で変更できます。通常の `~/.ruxex` は使用しません。

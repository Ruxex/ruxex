---
sidebar_position: 3
title: Model Catalog
---

# Model Catalog

## 役割

Model Catalogは「現在のProviderが利用可能なモデル」と、各モデルの表示・入力形式・推論設定・CapabilityをHostやUIが選ぶための情報です。ThreadがProviderを選択するためのRouting情報ではありません。

## 更新の流れ

```text
AppServer request
  -> current Runtime Provider
  -> provider.list_models()
  -> model catalog normalization
  -> cache / overrides / presets
  -> client response
```

Provider固有の `/models` APIや別Catalog APIはadapterが隠蔽し、CoreとClientにはProvider-neutralなCatalogを返します。

## CacheとRuntimeの関係

Catalog Cacheは起動時の永続Stateと同じ意味ではありません。Provider Homeに紐づき、現在のProvider DescriptorとRuntime設定のもとで更新されます。

モデル情報にOverrideがある場合でも、外部APIが要求するmodel identifier自体は変更せず、表示名・推奨設定・Capability情報の補正として扱います。

## 主なソース

- [`crates/models-manager/src/manager.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/models-manager/src/manager.rs)
- [`crates/models-manager/src/cache.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/models-manager/src/cache.rs)
- [`crates/model-provider/src/model_catalog_source.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/model-provider/src/model_catalog_source.rs)
- [`crates/app-server/src/request_processors/catalog_processor.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/app-server/src/request_processors/catalog_processor.rs)

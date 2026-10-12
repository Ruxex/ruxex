---
title: Provider Contract
---

# Provider Contract

## 役割

`ModelProvider` は、Coreがモデル提供者の具体的なHTTP・認証・Wire Formatを知らずに、Chatと関連機能を利用するための境界です。

現在の主な契約は [`crates/model-provider/src/contract.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/model-provider/src/contract.rs) にあります。

## 契約の構成

| 領域 | Provider-neutralな型・操作 |
| --- | --- |
| Identity | `ProviderDescriptor`、Protocol Version |
| Chat | `ChatRequest`、`ProviderChatStream`、`ProviderChatEvent` |
| Transport | Request Format、Connection Key、Stream Session |
| Model Catalog | `ListModelsRequest`、`ProviderModelCatalog` |
| Capability | `ProviderCapabilities`、Remote Compactionなど |
| Authentication | `ProviderAuthState`、Auth Fields、Request Snapshot |
| Account | Account State、Display Name、Account ID |
| Lifecycle | login、logout、delete、usage |
| Errors | Unsupported、Unauthenticated、Unauthorized、Rate Limited、Transportなど |

## `ModelProvider`が持つ責務

概念的には、次のメソッド群を持ちます。

```text
descriptor()
capabilities()
chat(request)
list_models(request)
auth_state()
account_state()
request_snapshot()
auth_fields() / auth_environment()
login(credentials)
logout()
delete()
usage(request)
```

Coreが必要とするのは、この契約の結果です。例えば「このProviderがRemote Compactionを実装しているか」はCapabilityで判断し、Provider固有のURLやHTTP payloadをCoreに書きません。

## Optional Capability

すべてのProviderがすべての機能を実装する必要はありません。未実装操作は暗黙の空値ではなく、`ProviderError::Unsupported` やCapabilityのUnsupported状態として表現します。

この区別により、次の3つを分けられます。

- Providerが機能を提供していない
- Providerが認証されていない
- Providerが機能を提供するが、現在のRequestが失敗した

## Adapterの内側

Provider実装側には、Provider固有の次の情報が閉じ込められます。

- HTTP endpointとWire API
- Request/Response fieldの変換
- Authentication headerとCredential source
- Provider固有のError detail
- Model Catalog API
- Unauthorized recovery

`genex-api` と `genex-client` は外部モデルEndpointとの通信境界ですが、Coreは `ModelProvider` を通じて利用します。

## 主なソース

- [`crates/model-provider/src/contract.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/model-provider/src/contract.rs)
- [`crates/model-provider/src/provider.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/model-provider/src/provider.rs)
- [`crates/genex-api/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/genex-api/src/lib.rs)
- [`crates/genex-client/src/lib.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/genex-client/src/lib.rs)

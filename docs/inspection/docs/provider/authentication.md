---
sidebar_position: 2
title: 認証とCredential Snapshot
---

# 認証とCredential Snapshot

## 共有される認証状態

Credential ManagerはAppServer Runtimeに共有されます。Clientごと、Threadごとに独立した認証状態を持つのではありません。

```text
AppServer Runtime
  ├─ one ModelProvider
  │    ├─ auth_state()
  │    ├─ request_snapshot()
  │    └─ login / logout
  └─ shared CredentialManager
```

認証状態の表示にはStatus、Account ID、Display Nameなどの非秘密情報を使います。Credential値そのものをThreadやRolloutへ保存しません。

## Turn開始時のSnapshot

Provider Requestには、次のような情報を持つ `ProviderRequestSnapshot` が使われます。

- 認証済みか
- Managed Authか
- Headerを付けるか
- Auth revision
- Provider種別とConnection Key
- Provider内部だけが読めるCredential Snapshot

Credential Snapshotは不透明なTrait Objectです。Coreは存在を運べますが、秘密値を読んだり永続化したりしません。

## Credential変更の振る舞い

| 操作 | 新しいTurn | 実行中のTurn |
| --- | --- | --- |
| Login / Credential更新 | 更新後のRuntime状態を使う | 開始時Snapshotを継続 |
| Logout | 新規Turnは認証エラーで拒否 | 終了まで進むか、ProviderがAuth Errorを返す |
| Auth revision変更 | Connection再利用のKeyも変わる | 古いSnapshotとの混在を避ける |

この挙動は、Credentialの更新を全Clientで共有しつつ、実行中のRequestの一貫性を保つためのものです。

## Auth ErrorとRetry

認証がない状態は `Unauthenticated`、ProviderがCredentialを拒否した状態は `Unauthorized` として区別します。Unauthorized RecoveryがProvider内部にある場合でも、CoreはProvider-neutralな結果だけを受け取ります。

## 主なソース

- [`crates/login/src/auth/manager.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/login/src/auth/manager.rs)
- [`crates/login/src/auth/change_state.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/login/src/auth/change_state.rs)
- [`crates/model-provider/src/contract.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/model-provider/src/contract.rs)
- [`crates/model-provider/src/provider.rs`](https://github.com/edge-m/codex-fork/blob/main/crates/model-provider/src/provider.rs)

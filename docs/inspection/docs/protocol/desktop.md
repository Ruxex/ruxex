---
sidebar_position: 4
title: Desktopクライアント
---

# Desktopクライアント

DesktopはTauriホストを持つClient Surfaceです。UIがThread/Turnの詳細を直接所有するのではなく、App Server Protocolまたは同等のRuntime境界を利用します。

読むときは、Desktopページから始めず、先に [App Server Protocol](./app-server) でThread/Turn lifecycleを理解してください。Desktop固有のページは、Login、Home、Threads、AccountなどがProtocolのどの操作に対応するかを見るためのものです。

## 主なソース

- [`apps/desktop/src/pages/NewChatPage.tsx`](https://github.com/edge-m/codex-fork/blob/main/apps/desktop/src/pages/NewChatPage.tsx)
- [`apps/desktop/src/pages/ThreadsPage.tsx`](https://github.com/edge-m/codex-fork/blob/main/apps/desktop/src/pages/ThreadsPage.tsx)
- [`apps/desktop/src/protocol/index.ts`](https://github.com/edge-m/codex-fork/blob/main/apps/desktop/src/protocol/index.ts)

---
slug: /
sidebar_position: 1
title: Genex 内部構造ガイド
description: Genexの設計と実行時の振る舞いを理解するためのコードリーディングガイド
---

# Genex 内部構造ガイド

Genexの設計と実行時の振る舞いを、コードの責務とデータの流れから理解するためのガイドです。

いきなり個別の関数を読むのではなく、まず実行単位と境界を把握し、その後にチャットループ、コンテキスト収集、永続化を読み進めます。

## まず読むページ

1. [このガイドの読み方](/intro/reading-order)
2. [システムマップ](/intro/system-map)
3. [実行モデル](/runtime/execution-model)
4. [チャットループ](/chat/chat-loop)
5. [チャット前のコンテキスト収集](/chat/context-assembly)
6. [保存領域全体図](/storage/storage-map)

## このサイトの立場

これは一般ユーザー向けの利用ガイドではなく、Genexを自分で設計・実装した人が持つべきメンタルモデルを作るための資料です。コードの全行を説明するのではなく、次の問いに答えます。

- どの部品が、どの状態を所有するのか
- 1回のチャットが、どの境界を何度通るのか
- モデルに渡る情報は、どこから集められるのか
- 何がRollout、SQLite、認証ストレージに保存されるのか
- Provider、Tool、MCP、UI、App Serverの責務はどこで分かれるのか

実装上の根拠は、ページ内の「主なソース」に記載したファイルと、現在のリポジトリの [Genex fork](https://github.com/edge-m/codex-fork) に対応づけています。

# `codex sandbox`

## エントリ

`HostSandboxArgs` は [`cli/src/main.rs`](../../../../crates/cli/src/main.rs#L414) で、macOS Seatbelt、Linux Landlock、Windows のサンドボックス実装のいずれかとしてコンパイル時に選択されます。ルートのディスパッチは [main.rs#L1442](../../../../crates/cli/src/main.rs#L1442) にあり、プラットフォーム固有のランナーは `crates/cli/src/debug_sandbox.rs` にあります。

## フロー

```text
codex sandbox [PROFILE OPTIONS] -- COMMAND
  -> host-specific parser
  -> config/profile and permission profile resolution
  -> platform command construction
  -> child process execution under sandbox
  -> child exit status propagation
```

サンドボックスコマンドはローカルプロセスのラッパーであり、モデルが発行するツールに適用される承認ポリシーと同じものではありません。どちらも最終的にはプラットフォームのポリシー基盤を利用しますが、呼び出し元とライフサイクルが異なります。[サンドボックスと権限](../common/sandbox-and-permissions.md)、[ポリシー](policy.md)、[出力/エラー](../common/output-and-errors.md)も参照してください。

リモートモードは拒否されます。対応していないオペレーティングシステムでは、サンドボックスなしで黙って実行するのではなく、明示的に失敗します。

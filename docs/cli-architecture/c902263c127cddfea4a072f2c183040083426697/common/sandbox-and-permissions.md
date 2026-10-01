# サンドボックス、承認、権限

## 4 つの層

1. CLI レベルのサンドボックスラッパーは、明示的に指定された子コマンドを実行します。
2. ツール承認は、モデルが発行した操作に人間の確認が必要かどうかを決めます。
3. Execpolicy はコマンドルールを評価し、許可/拒否/確認の判断に寄与します。
4. プラットフォームのサンドボックスは許可された子プロセスを制限し、ユーザー検証は別のセキュリティゲートを追加できます。

Core は [`core/src/tools/sandboxing.rs`](../../../../crates/core/src/tools/sandboxing.rs#L309) にある [`Approvable`](../reference/trait-index.md#approvable)、[`Sandboxable`](../reference/trait-index.md#sandboxable)、[`ToolRuntime`](../reference/trait-index.md#toolruntime) を通じて、ツールレベルの層を組み合わせます。`codex sandbox` は `genex-sandboxing` のプラットフォームコンストラクターと `genex-cli` のデバッグサンドボックスヘルパーを利用します。

## 安全性の不変条件

承認とサンドボックスは置き換え可能ではありません。承認によって操作の続行が許可されても、結果として起動するプロセスは選択されたサンドボックス/プロファイル内で実行されます。逆に、サンドボックスの制限だけでは、なぜ承認が要求されたか、または要求されなかったかは分かりません。

プラットフォームの選択はコンパイル時かつホスト固有です。macOS Seatbelt、Linux Landlock、Windows のサンドボックス経路は `HostSandboxArgs` で選択されます。対応していないプラットフォームでは明示的に失敗します。

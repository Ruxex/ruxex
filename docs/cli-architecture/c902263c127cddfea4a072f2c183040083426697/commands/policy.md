# Execution policy

## Entry

The hidden top-level `execpolicy` command currently exposes `execpolicy check`, defined by [`ExecpolicySubcommand`](../../../../crates/cli/src/main.rs#L439) and dispatched at [main.rs#L1546](../../../../crates/cli/src/main.rs#L1546).

## Role

The checker evaluates execpolicy files against a command. It is a policy inspection tool; it does not execute the command. During an agent turn, core/tool runtime uses related policy and approval paths before spawning commands.

The distinction is:

- approval policy decides whether human confirmation is required;
- execpolicy determines command policy behavior;
- sandboxing constrains the process after it is admitted;
- user verification may add an interactive security gate.

See [sandbox and permissions](../common/sandbox-and-permissions.md), [tool execution](../common/tool-execution.md), and [sandbox command](sandbox.md).

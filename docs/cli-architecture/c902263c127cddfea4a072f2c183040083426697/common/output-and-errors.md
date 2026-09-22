# Output and error propagation

## Output families

| Caller | Primary output |
| --- | --- |
| Interactive `codex` | TUI rendering and exit summary |
| `codex exec` / `review` | text, JSONL events, final message file, optional schema-shaped final result |
| app-server | protocol responses and notifications |
| utility commands | human text or command-specific JSON |
| sandbox wrapper | child process exit status |

`codex-exec` has an internal [`EventProcessor`](../reference/trait-index.md#eventprocessor) boundary for converting response events into non-interactive output. API errors, transport errors, core failures, and child exit statuses are kept distinct until the relevant command maps them to its public output/exit contract.

## Error rule

A diagnostic command should explain failure and remediation without silently mutating state. A runtime command should preserve enough error context to identify configuration, authentication, provider, transport, tool, policy, or process failure. See [doctor](../commands/diagnostics.md) and [tool execution](tool-execution.md).

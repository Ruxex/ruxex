# Provider host example

This example normally uses the existing Cloudflare provider. Enable the
dedicated feature when you want to exercise the fixed-response dummy provider.

```sh
just run-dummy app-server --stdio
```

The dummy provider exposes three fixed models: `A`, `B`, and `C`. After
authentication, each chat request selects a fixed scenario from the latest
user message and the request history. The scenario is not persisted by the
provider, and credentials are not part of the scenario state.

The data home defaults to a temporary directory named `ruxex-dummy-provider`.
Set `RUXEX_DUMMY_HOME` to use a different location. The example does not use
the normal `~/.ruxex` directory.

## Chat scenarios

The recommended way to select a scenario is an explicit marker in the user
message. Use the form `example:scenario`; `slow` and `stream` can be added
as modifiers.

```text
example:basic
example:basic:stream
example:basic:slow
example:tool
example:command
example:command:ls
example:command:sleep
example:command:slow
example:command:large
example:command:fail
example:command:timeout
example:approval
example:approval:deny
example:approval:cancel
example:workflow
example:parallel-tools
example:long-stream
example:long-stream:slow
example:user-input
example:permissions
example:goal
example:mcp
example:plan
example:reasoning
example:markdown
example:subagent
example:parallel-subagents
example:multi
example:error
example:rate-limit
example:disconnect
```

If there is no explicit marker, the provider checks the latest user message
for scenario keywords. The current keyword priority is:

1. `parallel-tools`
2. `workflow`
3. `long stream` or `long-stream`
4. `user input` or `user-input`
5. `permissions`
6. goal-setting phrases such as `set it as a goal`
7. `approval`
8. `mcp`
9. `command` or `exec`
10. `tool`
11. `multi`
12. `rate-limit`
13. `disconnect`
14. `error`

The `slow` and `delay` keywords add delays to the response stream. The
`stream` keyword splits the fixed basic response into multiple text deltas.
`example:basic:slow` combines both behaviors. `example:command:slow`
is retained as an alias for `example:command:sleep`. The `large`, `fail`,
and `timeout` keywords select the corresponding command variants when no
explicit marker is used.

`example:goal` is a goal lifecycle fixture. The first turn emits a
`create_goal` call from natural-language input; after the next user turn, it
emits `update_goal` with `complete` so clients can verify both goal
notifications.

Every successfully completed ChatCompletionAPI response from the dummy
provider reports the same fixed usage: 100 input tokens, 20 output tokens,
120 total tokens, and amount `0.001`. Amount is an opaque Example-provider
unit with no currency or cross-provider meaning. Tool-call responses and
continuation responses each report usage for their own API response. Errors
and interrupted streams do not report completed-response usage.

### Scenario behavior

- `basic` returns one fixed English response. The `stream` variant emits the
  response in multiple text deltas, while the `slow` variant emits those same
  deltas with delays between them.
- `tool` returns a fixed `example_tool` function call. After the tool result
  is included in the next request, it returns a fixed English completion.
- `command` returns a fixed `exec_command` call. The command is defined by
  the provider and is never generated from user input. `command:ls` runs `ls`.
  `command:sleep` runs `echo`, `sleep 3`, and `echo` in sequence so command
  output can be observed before and after the delay. The `slow` marker is a
  compatibility alias for `command:sleep` and also delays provider events.
  `command:large` produces many fixed output lines, `command:fail` writes
  fixed stderr and exits non-zero, and `command:timeout` sleeps with a fixed
  `timeout_ms` value.
- `approval` returns a fixed `exec_command` call with
  `sandbox_permissions: "require_escalated"` and a fixed justification. The
  dummy provider defaults to `approval_policy = "on-request"` with
  `sandbox_mode = "read-only"`, so the client/runtime shows an approval prompt
  without an additional override. Approval and denial are handled by the
  client.
  `approval:deny` and `approval:cancel` use distinct call IDs and provide
  fixed continuation text after the client returns the corresponding result.
- `workflow` emits an assistant message, then a fixed tool call, then a fixed
  command call, and finally a completion. Each step is selected from the
  exact output IDs in the request history.
- `parallel-tools` emits one fixed tool call and one fixed command call under a
  shared response ID. It completes after both outputs are returned.
- `long-stream` returns a fixed Markdown response with code content split into
  bounded text deltas. The `slow` modifier adds inter-delta delays without
  changing the content.
- `user-input` emits a fixed `request_user_input` call with one question and
  completes after the client returns its result.
- `permissions` emits a fixed `request_permissions` call with file-system
  permissions and completes after the client returns its result. The dummy
  provider enables `request_permissions_tool` in its default config. Permission
  selection remains client/runtime-owned.
- `plan` calls `update_plan` when that tool is available in the request, then
  returns a fixed completion after the tool result. Without the tool it returns
  an explanatory response. The call appears as Plan activity in Tauri.
- `reasoning` emits a fixed reasoning summary event followed by a fixed answer,
  exercising Tauri's Reasoning activity.
- `markdown` returns one fixed rich-Markdown response with headings, emphasis,
  strikethrough, inline code, links, ordered and unordered lists, a task list,
  blockquote, GFM table, horizontal rule, Rust and diff code fences, and a
  Mermaid diagram. It is intended for visual inspection in the conversation.
- `subagent` emits one `spawn_agent` call when the request contains the V1
  `multi_agent_v1.spawn_agent` or V2 `spawn_agent` tool. Otherwise it returns
  an explanatory response. The child prompt requests one brief sentence and
  asks the child not to call tools. Invoking this scenario can start a real
  configured child-agent run; Tauri currently shows its call as generic Agent
  collaboration activity.
- `parallel-subagents` emits two `spawn_agent` calls in one response and
  completes after both call outputs are returned. If only one output is
  present, it emits the missing call without repeating the completed one. It
  uses the same tool-availability, child-prompt, and real-agent behavior as
  `subagent`.
- `mcp` returns a fixed `example/echo` call when the fixture MCP tool is
  available. Otherwise it returns a fixed unavailable message.
- `multi` returns a different fixed English response for each of three turns.
  A later turn falls back to the basic scenario.
- `error` returns a transient provider error.
- `rate-limit` returns a rate-limit error with a three-second retry hint.
- `disconnect` emits partial output and then terminates the stream with a
  transport error.

Reconnect/resume behavior is intentionally not included in this scenario
catalog.

All user-visible fixture responses are in English. Only the user input and
request history select the scenario; the response content remains fixed.

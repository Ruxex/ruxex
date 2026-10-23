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
message. Use the form `[[example:scenario]]`; `slow` and `stream` can be added
as modifiers.

```text
[[example:basic]]
[[example:tool]]
[[example:command]]
[[example:command:slow]]
[[example:mcp]]
[[example:multi]]
[[example:error]]
[[example:rate-limit]]
[[example:disconnect]]
```

If there is no explicit marker, the provider checks the latest user message
for scenario keywords. The current keyword priority is:

1. `mcp`
2. `command` or `exec`
3. `tool`
4. `multi`
5. `rate-limit`
6. `disconnect`
7. `error`

The `slow` and `delay` keywords add delays to the response stream. The
`stream` keyword splits the fixed basic response into multiple text deltas.

### Scenario behavior

- `basic` returns one fixed English response.
- `tool` returns a fixed `example_tool` function call. After the tool result
  is included in the next request, it returns a fixed English completion.
- `command` returns a fixed `exec_command` call. The command is defined by
  the provider and is never generated from user input. The `slow` modifier
  uses a command that sleeps before printing its fixed output.
- `mcp` returns a fixed `example/echo` call when the fixture MCP tool is
  available. Otherwise it returns a fixed unavailable message.
- `multi` returns a different fixed English response for each of three turns.
  A later turn falls back to the basic scenario.
- `error` returns a transient provider error.
- `rate-limit` returns a rate-limit error with a three-second retry hint.
- `disconnect` emits partial output and then terminates the stream with a
  transport error.

All user-visible fixture responses are in English. Only the user input and
request history select the scenario; the response content remains fixed.

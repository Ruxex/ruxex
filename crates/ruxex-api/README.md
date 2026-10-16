# ruxex-api

Typed clients for Ruxex/provider APIs built on top of the generic transport in `ruxex-client`.

- Hosts the request/response models and request builders for Responses and related Ruxex APIs.
- Owns provider configuration (base URLs, headers, query params), auth header injection, retry tuning, and stream idle settings.
- Parses SSE streams into `ResponseEvent`/`ResponseStream`, including rate-limit snapshots and API-specific error mapping.
- Serves as the wire-level layer consumed by `ruxex-core`; higher layers handle auth refresh and business logic.

## Core interface

The public interface of this crate is intentionally small and uniform:

- **Responses endpoint**
  - Input:
    - `ResponsesApiRequest` for the request body (`model`, `instructions`, `input`, `tools`, `parallel_tool_calls`, reasoning/text controls).
    - `ResponsesOptions` for transport/header concerns (`conversation_id`, `session_source`, `extra_headers`, `compression`, `turn_state`).
  - Output: a `ResponseStream` of `ResponseEvent` (both re-exported from `common`).

All HTTP details (URLs, headers, retry/backoff policies, SSE framing) are encapsulated in `ruxex-api` and `ruxex-client`. Callers construct prompts/inputs using protocol types and work with typed streams of `ResponseEvent` or other endpoint-specific response values.

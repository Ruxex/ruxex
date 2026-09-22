# Telemetry and observability

## Runtime observations

Telemetry is initialized at different boundaries:

- API/client layers observe request/stream behavior through telemetry Traits;
- core/app-server initializes tracing/metrics according to loaded configuration and defaults;
- CLI daemon update paths use consent-aware config and bounded shutdown;
- exec-server has its own process lifetime and service name handling;
- doctor reports diagnostics but is not itself a long-lived telemetry service.

[`SseTelemetry`](../reference/trait-index.md#ssetelemetry), `WebsocketTelemetry`, and [`RequestTelemetry`](../reference/trait-index.md#requesttelemetry) are observation hooks, not agent behavior interfaces.

## Consent and defaults

App-server defaults and foreground CLI defaults can differ. The app-server command exposes a metrics-default option for integration scenarios, while config can still disable metrics. Detached daemon/update paths must not assume the foreground CLI's lifetime or interactive consent flow.

//! Metrics derived from loaded configuration at session start.

use super::Config;
use ruxex_otel::SessionTelemetry;

pub(crate) fn emit_session_start_metrics(config: &Config, telemetry: &SessionTelemetry) {
    config.features.emit_metrics(telemetry);
}

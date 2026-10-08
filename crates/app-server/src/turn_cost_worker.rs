//! Turn-cost collection is hosted-provider specific and intentionally disabled.

use genex_core::config::Config;
use genex_login::CredentialManager;
use genex_otel::SessionTelemetry;
use genex_protocol::ThreadId;
use genex_protocol::protocol::Event;
use std::sync::Arc;

pub(crate) struct TurnCostWorker;

#[derive(Clone)]
pub(crate) struct TurnCostWorkerHandle;

impl TurnCostWorker {
    pub(crate) fn spawn(_config: Arc<Config>, _credential_manager: Arc<CredentialManager>) -> Option<Self> {
        None
    }

    pub(crate) fn handle(&self) -> TurnCostWorkerHandle {
        TurnCostWorkerHandle
    }

    pub(crate) fn shutdown(&self) {}
}

impl TurnCostWorkerHandle {
    pub(crate) fn observe_event(
        &self,
        _thread_id: ThreadId,
        _thread_config: &Config,
        _event: &Event,
        _session_telemetry: impl FnOnce() -> SessionTelemetry,
    ) {
    }
}

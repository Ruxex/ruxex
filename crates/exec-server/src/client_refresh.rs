use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::OnceCell;
use tokio_util::sync::CancellationToken;

use super::ConnectionResult;
use crate::client_api::ExecServerTransportParams;

/// Shared startup result for the lazy local/stdio client.
#[derive(Default)]
pub(super) struct ConnectionAttempt {
    pub(super) result: OnceCell<ConnectionResult>,
    pub(super) cancelled: CancellationToken,
    pub(super) transport: Option<ExecServerTransportParams>,
}

impl super::LazyRemoteExecServerClient {
    pub(super) fn connect_once<'a>(
        &'a self,
        attempt: &'a ConnectionAttempt,
    ) -> BoxFuture<'a, super::ConnectionResult> {
        Box::pin(async move {
            let transport = attempt
                .transport
                .as_ref()
                .or(self.transport_params.as_ref())
                .ok_or_else(|| {
                    Arc::new(super::ExecServerError::Protocol(
                        "missing transport params for lazy exec-server connection".to_string(),
                    ))
                })?;
            let client = tokio::select! {
                biased;
                _ = attempt.cancelled.cancelled() => return Err(Arc::new(super::ExecServerError::Disconnected("connection attempt was superseded".to_string()))),
                result = super::ExecServerClient::connect_for_transport(transport.clone(), self.http_client_factory.clone()) => result.map_err(Arc::new)?,
            };
            let mut current = self
                .current_client
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !attempt.cancelled.is_cancelled() {
                client.attach_environment_connection_state(
                    self.environment_connection_state_tx.clone(),
                );
                *current = Some(client.clone());
                return Ok(client);
            }
            drop(current);
            Err(Arc::new(super::ExecServerError::Disconnected(
                "connection attempt was superseded".to_string(),
            )))
        })
    }
}

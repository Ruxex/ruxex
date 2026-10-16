//! Cancels connection work when its executor's proxy scope ends, including HTTP upgrades.

use rama_core::Service;
use rama_core::graceful::ShutdownGuard;

#[derive(Clone)]
pub(crate) struct CancelOnShutdown<S> {
    inner: S,
    guard: ShutdownGuard,
}

impl<S> CancelOnShutdown<S> {
    pub(crate) fn new(inner: S, guard: ShutdownGuard) -> Self {
        Self { inner, guard }
    }
}

impl<S, Request> Service<Request> for CancelOnShutdown<S>
where
    S: Service<Request, Output = ()>,
    Request: Send + 'static,
{
    type Output = ();
    type Error = S::Error;

    async fn serve(&self, request: Request) -> Result<(), Self::Error> {
        tokio::select! {
            biased;
            _ = self.guard.cancelled() => Ok(()),
            result = self.inner.serve(request) => result,
        }
    }
}

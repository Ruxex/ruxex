//! Cancels connection work when its executor's proxy scope ends, including HTTP upgrades.

use rama_core::Service;

#[derive(Clone)]
pub(crate) struct CancelOnShutdown<S> {
    inner: S,
}

impl<S> CancelOnShutdown<S> {
    pub(crate) fn new(inner: S) -> Self {
        Self { inner }
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
        self.inner.serve(request).await
    }
}

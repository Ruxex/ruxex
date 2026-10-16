use rama_core::error::BoxError;
use rama_core::extensions::Extension;
use rama_net::address::HostWithPort;

/// CONNECT target carried from an HTTP request to the upgraded I/O stream.
#[derive(Clone, Debug)]
pub(crate) struct ProxyTarget(pub(crate) HostWithPort);

impl Extension for ProxyTarget {}

/// Forward two Rama I/O streams while keeping the old proxy behavior of treating
/// ordinary connection-close errors as a normal tunnel completion.
pub(crate) async fn forward_io<A, B>(mut source: A, mut target: B) -> Result<(), BoxError>
where
    A: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    B: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    match tokio::io::copy_bidirectional(&mut source, &mut target).await {
        Ok(_) => Ok(()),
        Err(err) if is_connection_error(&err) => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn is_connection_error(err: &std::io::Error) -> bool {
    matches!(
        err.kind(),
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::UnexpectedEof
    )
}

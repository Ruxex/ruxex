use genex_http_client::HttpClientFactory;
use tokio::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

use crate::ExecServerRuntimePaths;
use crate::ExecServerTelemetry;
use crate::connection::JsonRpcConnection;
use crate::server::RequestDispatchMode;
use crate::server::processor::ConnectionProcessor;
use crate::telemetry::ConnectionTransport;

pub const DEFAULT_LISTEN_URL: &str = "stdio";

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum ExecServerListenTransport {
    Stdio,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExecServerListenUrlParseError {
    UnsupportedListenUrl(String),
}

impl std::fmt::Display for ExecServerListenUrlParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedListenUrl(listen_url) => write!(
                f,
                "unsupported --listen URL `{listen_url}`; expected `stdio` or `stdio://`"
            ),
        }
    }
}

impl std::error::Error for ExecServerListenUrlParseError {}

pub(crate) fn parse_listen_url(
    listen_url: &str,
) -> Result<ExecServerListenTransport, ExecServerListenUrlParseError> {
    if matches!(listen_url, "stdio" | "stdio://") {
        Ok(ExecServerListenTransport::Stdio)
    } else {
        Err(ExecServerListenUrlParseError::UnsupportedListenUrl(
            listen_url.to_string(),
        ))
    }
}

pub(crate) async fn run_transport(
    listen_url: &str,
    runtime_paths: ExecServerRuntimePaths,
    telemetry: ExecServerTelemetry,
    http_client_factory: HttpClientFactory,
    request_dispatch_mode: RequestDispatchMode,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match parse_listen_url(listen_url)? {
        ExecServerListenTransport::Stdio => {
            run_stdio_connection(
                runtime_paths,
                telemetry,
                http_client_factory,
                request_dispatch_mode,
            )
            .await
        }
    }
}

async fn run_stdio_connection(
    runtime_paths: ExecServerRuntimePaths,
    telemetry: ExecServerTelemetry,
    http_client_factory: HttpClientFactory,
    request_dispatch_mode: RequestDispatchMode,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    run_stdio_connection_with_io(
        io::stdin(),
        io::stdout(),
        runtime_paths,
        telemetry,
        http_client_factory,
        request_dispatch_mode,
    )
    .await
}

async fn run_stdio_connection_with_io<R, W>(
    reader: R,
    writer: W,
    runtime_paths: ExecServerRuntimePaths,
    telemetry: ExecServerTelemetry,
    http_client_factory: HttpClientFactory,
    request_dispatch_mode: RequestDispatchMode,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let processor = ConnectionProcessor::new_with_telemetry(
        runtime_paths,
        telemetry,
        http_client_factory,
        request_dispatch_mode,
    );
    tracing::info!("codex-exec-server listening on stdio");
    processor
        .run_connection(
            JsonRpcConnection::from_stdio(reader, writer, "exec-server stdio".to_string()),
            ConnectionTransport::Stdio,
        )
        .await;
    processor.shutdown().await;
    Ok(())
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod transport_tests;

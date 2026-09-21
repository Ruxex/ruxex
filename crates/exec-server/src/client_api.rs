use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use futures::future::BoxFuture;
use crate::ExecServerError;
use crate::HttpRequestParams;
use crate::HttpRequestResponse;
use crate::HttpResponseBodyStream;

pub(crate) const DEFAULT_EXEC_SERVER_INITIALIZE_TIMEOUT: Duration = Duration::from_secs(10);

/// Connection options for an exec-server client transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecServerClientConnectOptions {
    pub client_name: String,
    pub initialize_timeout: Duration,
    pub resume_session_id: Option<String>,
}

/// Stdio connection arguments for a command-backed exec-server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StdioExecServerConnectArgs {
    pub command: StdioExecServerCommand,
    pub client_name: String,
    pub initialize_timeout: Duration,
    pub resume_session_id: Option<String>,
}

/// Structured process command used to start an exec-server over stdio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StdioExecServerCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub cwd: Option<PathBuf>,
}

/// Parameters used to connect to an exec-server environment.
#[derive(Clone)]
pub(crate) enum ExecServerTransportParams {
    StdioCommand {
        command: StdioExecServerCommand,
        initialize_timeout: Duration,
    },
}

impl std::fmt::Debug for ExecServerTransportParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StdioCommand {
                command,
                initialize_timeout,
            } => f
                .debug_struct("StdioCommand")
                .field("command", command)
                .field("initialize_timeout", initialize_timeout)
                .finish(),
        }
    }
}

/// Sends HTTP requests through an environment's runtime-selected transport.
pub trait HttpClient: Send + Sync {
    /// Perform an HTTP request and buffer the response body.
    fn http_request(
        &self,
        params: HttpRequestParams,
    ) -> BoxFuture<'_, Result<HttpRequestResponse, ExecServerError>>;

    /// Perform an HTTP request and return a streamed body handle.
    fn http_request_stream(
        &self,
        params: HttpRequestParams,
    ) -> BoxFuture<'_, Result<(HttpRequestResponse, HttpResponseBodyStream), ExecServerError>>;
}

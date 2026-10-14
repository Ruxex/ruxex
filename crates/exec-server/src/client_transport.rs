use std::process::Stdio;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command;
use tokio::sync::OwnedSemaphorePermit;
use tracing::debug;
use tracing::warn;

use ruxex_protocol::shell_environment::scrub_non_inheritable_env_vars;

use crate::ExecServerClient;
use crate::ExecServerError;
use crate::client_api::ExecServerClientConnectOptions;
use crate::client_api::ExecServerTransportParams;
use crate::client_api::StdioExecServerCommand;
use crate::client_api::StdioExecServerConnectArgs;
use crate::connection::JsonRpcConnection;

const ENVIRONMENT_CLIENT_NAME: &str = "ruxex-environment";

pub(crate) struct ReconnectAttempt {
    connection: JsonRpcConnection,
    options: ExecServerClientConnectOptions,
    attempt_permit: Option<OwnedSemaphorePermit>,
}

impl ReconnectAttempt {
    pub(crate) fn into_parts(
        self,
    ) -> (
        JsonRpcConnection,
        ExecServerClientConnectOptions,
        Option<OwnedSemaphorePermit>,
    ) {
        (self.connection, self.options, self.attempt_permit)
    }
}

pub(crate) enum ExecServerReconnectStrategy {}

impl ExecServerReconnectStrategy {
    pub(crate) async fn resume(
        &self,
        _session_id: &str,
    ) -> Result<ReconnectAttempt, ExecServerError> {
        match *self {}
    }
}

impl ExecServerClient {
    pub(crate) async fn connect_for_transport(
        transport_params: ExecServerTransportParams,
        _http_client_factory: ruxex_http_client::HttpClientFactory,
    ) -> Result<Self, ExecServerError> {
        match transport_params {
            ExecServerTransportParams::StdioCommand {
                command,
                initialize_timeout,
            } => {
                Self::connect_stdio_command(StdioExecServerConnectArgs {
                    command,
                    client_name: ENVIRONMENT_CLIENT_NAME.to_string(),
                    initialize_timeout,
                    resume_session_id: None,
                })
                .await
            }
        }
    }

    pub(crate) async fn connect_stdio_command(
        args: StdioExecServerConnectArgs,
    ) -> Result<Self, ExecServerError> {
        let StdioExecServerConnectArgs {
            command,
            client_name,
            initialize_timeout,
            resume_session_id,
        } = args;
        let StdioExecServerCommand {
            program,
            args,
            env,
            cwd,
        } = command;
        let mut process = Command::new(&program);
        process.args(&args);
        process.envs(&env);
        if let Some(cwd) = cwd {
            process.current_dir(cwd);
        }
        scrub_non_inheritable_env_vars(process.as_std_mut());
        #[cfg(unix)]
        // Keep the spawned executor and its descendants in a private process
        // group so dropping the RPC client terminates the complete tree.
        // SAFETY: set_process_group only performs async-signal-safe setup.
        unsafe {
            process
                .as_std_mut()
                .pre_exec(ruxex_utils_pty::process_group::set_process_group);
        }
        process
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = process.spawn().map_err(ExecServerError::Spawn)?;
        let stdin = child.stdin.take().ok_or_else(|| {
            ExecServerError::Protocol("exec-server stdin was not piped".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ExecServerError::Protocol("exec-server stdout was not piped".to_string())
        })?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    debug!("exec-server stdio stderr: {line}");
                }
            });
        }
        let connection =
            JsonRpcConnection::from_stdio(stdout, stdin, format!("exec-server stdio `{program}`"))
                .with_child_process(child);
        debug!(program = %program, "started exec-server over stdio");
        let result = Self::connect(
            connection,
            ExecServerClientConnectOptions {
                client_name,
                initialize_timeout,
                resume_session_id,
            },
        )
        .await;
        if let Err(error) = &result {
            warn!(error = %error, "exec-server stdio initialization failed");
        }
        result
    }
}

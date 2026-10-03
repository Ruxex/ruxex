#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use anyhow::anyhow;
use genex_exec_server_protocol::JSONRPCMessage;
use genex_exec_server_protocol::JSONRPCNotification;
use genex_exec_server_protocol::JSONRPCRequest;
use genex_exec_server_protocol::RequestId;
use tempfile::TempDir;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::io::Lines;
use tokio::process::Child;
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use tokio::process::Command;
use tokio::time::Instant;
use tokio::time::timeout;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const EVENT_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) struct ExecServerHarness {
    genex_home: TempDir,
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    next_request_id: i64,
}

impl Drop for ExecServerHarness {
    fn drop(&mut self) {
        let _ = self.child.start_kill();
    }
}

pub(crate) struct TestGenexHelperPaths {
    pub(crate) genex_exe: PathBuf,
    pub(crate) genex_linux_sandbox_exe: Option<PathBuf>,
}

pub(crate) fn test_genex_helper_paths() -> anyhow::Result<TestGenexHelperPaths> {
    let (helper_binary, genex_linux_sandbox_exe) = super::current_test_binary_helper_paths()?;
    Ok(TestGenexHelperPaths {
        genex_exe: helper_binary,
        genex_linux_sandbox_exe,
    })
}

pub(crate) async fn exec_server() -> anyhow::Result<ExecServerHarness> {
    exec_server_with_env(std::iter::empty::<(&str, &str)>(), &[]).await
}

pub(crate) async fn exec_server_with_env<I, K, V>(
    env: I,
    args: &[&str],
) -> anyhow::Result<ExecServerHarness>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<std::ffi::OsStr>,
    V: AsRef<std::ffi::OsStr>,
{
    let helper_paths = test_genex_helper_paths()?;
    let mut child = Command::new(&helper_paths.genex_exe);
    child.args(["exec-server", "--listen", "stdio"]);
    child.args(args);
    child.envs(env);
    ExecServerHarness::start(child).await
}

impl ExecServerHarness {
    pub(crate) async fn start(mut command: Command) -> anyhow::Result<Self> {
        let genex_home = TempDir::new()?;
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::inherit());
        command.kill_on_drop(true);
        if !command
            .as_std()
            .get_envs()
            .any(|(key, value)| key == "GENEX_HOME" && value.is_some())
        {
            command.env("GENEX_HOME", genex_home.path());
        }
        let mut child = command.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to capture exec-server stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture exec-server stdout"))?;

        Ok(Self {
            genex_home,
            child,
            stdin,
            stdout: BufReader::new(stdout).lines(),
            next_request_id: 1,
        })
    }

    pub(crate) fn genex_home(&self) -> &std::path::Path {
        self.genex_home.path()
    }

    pub(crate) async fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<RequestId> {
        let id = RequestId::Integer(self.next_request_id);
        self.next_request_id += 1;
        self.send_message(JSONRPCMessage::Request(JSONRPCRequest {
            id: id.clone(),
            method: method.to_string(),
            params: Some(params),
            trace: None,
        }))
        .await?;
        Ok(id)
    }

    pub(crate) async fn send_notification(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<()> {
        self.send_message(JSONRPCMessage::Notification(JSONRPCNotification {
            method: method.to_string(),
            params: Some(params),
        }))
        .await
    }

    pub(crate) async fn send_raw_text(&mut self, text: &str) -> anyhow::Result<()> {
        self.stdin.write_all(text.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub(crate) async fn send_raw_binary(&mut self, bytes: Vec<u8>) -> anyhow::Result<()> {
        self.stdin.write_all(&bytes).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub(crate) async fn next_event(&mut self) -> anyhow::Result<JSONRPCMessage> {
        self.next_event_with_timeout(EVENT_TIMEOUT).await
    }

    pub(crate) async fn wait_for_event<F>(
        &mut self,
        mut predicate: F,
    ) -> anyhow::Result<JSONRPCMessage>
    where
        F: FnMut(&JSONRPCMessage) -> bool,
    {
        let deadline = Instant::now() + EVENT_TIMEOUT;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return Err(anyhow!(
                    "timed out waiting for matching exec-server event after {EVENT_TIMEOUT:?}"
                ));
            }
            let event = self
                .next_event_with_timeout(deadline.duration_since(now))
                .await?;
            if predicate(&event) {
                return Ok(event);
            }
        }
    }

    pub(crate) async fn shutdown(&mut self) -> anyhow::Result<()> {
        self.child.start_kill()?;
        timeout(CONNECT_TIMEOUT, self.child.wait())
            .await
            .map_err(|_| anyhow!("timed out waiting for exec-server shutdown"))??;
        Ok(())
    }

    async fn send_message(&mut self, message: JSONRPCMessage) -> anyhow::Result<()> {
        let encoded = serde_json::to_string(&message)?;
        self.send_raw_text(&encoded).await
    }

    async fn next_event_with_timeout(
        &mut self,
        timeout_duration: Duration,
    ) -> anyhow::Result<JSONRPCMessage> {
        let line = timeout(timeout_duration, self.stdout.next_line())
            .await
            .map_err(|_| anyhow!("timed out waiting for exec-server stdio event"))??
            .ok_or_else(|| anyhow!("exec-server stdio closed"))?;
        Ok(serde_json::from_str(&line)?)
    }
}

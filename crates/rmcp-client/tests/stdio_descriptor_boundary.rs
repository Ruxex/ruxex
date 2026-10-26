//! Local MCP servers must inherit only their explicit stdio descriptors.

#![cfg(unix)]

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::os::fd::AsRawFd;
use std::os::fd::FromRawFd;
use std::os::fd::OwnedFd;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::FutureExt as _;
use pretty_assertions::assert_eq;
use rmcp::model::ClientCapabilities;
use rmcp::model::Implementation;
use rmcp::model::InitializeRequestParams;
use rmcp::model::ProtocolVersion;
use ruxex_rmcp_client::ElicitationAction;
use ruxex_rmcp_client::ElicitationResponse;
use ruxex_rmcp_client::LocalStdioServerLauncher;
use ruxex_rmcp_client::RmcpClient;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_stdio_excludes_inheritable_fds_from_server_and_descendant() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let cwd = fs::canonicalize(temporary.path())?;
    let script = temporary.path().join("server.py");
    fs::write(
        &script,
        r#"import errno, json, os, subprocess, sys
sentinels = json.loads(sys.argv[2])
def probe():
    inherited = []
    for fd in sentinels:
        try:
            os.fstat(fd)
            inherited.append(True)
        except OSError as error:
            if error.errno != errno.EBADF:
                raise
            inherited.append(False)
    for fd in (0, 1, 2):
        os.fstat(fd)
    return inherited
if sys.argv[1] == "probe":
    print(json.dumps(probe()), flush=True)
    sys.exit(0)
for line in sys.stdin:
    message = json.loads(line)
    method = message.get("method")
    if method == "initialize":
        result = {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}}, "serverInfo": {"name": "descriptor-fixture", "version": "1"}}
    elif method == "tools/list":
        result = {"tools": [{"name": "probe", "inputSchema": {"type": "object"}}]}
    elif method == "tools/call":
        child = subprocess.run([sys.executable, __file__, "probe", sys.argv[2]], close_fds=False, capture_output=True, text=True, check=True)
        result = {"content": [], "structuredContent": {
            "server": probe(), "descendant": json.loads(child.stdout),
            "cwd": os.getcwd(), "env": os.environ["RUXEX_DESCRIPTOR_TEST"],
            "request": message["params"]["arguments"]}}
    else:
        continue
    print(json.dumps({"jsonrpc": "2.0", "id": message["id"], "result": result}), flush=True)
"#,
    )?;

    let (pipe_reader, _pipe_writer) = std::io::pipe()?;
    let (socket, _socket_peer) = UnixStream::pair()?;
    let file = tempfile::tempfile()?;
    let mut sentinels = Vec::new();
    for fd in [
        pipe_reader.as_raw_fd(),
        socket.as_raw_fd(),
        file.as_raw_fd(),
    ] {
        let duplicate = unsafe { libc::fcntl(fd, libc::F_DUPFD, 200) };
        if duplicate < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // SAFETY: fcntl returned a newly owned descriptor.
        sentinels.push(unsafe { OwnedFd::from_raw_fd(duplicate) });
    }
    let sentinel_fds: Vec<_> = sentinels.iter().map(AsRawFd::as_raw_fd).collect();
    let sentinel_args = serde_json::to_string(&sentinel_fds)?;
    let python = which::which("python3")?;
    let control = Command::new(&python)
        .arg(&script)
        .arg("probe")
        .arg(&sentinel_args)
        .output()?;
    assert!(control.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&control.stdout)?,
        json!([true, true, true])
    );

    let client = RmcpClient::new_stdio_client(
        python.into_os_string(),
        vec![
            script.clone().into_os_string(),
            "server".into(),
            sentinel_args.clone().into(),
        ],
        Some(HashMap::from([(
            OsString::from("RUXEX_DESCRIPTOR_TEST"),
            OsString::from("environment preserved"),
        )])),
        &[],
        Some(cwd.to_string_lossy().into_owned()),
        Arc::new(LocalStdioServerLauncher::new(std::env::current_dir()?)),
    )
    .await?;
    client
        .initialize(
            InitializeRequestParams::new(
                ClientCapabilities::default(),
                Implementation::new("descriptor-boundary-test", "1"),
            )
            .with_protocol_version(ProtocolVersion::V_2025_06_18),
            Some(Duration::from_secs(5)),
            Box::new(|_, _| {
                async {
                    Ok(ElicitationResponse {
                        action: ElicitationAction::Decline,
                        content: None,
                        meta: None,
                    })
                }
                .boxed()
            }),
        )
        .await?;
    let tools = client
        .list_tools(/*params*/ None, Some(Duration::from_secs(5)))
        .await?;
    assert_eq!(tools.tools.len(), 1);
    let result = client
        .call_tool(
            "probe".to_string(),
            Some(json!({"message": "stdio round trip"})),
            /*meta*/ None,
            Some(Duration::from_secs(5)),
        )
        .await?;
    client.shutdown().await;
    assert_eq!(
        result.structured_content,
        Some(json!({
            "server": [false, false, false],
            "descendant": [false, false, false],
            "cwd": cwd,
            "env": "environment preserved",
            "request": {"message": "stdio round trip"}
        }))
    );

    let flags: Vec<_> = sentinel_fds
        .iter()
        .map(|fd| unsafe { libc::fcntl(*fd, libc::F_GETFD) })
        .collect();
    assert_eq!(flags, vec![0, 0, 0]);
    Ok(())
}

#[tokio::test]
async fn local_stdio_preserves_exec_failure_reporting() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let script = temporary.path().join("missing-interpreter");
    let interpreter = temporary.path().join("does-not-exist");
    fs::write(&script, format!("#!{}\n", interpreter.display()))?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;

    let result = RmcpClient::new_stdio_client(
        script.into_os_string(),
        Vec::new(),
        /*env*/ None,
        &[],
        /*cwd*/ None,
        Arc::new(LocalStdioServerLauncher::new(std::env::current_dir()?)),
    )
    .await;
    assert!(
        result.is_err(),
        "exec failure was reported as a successful launch"
    );
    Ok(())
}

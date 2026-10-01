use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

use genex_exec_server_protocol::JSONRPCMessage;
use genex_exec_server_protocol::JSONRPCRequest;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::process::Child;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::timeout;
use tracing::debug;
use tracing::warn;

use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::io::BufWriter;

pub(crate) const CHANNEL_CAPACITY: usize = 128;
const MAX_STDIO_JSONRPC_MESSAGE_LEN: usize = 64 * 1024 * 1024;
const STDIO_TERMINATION_GRACE_PERIOD: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub(crate) enum JsonRpcConnectionEvent {
    Message(JSONRPCMessage),
    QueuedRequest {
        request: JSONRPCRequest,
        request_span: tracing::Span,
        queued_at: Instant,
    },
    MalformedMessage {
        reason: String,
    },
    Disconnected {
        reason: Option<String>,
    },
}

impl JsonRpcConnectionEvent {
    pub(crate) fn message(message: JSONRPCMessage) -> Self {
        let JSONRPCMessage::Request(request) = message else {
            return Self::Message(message);
        };

        let queued_at = Instant::now();
        let request_span = tracing::info_span!(
            "codex.exec_server.request",
            otel.kind = "server",
            otel.name = "unknown",
            method = request.method.as_str(),
            result = tracing::field::Empty,
        );
        if let Some(trace) = &request.trace
            && !genex_otel::set_parent_from_w3c_trace_context(&request_span, trace)
        {
            warn!(
                method = request.method.as_str(),
                "ignoring invalid inbound exec-server trace carrier"
            );
        }

        Self::QueuedRequest {
            request,
            request_span,
            queued_at,
        }
    }
}

#[derive(Clone)]
pub(crate) enum JsonRpcTransport {
    // Plain means no child process; transport bytes may still be encrypted.
    Plain,
    Stdio { transport: StdioTransport },
}

impl JsonRpcTransport {
    fn from_child_process(child_process: Child) -> Self {
        Self::Stdio {
            transport: StdioTransport::spawn(child_process),
        }
    }

    pub(crate) fn terminate(&self) {
        match self {
            Self::Plain => {}
            Self::Stdio { transport } => transport.terminate(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct StdioTransport {
    handle: Arc<StdioTransportHandle>,
}

struct StdioTransportHandle {
    terminate_tx: watch::Sender<bool>,
    terminate_requested: AtomicBool,
}

impl StdioTransport {
    fn spawn(child_process: Child) -> Self {
        let (terminate_tx, terminate_rx) = watch::channel(false);
        let handle = Arc::new(StdioTransportHandle {
            terminate_tx,
            terminate_requested: AtomicBool::new(false),
        });
        spawn_stdio_child_supervisor(child_process, terminate_rx);
        Self { handle }
    }

    fn terminate(&self) {
        self.handle.terminate();
    }
}

impl StdioTransportHandle {
    fn terminate(&self) {
        if !self.terminate_requested.swap(true, Ordering::AcqRel) {
            let _ = self.terminate_tx.send(true);
        }
    }
}

impl Drop for StdioTransportHandle {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn spawn_stdio_child_supervisor(mut child_process: Child, mut terminate_rx: watch::Receiver<bool>) {
    let process_group_id = child_process.id();
    tokio::spawn(async move {
        tokio::select! {
            result = child_process.wait() => {
                log_stdio_child_wait_result(result);
                kill_process_tree(&mut child_process, process_group_id);
            }
            () = wait_for_stdio_termination(&mut terminate_rx) => {
                terminate_stdio_child(&mut child_process, process_group_id).await;
            }
        }
    });
}

async fn wait_for_stdio_termination(terminate_rx: &mut watch::Receiver<bool>) {
    loop {
        if *terminate_rx.borrow() {
            return;
        }
        if terminate_rx.changed().await.is_err() {
            return;
        }
    }
}

async fn terminate_stdio_child(child_process: &mut Child, process_group_id: Option<u32>) {
    terminate_process_tree(child_process, process_group_id);
    match timeout(STDIO_TERMINATION_GRACE_PERIOD, child_process.wait()).await {
        Ok(result) => {
            log_stdio_child_wait_result(result);
        }
        Err(_) => {
            kill_process_tree(child_process, process_group_id);
            log_stdio_child_wait_result(child_process.wait().await);
        }
    }
}

fn terminate_process_tree(child_process: &mut Child, process_group_id: Option<u32>) {
    let Some(process_group_id) = process_group_id else {
        kill_direct_child(child_process, "terminate");
        return;
    };

    #[cfg(unix)]
    if let Err(err) = genex_utils_pty::process_group::terminate_process_group(process_group_id) {
        warn!("failed to terminate exec-server stdio process group {process_group_id}: {err}");
        kill_direct_child(child_process, "terminate");
    }

    #[cfg(windows)]
    if !kill_windows_process_tree(process_group_id) {
        kill_direct_child(child_process, "terminate");
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = process_group_id;
        kill_direct_child(child_process, "terminate");
    }
}

fn kill_process_tree(child_process: &mut Child, process_group_id: Option<u32>) {
    let Some(process_group_id) = process_group_id else {
        kill_direct_child(child_process, "kill");
        return;
    };

    #[cfg(unix)]
    if let Err(err) = genex_utils_pty::process_group::kill_process_group(process_group_id) {
        warn!("failed to kill exec-server stdio process group {process_group_id}: {err}");
    }

    #[cfg(windows)]
    if !kill_windows_process_tree(process_group_id) {
        kill_direct_child(child_process, "kill");
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = process_group_id;
        kill_direct_child(child_process, "kill");
    }
}

fn kill_direct_child(child_process: &mut Child, action: &str) {
    if let Err(err) = child_process.start_kill() {
        debug!("failed to {action} exec-server stdio child: {err}");
    }
}

#[cfg(windows)]
fn kill_windows_process_tree(pid: u32) -> bool {
    let pid = pid.to_string();
    match std::process::Command::new("taskkill")
        .args(["/PID", pid.as_str(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => status.success(),
        Err(err) => {
            warn!("failed to run taskkill for exec-server stdio process tree {pid}: {err}");
            false
        }
    }
}

fn log_stdio_child_wait_result(result: std::io::Result<std::process::ExitStatus>) {
    if let Err(err) = result {
        debug!("failed to wait for exec-server stdio child: {err}");
    }
}

pub(crate) struct JsonRpcConnection {
    pub(crate) outgoing_tx: mpsc::Sender<JSONRPCMessage>,
    pub(crate) incoming_rx: mpsc::Receiver<JsonRpcConnectionEvent>,
    pub(crate) disconnected_rx: watch::Receiver<bool>,
    pub(crate) task_handles: Vec<tokio::task::JoinHandle<()>>,
    pub(crate) transport: JsonRpcTransport,
}

impl JsonRpcConnection {
    pub(crate) fn from_stdio<R, W>(reader: R, writer: W, connection_label: String) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        Self::from_stdio_with_max_message_len(
            reader,
            writer,
            connection_label,
            MAX_STDIO_JSONRPC_MESSAGE_LEN,
        )
    }

    fn from_stdio_with_max_message_len<R, W>(
        reader: R,
        writer: W,
        connection_label: String,
        max_message_len: usize,
    ) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel(CHANNEL_CAPACITY);
        let (incoming_tx, incoming_rx) = mpsc::channel(CHANNEL_CAPACITY);
        let (disconnected_tx, disconnected_rx) = watch::channel(false);

        let reader_label = connection_label.clone();
        let incoming_tx_for_reader = incoming_tx.clone();
        let disconnected_tx_for_reader = disconnected_tx.clone();
        // Read one byte past the payload limit so an unterminated oversized
        // message fails promptly. A trailing CR gets one more byte of lookahead
        // because it may be the first half of a valid CRLF terminator.
        let read_limit = u64::try_from(max_message_len.saturating_add(1)).unwrap_or(u64::MAX);
        let reader_task = tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                let read_result = (&mut reader).take(read_limit).read_line(&mut line).await;
                match read_result {
                    Ok(0) => {
                        send_disconnected(
                            &incoming_tx_for_reader,
                            &disconnected_tx_for_reader,
                            /*reason*/ None,
                        )
                        .await;
                        break;
                    }
                    Ok(_) => {
                        if line.ends_with('\n') {
                            line.pop();
                            if line.ends_with('\r') {
                                line.pop();
                            }
                        } else if line.len() > max_message_len && line.ends_with('\r') {
                            match reader.read_u8().await {
                                Ok(b'\n') => {
                                    line.pop();
                                }
                                Ok(_) => {}
                                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {}
                                Err(err) => {
                                    send_disconnected(
                                        &incoming_tx_for_reader,
                                        &disconnected_tx_for_reader,
                                        Some(format!(
                                            "failed to read JSON-RPC message from {reader_label}: {err}"
                                        )),
                                    )
                                    .await;
                                    break;
                                }
                            }
                        }
                        if line.len() > max_message_len {
                            send_disconnected(
                                &incoming_tx_for_reader,
                                &disconnected_tx_for_reader,
                                Some(format!(
                                    "JSON-RPC message from {reader_label} exceeds maximum length of {max_message_len} bytes"
                                )),
                            )
                            .await;
                            break;
                        }
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<JSONRPCMessage>(&line) {
                            Ok(message) => {
                                if incoming_tx_for_reader
                                    .send(JsonRpcConnectionEvent::message(message))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(err) => {
                                send_malformed_message(
                                    &incoming_tx_for_reader,
                                    Some(format!(
                                        "failed to parse JSON-RPC message from {reader_label}: {err}"
                                    )),
                                )
                                .await;
                            }
                        }
                    }
                    Err(err) => {
                        send_disconnected(
                            &incoming_tx_for_reader,
                            &disconnected_tx_for_reader,
                            Some(format!(
                                "failed to read JSON-RPC message from {reader_label}: {err}"
                            )),
                        )
                        .await;
                        break;
                    }
                }
            }
        });

        let writer_task = tokio::spawn(async move {
            let mut writer = BufWriter::new(writer);
            while let Some(message) = outgoing_rx.recv().await {
                if let Err(err) = write_jsonrpc_line_message(&mut writer, &message).await {
                    send_disconnected(
                        &incoming_tx,
                        &disconnected_tx,
                        Some(format!(
                            "failed to write JSON-RPC message to {connection_label}: {err}"
                        )),
                    )
                    .await;
                    break;
                }
            }
        });

        Self {
            outgoing_tx,
            incoming_rx,
            disconnected_rx,
            task_handles: vec![reader_task, writer_task],
            transport: JsonRpcTransport::Plain,
        }
    }

    pub(crate) fn with_child_process(mut self, child_process: Child) -> Self {
        self.transport = JsonRpcTransport::from_child_process(child_process);
        self
    }
}

async fn send_disconnected(
    incoming_tx: &mpsc::Sender<JsonRpcConnectionEvent>,
    disconnected_tx: &watch::Sender<bool>,
    reason: Option<String>,
) {
    let _ = disconnected_tx.send(true);
    let _ = incoming_tx
        .send(JsonRpcConnectionEvent::Disconnected { reason })
        .await;
}

async fn send_malformed_message(
    incoming_tx: &mpsc::Sender<JsonRpcConnectionEvent>,
    reason: Option<String>,
) {
    let _ = incoming_tx
        .send(JsonRpcConnectionEvent::MalformedMessage {
            reason: reason.unwrap_or_else(|| "malformed JSON-RPC message".to_string()),
        })
        .await;
}

async fn write_jsonrpc_line_message<W>(
    writer: &mut BufWriter<W>,
    message: &JSONRPCMessage,
) -> std::io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let encoded =
        serialize_jsonrpc_message(message).map_err(|err| std::io::Error::other(err.to_string()))?;
    writer.write_all(encoded.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await
}

fn serialize_jsonrpc_message(message: &JSONRPCMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(message)
}

#[cfg(test)]
mod tests {
    use genex_exec_server_protocol::JSONRPCRequest;
    use genex_exec_server_protocol::RequestId;
    use pretty_assertions::assert_eq;
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    async fn stdio_connection_accepts_message_at_size_limit() -> anyhow::Result<()> {
        let message = test_jsonrpc_message();
        let encoded = serde_json::to_string(&message)?;
        let max_message_len = encoded.len();

        for line_ending in [b"\n".as_slice(), b"\r\n".as_slice()] {
            let (reader, mut peer) =
                tokio::io::duplex(max_message_len.saturating_add(line_ending.len()));
            let mut connection = JsonRpcConnection::from_stdio_with_max_message_len(
                reader,
                tokio::io::sink(),
                "test stdio peer".to_string(),
                max_message_len,
            );

            peer.write_all(encoded.as_bytes()).await?;
            peer.write_all(line_ending).await?;
            let event = timeout(Duration::from_secs(1), connection.incoming_rx.recv())
                .await?
                .expect("stdio connection should report the message");
            match event {
                JsonRpcConnectionEvent::QueuedRequest { request, .. } => {
                    assert_eq!(JSONRPCMessage::Request(request), message)
                }
                event => anyhow::bail!("expected JSON-RPC message, got {event:?}"),
            }

            drop(peer);
            drop(connection);
        }

        Ok(())
    }

    #[tokio::test]
    async fn stdio_connection_rejects_overlong_unterminated_message() -> anyhow::Result<()> {
        let max_message_len: usize = 32;
        let (reader, mut peer) = tokio::io::duplex(max_message_len.saturating_add(1));
        let mut connection = JsonRpcConnection::from_stdio_with_max_message_len(
            reader,
            tokio::io::sink(),
            "hostile stdio peer".to_string(),
            max_message_len,
        );
        let overlong_message = vec![b'x'; max_message_len + 1];

        peer.write_all(&overlong_message).await?;
        let event = timeout(Duration::from_secs(1), connection.incoming_rx.recv())
            .await?
            .expect("stdio connection should report the framing violation");
        match event {
            JsonRpcConnectionEvent::Disconnected { reason } => assert_eq!(
                reason,
                Some(
                    "JSON-RPC message from hostile stdio peer exceeds maximum length of 32 bytes"
                        .to_string()
                )
            ),
            event => anyhow::bail!("expected stdio disconnect, got {event:?}"),
        }

        drop(peer);
        drop(connection);
        Ok(())
    }

    fn test_jsonrpc_message() -> JSONRPCMessage {
        JSONRPCMessage::Request(JSONRPCRequest {
            id: RequestId::Integer(1),
            method: "test".to_string(),
            params: None,
            trace: None,
        })
    }
}

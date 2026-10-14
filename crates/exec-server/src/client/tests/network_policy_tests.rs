use std::sync::Arc;
use std::time::Duration;

use ruxex_exec_server_protocol::JSONRPCMessage;
use ruxex_exec_server_protocol::JSONRPCRequest;
use ruxex_exec_server_protocol::JSONRPCResponse;
use ruxex_exec_server_protocol::RequestId;
use ruxex_network_proxy::NetworkDecision;
use ruxex_network_proxy::NetworkPolicyDecider;
use ruxex_network_proxy::NetworkPolicyRequest;
use ruxex_utils_path_uri::PathUri;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::InMemorySpanExporter;
use opentelemetry_sdk::trace::SdkTracerProvider;
use pretty_assertions::assert_eq;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::io::BufWriter;
use tokio::io::ReadHalf;
use tokio::io::WriteHalf;
use tokio::io::duplex;
use tokio::io::split;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::prelude::*;

use super::super::ExecServerClient;
use super::super::ExecServerClientConnectOptions;
use super::super::NetworkPolicyDecisionController;
use super::super::SessionState;
use crate::ProcessId;
use crate::connection::JsonRpcConnection;
use crate::protocol::EXEC_METHOD;
use crate::protocol::EXEC_TERMINATE_METHOD;
use crate::protocol::ExecParams;
use crate::protocol::ExecServerNetworkPolicyDecision;
use crate::protocol::ExecServerNetworkPolicyRequest;
use crate::protocol::ExecServerNetworkProtocol;
use crate::protocol::NETWORK_POLICY_DECISION_METHOD;
use crate::protocol::NETWORK_POLICY_REQUEST_METHOD;
use crate::protocol::NetworkPolicyDecisionNotification;
use crate::protocol::NetworkPolicyRequestParams;
use crate::protocol::NetworkPolicyRequestResponse;
use crate::rpc_server_requests::MAX_IN_FLIGHT_SERVER_CALLS;

struct PendingDecisionGuard(mpsc::UnboundedSender<()>);

impl Drop for PendingDecisionGuard {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

fn policy_request(request_id: i64, process_id: ProcessId, host: &str) -> JSONRPCMessage {
    JSONRPCMessage::Request(JSONRPCRequest {
        id: RequestId::Integer(request_id),
        method: NETWORK_POLICY_REQUEST_METHOD.to_string(),
        params: Some(
            serde_json::to_value(NetworkPolicyRequestParams {
                process_id,
                request: ExecServerNetworkPolicyRequest {
                    protocol: ExecServerNetworkProtocol::HttpsConnect,
                    host: host.to_string(),
                    port: 443,
                },
            })
            .expect("policy request should serialize"),
        ),
        trace: None,
    })
}

struct StdioPeer {
    reader: tokio::io::Lines<BufReader<ReadHalf<tokio::io::DuplexStream>>>,
    writer: BufWriter<WriteHalf<tokio::io::DuplexStream>>,
}

fn test_transport() -> (JsonRpcConnection, StdioPeer) {
    let (client, server) = duplex(1 << 20);
    let (client_reader, client_writer) = split(client);
    let (server_reader, server_writer) = split(server);
    (
        JsonRpcConnection::from_stdio(
            client_reader,
            client_writer,
            "network-policy-test".to_string(),
        ),
        StdioPeer {
            reader: BufReader::new(server_reader).lines(),
            writer: BufWriter::new(server_writer),
        },
    )
}

async fn read_decision(peer: &mut StdioPeer, request_id: i64) -> ExecServerNetworkPolicyDecision {
    let JSONRPCMessage::Response(response) = read_jsonrpc_stdio(peer).await else {
        panic!("expected network policy response");
    };
    assert_eq!(response.id, RequestId::Integer(request_id));
    serde_json::from_value::<NetworkPolicyRequestResponse>(response.result)
        .expect("policy response should deserialize")
        .decision
}

async fn complete_stdio_initialize(peer: &mut StdioPeer, client_name: &str) {
    let JSONRPCMessage::Request(request) = read_jsonrpc_stdio(peer).await else {
        panic!("expected initialize request");
    };
    assert_eq!(request.method, "initialize");
    let params: crate::protocol::InitializeParams =
        serde_json::from_value(request.params.expect("initialize params")).expect("params");
    assert_eq!(params.client_name, client_name);
    write_jsonrpc_stdio(
        peer,
        JSONRPCMessage::Response(JSONRPCResponse {
            id: request.id,
            result: serde_json::json!({"sessionId":"network-policy-session"}),
        }),
    )
    .await;
    let JSONRPCMessage::Notification(notification) = read_jsonrpc_stdio(peer).await else {
        panic!("expected initialized notification");
    };
    assert_eq!(notification.method, "initialized");
}

async fn read_jsonrpc_stdio(peer: &mut StdioPeer) -> JSONRPCMessage {
    let line = peer
        .reader
        .next_line()
        .await
        .expect("stdio read should succeed")
        .expect("stdio should remain open");
    serde_json::from_str(&line).expect("JSON-RPC message should decode")
}

async fn write_jsonrpc_stdio(peer: &mut StdioPeer, message: JSONRPCMessage) {
    let encoded = serde_json::to_vec(&message).expect("JSON-RPC message should encode");
    peer.writer
        .write_all(&encoded)
        .await
        .expect("stdio write should succeed");
    peer.writer
        .write_all(b"\n")
        .await
        .expect("stdio newline should succeed");
    peer.writer
        .flush()
        .await
        .expect("stdio flush should succeed");
}

#[tokio::test]
async fn abandoned_process_start_unregisters_and_cleans_up() {
    let (connection, mut peer) = test_transport();
    let (start_seen_tx, start_seen_rx) = oneshot::channel();
    let (finish_start_tx, finish_start_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        complete_stdio_initialize(&mut peer, "ruxex-core").await;
        let JSONRPCMessage::Request(start) = read_jsonrpc_stdio(&mut peer).await else {
            panic!("expected process start request");
        };
        assert_eq!(start.method, EXEC_METHOD);
        start_seen_tx.send(()).expect("start should be observed");
        finish_start_rx.await.expect("start should be released");
        write_jsonrpc_stdio(
            &mut peer,
            JSONRPCMessage::Response(JSONRPCResponse {
                id: start.id,
                result: serde_json::json!({"processId": "pending-start"}),
            }),
        )
        .await;
        let JSONRPCMessage::Request(terminate) = read_jsonrpc_stdio(&mut peer).await else {
            panic!("expected process terminate request");
        };
        assert_eq!(terminate.method, EXEC_TERMINATE_METHOD);
        write_jsonrpc_stdio(
            &mut peer,
            JSONRPCMessage::Response(JSONRPCResponse {
                id: terminate.id,
                result: serde_json::json!({"running": true}),
            }),
        )
        .await;
    });

    let client = ExecServerClient::connect(connection, Default::default())
        .await
        .expect("client should connect");
    let process_id = ProcessId::from("pending-start");
    let start_client = client.clone();
    let start_process_id = process_id.clone();
    let start = tokio::spawn(async move {
        let params = ExecParams {
            metadata: Default::default(),
            process_id: start_process_id,
            argv: vec!["true".to_string()],
            cwd: PathUri::from_host_native_path(std::env::current_dir().expect("cwd"))
                .expect("cwd URI"),
            shell_snapshot: None,
            env_policy: None,
            env: Default::default(),
            tty: false,
            pipe_stdin: false,
            arg0: None,
            sandbox: None,
            enforce_managed_network: false,
            managed_network: None,
            network_proxy: None,
        };
        start_client
            .start_process(params, /*network_policy_decider*/ None)
            .await
    });
    start_seen_rx.await.expect("start should be observed");
    let state = client
        .inner
        .get_session(&process_id)
        .expect("pending process should be registered");
    let decider: Arc<dyn NetworkPolicyDecider> =
        Arc::new(|_request: NetworkPolicyRequest| async { NetworkDecision::Allow });
    let decider_weak = Arc::downgrade(&decider);
    state
        .network_policy
        .controller
        .store(Some(Arc::new(NetworkPolicyDecisionController {
            decider,
            timeout: Duration::from_secs(30),
        })));

    start.abort();
    assert!(start.await.is_err_and(|error| error.is_cancelled()));
    assert!(state.network_policy.cancelled.is_cancelled());
    assert!(client.inner.get_session(&process_id).is_none());
    assert!(decider_weak.upgrade().is_none());

    finish_start_tx.send(()).expect("start should be released");
    server.await.expect("server task should finish");
}

#[tokio::test]
async fn policy_requests_use_process_decider_and_cancel_on_unregister() {
    let span_exporter = InMemorySpanExporter::default();
    let tracer_provider = SdkTracerProvider::builder()
        .with_simple_exporter(span_exporter.clone())
        .build();
    let subscriber = tracing_subscriber::registry().with(
        tracing_opentelemetry::layer()
            .with_tracer(tracer_provider.tracer("exec-server-test"))
            .with_filter(filter_fn(ruxex_otel::OtelProvider::trace_export_filter)),
    );
    let _subscriber = tracing::subscriber::set_default(subscriber);
    tracing::callsite::rebuild_interest_cache();

    let (connection, mut peer) = test_transport();
    let process_id = ProcessId::from("policy-process");
    let server_process_id = process_id.clone();
    let (ready_tx, ready_rx) = oneshot::channel();
    let (overflow_checked_tx, overflow_checked_rx) = oneshot::channel();
    let (unregistered_tx, unregistered_rx) = oneshot::channel();
    let server = tokio::spawn(async move {
        complete_stdio_initialize(&mut peer, "ruxex-core").await;
        ready_rx.await.expect("process should be registered");

        for (request_id, host, expected) in [
            (0, "allowed.example", ExecServerNetworkPolicyDecision::Allow),
            (
                2,
                "denied.example",
                ExecServerNetworkPolicyDecision::Deny {
                    reason: "blocked".to_string(),
                },
            ),
            (
                3,
                "invalid host",
                ExecServerNetworkPolicyDecision::Deny {
                    reason: "not_allowed".to_string(),
                },
            ),
        ] {
            write_jsonrpc_stdio(
                &mut peer,
                policy_request(request_id, server_process_id.clone(), host),
            )
            .await;
            assert_eq!(read_decision(&mut peer, request_id).await, expected);
        }

        let first_pending_request_id = 100;
        for offset in 0..MAX_IN_FLIGHT_SERVER_CALLS {
            write_jsonrpc_stdio(
                &mut peer,
                policy_request(
                    first_pending_request_id + offset as i64,
                    server_process_id.clone(),
                    "pending.example",
                ),
            )
            .await;
        }
        let overflow_request_id = first_pending_request_id + MAX_IN_FLIGHT_SERVER_CALLS as i64;
        write_jsonrpc_stdio(
            &mut peer,
            policy_request(
                overflow_request_id,
                server_process_id.clone(),
                "pending.example",
            ),
        )
        .await;
        assert_eq!(
            read_decision(&mut peer, overflow_request_id).await,
            ExecServerNetworkPolicyDecision::Deny {
                reason: "not_allowed".to_string(),
            }
        );
        overflow_checked_tx.send(()).expect("overflow observed");

        unregistered_rx
            .await
            .expect("process should be unregistered");

        let post_unregister_request_id = 900;
        write_jsonrpc_stdio(
            &mut peer,
            policy_request(
                post_unregister_request_id,
                server_process_id,
                "allowed.example",
            ),
        )
        .await;
        assert_eq!(
            read_decision(&mut peer, post_unregister_request_id).await,
            ExecServerNetworkPolicyDecision::Deny {
                reason: "not_allowed".to_string(),
            }
        );
    });

    let client = ExecServerClient::connect(connection, Default::default())
        .await
        .expect("client should connect");
    let session = client
        .register_session(&process_id)
        .await
        .expect("session should register");
    let (started_tx, mut started_rx) = mpsc::unbounded_channel();
    let (dropped_tx, mut dropped_rx) = mpsc::unbounded_channel();
    let decider: Arc<dyn NetworkPolicyDecider> = Arc::new(move |request: NetworkPolicyRequest| {
        let started_tx = started_tx.clone();
        let dropped_tx = dropped_tx.clone();
        async move {
            assert_eq!(
                tracing::Span::current()
                    .metadata()
                    .map(tracing::Metadata::name),
                Some("ruxex.exec_server.request"),
                "network policy decisions must run inside the inbound request span"
            );
            match request.host.as_str() {
                "allowed.example" => NetworkDecision::Allow,
                "denied.example" => NetworkDecision::deny("blocked"),
                "pending.example" => {
                    started_tx.send(()).expect("decision should start");
                    let _drop_guard = PendingDecisionGuard(dropped_tx);
                    std::future::pending().await
                }
                host => panic!("unexpected policy host: {host}"),
            }
        }
    });
    session.state.network_policy.controller.store(Some(Arc::new(
        NetworkPolicyDecisionController {
            decider,
            timeout: Duration::from_secs(30),
        },
    )));
    ready_tx.send(()).expect("server should be waiting");
    timeout(Duration::from_secs(5), async {
        for _ in 0..MAX_IN_FLIGHT_SERVER_CALLS {
            started_rx
                .recv()
                .await
                .expect("pending decision should start");
        }
    })
    .await
    .expect("pending decisions should start");
    overflow_checked_rx
        .await
        .expect("overflow should be observed");
    session.unregister().await;
    timeout(Duration::from_secs(5), async {
        for _ in 0..MAX_IN_FLIGHT_SERVER_CALLS {
            dropped_rx
                .recv()
                .await
                .expect("unregistered decision should be dropped");
        }
    })
    .await
    .expect("unregistered decisions should be cancelled");
    unregistered_tx
        .send(())
        .expect("server should verify late responses");
    timeout(Duration::from_secs(2), server)
        .await
        .expect("policy routing should finish")
        .expect("server task should finish");

    tracer_provider.force_flush().expect("flush traces");
    let spans = span_exporter.get_finished_spans().expect("span export");
    let policy_spans = spans
        .iter()
        .filter(|span| span.name.as_ref() == NETWORK_POLICY_REQUEST_METHOD)
        .collect::<Vec<_>>();
    assert!(
        !policy_spans.is_empty(),
        "network policy requests should export server spans"
    );
    let outcomes = policy_spans
        .iter()
        .map(|span| {
            span.attributes
                .iter()
                .find(|attribute| attribute.key.as_str() == "result")
                .map(|attribute| attribute.value.as_str().into_owned())
        })
        .collect::<Vec<_>>();
    assert!(
        outcomes.iter().all(Option::is_some),
        "completed, rejected, and cancelled policy requests must all record an outcome"
    );
    assert!(
        outcomes
            .iter()
            .any(|outcome| outcome.as_deref() == Some("success")),
        "completed and capacity-rejected requests should record successful responses"
    );
    assert!(
        outcomes
            .iter()
            .any(|outcome| outcome.as_deref() == Some("disconnected")),
        "cancelled requests should record disconnection"
    );
}

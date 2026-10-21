use ruxex_app_server_mock::MockAppServer;
use ruxex_app_server_protocol::{
    JSONRPCError, JSONRPCErrorError, JSONRPCMessage, JSONRPCNotification, JSONRPCRequest,
    JSONRPCResponse, RequestId,
};
use serde_json::json;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

fn request(id: i64, method: &str) -> JSONRPCMessage {
    JSONRPCMessage::Request(JSONRPCRequest {
        id: RequestId::Integer(id),
        method: method.to_owned(),
        params: None,
        trace: None,
    })
}

#[test]
fn default_request_returns_deterministic_mock_payload() {
    let server = MockAppServer::builder().build().expect("build mock server");

    let response = server.handle_message(request(7, "thread/start"));

    assert_eq!(
        response,
        Some(JSONRPCMessage::Response(JSONRPCResponse {
            id: RequestId::Integer(7),
            result: json!({"mock": true, "method": "thread/start"}),
        }))
    );
}

#[test]
fn configured_method_returns_fixture_unchanged() {
    let server = MockAppServer::builder()
        .with_response("thread/start", json!({"threadId": "mock-thread"}))
        .build()
        .expect("build mock server");

    let response = server.handle_message(request(8, "thread/start"));

    assert_eq!(
        response,
        Some(JSONRPCMessage::Response(JSONRPCResponse {
            id: RequestId::Integer(8),
            result: json!({"threadId": "mock-thread"}),
        }))
    );
}

#[test]
fn initialize_returns_a_valid_mock_identity_payload() {
    let server = MockAppServer::builder().build().expect("build mock server");

    let Some(JSONRPCMessage::Response(response)) = server.handle_message(request(9, "initialize"))
    else {
        panic!("initialize must return a response");
    };

    assert_eq!(response.id, RequestId::Integer(9));
    assert_eq!(response.result["productName"], "Ruxex");
    assert_eq!(response.result["productSlug"], "ruxex");
    assert_eq!(
        response.result["appServerIdentity"]["signature"],
        "ruxex-app-server-mock"
    );
}

#[test]
fn notifications_do_not_produce_responses() {
    let server = MockAppServer::builder().build().expect("build mock server");

    assert_eq!(
        server.handle_message(JSONRPCMessage::Notification(JSONRPCNotification {
            method: "thread/started".to_owned(),
            params: None,
        })),
        None
    );
}

#[test]
fn incoming_responses_and_errors_do_not_produce_responses() {
    let server = MockAppServer::builder().build().expect("build mock server");

    assert_eq!(
        server.handle_message(JSONRPCMessage::Response(JSONRPCResponse {
            id: RequestId::Integer(10),
            result: json!({"already": "handled"}),
        })),
        None
    );
    assert_eq!(
        server.handle_message(JSONRPCMessage::Error(JSONRPCError {
            id: RequestId::Integer(11),
            error: JSONRPCErrorError {
                code: -32000,
                data: None,
                message: "already handled".to_owned(),
            },
        })),
        None
    );
}

#[test]
fn binary_serves_one_json_rpc_response_per_stdio_request() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ruxex-app-server-mock"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn mock app server");

    let mut stdin = child.stdin.take().expect("mock stdin");
    writeln!(stdin, r#"{{"id":12,"method":"thread/start"}}"#).expect("write request");
    drop(stdin);

    let mut stdout = String::new();
    child
        .stdout
        .take()
        .expect("mock stdout")
        .read_to_string(&mut stdout)
        .expect("read response");
    let status = child.wait().expect("wait for mock app server");

    assert!(status.success());
    assert_eq!(stdout.lines().count(), 1);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).expect("valid response"),
        json!({
            "id": 12,
            "result": {"mock": true, "method": "thread/start"}
        })
    );
}

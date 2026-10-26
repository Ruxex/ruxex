use super::*;
use anyhow::Result;
use pretty_assertions::assert_eq;
use ruxex_protocol::account::ProviderAuthStatus;
use ruxex_protocol::protocol::TurnAbortReason;
use serde_json::json;
use std::path::PathBuf;

#[test]
fn client_response_payload_serializes_without_an_intermediate_json_value() -> Result<()> {
    let payload = ClientResponsePayload::ThreadArchive(v2::ThreadArchiveResponse {});
    assert_eq!(serde_json::to_string(&payload)?, "{}");
    let Some(ClientResponse::ThreadArchive {
        request_id,
        response: _,
    }) = payload.into_client_response(RequestId::Integer(7))
    else {
        panic!("expected thread/archive client response");
    };
    assert_eq!(request_id, RequestId::Integer(7));
    Ok(())
}

#[test]
fn interrupt_conversation_payload_stays_jsonrpc_only() -> Result<()> {
    let request_id = RequestId::String("interrupt-7".into());
    let payload = ClientResponsePayload::InterruptConversation(v1::InterruptConversationResponse {
        abort_reason: TurnAbortReason::Interrupted,
    });
    assert_eq!(
        serde_json::to_value(&payload)?,
        json!({
            "abortReason": "interrupted",
        })
    );
    assert!(
        payload
            .clone()
            .into_client_response(request_id.clone())
            .is_none()
    );
    let expected = (request_id.clone(), json!({ "abortReason": "interrupted" }));
    assert_eq!(payload.to_jsonrpc_parts(request_id.clone())?, expected);
    assert_eq!(payload.into_jsonrpc_parts(request_id)?, expected);
    Ok(())
}

#[test]
fn client_response_jsonrpc_parts_preserve_payloads_and_request_ids() -> Result<()> {
    let payloads = [
        (
            ClientResponsePayload::ThreadArchive(v2::ThreadArchiveResponse {}),
            json!({}),
        ),
        (
            ClientResponsePayload::GetAccount(v2::GetAccountResponse {
                provider_id: "test-provider".into(),
                account: Some(v2::Account::ApiKey {}),
                account_id: Some("account-7".into()),
                display_name: Some("Test account".into()),
                auth_status: ProviderAuthStatus::Authenticated,
                usage: None,
            }),
            json!({
                "providerId": "test-provider",
                "account": { "type": "apiKey" },
                "accountId": "account-7",
                "displayName": "Test account",
                "authStatus": "authenticated",
                "usage": null,
            }),
        ),
    ];

    for (payload, expected_result) in payloads {
        for request_id in [RequestId::Integer(7), RequestId::String("request-7".into())] {
            let expected = (request_id.clone(), expected_result.clone());
            let response = payload
                .clone()
                .into_client_response(request_id.clone())
                .expect("request-backed payload has a typed response");

            assert_eq!(response.into_jsonrpc_parts()?, expected);
            assert_eq!(payload.to_jsonrpc_parts(request_id.clone())?, expected);
            assert_eq!(payload.clone().into_jsonrpc_parts(request_id)?, expected);
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn client_response_jsonrpc_parts_preserve_serialization_errors() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let request_id = RequestId::Integer(7);
    let ruxex_home = ruxex_utils_absolute_path::AbsolutePathBuf::from_absolute_path(PathBuf::from(
        OsString::from_vec(vec![b'/', 0xff]),
    ))
    .expect("invalid UTF-8 is still a valid absolute path");
    let response = v1::InitializeResponse {
        user_agent: "ruxex-app-server/1.0".into(),
        product_name: "Ruxex".into(),
        product_slug: "ruxex".into(),
        ruxex_home,
        platform_family: "unix".into(),
        platform_os: "linux".into(),
        app_server_identity: v1::AppServerIdentity {
            signature: "test".into(),
            version: "app-server/1;provider/1".into(),
            build_version: None,
        },
    };
    let describe = |error: serde_json::Error| (error.classify(), error.to_string());
    let expected = describe(serde_json::to_value(&response).expect_err("path must not serialize"));
    let payload = ClientResponsePayload::Initialize(response.clone());
    let typed = ClientResponse::Initialize {
        request_id: request_id.clone(),
        response,
    };

    assert_eq!(describe(typed.into_jsonrpc_parts().unwrap_err()), expected);
    assert_eq!(
        describe(payload.to_jsonrpc_parts(request_id.clone()).unwrap_err()),
        expected
    );
    assert_eq!(
        describe(payload.into_jsonrpc_parts(request_id).unwrap_err()),
        expected
    );
}

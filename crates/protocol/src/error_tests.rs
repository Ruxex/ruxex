use super::*;
use crate::exec_output::StreamOutput;
use genex_http_client::HttpResponse;
use http::Response as RawHttpResponse;
use http::StatusCode;
use pretty_assertions::assert_eq;
use std::time::Duration;

#[test]
fn genex_err_debug_preserves_legacy_shape() {
    let actual = [
        GenexErr::Timeout,
        GenexErr::Stream("disconnected".to_string()),
        GenexErr::Stream("retry later".to_string()).with_retry_delay(Duration::from_secs(2)),
        GenexErr::InternalServerError.with_retry_delay(Duration::from_secs(3)),
    ]
    .map(|err| format!("{err:?}"));

    assert_eq!(
        actual,
        [
            "Timeout".to_string(),
            "Stream(\"disconnected\", None)".to_string(),
            "Stream(\"retry later\", Some(2s))".to_string(),
            "InternalServerError".to_string(),
        ]
    );
}

#[test]
fn retryability_preserves_error_details_distinctions() {
    let errors = [
        (GenexErr::ServerOverloaded, false),
        (
            GenexErr::new(CodexErrorDetails::RateLimitExceeded("retry later".into())),
            true,
        ),
        (
            GenexErr::RetryLimit(RetryLimitReachedError {
                status: StatusCode::TOO_MANY_REQUESTS,
                request_id: None,
            }),
            false,
        ),
        (
            GenexErr::UnexpectedStatus(UnexpectedResponseError {
                status: StatusCode::TOO_MANY_REQUESTS,
                body: String::new(),
                user_message: None,
                url: None,
                cf_ray: None,
                request_id: None,
            }),
            true,
        ),
        (
            CodexErrorDetails::ToolCollision("functions.update_plan".to_string()).into(),
            false,
        ),
        (GenexErr::InternalServerError, true),
    ];

    for (err, expected) in errors {
        assert_eq!(
            err.is_retryable(),
            expected,
            "unexpected retryability for {err:?}"
        );
    }
}

#[test]
fn server_overloaded_maps_to_protocol() {
    let err = GenexErr::ServerOverloaded;
    assert_eq!(
        err.to_codex_protocol_error(),
        CodexErrorInfo::ServerOverloaded
    );
}

#[test]
fn sandbox_denied_uses_aggregated_output_when_stderr_empty() {
    let output = ExecToolCallOutput {
        exit_code: 77,
        stdout: StreamOutput::new(String::new()),
        stderr: StreamOutput::new(String::new()),
        aggregated_output: StreamOutput::new("aggregate detail".to_string()),
        duration: Duration::from_millis(10),
        timed_out: false,
    };
    let err = GenexErr::Sandbox(SandboxErr::Denied {
        output: Box::new(output),
        network_policy_decision: None,
    });
    assert_eq!(get_error_message_ui(&err), "aggregate detail");
}

#[test]
fn sandbox_denied_reports_both_streams_when_available() {
    let output = ExecToolCallOutput {
        exit_code: 9,
        stdout: StreamOutput::new("stdout detail".to_string()),
        stderr: StreamOutput::new("stderr detail".to_string()),
        aggregated_output: StreamOutput::new(String::new()),
        duration: Duration::from_millis(10),
        timed_out: false,
    };
    let err = GenexErr::Sandbox(SandboxErr::Denied {
        output: Box::new(output),
        network_policy_decision: None,
    });
    assert_eq!(get_error_message_ui(&err), "stderr detail\nstdout detail");
}

#[test]
fn sandbox_denied_reports_stdout_when_no_stderr() {
    let output = ExecToolCallOutput {
        exit_code: 11,
        stdout: StreamOutput::new("stdout only".to_string()),
        stderr: StreamOutput::new(String::new()),
        aggregated_output: StreamOutput::new(String::new()),
        duration: Duration::from_millis(8),
        timed_out: false,
    };
    let err = GenexErr::Sandbox(SandboxErr::Denied {
        output: Box::new(output),
        network_policy_decision: None,
    });
    assert_eq!(get_error_message_ui(&err), "stdout only");
}

#[test]
fn to_error_event_handles_response_stream_failed() {
    let response = RawHttpResponse::builder()
        .status(StatusCode::TOO_MANY_REQUESTS)
        .body("")
        .unwrap();
    let source = HttpResponse::from(response)
        .error_for_status_ref()
        .unwrap_err()
        .with_url("http://example.com".parse().unwrap());
    let err = GenexErr::ResponseStreamFailed(ResponseStreamFailed {
        source,
        request_id: Some("req-123".to_string()),
    });

    let event = err.to_error_event(Some("prefix".to_string()));

    assert_eq!(
        event.message,
        "prefix: Error while reading the server response: HTTP status client error (429 Too Many Requests) for url (http://example.com/), request id: req-123"
    );
    assert_eq!(
        event.codex_error_info,
        Some(CodexErrorInfo::ResponseStreamConnectionFailed {
            http_status_code: Some(429)
        })
    );
}

#[test]
fn sandbox_denied_reports_exit_code_when_no_output_available() {
    let output = ExecToolCallOutput {
        exit_code: 13,
        stdout: StreamOutput::new(String::new()),
        stderr: StreamOutput::new(String::new()),
        aggregated_output: StreamOutput::new(String::new()),
        duration: Duration::from_millis(5),
        timed_out: false,
    };
    let err = GenexErr::Sandbox(SandboxErr::Denied {
        output: Box::new(output),
        network_policy_decision: None,
    });
    assert_eq!(
        get_error_message_ui(&err),
        "command failed inside sandbox with exit code 13"
    );
}

#[test]
fn unexpected_status_non_html_is_unchanged() {
    let err = UnexpectedResponseError {
        status: StatusCode::FORBIDDEN,
        body: "plain text error".to_string(),
        user_message: None,
        url: Some("http://example.com/plain".to_string()),
        cf_ray: None,
        request_id: None,
    };
    let status = StatusCode::FORBIDDEN.to_string();
    let url = "http://example.com/plain";
    assert_eq!(
        err.to_string(),
        format!("unexpected status {status}: plain text error, url: {url}")
    );
}

#[test]
fn unexpected_status_uses_user_message_and_preserves_response_context() {
    let err = UnexpectedResponseError {
        status: StatusCode::UNAUTHORIZED,
        body: "provider-specific response".to_string(),
        user_message: Some("Provider-specific guidance".to_string()),
        url: Some("https://example.com/v1/responses".to_string()),
        cf_ray: None,
        request_id: Some("req-provider".to_string()),
    };

    assert_eq!(
        err.to_string(),
        "Provider-specific guidance, url: https://example.com/v1/responses, request id: req-provider"
    );
}

#[test]
fn unexpected_status_prefers_error_message_when_present() {
    let err = UnexpectedResponseError {
        status: StatusCode::UNAUTHORIZED,
        body: r#"{"error":{"message":"Workspace is not authorized in this region."},"status":401}"#
            .to_string(),
        user_message: None,
        url: Some("https://provider.example/responses".to_string()),
        cf_ray: None,
        request_id: Some("req-123".to_string()),
    };
    let status = StatusCode::UNAUTHORIZED.to_string();
    assert_eq!(
        err.to_string(),
        format!(
            "unexpected status {status}: Workspace is not authorized in this region., url: https://provider.example/responses, request id: req-123"
        )
    );
}

#[test]
fn unexpected_status_truncates_long_body_with_ellipsis() {
    let long_body = "x".repeat(UNEXPECTED_RESPONSE_BODY_MAX_BYTES + 10);
    let err = UnexpectedResponseError {
        status: StatusCode::BAD_GATEWAY,
        body: long_body,
        user_message: None,
        url: Some("http://example.com/long".to_string()),
        cf_ray: None,
        request_id: Some("req-long".to_string()),
    };
    let status = StatusCode::BAD_GATEWAY.to_string();
    let expected_body = format!("{}...", "x".repeat(UNEXPECTED_RESPONSE_BODY_MAX_BYTES));
    assert_eq!(
        err.to_string(),
        format!(
            "unexpected status {status}: {expected_body}, url: http://example.com/long, request id: req-long"
        )
    );
}

#[test]
fn unexpected_status_includes_cf_ray_and_request_id() {
    let err = UnexpectedResponseError {
        status: StatusCode::UNAUTHORIZED,
        body: "plain text error".to_string(),
        user_message: None,
        url: Some("https://provider.example/responses".to_string()),
        cf_ray: Some("9c81f9f18f2fa49d-LHR".to_string()),
        request_id: Some("req-xyz".to_string()),
    };
    let status = StatusCode::UNAUTHORIZED.to_string();
    assert_eq!(
        err.to_string(),
        format!(
            "unexpected status {status}: plain text error, url: https://provider.example/responses, cf-ray: 9c81f9f18f2fa49d-LHR, request id: req-xyz"
        )
    );
}

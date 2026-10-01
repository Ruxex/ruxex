use super::session_lifecycle_requests::recorded_params;
use super::session_lifecycle_requests::start_recording_app_server;
use super::*;
use codex_app_server_protocol::MisalignmentErrorDetails;
use codex_app_server_protocol::MisalignmentSteer;
use pretty_assertions::assert_eq;

fn policy_error() -> AppServerTurnError {
    AppServerTurnError {
        message: "Chat paused".into(),
        codex_error_info: Some(AppServerCodexErrorInfo::MisalignmentPolicyViolation),
        additional_details: None,
        misalignment: Some(MisalignmentErrorDetails {
            error_type: None,
            detailed_explanation: Some(
                "The proposed action exceeded the request.\n\n".repeat(1_500),
            ),
            steer: Some(MisalignmentSteer {
                message: "Continue **only** within the requested scope.\nDo not edit files.".into(),
            }),
        }),
    }
}

fn error_notification(
    thread_id: ThreadId,
    turn_id: &str,
    error: AppServerTurnError,
) -> ServerNotification {
    ServerNotification::Error(codex_app_server_protocol::ErrorNotification {
        thread_id: thread_id.to_string(),
        turn_id: turn_id.into(),
        error,
        will_retry: false,
    })
}

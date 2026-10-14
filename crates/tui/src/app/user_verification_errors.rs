//! Safe user-facing messages for typed local verification failures.

use ruxex_app_server_client::TypedRequestError;
use ruxex_app_server_protocol::UserVerificationCancellationReason;
use ruxex_app_server_protocol::UserVerificationErrorDetails;
use ruxex_app_server_protocol::UserVerificationFailureReason;
use ruxex_app_server_protocol::UserVerificationInvalidRequestReason;
use ruxex_app_server_protocol::UserVerificationUnavailableReason;

pub(super) fn verification_error_message(error: &TypedRequestError) -> &'static str {
    let TypedRequestError::Server { source, .. } = error else {
        return "Could not complete user verification with the local Ruxex binary.";
    };
    let details = source
        .data
        .clone()
        .and_then(|data| serde_json::from_value::<UserVerificationErrorDetails>(data).ok());
    match details {
        Some(UserVerificationErrorDetails::InvalidRequest {
            reason: UserVerificationInvalidRequestReason::InvalidParams,
        }) => "The local Ruxex binary could not verify this request.",
        Some(UserVerificationErrorDetails::Unavailable {
            reason: UserVerificationUnavailableReason::CredentialMissing,
        }) => "No user-verification credential is available in the local Ruxex binary.",
        Some(UserVerificationErrorDetails::Unavailable {
            reason: UserVerificationUnavailableReason::BiometricsUnavailable,
        }) => "Biometric verification is currently unavailable on this device.",
        Some(UserVerificationErrorDetails::Unavailable {
            reason: UserVerificationUnavailableReason::ProviderUnavailable,
        }) => "User verification is unavailable for this request.",
        Some(UserVerificationErrorDetails::Cancelled {
            reason: UserVerificationCancellationReason::UserCancelled,
        }) => "User verification was cancelled.",
        Some(UserVerificationErrorDetails::Cancelled {
            reason: UserVerificationCancellationReason::Interrupted,
        }) => "User verification was interrupted.",
        Some(UserVerificationErrorDetails::Failed {
            reason: UserVerificationFailureReason::AuthenticationFailed,
        }) => "Biometric verification did not succeed. The request was cancelled.",
        Some(UserVerificationErrorDetails::Failed {
            reason: UserVerificationFailureReason::Timeout,
        }) => "User verification timed out. The request was cancelled.",
        Some(UserVerificationErrorDetails::Failed {
            reason: UserVerificationFailureReason::ProviderError,
        })
        | Some(UserVerificationErrorDetails::Failed {
            reason: UserVerificationFailureReason::ServiceError,
        })
        | None => "The local Ruxex binary could not complete user verification.",
    }
}

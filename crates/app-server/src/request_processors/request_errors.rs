use super::*;
use genex_protocol::error::GenexErrorDetails;

pub(super) fn environment_selection_error(err: GenexErr) -> JSONRPCErrorError {
    match err.details() {
        GenexErrorDetails::InvalidRequest(message) => invalid_request(message.clone()),
        _ => internal_error(format!("failed to validate environment selections: {err}")),
    }
}

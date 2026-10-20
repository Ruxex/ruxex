use super::*;

pub(super) fn config_load_error(err: &std::io::Error) -> JSONRPCErrorError {
    let mut error = invalid_request(format!("failed to load configuration: {err}"));
    error.data = None;
    error
}

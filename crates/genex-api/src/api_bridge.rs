use crate::TransportError;
use crate::error::ApiError;
use genex_protocol::error::ConnectionFailedError;
use genex_protocol::error::GenexErr;
use genex_protocol::error::GenexErrorDetails;
use genex_protocol::error::RetryLimitReachedError;
use genex_protocol::error::UnexpectedResponseError;
use genex_protocol::protocol::MisalignmentErrorDetails;
use http::HeaderMap;
use serde_json::Value;

pub fn map_api_error(err: ApiError) -> GenexErr {
    match err {
        ApiError::ContextWindowExceeded => GenexErr::ContextWindowExceeded,
        ApiError::QuotaExceeded => GenexErr::QuotaExceeded,
        ApiError::Retryable { message, delay } => {
            let error = GenexErr::Stream(message);
            match delay {
                Some(delay) => error.with_retry_delay(delay),
                None => error,
            }
        }
        ApiError::RateLimitExceeded { message, delay } => {
            let error = GenexErr::new(GenexErrorDetails::RateLimitExceeded(message));
            match delay {
                Some(delay) => error.with_retry_delay(delay),
                None => error,
            }
        }
        ApiError::Stream(msg) => GenexErr::Stream(msg),
        ApiError::ServerOverloaded => GenexErr::ServerOverloaded,
        ApiError::Api { status, message } => {
            let user_message = api_error_user_message(status, &message);
            GenexErr::UnexpectedStatus(UnexpectedResponseError {
                status,
                body: message,
                user_message,
                url: None,
                cf_ray: None,
                request_id: None,
            })
        }
        ApiError::InvalidRequest { message } => GenexErr::InvalidRequest(message),
        ApiError::CyberPolicy { message } => {
            GenexErr::new(GenexErrorDetails::CyberPolicy { message })
        }
        ApiError::MisalignmentPolicyViolation {
            message,
            misalignment,
        } => GenexErr::new(GenexErrorDetails::MisalignmentPolicyViolation {
            message,
            misalignment,
        }),
        ApiError::Transport(transport) => match transport {
            TransportError::Http {
                status,
                url,
                headers,
                body,
            } => {
                let body_text = body.unwrap_or_default();

                if status == http::StatusCode::SERVICE_UNAVAILABLE
                    && let Ok(value) = serde_json::from_str::<serde_json::Value>(&body_text)
                    && let Some(error) = value.get("error")
                {
                    match error.get("code").and_then(Value::as_str) {
                        Some("server_is_overloaded") => return GenexErr::ServerOverloaded,
                        Some("slow_down") => {
                            return GenexErr::new(GenexErrorDetails::RateLimitExceeded(
                                error
                                    .get("message")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_owned(),
                            ));
                        }
                        _ => {}
                    }
                }

                if (status == http::StatusCode::BAD_REQUEST
                    || status == http::StatusCode::FORBIDDEN)
                    && let Ok(parsed) = serde_json::from_str::<Value>(&body_text)
                    && let Some(error) = parsed.get("error")
                    && error.get("code").and_then(Value::as_str)
                        == Some(MISALIGNMENT_POLICY_VIOLATION_ERROR_CODE)
                {
                    let message = error
                        .get("message")
                        .and_then(Value::as_str)
                        .filter(|message| !message.trim().is_empty())
                        .map(str::to_string)
                        .unwrap_or_else(|| {
                            MISALIGNMENT_POLICY_VIOLATION_FALLBACK_MESSAGE.to_string()
                        });
                    return GenexErr::new(GenexErrorDetails::MisalignmentPolicyViolation {
                        message,
                        misalignment: error.get("misalignment").cloned().and_then(|details| {
                            serde_json::from_value::<MisalignmentErrorDetails>(details).ok()
                        }),
                    });
                }

                if status == http::StatusCode::BAD_REQUEST {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&body_text)
                        && let Some(error) = parsed.get("error")
                        && error.get("code").and_then(Value::as_str)
                            == Some(CYBER_POLICY_ERROR_CODE)
                    {
                        let message = error
                            .get("message")
                            .and_then(Value::as_str)
                            .filter(|message| !message.trim().is_empty())
                            .map(str::to_string)
                            .unwrap_or_else(|| CYBER_POLICY_FALLBACK_MESSAGE.to_string());
                        GenexErr::new(GenexErrorDetails::CyberPolicy { message })
                    } else if body_text
                        .contains("The image data you provided does not represent a valid image")
                    {
                        GenexErr::InvalidImageRequest()
                    } else {
                        GenexErr::InvalidRequest(body_text)
                    }
                } else if status == http::StatusCode::INTERNAL_SERVER_ERROR {
                    GenexErr::InternalServerError
                } else if status == http::StatusCode::TOO_MANY_REQUESTS {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&body_text)
                        && let Some(error) = parsed.get("error")
                    {
                        if matches!(
                            error.get("type").and_then(Value::as_str),
                            Some("insufficient_quota")
                        ) || matches!(
                            error.get("code").and_then(Value::as_str),
                            Some(
                                "insufficient_quota"
                                    | "credit_balance_exhausted"
                                    | "organization_spend_limit_exceeded"
                                    | "project_spend_limit_exceeded"
                                    | "organization_usage_limit_exceeded"
                            )
                        ) {
                            return GenexErr::QuotaExceeded;
                        }
                        let message = error
                            .get("message")
                            .and_then(Value::as_str)
                            .filter(|message| !message.trim().is_empty())
                            .unwrap_or("Rate limit exceeded")
                            .to_owned();
                        return GenexErr::new(GenexErrorDetails::RateLimitExceeded(message));
                    }

                    GenexErr::RetryLimit(RetryLimitReachedError {
                        status,
                        request_id: extract_request_tracking_id(headers.as_ref()),
                    })
                } else {
                    GenexErr::UnexpectedStatus(UnexpectedResponseError {
                        status,
                        user_message: api_error_user_message(status, &body_text),
                        body: body_text,
                        url,
                        cf_ray: extract_header(headers.as_ref(), CF_RAY_HEADER),
                        request_id: extract_request_id(headers.as_ref()),
                    })
                }
            }
            TransportError::RetryLimit => GenexErr::RetryLimit(RetryLimitReachedError {
                status: http::StatusCode::INTERNAL_SERVER_ERROR,
                request_id: None,
            }),
            TransportError::Timeout => GenexErr::RequestTimeout,
            TransportError::Connection(source) => {
                GenexErr::ConnectionFailed(ConnectionFailedError { source })
            }
            TransportError::Network(msg) | TransportError::Build(msg) => GenexErr::Stream(msg),
            error @ TransportError::ResponseTooLarge { .. } => {
                GenexErr::InvalidRequest(error.to_string())
            }
        },
        ApiError::RateLimit(msg) => GenexErr::Stream(msg),
    }
}

const REQUEST_ID_HEADER: &str = "x-request-id";
const CF_RAY_HEADER: &str = "cf-ray";
const CYBER_POLICY_ERROR_CODE: &str = "cyber_policy";
const CYBER_POLICY_FALLBACK_MESSAGE: &str =
    "This request has been flagged for possible cybersecurity risk.";
const MISALIGNMENT_POLICY_VIOLATION_ERROR_CODE: &str = "misalignment_policy_violation";
const MISALIGNMENT_POLICY_VIOLATION_FALLBACK_MESSAGE: &str =
    "This request was blocked due to a misalignment policy violation.";
const CLOUDFLARE_BLOCKED_MESSAGE: &str =
    "Access blocked by Cloudflare. This usually happens when connecting from a restricted region";

#[cfg(test)]
#[path = "api_bridge_tests.rs"]
mod tests;

fn extract_request_tracking_id(headers: Option<&HeaderMap>) -> Option<String> {
    extract_request_id(headers).or_else(|| extract_header(headers, CF_RAY_HEADER))
}

fn api_error_user_message(status: http::StatusCode, body: &str) -> Option<String> {
    if status == http::StatusCode::FORBIDDEN
        && body.contains("Cloudflare")
        && body.contains("blocked")
    {
        Some(format!("{CLOUDFLARE_BLOCKED_MESSAGE} (status {status})"))
    } else {
        None
    }
}

fn extract_request_id(headers: Option<&HeaderMap>) -> Option<String> {
    extract_header(headers, REQUEST_ID_HEADER)
}

fn extract_header(headers: Option<&HeaderMap>, name: &str) -> Option<String> {
    headers.and_then(|map| {
        map.get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    })
}

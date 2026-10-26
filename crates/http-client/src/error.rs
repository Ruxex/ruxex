//! Errors returned by the shared Ruxex HTTP transport.

use crate::client::HttpError;
use crate::retry_after::RetryAfter;
use http::HeaderMap;
use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("http {status}: {body:?}")]
    Http {
        status: StatusCode,
        url: Option<String>,
        headers: Option<HeaderMap>,
        body: Option<String>,
    },
    #[error("retry limit reached")]
    RetryLimit,
    #[error("timeout")]
    Timeout,
    #[error("connection failed: {0}")]
    Connection(#[source] HttpError),
    #[error("network error: {0}")]
    Network(String),
    #[error("request build error: {0}")]
    Build(String),
    #[error("response body exceeds the {max_bytes} byte limit")]
    ResponseTooLarge { max_bytes: usize },
}

impl TransportError {
    /// Returns server retry advice captured in an HTTP response, if present.
    pub fn retry_after(&self) -> Option<RetryAfter> {
        match self {
            Self::Http { headers, .. } => headers.as_ref().and_then(RetryAfter::from_headers),
            Self::RetryLimit
            | Self::Timeout
            | Self::Connection(_)
            | Self::Network(_)
            | Self::Build(_)
            | Self::ResponseTooLarge { .. } => None,
        }
    }
}

#[derive(Debug, Error)]
pub enum StreamError {
    #[error("stream failed: {0}")]
    Stream(String),
    #[error("timeout")]
    Timeout,
}

use std::fmt;

use http::HeaderMap;

/// Request headers returned by an external auth provider.
///
/// The provider owns credential validation, rotation, and persistence. Genex
/// keeps the resolved headers in memory and attaches them to backend requests.
/// Providers may also supply an opaque identity separately from the request
/// headers so Genex can enforce managed authentication policy without
/// prescribing a wire header name.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthHeaders {
    headers: HeaderMap,
    account_id: Option<String>,
}

impl AuthHeaders {
    pub fn new(headers: HeaderMap) -> Self {
        Self {
            headers,
            account_id: None,
        }
    }

    pub fn with_account_id(mut self, account_id: impl Into<String>) -> Self {
        self.account_id = Some(account_id.into());
        self
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn account_id(&self) -> Option<&str> {
        self.account_id.as_deref()
    }
}

impl fmt::Debug for AuthHeaders {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthHeaders")
            .field("headers", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_provider_neutral_identity_separately_from_request_headers() {
        let auth = AuthHeaders::new(HeaderMap::new()).with_account_id("workspace-123");

        assert_eq!(auth.account_id(), Some("workspace-123"));
        assert!(auth.headers().is_empty());
    }
}

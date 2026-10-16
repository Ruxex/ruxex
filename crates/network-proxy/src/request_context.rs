use rama_core::error::extra::OpaqueError;
use rama_http::request::HttpRequestParts;
use rama_http::{Request, Version};
use rama_net::address::{HostWithOptPort, HostWithPort};
use rama_net::{AuthorityInputExt, HttpVersionInputExt, Protocol, ProtocolInputExt};

/// The small subset of HTTP request context needed by the proxy policy layer.
/// Stable Rama exposes these values through scoped input traits instead of the
/// removed `rama_net::http::RequestContext` type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RequestContext {
    pub(crate) http_version: Version,
    pub(crate) protocol: Protocol,
    pub(crate) authority: HostWithOptPort,
}

impl RequestContext {
    pub(crate) fn authority_has_default_port(&self) -> bool {
        self.authority.port_u16() == self.protocol.default_port()
    }

    pub(crate) fn host_with_port(&self) -> HostWithPort {
        HostWithPort::new(
            self.authority.host.clone(),
            self.authority
                .port_u16()
                .or_else(|| self.protocol.default_port())
                .unwrap_or(80),
        )
    }
}

impl<Body> TryFrom<&Request<Body>> for RequestContext {
    type Error = OpaqueError;

    fn try_from(request: &Request<Body>) -> Result<Self, Self::Error> {
        Self::try_from_parts(request)
    }
}

impl TryFrom<&rama_http::request::Parts> for RequestContext {
    type Error = OpaqueError;

    fn try_from(parts: &rama_http::request::Parts) -> Result<Self, Self::Error> {
        let authority = parts
            .authority()
            .ok_or_else(|| OpaqueError::from_static_str("request has no authority"))?;
        let protocol = parts.protocol().cloned().unwrap_or(Protocol::HTTP);
        Ok(Self {
            http_version: parts.http_version().unwrap_or_else(|| parts.version()),
            protocol,
            authority,
        })
    }
}

impl RequestContext {
    fn try_from_parts<Body>(request: &Request<Body>) -> Result<Self, OpaqueError> {
        let authority = request
            .authority()
            .ok_or_else(|| OpaqueError::from_static_str("request has no authority"))?;
        let protocol = request.protocol().cloned().unwrap_or(Protocol::HTTP);
        Ok(Self {
            http_version: request.http_version().unwrap_or_else(|| request.version()),
            protocol,
            authority,
        })
    }
}

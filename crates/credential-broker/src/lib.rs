use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// The component that owns the credential namespace being requested.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialOwner {
    Plugin(String),
    Marketplace(String),
}

/// The operation that is requesting a credential.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialPurpose {
    MarketplaceRequest,
    PluginResource,
}

/// A scoped credential lookup. The owner and purpose are both part of the
/// authorization boundary and must not be inferred by a downstream caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialRequest {
    pub owner: CredentialOwner,
    pub purpose: CredentialPurpose,
}

impl CredentialRequest {
    pub const fn new(owner: CredentialOwner, purpose: CredentialPurpose) -> Self {
        Self { owner, purpose }
    }
}

/// Opaque, non-serializable credential material owned by the broker adapter.
///
/// This type intentionally has no `Debug`, `Serialize`, or public value
/// accessors. It can be carried across an in-process boundary without making
/// credentials part of protocol, persistence, or logging types.
#[derive(Clone)]
pub struct CredentialSnapshot {
    _opaque: Arc<dyn Any + Send + Sync>,
}

impl CredentialSnapshot {
    pub fn from_opaque<T>(value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        Self {
            _opaque: Arc::new(value),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialError {
    Unsupported,
    Denied,
    Unavailable,
}

pub type CredentialResult = Result<Option<CredentialSnapshot>, CredentialError>;
pub type CredentialFuture<'a> = Pin<Box<dyn Future<Output = CredentialResult> + Send + 'a>>;

/// Provider-neutral access to credentials owned by Plugin/Marketplace hosts.
pub trait CredentialBroker: Send + Sync {
    fn resolve<'a>(&'a self, request: CredentialRequest) -> CredentialFuture<'a>;
}

pub type SharedCredentialBroker = Arc<dyn CredentialBroker>;

/// Host default for runtimes that do not expose external Plugin/Marketplace credentials.
#[derive(Debug, Default)]
pub struct UnsupportedCredentialBroker;

impl UnsupportedCredentialBroker {
    pub fn shared() -> SharedCredentialBroker {
        Arc::new(Self)
    }
}

impl CredentialBroker for UnsupportedCredentialBroker {
    fn resolve<'a>(&'a self, _request: CredentialRequest) -> CredentialFuture<'a> {
        Box::pin(async { Err(CredentialError::Unsupported) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct ExactBroker {
        request: CredentialRequest,
        snapshot: CredentialSnapshot,
    }

    impl CredentialBroker for ExactBroker {
        fn resolve<'a>(&'a self, request: CredentialRequest) -> CredentialFuture<'a> {
            Box::pin(async move {
                if request == self.request {
                    Ok(Some(self.snapshot.clone()))
                } else {
                    Err(CredentialError::Unsupported)
                }
            })
        }
    }

    #[test]
    fn request_owner_and_purpose_are_part_of_the_lookup_key() {
        let request = CredentialRequest::new(
            CredentialOwner::Plugin("weather".to_string()),
            CredentialPurpose::PluginResource,
        );
        assert_eq!(
            request.owner,
            CredentialOwner::Plugin("weather".to_string())
        );
        assert_eq!(request.purpose, CredentialPurpose::PluginResource);
    }

    #[tokio::test]
    async fn unsupported_resolution_is_explicit() {
        let broker = ExactBroker {
            request: CredentialRequest::new(
                CredentialOwner::Plugin("weather".to_string()),
                CredentialPurpose::PluginResource,
            ),
            snapshot: CredentialSnapshot::from_opaque(()),
        };

        let result = broker
            .resolve(CredentialRequest::new(
                CredentialOwner::Marketplace("private".to_string()),
                CredentialPurpose::MarketplaceRequest,
            ))
            .await;

        assert!(matches!(result, Err(CredentialError::Unsupported)));
    }

    #[tokio::test]
    async fn exact_owner_and_purpose_receive_a_snapshot() {
        let request = CredentialRequest::new(
            CredentialOwner::Plugin("weather".to_string()),
            CredentialPurpose::PluginResource,
        );
        let broker = ExactBroker {
            request: request.clone(),
            snapshot: CredentialSnapshot::from_opaque(Arc::new("secret")),
        };

        let result = broker.resolve(request).await;

        assert!(result.unwrap().is_some());
    }
}

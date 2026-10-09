use std::sync::Arc;

use genex_api::AuthError;
use genex_api::AuthHeadersFuture;
use genex_api::AuthProvider;
use genex_api::SharedAuthProvider;
use genex_login::AuthHeaders;
use genex_login::CredentialManager;
use genex_login::GenexAuth;
use genex_model_provider_info::ModelProviderInfo;
use http::HeaderMap;

use crate::bearer_auth_provider::BearerAuthProvider;

#[derive(Clone, Copy, Debug, Default)]
pub struct ProviderAuthScope;

/// Provider auth resolved for a request, plus metadata describing the effective auth.
#[derive(Clone)]
pub struct ResolvedProviderAuth {
    pub auth: SharedAuthProvider,
}

impl ResolvedProviderAuth {
    pub(crate) fn new(auth: SharedAuthProvider) -> Self {
        Self { auth }
    }
}

#[derive(Clone, Debug)]
struct HeaderAuthProvider {
    auth: AuthHeaders,
}

impl AuthProvider for HeaderAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        headers.extend(self.auth.headers().clone());
    }
}

struct CredentialManagerAuthProvider {
    credential_manager: Arc<CredentialManager>,
    // Request headers always come from the current CredentialManager snapshot below.
    expected_auth: GenexAuth,
}

impl CredentialManagerAuthProvider {
    fn is_expected_auth(&self, auth: &GenexAuth) -> bool {
        auth.api_auth_mode() == self.expected_auth.api_auth_mode()
            && auth.get_account_id() == self.expected_auth.get_account_id()
    }

    fn current_auth(&self) -> Option<GenexAuth> {
        self.credential_manager
            .auth_cached()
            .filter(|auth| self.is_expected_auth(auth))
    }
}

impl AuthProvider for CredentialManagerAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        let Some(auth) = self.current_auth() else {
            return;
        };
        auth_provider_from_auth(&auth).add_auth_headers(headers);
    }

    fn resolve_auth_headers(&self) -> AuthHeadersFuture<'_> {
        Box::pin(async move {
            let auth = self
                .credential_manager
                .auth()
                .await
                .filter(|auth| self.is_expected_auth(auth))
                .ok_or_else(|| {
                    AuthError::Transient("managed authentication is unavailable".to_string())
                })?;
            Ok(auth_provider_from_auth(&auth).to_auth_headers())
        })
    }
}

// Providers without a resolved credential send no auth headers until the user
// completes the provider authentication flow.
#[derive(Clone, Debug)]
struct UnauthenticatedAuthProvider;

impl AuthProvider for UnauthenticatedAuthProvider {
    fn add_auth_headers(&self, _headers: &mut HeaderMap) {}
}

pub fn unauthenticated_auth_provider() -> SharedAuthProvider {
    Arc::new(UnauthenticatedAuthProvider)
}

/// Returns the provider-scoped auth manager when this provider uses command-backed auth.
///
/// Providers without custom auth continue using the caller-supplied base manager, when present.
pub(crate) fn credential_manager_for_provider(
    credential_manager: Option<Arc<CredentialManager>>,
    provider: &ModelProviderInfo,
) -> Option<Arc<CredentialManager>> {
    match provider.auth.clone() {
        Some(config) => Some(CredentialManager::external_bearer_only(config)),
        None => credential_manager,
    }
}

pub(crate) fn resolve_provider_auth(
    auth: Option<&GenexAuth>,
    provider: &ModelProviderInfo,
) -> genex_protocol::error::Result<SharedAuthProvider> {
    // Explicitly selected/stored credentials take precedence over provider
    // environment configuration. This lets the login flow work even when an
    // env-backed provider does not have its env var set.
    if let Some(auth) = auth {
        return Ok(auth_provider_from_auth(auth));
    }

    if let Some(auth) = bearer_auth_for_provider(provider)? {
        return Ok(Arc::new(auth));
    }

    Ok(unauthenticated_auth_provider())
}

pub(crate) async fn resolve_provider_auth_for_scope(
    credential_manager: Option<Arc<CredentialManager>>,
    auth: Option<&GenexAuth>,
    provider: &ModelProviderInfo,
    scope: ProviderAuthScope,
) -> genex_protocol::error::Result<ResolvedProviderAuth> {
    let auth = if provider.auth.is_some() {
        match credential_manager {
            Some(credential_manager) => credential_manager.auth().await,
            None => auth.cloned(),
        }
    } else if let Some(credential_manager) = credential_manager {
        credential_manager.auth().await
    } else {
        auth.cloned()
    };
    let _ = scope;
    resolve_provider_auth(auth.as_ref(), provider).map(ResolvedProviderAuth::new)
}

pub(crate) async fn provider_auth(
    credential_manager: Option<&Arc<CredentialManager>>,
    fallback_auth: Option<GenexAuth>,
    provider: &ModelProviderInfo,
) -> Option<GenexAuth> {
    if provider.auth.is_some() {
        return match credential_manager {
            Some(credential_manager) => credential_manager.auth().await,
            None => fallback_auth,
        };
    }
    if credential_manager
        .is_some_and(|credential_manager| credential_manager.current_auth_uses_managed_auth())
    {
        return match credential_manager {
            Some(credential_manager) => credential_manager.auth().await,
            None => fallback_auth,
        };
    }
    match credential_manager {
        Some(credential_manager) => credential_manager.auth().await,
        None => fallback_auth,
    }
}

pub(crate) fn cached_provider_auth(
    credential_manager: &CredentialManager,
) -> Option<GenexAuth> {
    credential_manager.auth_cached()
}

fn bearer_auth_for_provider(
    provider: &ModelProviderInfo,
) -> genex_protocol::error::Result<Option<BearerAuthProvider>> {
    if let Some(api_key) = provider.api_key()? {
        return Ok(Some(BearerAuthProvider::new(api_key)));
    }

    if let Some(token) = provider.experimental_bearer_token.clone() {
        return Ok(Some(BearerAuthProvider::new(token.into_inner())));
    }

    Ok(None)
}

/// Builds request-header auth for a stored Genex auth snapshot.
pub fn auth_provider_from_auth(auth: &GenexAuth) -> SharedAuthProvider {
    match auth {
        GenexAuth::Headers(auth) => Arc::new(HeaderAuthProvider { auth: auth.clone() }),
        GenexAuth::ApiKey(_) => Arc::new(BearerAuthProvider {
            token: auth.get_token().ok(),
        }),
    }
}

/// Builds request-header auth that reads the current auth snapshot on
/// every request while remaining scoped to the expected auth identity.
///
/// Callers with account-scoped state should pass the same snapshot that keyed
/// that state so a later account switch cannot reuse it.
pub fn auth_provider_from_credential_manager(
    credential_manager: Arc<CredentialManager>,
    expected_auth: &GenexAuth,
) -> SharedAuthProvider {
    Arc::new(CredentialManagerAuthProvider {
        credential_manager,
        expected_auth: expected_auth.clone(),
    })
}

#[cfg(test)]
mod tests {
    use genex_model_provider_info::WireApi;
    use genex_model_provider_info::create_responses_provider_with_base_url;
    use genex_protocol::config_types::ModelProviderAuthInfo;
    use http::HeaderValue;
    use http::header::AUTHORIZATION;
    use pretty_assertions::assert_eq;
    use std::num::NonZeroU64;

    use super::*;

    #[test]
    fn unauthenticated_auth_provider_adds_no_headers() {
        let provider = create_responses_provider_with_base_url(
            "http://localhost:11434/v1",
            WireApi::Responses,
        );
        let auth = resolve_provider_auth(/*auth*/ None, &provider).expect("auth should resolve");

        assert!(auth.to_auth_headers().is_empty());
    }

    #[test]
    fn custom_provider_does_not_inherit_ambient_auth_headers() {
        let provider = create_responses_provider_with_base_url(
            "http://localhost:11434/v1",
            WireApi::Responses,
        );
        let mut ambient_headers = HeaderMap::new();
        ambient_headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer ambient-token"),
        );
        let ambient_auth = GenexAuth::Headers(AuthHeaders::new(ambient_headers));

        let auth =
            resolve_provider_auth(Some(&ambient_auth), &provider).expect("auth should resolve");

        assert!(auth.to_auth_headers().is_empty());
    }

    #[test]
    fn custom_provider_uses_explicit_bearer_instead_of_ambient_auth() {
        let mut provider = create_responses_provider_with_base_url(
            "http://localhost:11434/v1",
            WireApi::Responses,
        );
        provider.experimental_bearer_token = Some("provider-token".into());
        let ambient_auth = GenexAuth::from_api_key("ambient-api-key");

        let headers = resolve_provider_auth(Some(&ambient_auth), &provider)
            .expect("auth should resolve")
            .to_auth_headers();

        assert_eq!(
            headers.get(AUTHORIZATION),
            Some(&HeaderValue::from_static("Bearer provider-token"))
        );
        assert_eq!(headers.len(), 1);
    }

    #[test]
    fn custom_provider_uses_command_resolved_auth() {
        let mut provider = create_responses_provider_with_base_url(
            "http://localhost:11434/v1",
            WireApi::Responses,
        );
        provider.auth = Some(ModelProviderAuthInfo {
            command: "print-token".to_string(),
            args: Vec::new(),
            timeout_ms: NonZeroU64::new(5_000).expect("timeout should be non-zero"),
            refresh_interval_ms: 300_000,
            cwd: std::env::current_dir()
                .expect("current directory should be available")
                .try_into()
                .expect("current directory should be absolute"),
        });
        let command_auth = GenexAuth::from_api_key("command-token");

        let headers = resolve_provider_auth(Some(&command_auth), &provider)
            .expect("auth should resolve")
            .to_auth_headers();

        assert_eq!(
            headers.get(AUTHORIZATION),
            Some(&HeaderValue::from_static("Bearer command-token"))
        );
    }

    #[test]
    fn responses_provider_preserves_ambient_auth_headers() {
        let provider = ModelProviderInfo::create_responses_provider(/*base_url*/ None);
        let mut expected = HeaderMap::new();
        expected.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer ambient-token"),
        );
        let ambient_auth = GenexAuth::Headers(AuthHeaders::new(expected.clone()));

        let auth =
            resolve_provider_auth(Some(&ambient_auth), &provider).expect("auth should resolve");

        assert_eq!(auth.to_auth_headers(), expected);
    }

    #[test]
    fn header_auth_adds_predefined_headers() {
        let mut expected = HeaderMap::new();
        expected.insert(
            http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer external"),
        );
        expected.insert("x-external-auth", HeaderValue::from_static("enabled"));
        let auth = GenexAuth::Headers(AuthHeaders::new(expected.clone()));

        let actual = auth_provider_from_auth(&auth).to_auth_headers();

        assert_eq!(actual, expected);
    }
}

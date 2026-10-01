use std::sync::Arc;

use genex_api::AuthError;
use genex_api::AuthHeadersFuture;
use genex_api::AuthProvider;
use genex_api::SharedAuthProvider;
use genex_login::AuthHeaders;
use genex_login::AuthManager;
use genex_login::CodexAuth;
use genex_model_provider_info::ModelProviderInfo;
use genex_protocol::error::CodexErr;
use http::HeaderMap;

use crate::bearer_auth_provider::BearerAuthProvider;

const BEDROCK_API_KEY_UNSUPPORTED_MESSAGE: &str =
    "Bedrock API key auth is only supported by the Amazon Bedrock model provider";

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

struct AuthManagerAuthProvider {
    auth_manager: Arc<AuthManager>,
    // Request headers always come from the current AuthManager snapshot below.
    expected_auth: CodexAuth,
}

impl AuthManagerAuthProvider {
    fn is_expected_auth(&self, auth: &CodexAuth) -> bool {
        auth.api_auth_mode() == self.expected_auth.api_auth_mode()
            && auth.get_account_id() == self.expected_auth.get_account_id()
    }

    fn current_auth(&self) -> Option<CodexAuth> {
        self.auth_manager
            .auth_cached()
            .filter(|auth| self.is_expected_auth(auth))
    }
}

impl AuthProvider for AuthManagerAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        let Some(auth) = self.current_auth() else {
            return;
        };
        auth_provider_from_auth(&auth).add_auth_headers(headers);
    }

    fn resolve_auth_headers(&self) -> AuthHeadersFuture<'_> {
        Box::pin(async move {
            let auth = self
                .auth_manager
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

// Some providers are meant to send no auth headers. Examples include local OSS
// providers and custom test providers with `requires_auth = false`.
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
pub(crate) fn auth_manager_for_provider(
    auth_manager: Option<Arc<AuthManager>>,
    provider: &ModelProviderInfo,
) -> Option<Arc<AuthManager>> {
    match provider.auth.clone() {
        Some(config) => Some(AuthManager::external_bearer_only(config)),
        None => auth_manager,
    }
}

pub(crate) fn resolve_provider_auth(
    auth: Option<&CodexAuth>,
    provider: &ModelProviderInfo,
) -> genex_protocol::error::Result<SharedAuthProvider> {
    if let Some(auth) = bearer_auth_for_provider(provider)? {
        return Ok(Arc::new(auth));
    }

    if !provider.requires_auth && provider.auth.is_none() {
        return Ok(unauthenticated_auth_provider());
    }

    if matches!(
        auth,
        Some(CodexAuth::BedrockApiKey(_) | CodexAuth::BedrockAccessKeys(_))
    ) {
        return Err(CodexErr::UnsupportedOperation(
            BEDROCK_API_KEY_UNSUPPORTED_MESSAGE.to_string(),
        ));
    }

    Ok(match auth {
        Some(auth) => auth_provider_from_auth(auth),
        None => unauthenticated_auth_provider(),
    })
}

pub(crate) async fn resolve_provider_auth_for_scope(
    auth_manager: Option<Arc<AuthManager>>,
    auth: Option<&CodexAuth>,
    provider: &ModelProviderInfo,
    scope: ProviderAuthScope,
) -> genex_protocol::error::Result<ResolvedProviderAuth> {
    let _ = (auth_manager, scope);
    resolve_provider_auth(auth, provider).map(ResolvedProviderAuth::new)
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

/// Builds request-header auth for a stored Codex auth snapshot.
pub fn auth_provider_from_auth(auth: &CodexAuth) -> SharedAuthProvider {
    match auth {
        CodexAuth::Headers(auth) => Arc::new(HeaderAuthProvider { auth: auth.clone() }),
        CodexAuth::BedrockApiKey(_) | CodexAuth::BedrockAccessKeys(_) => {
            unreachable!("{BEDROCK_API_KEY_UNSUPPORTED_MESSAGE}")
        }
        CodexAuth::ApiKey(_) => Arc::new(BearerAuthProvider {
            token: auth.get_token().ok(),
        }),
    }
}

/// Builds request-header auth that reads the current auth snapshot on
/// every request while remaining scoped to the expected auth identity.
///
/// Callers with account-scoped state should pass the same snapshot that keyed
/// that state so a later account switch cannot reuse it.
pub fn auth_provider_from_auth_manager(
    auth_manager: Arc<AuthManager>,
    expected_auth: &CodexAuth,
) -> SharedAuthProvider {
    Arc::new(AuthManagerAuthProvider {
        auth_manager,
        expected_auth: expected_auth.clone(),
    })
}

#[cfg(test)]
mod tests {
    use genex_login::auth::BedrockApiKeyAuth;
    use genex_model_provider_info::WireApi;
    use genex_model_provider_info::create_responses_provider_with_base_url;
    use genex_protocol::config_types::ModelProviderAuthInfo;
    use genex_protocol::error::CodexErrorDetails;
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
        let ambient_auth = CodexAuth::Headers(AuthHeaders::new(ambient_headers));

        let auth =
            resolve_provider_auth(Some(&ambient_auth), &provider).expect("auth should resolve");

        assert!(auth.to_auth_headers().is_empty());
    }

    #[test]
    fn custom_provider_does_not_inherit_ambient_bedrock_auth() {
        let provider = create_responses_provider_with_base_url(
            "http://localhost:11434/v1",
            WireApi::Responses,
        );
        let ambient_auth = CodexAuth::BedrockApiKey(BedrockApiKeyAuth {
            api_key: "bedrock-api-key-test".to_string(),
            region: "us-east-1".to_string(),
        });

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
        let ambient_auth = CodexAuth::BedrockApiKey(BedrockApiKeyAuth {
            api_key: "bedrock-api-key-test".to_string(),
            region: "us-east-1".to_string(),
        });

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
        let command_auth = CodexAuth::from_api_key("command-token");

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
        let ambient_auth = CodexAuth::Headers(AuthHeaders::new(expected.clone()));

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
        let auth = CodexAuth::Headers(AuthHeaders::new(expected.clone()));

        let actual = auth_provider_from_auth(&auth).to_auth_headers();

        assert_eq!(actual, expected);
    }

    #[test]
    fn responses_provider_rejects_bedrock_api_key_auth() {
        let provider = ModelProviderInfo::create_responses_provider(/*base_url*/ None);
        let auth = CodexAuth::BedrockApiKey(BedrockApiKeyAuth {
            api_key: "bedrock-api-key-test".to_string(),
            region: "us-east-1".to_string(),
        });

        match resolve_provider_auth(Some(&auth), &provider) {
            Err(err) => match err.details() {
                CodexErrorDetails::UnsupportedOperation(message) => {
                    assert_eq!(message, BEDROCK_API_KEY_UNSUPPORTED_MESSAGE);
                }
                details => panic!("unexpected auth error: {details:?}"),
            },
            Ok(_) => panic!("Bedrock API key auth should be rejected"),
        }
    }
}

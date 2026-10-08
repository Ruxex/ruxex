//! Model catalog cache identity follows provider routing and effective auth.
//! Only a digest is persisted; credential values are included only through the
//! resulting request headers when the provider explicitly supplies them.

use genex_login::GenexAuth;
use genex_model_provider_info::ModelProviderInfo;
use genex_protocol::error::Result as CoreResult;
use sha2::Digest;
use sha2::Sha256;

use crate::auth::resolve_provider_auth;

pub(crate) fn identity(
    provider_id: Option<&str>,
    provider_info: &ModelProviderInfo,
    auth: Option<&GenexAuth>,
) -> CoreResult<String> {
    let mut provider = provider_info.to_api_provider(auth.map(GenexAuth::auth_mode))?;
    let mut digest = Sha256::new();
    // Length-prefix every field to avoid ambiguity between adjacent values.
    let mut field = |value: &[u8]| {
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value);
    };
    field(b"models-cache-v1");
    field(provider_id.unwrap_or_default().as_bytes());
    field(provider.name.as_bytes());
    field(provider.base_url.as_bytes());
    let mut query: Vec<_> = provider.query_params.iter().flatten().collect();
    query.sort();
    field(&(query.len() as u64).to_le_bytes());
    for (name, value) in query {
        field(name.as_bytes());
        field(value.as_bytes());
    }
    field(&[u8::from(provider_info.has_command_auth())]);
    field(format!("{:?}", auth.map(GenexAuth::api_auth_mode)).as_bytes());
    if let Some(auth) = auth {
        field(format!("{:?}", auth.get_account_id()).as_bytes());
    }
    let explicit_bearer =
        provider_info.env_key.is_some() || provider_info.experimental_bearer_token.is_some();
    // Command auth also returns opaque API keys. A changed key may belong to a
    // different account, so only credentials with stable owner metadata can be reused.
    if explicit_bearer || auth.is_some() {
        let headers = resolve_provider_auth(auth, provider_info)?.to_auth_headers();
        provider.headers.extend(headers);
    }
    let mut headers: Vec<_> = provider.headers.iter().collect();
    headers.sort_by(|(a, av), (b, bv)| {
        a.as_str()
            .cmp(b.as_str())
            .then(av.as_bytes().cmp(bv.as_bytes()))
    });
    field(&(headers.len() as u64).to_le_bytes());
    for (name, value) in headers {
        field(name.as_str().as_bytes());
        field(value.as_bytes());
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
#[path = "models_identity_tests.rs"]
mod tests;

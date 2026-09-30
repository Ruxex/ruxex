use super::CredentialHostBinding;
use super::CredentialProvider;
use super::CredentialSource;
use super::shaped_dummy_value;
use rama_http::HeaderMap;
use rama_http::HeaderValue;
use rama_http::header::AUTHORIZATION;
use std::collections::HashMap;

const OPENAI_API_KEY_ENV_VARS: &[&str] = &["OPENAI_API_KEY"];
const OPENAI_API_KEY_PREFIXES: &[&str] = &["sk-proj-", "sk-svcacct-", "sk-admin-", "sk-"];
const OPENAI_API_KEY_MIN_LEN: usize = 51;
const OPENAI_API_HOST: &str = "api.openai.com";

pub(super) static PROVIDER: CredentialProvider = CredentialProvider {
    context_env_vars: &[],
    credential_prefixes: OPENAI_API_KEY_PREFIXES,
    ignored_credential_prefixes: &["sk-ant-", "sk-or-"],
    credential_watermark: Some("T3BlbkFJ"),
    minimum_credential_len: OPENAI_API_KEY_MIN_LEN,
    sources: &[CredentialSource {
        env_vars: OPENAI_API_KEY_ENV_VARS,
        binding_env_vars: &[],
        invalidates_host_binding: |_| false,
        host_binding,
    }],
    dummy_value,
    translate_request_header,
    request_header_value,
    insert_request_header,
};

fn dummy_value(real_value: &str) -> String {
    shaped_dummy_value(
        real_value,
        openai_api_key_prefix(real_value),
        OPENAI_API_KEY_MIN_LEN,
    )
}

fn translate_request_header(
    headers: &HeaderMap,
    expected_value: &str,
    replacement_value: &str,
) -> Option<HeaderValue> {
    headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .filter(|header| header.contains(expected_value))?;
    request_header_value(replacement_value)
}

fn request_header_value(value: &str) -> Option<HeaderValue> {
    HeaderValue::from_str(&format!("Bearer {value}")).ok()
}

fn insert_request_header(headers: &mut HeaderMap, value: HeaderValue) {
    headers.insert(AUTHORIZATION, value);
}

fn host_binding(_env: &HashMap<String, String>) -> Option<CredentialHostBinding> {
    Some(CredentialHostBinding::ExactHost(OPENAI_API_HOST.to_string()))
}

fn openai_api_key_prefix(value: &str) -> &str {
    OPENAI_API_KEY_PREFIXES
        .iter()
        .copied()
        .find(|prefix| value.starts_with(prefix))
        .unwrap_or("sk-")
}

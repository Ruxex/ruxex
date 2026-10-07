use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use genex_config::types::AuthCredentialsStoreMode;
use genex_login::AuthDotJson;
use genex_login::AuthKeyringBackendKind;
use genex_login::save_auth;
use genex_protocol::auth::AuthMode;
use serde_json::json;

/// Builder for writing API-key credentials in tests.
#[derive(Debug, Clone)]
pub struct ApiKeyAuthFixture {
    api_key: String,
}

impl ApiKeyAuthFixture {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }

    pub fn account_id(self, _account_id: impl Into<String>) -> Self {
        self
    }
    pub fn workspace_id(self, _workspace_id: impl Into<String>) -> Self {
        self
    }
    pub fn user_id(self, _user_id: impl Into<String>) -> Self {
        self
    }
    pub fn email(self, _email: impl Into<String>) -> Self {
        self
    }
    pub fn plan_type(self, _plan_type: impl Into<String>) -> Self {
        self
    }
    pub fn last_refresh(self, _last_refresh: Option<chrono::DateTime<chrono::Utc>>) -> Self {
        self
    }
    pub fn claims(self, _claims: AuthFixtureClaims) -> Self {
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuthFixtureClaims {
    pub email: Option<String>,
    pub plan_type: Option<String>,
    pub user_id: Option<String>,
    pub workspace_id: Option<String>,
    pub account_is_fedramp: bool,
}

impl AuthFixtureClaims {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }
    pub fn plan_type(mut self, plan_type: impl Into<String>) -> Self {
        self.plan_type = Some(plan_type.into());
        self
    }
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }
    pub fn workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }
}

/// Keeps callers that need an opaque test token independent of provider claims.
pub fn encode_fixture_token(claims: &AuthFixtureClaims) -> Result<String> {
    Ok(serde_json::to_string(&json!({
        "email": claims.email,
        "plan": claims.plan_type,
        "user": claims.user_id,
        "workspace": claims.workspace_id,
        "fedramp": claims.account_is_fedramp,
    }))?)
}

pub fn write_api_key_auth(
    genex_home: &Path,
    fixture: ApiKeyAuthFixture,
    cli_auth_credentials_store_mode: AuthCredentialsStoreMode,
) -> Result<()> {
    save_auth(
        genex_home,
        &AuthDotJson::legacy(Some(AuthMode::ApiKey), Some(fixture.api_key)),
        cli_auth_credentials_store_mode,
        AuthKeyringBackendKind::default(),
    )
    .context("write auth.json")
}

use std::path::PathBuf;
use std::sync::Arc;

use codex_api::AuthProvider;
use codex_api::SharedAuthProvider;
use codex_login::AuthManager;
use codex_login::CodexAuth;
use codex_model_provider_info::ModelProviderInfo;
use codex_models_manager::manager::SharedModelsManager;
use codex_models_manager::manager::StaticModelsManager;
use codex_protocol::account::ProviderAccount;
use codex_protocol::openai_models::ModelsResponse;
use http::HeaderMap;
use http::HeaderValue;

use super::auth::unauthenticated_auth_provider;
use super::provider::ModelProvider;
use super::provider::ModelProviderFuture;
use super::provider::ProviderAccountResult;
use super::provider::ProviderAccountState;

#[derive(Clone, Debug)]
pub(crate) struct MyModelProvider {
    info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
}

impl MyModelProvider {
    pub(crate) fn new(info: ModelProviderInfo, auth_manager: Option<Arc<AuthManager>>) -> Self {
        Self { info, auth_manager }
    }
}

#[derive(Clone, Debug)]
struct MyApiKeyAuthProvider {
    api_key: String,
}

impl AuthProvider for MyApiKeyAuthProvider {
    fn add_auth_headers(&self, headers: &mut HeaderMap) {
        if let Ok(value) = HeaderValue::from_str(&format!("apikey {}", self.api_key)) {
            headers.insert("APIKEY", value);
        }
    }
}

impl ModelProvider for MyModelProvider {
    fn info(&self) -> &ModelProviderInfo {
        &self.info
    }

    fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        self.auth_manager.clone()
    }

    fn auth(&self) -> ModelProviderFuture<'_, Option<CodexAuth>> {
        Box::pin(async move {
            match self.auth_manager.as_ref() {
                Some(auth_manager) => auth_manager.auth().await,
                None => None,
            }
        })
    }

    fn account_state(&self) -> ProviderAccountResult {
        Ok(ProviderAccountState {
            account: Some(ProviderAccount::ApiKey),
            requires_first_party_auth: false,
        })
    }

    fn api_auth(
        &self,
    ) -> ModelProviderFuture<'_, codex_protocol::error::Result<SharedAuthProvider>> {
        Box::pin(async move {
            let Some(auth) = self.auth().await else {
                return Ok(unauthenticated_auth_provider());
            };
            let Ok(api_key) = auth.get_token() else {
                return Ok(unauthenticated_auth_provider());
            };
            Ok(Arc::new(MyApiKeyAuthProvider { api_key }) as SharedAuthProvider)
        })
    }

    fn models_manager(
        &self,
        _codex_home: PathBuf,
        config_model_catalog: Option<ModelsResponse>,
    ) -> SharedModelsManager {
        self.models_manager_without_cache(config_model_catalog)
    }

    fn models_manager_without_cache(
        &self,
        config_model_catalog: Option<ModelsResponse>,
    ) -> SharedModelsManager {
        let model_catalog = config_model_catalog
            .or_else(|| codex_models_manager::bundled_models_response().ok())
            .unwrap_or_default();
        Arc::new(StaticModelsManager::new(
            self.auth_manager.clone(),
            model_catalog,
        ))
    }
}

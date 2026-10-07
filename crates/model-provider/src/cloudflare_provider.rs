use std::sync::Arc;
use std::time::Duration;

use genex_api::ModelsClient;
use genex_api::Provider;
use genex_api::ReqwestTransport;
use genex_api::SharedAuthProvider;
use genex_api::StandardChatCompletionApi;
use genex_api::map_api_error;
use genex_http_client::ClientRouteClass;
use genex_http_client::HttpClientFactory;
use genex_login::AuthManager;
use genex_login::GenexAuth;
use genex_login::default_client::create_client_for_route_async;
use genex_model_provider_info::CLOUDFLARE_ACCOUNT_ID_ENV_VAR;
use genex_model_provider_info::ModelProviderInfo;
use genex_models_manager::manager::ModelCatalogSnapshot;
use genex_models_manager::model_info::model_info_template_from_slug;
use genex_protocol::error::EnvVarError;
use genex_protocol::error::GenexErr;
use genex_protocol::error::Result as CoreResult;
use genex_protocol::model_types::ModelCatalog;
use genex_protocol::model_types::ModelInfo;
use http::HeaderMap;
use serde_json::Value;
use tokio::time::timeout;

use super::auth::resolve_provider_auth;
use super::models_identity;
use super::provider::ModelProvider;
use super::provider::ModelProviderFuture;
use super::provider::ProviderAccountResult;
use super::provider::ProviderAccountState;
use super::provider::ProviderApi;

const CLOUDFLARE_API_ROOT: &str = "https://api.cloudflare.com/client/v4";
const CLOUDFLARE_MODELS_TIMEOUT: Duration = Duration::from_secs(5);
const CLOUDFLARE_MODELS_PAGE_SIZE: usize = 100;
const CLOUDFLARE_MAX_MODEL_PAGES: usize = 100;

#[derive(Clone, Debug)]
pub(crate) struct CloudflareModelProvider {
    info: ModelProviderInfo,
    auth_manager: Option<Arc<AuthManager>>,
}

impl CloudflareModelProvider {
    pub(crate) fn new(info: ModelProviderInfo, auth_manager: Option<Arc<AuthManager>>) -> Self {
        Self { info, auth_manager }
    }

    fn chat_base_url(&self) -> CoreResult<String> {
        if let Some(base_url) = &self.info.base_url {
            return Ok(base_url.clone());
        }

        let account_id = cloudflare_account_id()?;
        Ok(format!("{CLOUDFLARE_API_ROOT}/accounts/{account_id}/ai/v1"))
    }

    fn effective_info(&self, base_url: String) -> ModelProviderInfo {
        ModelProviderInfo {
            base_url: Some(base_url),
            ..self.info.clone()
        }
    }

    fn models_info(&self) -> CoreResult<ModelProviderInfo> {
        let base_url = if let Some(base_url) = &self.info.base_url {
            base_url.clone()
        } else {
            let account_id = cloudflare_account_id()?;
            format!("{CLOUDFLARE_API_ROOT}/accounts/{account_id}/ai")
        };
        Ok(self.effective_info(base_url))
    }

    fn model_identity(&self) -> Option<String> {
        let auth = self
            .auth_manager
            .as_ref()
            .and_then(|manager| manager.auth_cached());
        let provider_info = self.models_info().ok()?;
        models_identity::identity(&provider_info, auth.as_ref()).ok()
    }

    async fn fetch_models(
        &self,
        http_client_factory: HttpClientFactory,
    ) -> CoreResult<(Vec<ModelInfo>, Option<String>)> {
        let auth = self.auth().await;
        let provider_info = self.models_info()?;
        let api_provider =
            provider_info.to_api_provider(auth.as_ref().map(GenexAuth::auth_mode))?;
        let api_auth = resolve_provider_auth(auth.as_ref(), &provider_info)?;
        let mut models = Vec::new();
        let mut etag = None;

        for page in 1..=CLOUDFLARE_MAX_MODEL_PAGES {
            let request_url = format!(
                "{}?task=text-generation&hide_experimental=true&per_page={CLOUDFLARE_MODELS_PAGE_SIZE}&page={page}",
                api_provider.url_for_path("models/search")
            );
            let transport = create_client_for_route_async(
                http_client_factory.clone(),
                request_url.clone(),
                ClientRouteClass::Api,
            )
            .await
            .map(ReqwestTransport::from_http_client)?;
            let client = ModelsClient::new(transport, api_provider.clone(), api_auth.clone());
            let (body, page_etag) = timeout(CLOUDFLARE_MODELS_TIMEOUT, async {
                client
                    .list_models_raw(request_url, HeaderMap::new(), None)
                    .await
                    .map_err(map_api_error)
            })
            .await
            .map_err(|_| GenexErr::RequestTimeout)??;

            let decoded = decode_cloudflare_models(&body, page)?;
            models.extend(decoded.models);
            etag = page_etag.or(etag);
            if !decoded.has_more {
                break;
            }
        }

        Ok((models, etag))
    }
}

impl ModelProvider for CloudflareModelProvider {
    fn info(&self) -> &ModelProviderInfo {
        &self.info
    }

    fn api(&self) -> ProviderApi {
        ProviderApi::Chat(Arc::new(StandardChatCompletionApi))
    }

    fn api_key_model_discovery_enabled_by_default(&self) -> bool {
        true
    }

    fn list_models(
        &self,
        _client_version: &str,
        http_client_factory: HttpClientFactory,
    ) -> ModelProviderFuture<'_, CoreResult<ModelCatalogSnapshot>> {
        Box::pin(async move {
            let identity = self.model_identity().ok_or_else(|| {
                GenexErr::InvalidRequest("Cloudflare provider identity is unavailable".to_string())
            })?;
            let (models, etag) = self.fetch_models(http_client_factory).await?;
            Ok(ModelCatalogSnapshot {
                catalog: ModelCatalog { models },
                identity,
                etag,
            })
        })
    }

    fn auth_manager(&self) -> Option<Arc<AuthManager>> {
        self.auth_manager.clone()
    }

    fn auth(&self) -> ModelProviderFuture<'_, Option<GenexAuth>> {
        Box::pin(async move {
            match self.auth_manager.as_ref() {
                Some(auth_manager) => auth_manager.auth().await,
                None => None,
            }
        })
    }

    fn account_state(&self) -> ProviderAccountResult {
        Ok(ProviderAccountState {
            account: None,
            requires_auth: false,
        })
    }

    fn api_provider(&self) -> ModelProviderFuture<'_, CoreResult<Provider>> {
        Box::pin(async move {
            let auth = self.auth().await;
            self.effective_info(self.chat_base_url()?)
                .to_api_provider(auth.as_ref().map(GenexAuth::auth_mode))
        })
    }

    fn runtime_base_url(&self) -> ModelProviderFuture<'_, CoreResult<Option<String>>> {
        Box::pin(async move { Ok(Some(self.chat_base_url()?)) })
    }

    fn api_auth(&self) -> ModelProviderFuture<'_, CoreResult<SharedAuthProvider>> {
        Box::pin(async move {
            let auth = self.auth().await;
            resolve_provider_auth(auth.as_ref(), &self.info)
        })
    }
}

fn cloudflare_account_id() -> CoreResult<String> {
    std::env::var(CLOUDFLARE_ACCOUNT_ID_ENV_VAR)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            GenexErr::EnvVar(EnvVarError {
                var: CLOUDFLARE_ACCOUNT_ID_ENV_VAR.to_string(),
                instructions: Some(
                    "Set CLOUDFLARE_ACCOUNT_ID to the account that owns Workers AI.".to_string(),
                ),
            })
        })
}

#[derive(Debug)]
struct DecodedCloudflareModels {
    models: Vec<ModelInfo>,
    has_more: bool,
}

fn decode_cloudflare_models(body: &[u8], page: usize) -> CoreResult<DecodedCloudflareModels> {
    let payload: Value = serde_json::from_slice(body)?;
    if payload.get("success").and_then(Value::as_bool) == Some(false) {
        return Err(GenexErr::InvalidRequest(
            "Cloudflare model search failed".to_string(),
        ));
    }

    let result = payload
        .get("result")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            GenexErr::InvalidRequest("Cloudflare model search returned no result array".to_string())
        })?;

    let models = result
        .iter()
        .filter_map(|model| {
            let slug = ["name", "model", "id", "slug"]
                .iter()
                .find_map(|field| model.get(*field).and_then(Value::as_str))?;
            let mut info = model_info_template_from_slug(slug);
            info.description = model
                .get("description")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            Some(info)
        })
        .collect();
    let has_more = payload
        .get("result_info")
        .and_then(|info| info.get("total_pages"))
        .and_then(Value::as_u64)
        .is_some_and(|total_pages| page < total_pages as usize);

    Ok(DecodedCloudflareModels { models, has_more })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloudflare_model_search_response_is_converted_to_provider_models() {
        let models = decode_cloudflare_models(
            br#"{
                "success": true,
                "result": [{
                    "name": "@cf/meta/llama-3.1-8b-instruct",
                    "description": "Llama 3.1 8B instruct"
                }]
            }"#,
            1,
        )
        .expect("Cloudflare response should decode")
        .models;

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].slug, "@cf/meta/llama-3.1-8b-instruct");
        assert_eq!(
            models[0].description.as_deref(),
            Some("Llama 3.1 8B instruct")
        );
    }

    #[test]
    fn cloudflare_model_search_response_rejects_failed_requests() {
        let error = decode_cloudflare_models(
            br#"{"success": false, "errors": [{"message": "forbidden"}]}"#,
            1,
        )
        .expect_err("failed Cloudflare response should be rejected");

        assert!(error.to_string().contains("model search failed"));
    }
}

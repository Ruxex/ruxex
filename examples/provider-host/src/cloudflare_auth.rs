use std::collections::BTreeMap;
use std::fmt;

use ruxex::provider::ModelProvider;
use ruxex::provider::ProviderError;
use ruxex::provider::ProviderFuture;
use ruxex::provider::ProviderResult;
use ruxex::provider::credentials::CredentialFlow;
use ruxex::provider::credentials::CredentialInput;
use ruxex::provider::credentials::CredentialOption;
use ruxex::provider::credentials::CredentialStep;
use ruxex::provider::credentials::CredentialValue;
use ruxex::provider::credentials::ProviderCredentialResolver;
use ruxex::provider::credentials::ProviderCredentialSink;
use ruxex::provider::credentials::ProviderCredentials;
use ruxex::provider::transport::ClientRouteClass;
use ruxex::provider::transport::HttpClientFactory;
use serde::Deserialize;
use serde::Serialize;

use crate::cloudflare::CloudflareProvider;

const CLOUDFLARE_PROVIDER_ID: &str = "cloudflare";
const CLOUDFLARE_API_ROOT: &str = "https://api.cloudflare.com/client/v4";

#[derive(Clone, Eq, PartialEq, Serialize)]
pub struct CloudflareCredentials {
    pub api_key: String,
    pub account_id: String,
}

impl fmt::Debug for CloudflareCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CloudflareCredentials")
            .field("api_key", &"<redacted>")
            .field("account_id", &self.account_id)
            .finish()
    }
}

impl ProviderCredentials for CloudflareCredentials {
    const PROVIDER_ID: &'static str = CLOUDFLARE_PROVIDER_ID;

    fn into_fields(self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("api_key".to_string(), self.api_key),
            ("account_id".to_string(), self.account_id),
        ])
    }
}

#[derive(Clone, Debug)]
pub struct CloudflareCredentialResolver {
    http_client_factory: HttpClientFactory,
    api_root: String,
}

impl CloudflareCredentialResolver {
    pub fn new(http_client_factory: HttpClientFactory) -> Self {
        Self {
            http_client_factory,
            api_root: CLOUDFLARE_API_ROOT.to_string(),
        }
    }

    async fn list_accounts(&self, api_key: &str) -> ProviderResult<Vec<CredentialOption>> {
        let request_url = format!("{}/accounts", self.api_root.trim_end_matches('/'));
        let client = ruxex::client::create_client_for_route_async(
            self.http_client_factory.clone(),
            request_url.clone(),
            ClientRouteClass::Api,
        )
        .await
        .map_err(|error| ProviderError::Transport(error.to_string()))?;
        let response = client
            .get(request_url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|error| ProviderError::Transport(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ProviderError::Provider(format!(
                "Cloudflare account discovery failed with HTTP {}",
                response.status()
            )));
        }
        let body = response
            .json::<CloudflareAccountsResponse>()
            .await
            .map_err(|error| ProviderError::Provider(error.to_string()))?;
        if !body.success {
            return Err(ProviderError::Provider(
                "Cloudflare account discovery was rejected".to_string(),
            ));
        }
        Ok(body
            .result
            .into_iter()
            .map(|account| CredentialOption::new(account.id, account.name))
            .collect())
    }
}

impl ProviderCredentialResolver<CloudflareCredentials> for CloudflareCredentialResolver {
    fn begin(
        &self,
        input: CredentialInput,
    ) -> ProviderFuture<'_, ProviderResult<Box<dyn CredentialFlow<CloudflareCredentials>>>> {
        let flow = CloudflareCredentialFlow {
            resolver: self.clone(),
            api_key: input.values.get("api_key").cloned(),
            account_options: Vec::new(),
            account_id: None,
        };
        Box::pin(
            async move { Ok(Box::new(flow) as Box<dyn CredentialFlow<CloudflareCredentials>>) },
        )
    }
}

impl ProviderCredentialSink<CloudflareCredentials> for CloudflareProvider {
    fn provider_id(&self) -> &str {
        CLOUDFLARE_PROVIDER_ID
    }

    fn login_typed(
        &self,
        credentials: CloudflareCredentials,
    ) -> ProviderFuture<'_, ProviderResult<()>> {
        ModelProvider::login(self, credentials.into_fields())
    }
}

pub fn create_cloudflare_credential_resolver(
    http_client_factory: HttpClientFactory,
) -> CloudflareCredentialResolver {
    CloudflareCredentialResolver::new(http_client_factory)
}

struct CloudflareCredentialFlow {
    resolver: CloudflareCredentialResolver,
    api_key: Option<String>,
    account_options: Vec<CredentialOption>,
    account_id: Option<String>,
}

impl CredentialFlow<CloudflareCredentials> for CloudflareCredentialFlow {
    fn current_step(&self) -> CredentialStep {
        if self.api_key.is_none() {
            CredentialStep::input("api_key", "API key", true)
        } else if self.account_id.is_none() {
            CredentialStep::select(
                "account_id",
                "Cloudflare account",
                self.account_options.clone(),
            )
        } else {
            CredentialStep::Complete
        }
    }

    fn submit(
        &mut self,
        value: CredentialValue,
    ) -> ProviderFuture<'_, ProviderResult<CredentialStep>> {
        if self.api_key.is_none() {
            let api_key = value.into_string().trim().to_string();
            let resolver = self.resolver.clone();
            return Box::pin(async move {
                if api_key.is_empty() {
                    return Err(ProviderError::InvalidRequest(
                        "Cloudflare API key is required".to_string(),
                    ));
                }
                let options = resolver.list_accounts(&api_key).await?;
                if options.is_empty() {
                    return Err(ProviderError::InvalidRequest(
                        "Cloudflare API key has no accessible accounts".to_string(),
                    ));
                }
                self.api_key = Some(api_key);
                self.account_options = options;
                Ok(self.current_step())
            });
        }

        let account_id = value.into_string();
        if self
            .account_options
            .iter()
            .any(|option| option.value == account_id)
        {
            self.account_id = Some(account_id);
            return Box::pin(async { Ok(CredentialStep::Complete) });
        }
        Box::pin(async {
            Err(ProviderError::InvalidRequest(
                "Cloudflare account selection is invalid".to_string(),
            ))
        })
    }

    fn finish(self: Box<Self>) -> ProviderResult<CloudflareCredentials> {
        let Self {
            api_key,
            account_id,
            ..
        } = *self;
        Ok(CloudflareCredentials {
            api_key: api_key.ok_or_else(|| {
                ProviderError::InvalidRequest("Cloudflare API key is missing".to_string())
            })?,
            account_id: account_id.ok_or_else(|| {
                ProviderError::InvalidRequest("Cloudflare account ID is missing".to_string())
            })?,
        })
    }
}

#[derive(Debug, Deserialize)]
struct CloudflareAccountsResponse {
    success: bool,
    #[serde(default)]
    result: Vec<CloudflareAccount>,
}

#[derive(Debug, Deserialize)]
struct CloudflareAccount {
    id: String,
    name: String,
}

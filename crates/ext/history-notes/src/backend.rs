use std::time::Duration;

use genex_http_client::HttpClientFactory;
use genex_model_provider::ProviderBackendRequest;
use genex_model_provider::ProviderTransportContext;
use genex_model_provider::SharedModelProvider;
use genex_utils_output_truncation::TruncationPolicy;
use http::HeaderValue;
use serde_json::Value;
use serde_json::json;

const HISTORY_NOTES_BACKEND_TIMEOUT: Duration = Duration::from_secs(35);
const ENCRYPTED_TOOL_ARGUMENTS_HEADER: &str = "x-provider-encrypted-tool-arguments";
const TOOL_OUTPUT_TRUNCATION_POLICY_HEADER: &str = "x-provider-tool-output-truncation-policy";
const OPERATION_ERROR_PREFIX: &str = "Unable to perform operation:";

#[derive(Clone)]
pub(crate) struct HistoryNotesBackend {
    provider: SharedModelProvider,
    http_client_factory: HttpClientFactory,
}

impl HistoryNotesBackend {
    pub(crate) fn new(
        provider: SharedModelProvider,
        http_client_factory: HttpClientFactory,
    ) -> Self {
        Self {
            provider,
            http_client_factory,
        }
    }

    pub(crate) async fn call(
        &self,
        path: &str,
        session_id: &str,
        current_agent_name: &str,
        mut arguments: Value,
        truncation_policy: TruncationPolicy,
    ) -> Result<Value, String> {
        let Some(arguments_object) = arguments.as_object_mut() else {
            return Err("History tool arguments must be a JSON object".to_string());
        };
        arguments_object.insert(
            "context".to_string(),
            json!({
                "session_id": session_id,
                "current_agent_name": current_agent_name,
            }),
        );

        let encoded_truncation_policy =
            serde_json::to_string(&truncation_policy).map_err(|_| {
                format!("{OPERATION_ERROR_PREFIX} Could not encode the output truncation policy.")
            })?;
        let header_value = HeaderValue::from_str(&encoded_truncation_policy).map_err(|_| {
            format!(
                "{OPERATION_ERROR_PREFIX} Could not construct the output truncation policy header."
            )
        })?;
        let mut headers = std::collections::BTreeMap::new();
        headers.insert(
            TOOL_OUTPUT_TRUNCATION_POLICY_HEADER.to_string(),
            header_value
                .to_str()
                .map_err(|_| {
                    format!(
                        "{OPERATION_ERROR_PREFIX} Could not construct the output truncation policy header."
                    )
                })?
                .to_string(),
        );
        if matches!(
            path,
            "alpha/history/v2/search_contents"
                | "alpha/notes/v2/search_contents"
                | "alpha/notes/v2/append_to_file"
                | "alpha/notes/v2/write_file"
        ) {
            headers.insert(
                ENCRYPTED_TOOL_ARGUMENTS_HEADER.to_string(),
                "true".to_string(),
            );
        }
        let response = self
            .provider
            .backend(ProviderBackendRequest {
                path: path.to_string(),
                body: arguments,
                headers,
                timeout: HISTORY_NOTES_BACKEND_TIMEOUT,
                transport: ProviderTransportContext::from_http_client_factory(
                    self.http_client_factory.clone(),
                ),
            })
            .await
            .map_err(|error| match error {
                genex_model_provider::ProviderError::Unauthenticated
                | genex_model_provider::ProviderError::Unauthorized => {
                    format!("{OPERATION_ERROR_PREFIX} Could not apply backend authentication.")
                }
                genex_model_provider::ProviderError::Provider(message)
                    if message.contains("invalid JSON") =>
                {
                    format!("{OPERATION_ERROR_PREFIX} The backend returned invalid JSON.")
                }
                _ => format!("{OPERATION_ERROR_PREFIX} The backend request failed."),
            })?;

        Ok(response.body)
    }
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;

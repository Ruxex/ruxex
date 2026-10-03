#[cfg(test)]
mod tests {
    use super::*;
    use crate::Provider;
    use crate::RetryConfig;
    use genex_protocol::models::ResponseItem;
    use serde_json::Value;
    use std::time::Duration;

    fn provider() -> Provider {
        Provider {
            name: "test".to_string(),
            base_url: "https://example.test/v1".to_string(),
            query_params: None,
            headers: http::HeaderMap::new(),
            retry: RetryConfig {
                max_attempts: 1,
                base_delay: Duration::from_millis(1),
                retry_429: false,
                retry_5xx: false,
                retry_transport: false,
            },
            stream_idle_timeout: Duration::from_secs(1),
        }
    }

    #[test]
    fn standard_adapter_preserves_chat_request_payload_shape() {
        let provider = provider();
        let input = ChatRequestInput {
            provider: &provider,
            model: "gpt-test",
            instructions: "be concise",
            input: &[] as &[ResponseItem],
            tools: &[] as &[Value],
            conversation_id: None,
            session_source: None,
        };

        let request = StandardChatCompletionApi
            .build_request(input)
            .expect("standard Chat adapter should build a request");

        assert_eq!(request.body["model"], "gpt-test");
        assert_eq!(
            request.body["messages"],
            serde_json::json!([
                {"role": "system", "content": "be concise"}
            ])
        );
        assert_eq!(request.body["stream"], true);
        assert_eq!(request.body["tools"], serde_json::json!([]));
    }
}
use crate::ApiError;
use crate::Provider;
use crate::ResponseStream;
use crate::SseTelemetry;
use crate::requests::ChatRequest;
use crate::requests::ChatRequestBuilder;
use crate::sse::chat::spawn_chat_stream;
use genex_client::StreamResponse;
use genex_protocol::models::ResponseItem;
use genex_protocol::protocol::SessionSource;
use genex_protocol::protocol::TokenUsage;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;

/// Inputs used to construct one Chat Completions request.
pub struct ChatRequestInput<'a> {
    pub provider: &'a Provider,
    pub model: &'a str,
    pub instructions: &'a str,
    pub input: &'a [ResponseItem],
    pub tools: &'a [Value],
    pub conversation_id: Option<String>,
    pub session_source: Option<SessionSource>,
}

/// Runtime adapter for a Chat Completions wire protocol.
pub trait ChatCompletionApi: Send + Sync {
    fn build_request(&self, input: ChatRequestInput<'_>) -> Result<ChatRequest, ApiError>;

    fn spawn_stream(
        &self,
        response: StreamResponse,
        idle_timeout: Duration,
        telemetry: Option<Arc<dyn SseTelemetry>>,
    ) -> ResponseStream;
}

/// Converts one Chat Completions `usage` payload into provider token usage.
pub trait ChatUsageDecoder: Send + Sync {
    fn decode(&self, usage: &Value) -> Option<TokenUsage>;
}

/// Decoder for the standard Chat Completions usage shape.
#[derive(Debug, Default, Clone, Copy)]
pub struct StandardChatUsageDecoder;

impl ChatUsageDecoder for StandardChatUsageDecoder {
    fn decode(&self, usage: &Value) -> Option<TokenUsage> {
        let input_tokens = usage.get("prompt_tokens")?.as_i64()?;
        let output_tokens = usage.get("completion_tokens")?.as_i64()?;
        let total_tokens = usage.get("total_tokens")?.as_i64()?;
        let cached_input_tokens = usage
            .get("prompt_tokens_details")
            .and_then(|details| details.get("cached_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let reasoning_output_tokens = usage
            .get("completion_tokens_details")
            .and_then(|details| details.get("reasoning_tokens"))
            .and_then(Value::as_i64)
            .unwrap_or_default();

        Some(TokenUsage {
            input_tokens,
            cached_input_tokens,
            output_tokens,
            reasoning_output_tokens,
            total_tokens,
            ..TokenUsage::default()
        })
    }
}

/// Adapter for the standard Chat Completions request and SSE format.
#[derive(Debug, Default, Clone, Copy)]
pub struct StandardChatCompletionApi;

impl ChatCompletionApi for StandardChatCompletionApi {
    fn build_request(&self, input: ChatRequestInput<'_>) -> Result<ChatRequest, ApiError> {
        ChatRequestBuilder::new(input.model, input.instructions, input.input, input.tools)
            .conversation_id(input.conversation_id)
            .session_source(input.session_source)
            .build(input.provider)
    }

    fn spawn_stream(
        &self,
        response: StreamResponse,
        idle_timeout: Duration,
        telemetry: Option<Arc<dyn SseTelemetry>>,
    ) -> ResponseStream {
        spawn_chat_stream(
            response,
            idle_timeout,
            telemetry,
            Arc::new(StandardChatUsageDecoder),
        )
    }
}

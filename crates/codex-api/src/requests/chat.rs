use crate::error::ApiError;
use crate::provider::Provider;
use crate::requests::headers::build_conversation_headers;
use crate::requests::headers::insert_header;
use crate::requests::headers::subagent_header;
use codex_protocol::models::ContentItem;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::ImageReference;
use codex_protocol::models::ReasoningItemContent;
use codex_protocol::models::ResponseItem;
use codex_protocol::protocol::SessionSource;
use http::HeaderMap;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;

/// Assembled request body plus headers for Chat Completions streaming calls.
pub struct ChatRequest {
    pub body: Value,
    pub headers: HeaderMap,
}

pub struct ChatRequestBuilder<'a> {
    model: &'a str,
    instructions: &'a str,
    input: &'a [ResponseItem],
    tools: &'a [Value],
    conversation_id: Option<String>,
    session_source: Option<SessionSource>,
}

impl<'a> ChatRequestBuilder<'a> {
    pub fn new(
        model: &'a str,
        instructions: &'a str,
        input: &'a [ResponseItem],
        tools: &'a [Value],
    ) -> Self {
        Self {
            model,
            instructions,
            input,
            tools,
            conversation_id: None,
            session_source: None,
        }
    }

    pub fn conversation_id(mut self, id: Option<String>) -> Self {
        self.conversation_id = id;
        self
    }

    pub fn session_source(mut self, source: Option<SessionSource>) -> Self {
        self.session_source = source;
        self
    }

    pub fn build(self, _provider: &Provider) -> Result<ChatRequest, ApiError> {
        let mut messages = vec![json!({"role": "system", "content": self.instructions})];
        let mut reasoning_by_anchor_index = HashMap::<usize, String>::new();

        let last_emitted_role = self.input.iter().rev().find_map(|item| match item {
            ResponseItem::Message { role, .. } => Some(role.as_str()),
            ResponseItem::FunctionCall { .. } | ResponseItem::LocalShellCall { .. } => {
                Some("assistant")
            }
            ResponseItem::FunctionCallOutput { .. } | ResponseItem::CustomToolCallOutput { .. } => {
                Some("tool")
            }
            _ => None,
        });

        let last_user_index = self.input.iter().rposition(ResponseItem::is_user_message);
        if last_emitted_role != Some("user") {
            for (idx, item) in self.input.iter().enumerate() {
                if last_user_index.is_some_and(|user_idx| idx <= user_idx) {
                    continue;
                }
                if let ResponseItem::Reasoning {
                    content: Some(items),
                    ..
                } = item
                {
                    let text = items
                        .iter()
                        .map(|entry| match entry {
                            ReasoningItemContent::ReasoningText { text }
                            | ReasoningItemContent::Text { text } => text.as_str(),
                        })
                        .collect::<String>();
                    if text.trim().is_empty() {
                        continue;
                    }

                    let mut attached = false;
                    if idx > 0
                        && matches!(
                            &self.input[idx - 1],
                            ResponseItem::Message { role, .. } if role == "assistant"
                        )
                    {
                        reasoning_by_anchor_index
                            .entry(idx - 1)
                            .and_modify(|value| value.push_str(&text))
                            .or_insert_with(|| text.clone());
                        attached = true;
                    }
                    let next_is_anchor = idx + 1 < self.input.len()
                        && (matches!(
                            &self.input[idx + 1],
                            ResponseItem::Message { role, .. } if role == "assistant"
                        ) || matches!(
                            &self.input[idx + 1],
                            ResponseItem::FunctionCall { .. }
                                | ResponseItem::LocalShellCall { .. }
                                | ResponseItem::CustomToolCall { .. }
                        ));
                    if !attached && next_is_anchor {
                        reasoning_by_anchor_index
                            .entry(idx + 1)
                            .and_modify(|value| value.push_str(&text))
                            .or_insert(text);
                    }
                }
            }
        }

        let mut last_assistant_text: Option<String> = None;
        for (idx, item) in self.input.iter().enumerate() {
            match item {
                ResponseItem::Message { role, content, .. } => {
                    let mut text = String::new();
                    let mut parts = Vec::new();
                    let mut saw_image = false;
                    for content_item in content {
                        match content_item {
                            ContentItem::InputText { text: value }
                            | ContentItem::OutputText { text: value } => {
                                text.push_str(value);
                                parts.push(json!({"type": "text", "text": value}));
                            }
                            ContentItem::InputImage { image, .. } => {
                                saw_image = true;
                                parts.push(json!({
                                    "type": "image_url",
                                    "image_url": {"url": image_url(image)},
                                }));
                            }
                            _ => {}
                        }
                    }

                    if role == "assistant" {
                        if last_assistant_text.as_deref() == Some(text.as_str()) {
                            continue;
                        }
                        last_assistant_text = Some(text.clone());
                    }

                    let content_value = if role == "assistant" || !saw_image {
                        json!(text)
                    } else {
                        json!(parts)
                    };
                    let mut message = json!({"role": role, "content": content_value});
                    if role == "assistant"
                        && let Some(reasoning) = reasoning_by_anchor_index.get(&idx)
                    {
                        message["reasoning"] = json!(reasoning);
                    }
                    messages.push(message);
                }
                ResponseItem::FunctionCall {
                    name,
                    arguments,
                    call_id,
                    ..
                } => {
                    let reasoning = reasoning_by_anchor_index.get(&idx).map(String::as_str);
                    push_tool_call_message(
                        &mut messages,
                        json!({
                            "id": call_id,
                            "type": "function",
                            "function": {"name": name, "arguments": arguments},
                        }),
                        reasoning,
                    );
                }
                ResponseItem::LocalShellCall {
                    id,
                    call_id,
                    status,
                    action,
                    ..
                } => {
                    let reasoning = reasoning_by_anchor_index.get(&idx).map(String::as_str);
                    push_tool_call_message(
                        &mut messages,
                        json!({
                            "id": id.as_ref().map(ToString::to_string).unwrap_or_default(),
                            "type": "local_shell_call",
                            "call_id": call_id,
                            "status": status,
                            "action": action,
                        }),
                        reasoning,
                    );
                }
                ResponseItem::FunctionCallOutput {
                    call_id, output, ..
                } => {
                    if let Some(call_id) = call_id {
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": call_id,
                            "content": output_value(output),
                        }));
                    }
                }
                ResponseItem::CustomToolCall {
                    id, name, input, ..
                } => {
                    let reasoning = reasoning_by_anchor_index.get(&idx).map(String::as_str);
                    push_tool_call_message(
                        &mut messages,
                        json!({
                            "id": id,
                            "type": "custom",
                            "custom": {"name": name, "input": input},
                        }),
                        reasoning,
                    );
                }
                ResponseItem::CustomToolCallOutput {
                    call_id, output, ..
                } => {
                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": output_value(output),
                    }));
                }
                _ => {}
            }
        }

        let payload = json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
            "tools": self.tools,
        });
        let mut headers = build_conversation_headers(self.conversation_id);
        if let Some(subagent) = subagent_header(&self.session_source) {
            insert_header(&mut headers, "x-openai-subagent", &subagent);
        }
        Ok(ChatRequest {
            body: payload,
            headers,
        })
    }
}

fn image_url(image: &ImageReference) -> &str {
    match image {
        ImageReference::Inline { image_url } | ImageReference::File { file_id: image_url } => {
            image_url
        }
    }
}

fn output_value(output: &codex_protocol::models::FunctionCallOutputPayload) -> Value {
    if let Some(items) = output.content_items() {
        let mapped = items
            .iter()
            .filter_map(|item| match item {
                FunctionCallOutputContentItem::InputText { text } => {
                    Some(json!({"type": "text", "text": text}))
                }
                FunctionCallOutputContentItem::InputImage { image, .. } => Some(json!({
                    "type": "image_url",
                    "image_url": {"url": image_url(image)},
                })),
                _ => None,
            })
            .collect::<Vec<_>>();
        json!(mapped)
    } else {
        json!(output.text_content().unwrap_or_default())
    }
}

fn push_tool_call_message(messages: &mut Vec<Value>, tool_call: Value, reasoning: Option<&str>) {
    if let Some(Value::Object(object)) = messages.last_mut()
        && object.get("role").and_then(Value::as_str) == Some("assistant")
        && object.get("content").is_some_and(Value::is_null)
        && let Some(tool_calls) = object.get_mut("tool_calls").and_then(Value::as_array_mut)
    {
        tool_calls.push(tool_call);
        if let Some(reasoning) = reasoning {
            if let Some(Value::String(existing)) = object.get_mut("reasoning") {
                if !existing.is_empty() {
                    existing.push('\n');
                }
                existing.push_str(reasoning);
            } else {
                object.insert(
                    "reasoning".to_string(),
                    Value::String(reasoning.to_string()),
                );
            }
        }
        return;
    }

    let mut message = json!({
        "role": "assistant",
        "content": null,
        "tool_calls": [tool_call],
    });
    if let Some(reasoning) = reasoning {
        message["reasoning"] = json!(reasoning);
    }
    messages.push(message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::RetryConfig;
    use codex_protocol::models::FunctionCallOutputPayload;
    use codex_protocol::protocol::SessionSource;
    use codex_protocol::protocol::SubAgentSource;
    use http::HeaderValue;
    use pretty_assertions::assert_eq;
    use std::time::Duration;

    fn provider() -> Provider {
        Provider {
            name: "test".to_string(),
            base_url: "https://example.test/v1".to_string(),
            query_params: None,
            headers: HeaderMap::new(),
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
    fn attaches_conversation_and_subagent_headers() {
        let input = vec![ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: "hi".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        }];
        let request = ChatRequestBuilder::new("gpt-test", "instructions", &input, &[])
            .conversation_id(Some("conv-1".to_string()))
            .session_source(Some(SessionSource::SubAgent(SubAgentSource::Review)))
            .build(&provider())
            .expect("request");

        assert_eq!(
            request.headers.get("session_id"),
            Some(&HeaderValue::from_static("conv-1"))
        );
        assert_eq!(
            request.headers.get("x-openai-subagent"),
            Some(&HeaderValue::from_static("review"))
        );
    }

    #[test]
    fn groups_consecutive_tool_calls_into_one_assistant_message() {
        let mut input = vec![ResponseItem::Message {
            id: None,
            role: "user".to_string(),
            content: vec![ContentItem::InputText {
                text: "read these".to_string(),
            }],
            phase: None,
            internal_chat_message_metadata_passthrough: None,
        }];
        for (call_id, path) in [
            ("call-a", "a.txt"),
            ("call-b", "b.txt"),
            ("call-c", "c.txt"),
        ] {
            input.push(ResponseItem::FunctionCall {
                id: None,
                namespace: None,
                name: "read_file".to_string(),
                arguments: json!({"path": path}).to_string(),
                encrypted_function_args: None,
                call_id: call_id.to_string(),
                internal_chat_message_metadata_passthrough: None,
            });
            input.push(ResponseItem::FunctionCallOutput {
                id: None,
                call_id: Some(call_id.to_string()),
                name: None,
                namespace: None,
                output: FunctionCallOutputPayload::from_text(call_id.to_string()),
                internal_chat_message_metadata_passthrough: None,
            });
        }

        let request = ChatRequestBuilder::new("gpt-test", "instructions", &input, &[])
            .build(&provider())
            .expect("request");
        let messages = request.body["messages"].as_array().expect("messages array");

        assert_eq!(messages.len(), 6);
        assert_eq!(messages[2]["role"], "assistant");
        assert_eq!(messages[2]["content"], Value::Null);
        let tool_calls = messages[2]["tool_calls"]
            .as_array()
            .expect("tool_calls array");
        assert_eq!(tool_calls.len(), 3);
        assert_eq!(tool_calls[0]["id"], "call-a");
        assert_eq!(tool_calls[1]["id"], "call-b");
        assert_eq!(tool_calls[2]["id"], "call-c");
        assert_eq!(messages[3]["tool_call_id"], "call-a");
        assert_eq!(messages[4]["tool_call_id"], "call-b");
        assert_eq!(messages[5]["tool_call_id"], "call-c");
    }
}

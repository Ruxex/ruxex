use super::*;
use genex_model_provider::build_provider_text_controls;
use genex_models_manager::model_info::model_info_from_slug;
use genex_protocol::config_types::ServiceTier;
use genex_protocol::models::FunctionCallOutputPayload;
use genex_protocol::models::ImageDetail;
use genex_protocol::models::ImageReference;
use pretty_assertions::assert_eq;

fn prompt_with_image_outputs(detail: Option<ImageDetail>) -> Prompt {
    Prompt {
        input: vec![
            ResponseItem::Message {
                id: None,
                role: "user".to_string(),
                content: vec![
                    ContentItem::InputText {
                        text: "Describe this image.".to_string(),
                    },
                    ContentItem::InputImage {
                        image: ImageReference::Inline {
                            image_url: "https://example.com/image.png".to_string(),
                        },
                        detail,
                    },
                ],
                phase: None,
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::FunctionCallOutput {
                id: None,
                call_id: Some("function-call".to_string()),
                name: None,
                namespace: None,
                output: FunctionCallOutputPayload::from_content_items(vec![
                    FunctionCallOutputContentItem::InputImage {
                        image: ImageReference::Inline {
                            image_url: "data:image/png;base64,function".to_string(),
                        },
                        detail,
                    },
                ]),
                internal_chat_message_metadata_passthrough: None,
            },
            ResponseItem::CustomToolCallOutput {
                id: None,
                call_id: "custom-call".to_string(),
                name: None,
                output: FunctionCallOutputPayload::from_content_items(vec![
                    FunctionCallOutputContentItem::InputImage {
                        image: ImageReference::Inline {
                            image_url: "data:image/png;base64,custom".to_string(),
                        },
                        detail,
                    },
                ]),
                internal_chat_message_metadata_passthrough: None,
            },
        ],
        ..Default::default()
    }
}

#[test_case::test_case(Some(ImageDetail::Original), Some(ImageDetail::High); "original")]
#[test_case::test_case(Some(ImageDetail::High), Some(ImageDetail::High); "high")]
#[test_case::test_case(Some(ImageDetail::Auto), Some(ImageDetail::Auto); "auto")]
#[test_case::test_case(Some(ImageDetail::Low), Some(ImageDetail::Low); "low")]
#[test_case::test_case(None, None; "unspecified")]
fn request_copies_project_image_details_for_receiving_model(
    detail: Option<ImageDetail>,
    unsupported_detail: Option<ImageDetail>,
) {
    let prompt = prompt_with_image_outputs(detail);
    let original = prompt.input.clone();
    let mut model_info = model_info_from_slug("legacy-model");
    model_info.use_responses_lite = false;
    model_info.supports_image_detail_original = true;
    assert_eq!(
        prompt.get_formatted_input_for_request(&model_info),
        original
    );

    model_info.supports_image_detail_original = false;
    assert_eq!(
        prompt.get_formatted_input_for_request(&model_info),
        prompt_with_image_outputs(unsupported_detail).input
    );

    model_info.use_responses_lite = true;
    for supports_original in [false, true] {
        model_info.supports_image_detail_original = supports_original;
        assert_eq!(
            prompt.get_formatted_input_for_request(&model_info),
            prompt_with_image_outputs(/*detail*/ None).input
        );
    }
    assert_eq!(prompt.input, original);
}

fn chat_request(
    text: Option<genex_model_provider::ProviderTextControls>,
    service_tier: Option<String>,
) -> genex_model_provider::ChatRequest {
    genex_model_provider::ChatRequest {
        model: "legacy-model".to_string(),
        instructions: "i".to_string(),
        input: vec![],
        tools: vec![],
        metadata: std::collections::BTreeMap::new(),
        conversation_id: None,
        session_source: None,
        tool_choice: "auto".to_string(),
        parallel_tool_calls: true,
        reasoning: None,
        store: false,
        stream: true,
        stream_options: None,
        include: vec![],
        service_tier,
        prompt_cache_key: None,
        text,
        client_metadata: None,
        access_programs: None,
        transport: genex_model_provider::ChatTransportOptions {
            session_id: None,
            thread_id: None,
            extra_headers: std::collections::BTreeMap::new(),
            compression: genex_model_provider::ProviderRequestCompression::None,
            turn_state: None,
        },
    }
}

#[test]
fn builds_text_verbosity_when_set() {
    let text = build_provider_text_controls(
        Some(genex_protocol::config_types::Verbosity::Low),
        &None,
        false,
    );
    let v = serde_json::to_value(&text).expect("json");
    assert_eq!(v.get("verbosity").and_then(|s| s.as_str()), Some("low"));
}

#[test]
fn builds_text_schema_with_strict_format() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "answer": {"type": "string"}
        },
        "required": ["answer"],
    });
    let text = build_provider_text_controls(None, &Some(schema.clone()), true);
    let v = serde_json::to_value(&text).expect("json");
    let text = v;
    assert!(text.get("verbosity").is_none());
    let format = text.get("format").expect("format field");

    assert_eq!(
        format.get("name"),
        Some(&serde_json::Value::String("genex_output_schema".into()))
    );
    assert_eq!(
        format.get("type"),
        Some(&serde_json::Value::String("json_schema".into()))
    );
    assert_eq!(format.get("strict"), Some(&serde_json::Value::Bool(true)));
    assert_eq!(format.get("schema"), Some(&schema));
}

#[test]
fn serializes_text_schema_with_non_strict_format() {
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "answer": {"type": "string"},
            "rationale": {"type": "string"}
        },
        "required": ["answer"],
        "additionalProperties": false
    });
    let text = build_provider_text_controls(None, &Some(schema.clone()), false);
    let format = serde_json::to_value(text)
        .expect("json")
        .get("format")
        .cloned()
        .expect("format field");
    assert_eq!(format.get("strict"), Some(&serde_json::Value::Bool(false)));
    assert_eq!(format.get("schema"), Some(&schema));
}

#[test]
fn omits_text_when_not_set() {
    let req = chat_request(None, None);
    assert!(req.text.is_none());
}

#[test]
fn preserves_flex_service_tier_when_set() {
    let req = chat_request(None, Some(ServiceTier::Flex.to_string()));
    assert_eq!(req.service_tier.as_deref(), Some("flex"));
}

use anyhow::Result;
use core_test_support::responses;
use core_test_support::skip_if_no_network;
use core_test_support::submit_thread_settings;
use core_test_support::test_ruxex::test_ruxex;
use core_test_support::wait_for_event;
use core_test_support::wait_for_event_match;
use ruxex_core::TurnInputRequest;
use ruxex_features::Feature;
use ruxex_models_manager::test_support::test_models_response;
use ruxex_protocol::config_types::ReasoningSummary;
use ruxex_protocol::config_types::ServiceTier;
use ruxex_protocol::model_types::ModelCatalog;
use ruxex_protocol::model_types::ReasoningEffort;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::Op;
use ruxex_protocol::protocol::ThreadSettingsOverrides;
use ruxex_protocol::request_user_input::RequestUserInputAnswer;
use ruxex_protocol::request_user_input::RequestUserInputResponse;
use ruxex_protocol::user_input::UserInput;
use pretty_assertions::assert_eq;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ordinary_settings_updates_keep_the_running_turn_snapshot() -> Result<()> {
    skip_if_no_network!(Ok(()));
    let server = responses::start_mock_server().await;
    let response_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_response_created("response-1"),
                responses::ev_function_call(
                    "pause",
                    "request_user_input",
                    &json!({ "questions": [{
                        "id": "continue",
                        "header": "Continue",
                        "question": "Continue after the settings update?",
                        "options": [
                            { "label": "Yes", "description": "Continue the turn." },
                            { "label": "No", "description": "Stop the turn." }
                        ]
                    }] })
                    .to_string(),
                ),
                responses::ev_completed("response-1"),
            ]),
            responses::sse_completed("response-2"),
            responses::sse_completed("response-3"),
        ],
    )
    .await;
    let test = test_ruxex()
        .with_model("snapshot-model-a")
        .with_config(|config| {
            // This regression exercises ordinary turn construction without
            // enabling the diagnostic active-settings path.
            config
                .features
                .enable(Feature::DefaultModeRequestUserInput)
                .unwrap();
            let template = test_models_response()
                .unwrap()
                .models
                .into_iter()
                .find(|model| model.slug == "example-model")
                .unwrap();
            config.model_catalog = Some(ModelCatalog {
                models: ["snapshot-model-a", "snapshot-model-b"]
                    .into_iter()
                    .map(|slug| {
                        let mut model = template.clone();
                        model.slug = slug.to_string();
                        model.default_reasoning_summary = ReasoningSummary::Concise;
                        model
                    })
                    .collect(),
            });
            config.model_reasoning_effort = Some(ReasoningEffort::Low);
            config.model_reasoning_summary = None;
            config.service_tier = None;
        })
        .build_with_auto_env(&server)
        .await?;
    test.ruxex
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: "pause before continuing".into(),
            text_elements: Vec::new(),
        }]))
        .await?;
    let paused = wait_for_event_match(&test.ruxex, |event| match event {
        EventMsg::RequestUserInput(request) => Some(request.clone()),
        _ => None,
    })
    .await;

    let mut expected = test.ruxex.thread_settings_snapshot().await;
    submit_thread_settings(
        &test.ruxex,
        ThreadSettingsOverrides {
            model: Some("snapshot-model-b".to_string()),
            effort: Some(Some(ReasoningEffort::High)),
            summary: Some(ReasoningSummary::Detailed),
            service_tier: Some(Some(ServiceTier::Flex.request_value().to_string())),
            ..Default::default()
        },
    )
    .await?;
    expected.model = "snapshot-model-b".to_string();
    expected.reasoning_effort = Some(ReasoningEffort::High);
    expected.reasoning_summary = Some(ReasoningSummary::Detailed);
    expected.service_tier = Some(ServiceTier::Flex.request_value().to_string());
    expected.collaboration_mode = expected.collaboration_mode.with_updates(
        Some(expected.model.clone()),
        Some(expected.reasoning_effort.clone()),
        /*developer_instructions*/ None,
    );
    assert_eq!(test.ruxex.thread_settings_snapshot().await, expected);
    test.ruxex
        .submit(Op::UserInputAnswer {
            id: paused.turn_id.clone(),
            response: RequestUserInputResponse {
                answers: HashMap::from([(
                    "continue".to_string(),
                    RequestUserInputAnswer {
                        answers: vec!["Yes".to_string()],
                    },
                )]),
            },
        })
        .await?;
    wait_for_event(&test.ruxex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    test.submit_text_turn("start the next turn").await?;

    let requests = response_mock.requests();
    let actual = requests
        .iter()
        .map(|request| {
            let body = request.body_json();
            json!({
                "model": body["model"],
                "reasoning": body["reasoning"],
                "service_tier": body.get("service_tier"),
            })
        })
        .collect::<Vec<_>>();
    let original = json!({
        "model": "snapshot-model-a",
        "reasoning": { "effort": "low", "summary": "concise" },
        "service_tier": null,
    });
    assert_eq!(
        actual,
        vec![
            original.clone(),
            original,
            json!({
                "model": "snapshot-model-b",
                "reasoning": { "effort": "high", "summary": "detailed" },
                "service_tier": "flex",
            })
        ]
    );
    Ok(())
}

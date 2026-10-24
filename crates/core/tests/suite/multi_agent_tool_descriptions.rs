//! Verifies catalog descriptions reach V2 tools without changing their schemas or availability.

use anyhow::Result;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::sse;
use core_test_support::responses::sse_completed;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_ruxex::test_ruxex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use ruxex_core::config::AgentRoleConfig;
use ruxex_core::config::Constrained;
use ruxex_features::Feature;
use ruxex_protocol::model_types::ModelInfo;
use ruxex_protocol::model_types::MultiAgentToolMessages;
use ruxex_protocol::model_types::ToolMessage;
use ruxex_protocol::model_types::ToolMessages;
use ruxex_protocol::model_types::ToolMode;
use ruxex_protocol::protocol::AskForApproval;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::MultiAgentVersion;
use ruxex_protocol::protocol::Op;
use ruxex_protocol::protocol::TurnSettingsUpdate;
use ruxex_protocol::protocol::TurnSettingsUpdateOutcome;
use ruxex_protocol::request_user_input::RequestUserInputAnswer;
use ruxex_protocol::request_user_input::RequestUserInputResponse;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use test_case::test_case;

const TOOL_NAMES: [&str; 6] = [
    "spawn_agent",
    "send_message",
    "followup_task",
    "wait_agent",
    "interrupt_agent",
    "list_agents",
];

#[derive(Clone, Copy)]
enum Exposure {
    Namespaced,
    Plain,
    CodeMode,
    V1,
}

fn all_tool_messages(message: Value) -> Value {
    json!({
        "multi_agent": TOOL_NAMES
            .map(|name| {
                let mut message = message.clone();
                if let Some(description) = message["description"].as_str() {
                    message["description"] = json!(description.replace("TOOL_NAME", name));
                }
                (name.to_string(), message)
            })
            .into_iter()
            .collect::<serde_json::Map<String, Value>>()
    })
}

fn model_catalog_tool_messages(model_name: &str) -> ToolMessages {
    let description = |tool_name: &str| {
        Some(ToolMessage {
            description: Some(format!("{tool_name} description for {model_name}.")),
        })
    };
    ToolMessages {
        send_user_message_async: None,
        multi_agent: Some(MultiAgentToolMessages {
            spawn_agent: description("spawn_agent"),
            send_message: description("send_message"),
            followup_task: description("followup_task"),
            wait_agent: description("wait_agent"),
            interrupt_agent: description("interrupt_agent"),
            list_agents: description("list_agents"),
        }),
    }
}

fn configure_model_catalog(model: &mut ModelInfo, model_name: &str) {
    model.multi_agent_version = Some(MultiAgentVersion::V2);
    model.model_messages.as_mut().expect("model messages").tools =
        Some(model_catalog_tool_messages(model_name));
}

#[test_case(json!(null), Exposure::Namespaced; "missing_tools")]
#[test_case(json!({}), Exposure::Namespaced; "missing_multi_agent")]
#[test_case(json!({"multi_agent": null}), Exposure::Namespaced; "null_multi_agent")]
#[test_case(json!({"multi_agent": {}}), Exposure::Namespaced; "missing_tools_in_family")]
#[test_case(all_tool_messages(json!(null)), Exposure::Namespaced; "null_tool")]
#[test_case(all_tool_messages(json!({})), Exposure::Namespaced; "missing_description")]
#[test_case(all_tool_messages(json!({"description": null})), Exposure::Namespaced; "null_description")]
#[test_case(all_tool_messages(json!({"description": "  Catalog TOOL_NAME description.\n{{literal_placeholder}}  "})), Exposure::Namespaced; "catalog_description")]
#[test_case(all_tool_messages(json!({"description": ""})), Exposure::Namespaced; "empty_description")]
#[test_case(json!({"multi_agent": {"send_message": {"description": "Catalog send."}}}), Exposure::Namespaced; "sparse_sibling_fallback")]
#[test_case(all_tool_messages(json!({"description": "Catalog TOOL_NAME description."})), Exposure::Plain; "plain_tools")]
#[test_case(all_tool_messages(json!({"description": "Catalog TOOL_NAME description."})), Exposure::CodeMode; "code_mode_declarations")]
#[test_case(all_tool_messages(json!({"description": ""})), Exposure::CodeMode; "empty_code_mode_descriptions")]
#[test_case(all_tool_messages(json!({"description": "Catalog TOOL_NAME description."})), Exposure::V1; "v1_unchanged")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_agent_catalog_descriptions_preserve_outbound_schema(
    tool_messages: Value,
    exposure: Exposure,
) -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response = mount_sse_sequence(
        &server,
        vec![sse_completed("resp-default"), sse_completed("resp-catalog")],
    )
    .await;
    for messages in [
        None,
        serde_json::from_value::<Option<ToolMessages>>(tool_messages.clone())?,
    ] {
        let test = test_ruxex()
            .with_model_info_override("example-model-2", move |model| {
                model.multi_agent_version = Some(if matches!(exposure, Exposure::V1) {
                    MultiAgentVersion::V1
                } else {
                    MultiAgentVersion::V2
                });
                if matches!(exposure, Exposure::CodeMode) {
                    model.tool_mode = Some(ToolMode::CodeMode);
                }
                model.model_messages.as_mut().expect("model messages").tools = messages;
            })
            .with_config(move |config| {
                if matches!(exposure, Exposure::V1) {
                    config.features.enable(Feature::Collab).expect("enable V1");
                    config
                        .features
                        .disable(Feature::MultiAgentV2)
                        .expect("disable V2");
                } else {
                    config
                        .features
                        .enable(Feature::MultiAgentV2)
                        .expect("enable V2");
                }
                if matches!(exposure, Exposure::CodeMode) {
                    config
                        .features
                        .enable(Feature::CodeMode)
                        .expect("enable Code Mode");
                    config.code_mode.disable_in_process_fallback = true;
                    config.multi_agent_v2.non_code_mode_only = false;
                }
                config.multi_agent_v2.tool_namespace =
                    (!matches!(exposure, Exposure::Plain)).then(|| "delegation".to_string());
                config.multi_agent_v2.hide_spawn_agent_metadata = false;
                config.multi_agent_v2.expose_spawn_agent_model_overrides = true;
                config.multi_agent_v2.usage_hint_text = Some("Local delegation hint.".to_string());
                config.agent_roles.insert(
                    "researcher".to_string(),
                    AgentRoleConfig {
                        description: Some("Research the assigned question.".to_string()),
                        config_file: None,
                        nickname_candidates: None,
                    },
                );
            })
            .build_with_auto_env(&server)
            .await?;
        test.submit_turn("Inspect the available tools.").await?;
    }

    let requests = response.requests();
    assert_eq!(requests.len(), 2);
    let mut expected = requests[0].body_json()["tools"].clone();
    let actual = requests[1].body_json()["tools"].clone();
    if !matches!(exposure, Exposure::V1) {
        let tools = if matches!(exposure, Exposure::Plain) {
            expected.as_array_mut().expect("plain tools")
        } else {
            expected
                .as_array_mut()
                .expect("tools")
                .iter_mut()
                .find(|tool| tool["name"] == "delegation")
                .expect("delegation namespace")["tools"]
                .as_array_mut()
                .expect("namespace tools")
        };
        let actual_tools = if matches!(exposure, Exposure::Plain) {
            actual.as_array().expect("plain tools")
        } else {
            actual
                .as_array()
                .expect("tools")
                .iter()
                .find(|tool| tool["name"] == "delegation")
                .expect("delegation namespace")["tools"]
                .as_array()
                .expect("namespace tools")
        };
        for name in TOOL_NAMES {
            let expected_tool = tools
                .iter_mut()
                .find(|tool| tool["name"] == name)
                .expect(name);
            let actual_tool = actual_tools
                .iter()
                .find(|tool| tool["name"] == name)
                .expect(name);
            if let Some(description) = tool_messages["multi_agent"][name]["description"].as_str() {
                let bundled = expected_tool["description"]
                    .as_str()
                    .expect("bundled description");
                let declaration = bundled
                    .find("\n\nexec tool declaration:")
                    .map(|index| &bundled[index..])
                    .unwrap_or_default();
                if matches!(exposure, Exposure::CodeMode) {
                    assert!(
                        declaration.contains(&format!("delegation__{name}")),
                        "Code Mode description for {name}: {bundled}"
                    );
                }
                let replacement = if name == "spawn_agent" {
                    let actual_description = actual_tool["description"]
                        .as_str()
                        .expect("spawn description");
                    assert!(actual_description.contains(description));
                    assert!(
                        !actual_description
                            .contains("Spawns an agent to work on the specified task.")
                    );
                    assert!(
                        actual_description
                            .strip_suffix(declaration)
                            .expect("unchanged declaration")
                            .ends_with("Local delegation hint.")
                    );
                    actual_description.to_string()
                } else {
                    format!("{description}{declaration}")
                };
                expected_tool["description"] = json!(replacement);
            }
        }
    }
    assert_eq!(actual, expected);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_agent_catalog_descriptions_follow_mid_turn_model_changes() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response = mount_sse_sequence(
        &server,
        vec![ruxex_test_pause_response(), sse_completed("resp-model-b")],
    )
    .await;
    let mut builder = test_ruxex()
        .with_model_info_override("provider-model-d", |model| {
            configure_model_catalog(model, "provider-model-d");
        })
        .with_model_info_override("example-model", |model| {
            configure_model_catalog(model, "example-model");
        })
        .with_model("provider-model-d")
        .with_config(|config| {
            config
                .features
                .enable(Feature::MultiAgentV2)
                .expect("enable V2");
            config
                .features
                .enable(Feature::DefaultModeRequestUserInput)
                .expect("enable request_user_input");
            config
                .features
                .enable(Feature::StepModelSwitching)
                .expect("enable step model switching");
            config.multi_agent_v2.tool_namespace = Some("delegation".to_string());
            config.multi_agent_v2.expose_spawn_agent_model_overrides = false;
            config.multi_agent_v2.usage_hint_text = Some("Local delegation hint.".to_string());
            config.permissions.approval_policy = Constrained::allow_any(AskForApproval::OnRequest);
        });
    let test = builder.build_with_auto_env(&server).await?;

    test.ruxex
        .start_or_steer_turn(ruxex_core::TurnInputRequest::user_input(vec![
            ruxex_protocol::user_input::UserInput::Text {
                text: "pause before changing model".into(),
                text_elements: Vec::new(),
            },
        ]))
        .await?;
    let request = core_test_support::wait_for_event_match(&test.ruxex, |event| match event {
        EventMsg::RequestUserInput(request) => Some(request.clone()),
        _ => None,
    })
    .await;

    let (reply, outcome) = tokio::sync::oneshot::channel();
    test.ruxex
        .submit(Op::TurnSettings {
            turn_id: request.turn_id.clone(),
            update: TurnSettingsUpdate {
                model: Some("example-model".to_string()),
                ..Default::default()
            },
            reply,
        })
        .await?;
    assert_eq!(outcome.await?, TurnSettingsUpdateOutcome::Applied);
    test.ruxex
        .submit(Op::UserInputAnswer {
            id: request.turn_id,
            response: RequestUserInputResponse {
                answers: HashMap::from([(
                    "continue".to_string(),
                    RequestUserInputAnswer {
                        answers: vec!["Yes (Recommended)".to_string()],
                    },
                )]),
            },
        })
        .await?;
    wait_for_event(&test.ruxex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;

    let requests = response.requests();
    assert_eq!(requests.len(), 2);
    for (request, model_name) in requests.iter().zip(["provider-model-d", "example-model"]) {
        let body = request.body_json();
        assert_eq!(body["model"], model_name);
        let tools = body
            .get("tools")
            .and_then(Value::as_array)
            .or_else(|| {
                body["input"]
                    .as_array()?
                    .iter()
                    .find_map(|item| item.get("tools").and_then(Value::as_array))
            })
            .unwrap_or_else(|| panic!("request {model_name} has no tools: {body}"));
        for tool_name in TOOL_NAMES {
            let tool = tools
                .iter()
                .find(|tool| tool["name"] == "delegation")
                .expect("delegation namespace")["tools"]
                .as_array()
                .expect("namespace tools")
                .iter()
                .find(|tool| tool["name"] == tool_name)
                .expect(tool_name);
            let description = tool["description"].as_str().expect("tool description");
            assert!(description.contains(&format!("{tool_name} description for {model_name}.")));
        }
    }

    Ok(())
}

fn ruxex_test_pause_response() -> String {
    sse(vec![
        ev_response_created("resp-model-a"),
        ev_function_call(
            "pause-turn",
            "request_user_input",
            &serde_json::to_string(&json!({
                "questions": [{
                    "id": "continue",
                    "header": "Continue",
                    "question": "Continue after settings update?",
                    "options": [
                        {"label": "Yes (Recommended)", "description": "Continue the current turn."},
                        {"label": "No", "description": "Stop the current turn."}
                    ]
                }]
            }))
            .expect("serialize pause arguments"),
        ),
        ev_completed("resp-model-a"),
    ])
}

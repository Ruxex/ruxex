use anyhow::Result;
use app_test_support::create_mock_chat_completions_server_retry;
use app_test_support::create_mock_chat_completions_server_sequence;
use app_test_support::create_mock_chat_completions_server_sequence_unchecked;
use app_test_support::create_mock_responses_server_retry;
use core_test_support::responses;
use ruxex_app_server::AppServerRuntimeBuilder;
use ruxex_app_server::in_process;
use ruxex_app_server::in_process::InProcessClientHandle;
use ruxex_app_server::in_process::InProcessServerEvent;
use ruxex_app_server::in_process::InProcessStartArgs;
use ruxex_app_server_protocol::ClientInfo;
use ruxex_app_server_protocol::ClientRequest;
use ruxex_app_server_protocol::DynamicToolCallOutputContentItem;
use ruxex_app_server_protocol::DynamicToolCallResponse;
use ruxex_app_server_protocol::DynamicToolCallStatus;
use ruxex_app_server_protocol::DynamicToolFunctionSpec;
use ruxex_app_server_protocol::DynamicToolSpec;
use ruxex_app_server_protocol::InitializeCapabilities;
use ruxex_app_server_protocol::InitializeParams;
use ruxex_app_server_protocol::RequestId;
use ruxex_app_server_protocol::ServerNotification;
use ruxex_app_server_protocol::ServerRequest;
use ruxex_app_server_protocol::ThreadItem;
use ruxex_app_server_protocol::ThreadResumeParams;
use ruxex_app_server_protocol::ThreadResumeResponse;
use ruxex_app_server_protocol::ThreadStartParams;
use ruxex_app_server_protocol::ThreadStartResponse;
use ruxex_app_server_protocol::TurnCompletedNotification;
use ruxex_app_server_protocol::TurnInterruptParams;
use ruxex_app_server_protocol::TurnStartParams;
use ruxex_app_server_protocol::TurnStartResponse;
use ruxex_app_server_protocol::TurnStatus;
use ruxex_app_server_protocol::UserInput as V2UserInput;
use ruxex_arg0::Arg0DispatchPaths;
use ruxex_config::LoaderOverrides;
use ruxex_config::NoopThreadConfigLoader;
use ruxex_core::config::ConfigBuilder;
use ruxex_exec_server::EnvironmentManager;
use ruxex_login::CredentialManager;
use ruxex_login::RuxexAuth;
use ruxex_model_provider_info::ModelProviderInfo;
use ruxex_model_provider_info::WireApi;
use ruxex_protocol::protocol::SessionSource;
use ruxex_utils_home_dir::ProductIdentity;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use wiremock::Mock;
use wiremock::ResponseTemplate;
use wiremock::matchers::method;
use wiremock::matchers::path_regex;

#[tokio::test]
async fn chat_completions_regression_round_trip() -> Result<()> {
    run_regression_round_trip(WireApi::Chat).await
}

#[tokio::test]
async fn responses_regression_round_trip() -> Result<()> {
    run_regression_round_trip(WireApi::Responses).await
}

#[tokio::test]
async fn chat_completions_regression_provider_error() -> Result<()> {
    run_provider_error_regression(WireApi::Chat).await
}

#[tokio::test]
async fn responses_regression_provider_error() -> Result<()> {
    run_provider_error_regression(WireApi::Responses).await
}

#[tokio::test]
async fn chat_completions_regression_tool_call_round_trip() -> Result<()> {
    run_tool_call_regression(WireApi::Chat).await
}

#[tokio::test]
async fn responses_regression_tool_call_round_trip() -> Result<()> {
    run_tool_call_regression(WireApi::Responses).await
}

#[tokio::test]
async fn chat_completions_regression_interrupts_pending_tool_call() -> Result<()> {
    run_interrupt_regression(WireApi::Chat).await
}

#[tokio::test]
async fn responses_regression_interrupts_pending_tool_call() -> Result<()> {
    run_interrupt_regression(WireApi::Responses).await
}

#[tokio::test]
async fn chat_completions_regression_resumes_conversation() -> Result<()> {
    run_resume_regression(WireApi::Chat).await
}

#[tokio::test]
async fn responses_regression_resumes_conversation() -> Result<()> {
    run_resume_regression(WireApi::Responses).await
}

#[tokio::test]
async fn chat_completions_regression_retries_provider_failure() -> Result<()> {
    run_retry_regression(WireApi::Chat).await
}

#[tokio::test]
async fn responses_regression_retries_provider_failure() -> Result<()> {
    run_retry_regression(WireApi::Responses).await
}

async fn run_regression_round_trip(wire_api: WireApi) -> Result<()> {
    let (server, expected_message, expected_path) = match wire_api {
        WireApi::Chat => {
            let response_body = format!(
                "{}data: [DONE]\n\n",
                responses::sse(vec![
                    json!({
                        "type": "message",
                        "id": "chatcmpl-1",
                        "object": "chat.completion.chunk",
                        "created": 1,
                        "model": "mock-model",
                        "choices": [{
                            "index": 0,
                            "delta": {"role": "assistant", "content": "Hello from "},
                            "finish_reason": null
                        }]
                    }),
                    json!({
                        "type": "message",
                        "id": "chatcmpl-1",
                        "object": "chat.completion.chunk",
                        "created": 1,
                        "model": "mock-model",
                        "choices": [{"index": 0, "delta": {"content": "Chat Completions"}, "finish_reason": "stop"}],
                        "usage": {"prompt_tokens": 3, "completion_tokens": 5, "total_tokens": 8}
                    }),
                ])
            );
            (
                create_mock_chat_completions_server_sequence(vec![response_body]).await,
                "Hello from Chat Completions",
                "/v1/chat/completions",
            )
        }
        WireApi::Responses => (
            app_test_support::create_mock_responses_server_sequence(vec![responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_message_item_added("msg-1", ""),
                responses::ev_output_text_delta("Hello from "),
                responses::ev_output_text_delta("Responses"),
                responses::ev_assistant_message("msg-1", "Hello from Responses"),
                responses::ev_completed("resp-1"),
            ])])
            .await,
            "Hello from Responses",
            "/v1/responses",
        ),
    };

    let (mut app_server, _ruxex_home) = start_regression_app(wire_api, &server.uri()).await?;
    start_regression_turn(&mut app_server, "Say hello").await?;

    let mut assistant_message = None;
    let mut assistant_deltas = String::new();
    let completed = loop {
        let event = app_server
            .next_event()
            .await
            .ok_or_else(|| anyhow::anyhow!("in-process AppServer exited before turn completion"))?;
        if let InProcessServerEvent::ServerNotification(notification) = event {
            match *notification {
                ServerNotification::ItemCompleted(notification) => {
                    if let ThreadItem::AgentMessage { text, .. } = notification.item {
                        assistant_message = Some(text);
                    }
                }
                ServerNotification::AgentMessageDelta(notification) => {
                    assistant_deltas.push_str(&notification.delta);
                }
                ServerNotification::TurnCompleted(completed) => break completed,
                _ => {}
            }
        }
    };

    assert_eq!(
        completed.turn.status,
        TurnStatus::Completed,
        "turn error: {:?}",
        completed.turn.error
    );
    assert_eq!(assistant_message.as_deref(), Some(expected_message));
    assert_eq!(assistant_deltas, expected_message);
    app_server.shutdown().await?;

    let requests = server
        .received_requests()
        .await
        .expect("model request should be recorded");
    let model_requests = requests
        .iter()
        .filter(|request| request.url.path() == expected_path)
        .collect::<Vec<_>>();
    assert_eq!(model_requests.len(), 1);

    let request_body: serde_json::Value = model_requests[0]
        .body_json()
        .expect("model request should contain JSON");
    assert_eq!(request_body["model"], "mock-model");
    assert_eq!(request_body["stream"], true);
    match wire_api {
        WireApi::Chat => {
            assert!(request_body["messages"].is_array());
            assert!(request_body.get("input").is_none());
        }
        WireApi::Responses => {
            assert!(request_body["input"].is_array());
            assert!(request_body.get("messages").is_none());
        }
    }

    Ok(())
}

async fn run_provider_error_regression(wire_api: WireApi) -> Result<()> {
    let server = responses::start_mock_server().await;
    let (endpoint, expected_path) = match wire_api {
        WireApi::Chat => (".*/chat/completions$", "/v1/chat/completions"),
        WireApi::Responses => (".*/responses$", "/v1/responses"),
    };
    Mock::given(method("POST"))
        .and(path_regex(endpoint))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": {"code": "server_error", "message": "simulated provider failure"}
        })))
        .expect(1)
        .mount(&server)
        .await;

    let (mut app_server, _ruxex_home) = start_regression_app(wire_api, &server.uri()).await?;
    start_regression_turn(&mut app_server, "Trigger provider failure").await?;

    let completed = loop {
        let event = app_server
            .next_event()
            .await
            .ok_or_else(|| anyhow::anyhow!("in-process AppServer exited before turn failure"))?;
        if let InProcessServerEvent::ServerNotification(notification) = event
            && let ServerNotification::TurnCompleted(completed) = *notification
        {
            break completed;
        }
    };

    assert_eq!(completed.turn.status, TurnStatus::Failed);
    assert!(completed.turn.error.is_some());
    app_server.shutdown().await?;

    let requests = server
        .received_requests()
        .await
        .expect("provider request should be recorded");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.url.path() == expected_path)
            .count(),
        1
    );

    Ok(())
}

async fn run_tool_call_regression(wire_api: WireApi) -> Result<()> {
    let call_id = "regression-call-1";
    let server = match wire_api {
        WireApi::Chat => {
            let first = chat_tool_call_sse(call_id)?;
            let second = chat_assistant_sse("Done");
            create_mock_chat_completions_server_sequence(vec![first, second]).await
        }
        WireApi::Responses => {
            let first = responses::sse(vec![
                responses::ev_response_created("resp-tool-1"),
                responses::ev_function_call(call_id, "demo_tool", "{}"),
                responses::ev_completed("resp-tool-1"),
            ]);
            let second = app_test_support::create_final_assistant_message_sse_response("Done")?;
            app_test_support::create_mock_responses_server_sequence(vec![first, second]).await
        }
    };

    let (mut app_server, _ruxex_home) = start_regression_app(wire_api, &server.uri()).await?;
    start_regression_turn_with_options(
        &mut app_server,
        "Run the tool",
        Some(vec![demo_dynamic_tool()]),
    )
    .await?;

    let mut tool_status = None;
    let mut assistant_message = None;
    let completed = loop {
        let event = app_server
            .next_event()
            .await
            .ok_or_else(|| anyhow::anyhow!("in-process AppServer exited before tool completion"))?;
        if let InProcessServerEvent::ServerRequest(request) = event {
            let ServerRequest::DynamicToolCall { request_id, params } = *request else {
                continue;
            };
            assert_eq!(params.call_id, call_id);
            app_server.respond_to_server_request(
                request_id,
                serde_json::to_value(DynamicToolCallResponse {
                    content_items: vec![DynamicToolCallOutputContentItem::InputText {
                        text: "dynamic-tool-ok".to_string(),
                    }],
                    success: true,
                })?,
            )?;
        } else if let InProcessServerEvent::ServerNotification(notification) = event {
            match *notification {
                ServerNotification::ItemCompleted(notification) => match notification.item {
                    ThreadItem::DynamicToolCall { status, .. } => {
                        tool_status = Some(status);
                    }
                    ThreadItem::AgentMessage { text, .. } => {
                        assistant_message = Some(text);
                    }
                    _ => {}
                },
                ServerNotification::TurnCompleted(completed) => break completed,
                _ => {}
            }
        }
    };

    assert_eq!(completed.turn.status, TurnStatus::Completed);
    assert_eq!(
        tool_status,
        Some(DynamicToolCallStatus::Completed),
        "the model tool call should complete successfully"
    );
    assert_eq!(assistant_message.as_deref(), Some("Done"));
    app_server.shutdown().await?;

    let requests = server
        .received_requests()
        .await
        .expect("tool-call model requests should be recorded");
    let model_requests = requests
        .iter()
        .filter(|request| {
            request.url.path()
                == match wire_api {
                    WireApi::Chat => "/v1/chat/completions",
                    WireApi::Responses => "/v1/responses",
                }
        })
        .collect::<Vec<_>>();
    assert_eq!(model_requests.len(), 2);
    let follow_up: serde_json::Value = model_requests[1]
        .body_json()
        .expect("tool-call follow-up should contain JSON");
    match wire_api {
        WireApi::Chat => assert!(
            follow_up["messages"]
                .as_array()
                .is_some_and(|messages| messages.iter().any(|message| message["role"] == "tool"))
        ),
        WireApi::Responses => assert!(follow_up["input"].as_array().is_some_and(|items| {
            items
                .iter()
                .any(|item| item["type"] == "function_call_output")
        })),
    }

    Ok(())
}

async fn run_interrupt_regression(wire_api: WireApi) -> Result<()> {
    let call_id = "regression-interrupt-call";
    let first = match wire_api {
        WireApi::Chat => chat_tool_call_sse(call_id)?,
        WireApi::Responses => responses::sse(vec![
            responses::ev_response_created("resp-interrupt-1"),
            responses::ev_function_call(call_id, "demo_tool", "{}"),
            responses::ev_completed("resp-interrupt-1"),
        ]),
    };
    let server = match wire_api {
        WireApi::Chat => create_mock_chat_completions_server_sequence(vec![first]).await,
        WireApi::Responses => {
            app_test_support::create_mock_responses_server_sequence(vec![first]).await
        }
    };

    let (mut app_server, _ruxex_home) = start_regression_app(wire_api, &server.uri()).await?;
    let (thread_id, turn_id) = start_regression_turn_with_options(
        &mut app_server,
        "Interrupt the tool",
        Some(vec![demo_dynamic_tool()]),
    )
    .await?;

    let completed = loop {
        let event = app_server
            .next_event()
            .await
            .ok_or_else(|| anyhow::anyhow!("in-process AppServer exited before interruption"))?;
        if let InProcessServerEvent::ServerRequest(request) = event {
            let ServerRequest::DynamicToolCall {
                request_id: _,
                params,
            } = *request
            else {
                continue;
            };
            assert_eq!(params.call_id, call_id);
            assert_eq!(params.thread_id, thread_id);
            assert_eq!(params.turn_id, turn_id);
            app_server
                .request(ClientRequest::TurnInterrupt {
                    request_id: RequestId::Integer(3),
                    params: TurnInterruptParams {
                        thread_id: thread_id.clone(),
                        turn_id: turn_id.clone(),
                    },
                })
                .await?
                .map_err(|error| anyhow::anyhow!("turn/interrupt failed: {}", error.message))?;
        } else if let InProcessServerEvent::ServerNotification(notification) = event
            && let ServerNotification::TurnCompleted(completed) = *notification
        {
            break completed;
        }
    };

    assert_eq!(completed.turn.status, TurnStatus::Interrupted);
    app_server.shutdown().await?;
    let requests = server
        .received_requests()
        .await
        .expect("interrupted tool request should be recorded");
    let expected_path = match wire_api {
        WireApi::Chat => "/v1/chat/completions",
        WireApi::Responses => "/v1/responses",
    };
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.url.path() == expected_path)
            .count(),
        1
    );
    Ok(())
}

async fn run_resume_regression(wire_api: WireApi) -> Result<()> {
    let server = match wire_api {
        WireApi::Chat => {
            create_mock_chat_completions_server_sequence_unchecked(vec![
                chat_assistant_sse("First"),
                chat_assistant_sse("Second"),
            ])
            .await
        }
        WireApi::Responses => {
            app_test_support::create_mock_responses_server_sequence_unchecked(vec![
                responses::sse(vec![
                    responses::ev_response_created("resp-resume-1"),
                    responses::ev_assistant_message("msg-resume-1", "First"),
                    responses::ev_completed("resp-resume-1"),
                ]),
                responses::sse(vec![
                    responses::ev_response_created("resp-resume-2"),
                    responses::ev_assistant_message("msg-resume-2", "Second"),
                    responses::ev_completed("resp-resume-2"),
                ]),
            ])
            .await
        }
    };

    let (mut app_server, _ruxex_home) = start_regression_app(wire_api, &server.uri()).await?;
    let (thread_id, _) = start_regression_turn(&mut app_server, "First turn").await?;
    let (completed, message) = wait_for_turn_completed(&mut app_server).await?;
    assert_eq!(completed.turn.status, TurnStatus::Completed);
    assert_eq!(message.as_deref(), Some("First"));

    let resume_response = app_server
        .request(ClientRequest::ThreadResume {
            request_id: RequestId::Integer(3),
            params: ThreadResumeParams {
                thread_id: thread_id.clone(),
                ..Default::default()
            },
        })
        .await?
        .map_err(|error| anyhow::anyhow!("thread/resume failed: {}", error.message))?;
    let _: ThreadResumeResponse = serde_json::from_value(resume_response)?;

    let _ = start_regression_turn(&mut app_server, "Second turn").await?;
    let (completed, message) = wait_for_turn_completed(&mut app_server).await?;
    assert_eq!(completed.turn.status, TurnStatus::Completed);
    assert_eq!(message.as_deref(), Some("Second"));
    app_server.shutdown().await?;

    let expected_path = match wire_api {
        WireApi::Chat => "/v1/chat/completions",
        WireApi::Responses => "/v1/responses",
    };
    let requests = server
        .received_requests()
        .await
        .expect("resumed model requests should be recorded");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.url.path() == expected_path)
            .count(),
        2
    );
    Ok(())
}

async fn run_retry_regression(wire_api: WireApi) -> Result<()> {
    let response = match wire_api {
        WireApi::Chat => chat_assistant_sse("Retried"),
        WireApi::Responses => responses::sse(vec![
            responses::ev_response_created("resp-retry-1"),
            responses::ev_assistant_message("msg-retry-1", "Retried"),
            responses::ev_completed("resp-retry-1"),
        ]),
    };
    let server = match wire_api {
        WireApi::Chat => create_mock_chat_completions_server_retry(response).await,
        WireApi::Responses => create_mock_responses_server_retry(response).await,
    };

    let (mut app_server, _ruxex_home) =
        start_regression_app_with_retries(wire_api, &server.uri(), 1).await?;
    start_regression_turn(&mut app_server, "Retry this request").await?;
    let (completed, message) = wait_for_turn_completed(&mut app_server).await?;
    assert_eq!(completed.turn.status, TurnStatus::Completed);
    assert_eq!(message.as_deref(), Some("Retried"));
    app_server.shutdown().await?;

    let expected_path = match wire_api {
        WireApi::Chat => "/v1/chat/completions",
        WireApi::Responses => "/v1/responses",
    };
    let requests = server
        .received_requests()
        .await
        .expect("retried model requests should be recorded");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.url.path() == expected_path)
            .count(),
        2
    );
    Ok(())
}

async fn wait_for_turn_completed(
    app_server: &mut InProcessClientHandle,
) -> Result<(TurnCompletedNotification, Option<String>)> {
    let mut assistant_message = None;
    loop {
        let event = app_server
            .next_event()
            .await
            .ok_or_else(|| anyhow::anyhow!("in-process AppServer exited before turn completion"))?;
        if let InProcessServerEvent::ServerNotification(notification) = event {
            match *notification {
                ServerNotification::ItemCompleted(notification) => {
                    if let ThreadItem::AgentMessage { text, .. } = notification.item {
                        assistant_message = Some(text);
                    }
                }
                ServerNotification::TurnCompleted(completed) => {
                    return Ok((completed, assistant_message));
                }
                _ => {}
            }
        }
    }
}

fn demo_dynamic_tool() -> DynamicToolSpec {
    DynamicToolSpec::Function(DynamicToolFunctionSpec {
        name: "demo_tool".to_string(),
        description: "A deterministic regression-test tool".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        }),
        defer_loading: false,
    })
}

fn chat_tool_call_sse(call_id: &str) -> Result<String> {
    let arguments = serde_json::to_string(&json!({}))?;
    Ok(format!(
        "{}data: [DONE]\n\n",
        responses::sse(vec![
            json!({
                "type": "message",
                "id": "chatcmpl-tool-1",
                "object": "chat.completion.chunk",
                "created": 1,
                "model": "mock-model",
                "choices": [{
                    "index": 0,
                    "delta": {
                        "role": "assistant",
                        "tool_calls": [{
                            "index": 0,
                            "id": call_id,
                            "type": "function",
                            "function": {
                                "name": "demo_tool",
                                "arguments": arguments,
                            }
                        }]
                    },
                    "finish_reason": null
                }]
            }),
            json!({
                "type": "message",
                "id": "chatcmpl-tool-1",
                "object": "chat.completion.chunk",
                "created": 1,
                "model": "mock-model",
                "choices": [{"index": 0, "delta": {}, "finish_reason": "tool_calls"}]
            }),
        ])
    ))
}

fn chat_assistant_sse(message: &str) -> String {
    format!(
        "{}data: [DONE]\n\n",
        responses::sse(vec![
            json!({
                "type": "message",
                "id": "chatcmpl-final-1",
                "object": "chat.completion.chunk",
                "created": 1,
                "model": "mock-model",
                "choices": [{
                    "index": 0,
                    "delta": {"role": "assistant", "content": message},
                    "finish_reason": null
                }]
            }),
            json!({
                "type": "message",
                "id": "chatcmpl-final-1",
                "object": "chat.completion.chunk",
                "created": 1,
                "model": "mock-model",
                "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}]
            }),
        ])
    )
}

async fn start_regression_app(
    wire_api: WireApi,
    server_uri: &str,
) -> Result<(InProcessClientHandle, TempDir)> {
    start_regression_app_with_retries(wire_api, server_uri, 0).await
}

async fn start_regression_app_with_retries(
    wire_api: WireApi,
    server_uri: &str,
    request_max_retries: u64,
) -> Result<(InProcessClientHandle, TempDir)> {
    let ruxex_home = TempDir::new()?;
    let loader_overrides = LoaderOverrides::without_managed_config_for_tests();
    let config = Arc::new(
        ConfigBuilder::default()
            .ruxex_home(ruxex_home.path().to_path_buf())
            .fallback_cwd(Some(ruxex_home.path().to_path_buf()))
            .loader_overrides(loader_overrides.clone())
            .build()
            .await?,
    );
    let mut provider_info =
        ModelProviderInfo::create_responses_provider(Some(format!("{server_uri}/v1")));
    provider_info.id = Some("mock_provider".to_string());
    provider_info.wire_api = wire_api;
    provider_info.supports_websockets = false;
    provider_info.request_max_retries = Some(request_max_retries);
    provider_info.stream_max_retries = Some(0);
    let credential_manager =
        CredentialManager::from_auth_for_testing(RuxexAuth::from_api_key("test-key"));
    let runtime = AppServerRuntimeBuilder::new(ruxex_providers::create_responses_provider(
        provider_info,
        Some(credential_manager),
    ))
    .build();
    ruxex_app_server::runtime::mount_provider_home_for_runtime(&config, &runtime)?;
    let state_db = ruxex_rollout::state_db::try_init(config.as_ref()).await?;

    let app_server = in_process::start(InProcessStartArgs {
        arg0_paths: Arg0DispatchPaths::default(),
        config,
        cli_overrides: Vec::new(),
        loader_overrides,
        strict_config: false,
        thread_config_loader: Arc::new(NoopThreadConfigLoader),
        log_db: None,
        state_db: Some(state_db),
        environment_manager: Arc::new(EnvironmentManager::default_for_tests()),
        config_warnings: Vec::new(),
        session_source: SessionSource::Cli,
        enable_api_key_env: false,
        runtime: Some(runtime),
        product_identity: ProductIdentity::ruxex(),
        initialize: InitializeParams {
            client_info: ClientInfo {
                name: "ruxex-app-server-regression-tests".to_string(),
                title: None,
                version: "0.1.0".to_string(),
            },
            capabilities: Some(InitializeCapabilities {
                experimental_api: true,
                ..Default::default()
            }),
        },
        channel_capacity: in_process::DEFAULT_IN_PROCESS_CHANNEL_CAPACITY,
    })
    .await?;
    Ok((app_server, ruxex_home))
}

async fn start_regression_turn(
    app_server: &mut InProcessClientHandle,
    input: &str,
) -> Result<(String, String)> {
    start_regression_turn_with_options(app_server, input, None).await
}

async fn start_regression_turn_with_options(
    app_server: &mut InProcessClientHandle,
    input: &str,
    dynamic_tools: Option<Vec<DynamicToolSpec>>,
) -> Result<(String, String)> {
    let thread_response = app_server
        .request(ClientRequest::ThreadStart {
            request_id: RequestId::Integer(1),
            params: ThreadStartParams {
                model: Some("mock-model".to_string()),
                dynamic_tools,
                ..Default::default()
            },
        })
        .await?
        .map_err(|error| anyhow::anyhow!("thread/start failed: {}", error.message))?;
    let thread: ThreadStartResponse = serde_json::from_value(thread_response)?;

    let thread_id = thread.thread.id;
    let turn_response = app_server
        .request(ClientRequest::TurnStart {
            request_id: RequestId::Integer(2),
            params: TurnStartParams {
                thread_id: thread_id.clone(),
                input: vec![V2UserInput::Text {
                    text: input.to_string(),
                    text_elements: Vec::new(),
                }],
                model: Some("mock-model".to_string()),
                ..Default::default()
            },
        })
        .await?
        .map_err(|error| anyhow::anyhow!("turn/start failed: {}", error.message))?;
    let turn: TurnStartResponse = serde_json::from_value(turn_response)?;
    Ok((thread_id, turn.turn.id))
}

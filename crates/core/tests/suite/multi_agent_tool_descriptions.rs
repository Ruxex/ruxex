//! Verifies catalog descriptions reach every retained Multi-Agent V2 tool.

use anyhow::Result;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::namespace_child_tool;
use core_test_support::responses::sse_completed;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_ruxex::test_ruxex;
use ruxex_core::config::AgentRoleConfig;
use ruxex_features::Feature;
use ruxex_protocol::model_types::MultiAgentToolMessages;
use ruxex_protocol::model_types::ToolMessage;
use ruxex_protocol::model_types::ToolMessages;
use ruxex_protocol::protocol::MultiAgentVersion;
use serde_json::json;

const MULTI_AGENT_TOOLS: [&str; 6] = [
    "spawn_agent",
    "send_message",
    "followup_task",
    "wait_agent",
    "interrupt_agent",
    "list_agents",
];

fn catalog_message(tool_name: &str) -> Option<ToolMessage> {
    Some(ToolMessage {
        description: Some(format!("Catalog {tool_name} description.")),
    })
}

fn catalog_tool_messages() -> ToolMessages {
    ToolMessages {
        send_user_message_async: None,
        multi_agent: Some(MultiAgentToolMessages {
            spawn_agent: catalog_message("spawn_agent"),
            send_message: catalog_message("send_message"),
            followup_task: catalog_message("followup_task"),
            wait_agent: catalog_message("wait_agent"),
            interrupt_agent: catalog_message("interrupt_agent"),
            list_agents: catalog_message("list_agents"),
        }),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn multi_agent_catalog_descriptions_reach_all_v2_tools() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let response = mount_sse_sequence(
        &server,
        vec![sse_completed("resp-default"), sse_completed("resp-catalog")],
    )
    .await;

    for messages in [None, Some(catalog_tool_messages())] {
        let test = test_ruxex()
            .with_model_info_override("example-model-2", move |model| {
                model.multi_agent_version = Some(MultiAgentVersion::V2);
                model.model_messages.as_mut().expect("model messages").tools = messages;
            })
            .with_config(|config| {
                config
                    .features
                    .enable(Feature::MultiAgentV2)
                    .expect("enable V2");
                config.multi_agent_v2.tool_namespace = Some("delegation".to_string());
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
    let default_tools = requests[0].body_json();
    let catalog_tools = requests[1].body_json();
    for name in MULTI_AGENT_TOOLS {
        let default_tool = namespace_child_tool(&default_tools, "delegation", name)
            .expect("default tool")
            .clone();
        let catalog_tool = namespace_child_tool(&catalog_tools, "delegation", name)
            .expect("catalog tool")
            .clone();
        let catalog_description = format!("Catalog {name} description.");

        if name == "spawn_agent" {
            let description = catalog_tool["description"].as_str().expect("description");
            assert!(description.contains(&catalog_description));
            assert!(description.ends_with("Local delegation hint."));
        } else {
            assert_eq!(catalog_tool["description"], catalog_description);
        }

        let mut expected = default_tool;
        expected["description"] = json!(catalog_tool["description"].clone());
        assert_eq!(catalog_tool, expected);
    }

    Ok(())
}

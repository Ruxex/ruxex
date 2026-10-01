use anyhow::Result;
use genex_config::Constrained;
use genex_core::EnvironmentConfig;
use genex_core::TurnInputRequest;
use genex_core::config::Config;
use genex_core::windows_sandbox::WindowsSandboxLevelExt;
use genex_extension_api::ExtensionFuture;
use genex_extension_api::ExtensionRegistryBuilder;
use genex_extension_api::McpServerContribution;
use genex_extension_api::McpServerContributionContext;
use genex_extension_api::McpServerContributor;
use genex_extension_api::ThreadLifecycleContributor;
use genex_extension_api::ThreadStartInput;
use genex_features::Feature;
use genex_history::RolloutItem;
use genex_mcp::McpResourceClient;
use genex_protocol::capabilities::CapabilityRootLocation;
use genex_protocol::capabilities::SelectedCapabilityRoot;
use genex_protocol::config_types::WindowsSandboxLevel;
use genex_protocol::models::PermissionProfile;
use genex_protocol::models::PermissionProfileSnapshot;
use genex_protocol::protocol::AskForApproval;
use genex_protocol::protocol::EventMsg;
use genex_protocol::protocol::Op;
use genex_protocol::protocol::SessionSource;
use genex_protocol::protocol::SubAgentSource;
use genex_protocol::user_input::UserInput;
use core_test_support::context_snapshot;
use core_test_support::context_snapshot::ContextSnapshotOptions;
use core_test_support::context_snapshot::SnapshotEntry;
use core_test_support::responses;
use core_test_support::responses::ResponsesRequest;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_function_call;
use core_test_support::responses::ev_function_call_with_namespace;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once_match;
use core_test_support::responses::mount_sse_sequence;
use core_test_support::responses::namespace_child_tool;
use core_test_support::responses::sse;
use core_test_support::skip_if_no_network;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::Semaphore;
use wiremock::Mock;
use wiremock::Request;
use wiremock::ResponseTemplate;
use wiremock::matchers::body_partial_json;
use wiremock::matchers::method;
use wiremock::matchers::path_regex;

struct McpResourceClientCapture {
    client: Arc<Mutex<Option<McpResourceClient>>>,
}

struct CoalescingMcpContributor {
    block_next: AtomicBool,
    entered: Semaphore,
    release: Semaphore,
    observed_markers: Mutex<Vec<String>>,
}

struct SessionSourceMcpContributor {
    observed_sources: Arc<Mutex<Vec<SessionSource>>>,
}

impl CoalescingMcpContributor {
    fn new() -> Self {
        Self {
            block_next: AtomicBool::new(false),
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
            observed_markers: Mutex::new(Vec::new()),
        }
    }
}

impl McpServerContributor<Config> for CoalescingMcpContributor {
    fn id(&self) -> &'static str {
        "coalescing_mcp_refresh_test"
    }

    fn contribute<'a>(
        &'a self,
        context: McpServerContributionContext<'a, Config>,
    ) -> ExtensionFuture<'a, Vec<McpServerContribution>> {
        Box::pin(async move {
            let marker = context
                .config()
                .mcp_servers
                .get()
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "initial".to_string());
            self.observed_markers
                .lock()
                .expect("observed markers lock should not be poisoned")
                .push(marker.clone());
            if marker != "initial" {
                self.entered.add_permits(1);
            }
            if self.block_next.swap(false, Ordering::SeqCst) {
                self.release
                    .acquire()
                    .await
                    .expect("release semaphore should remain open")
                    .forget();
            }
            Vec::new()
        })
    }
}

impl McpServerContributor<Config> for SessionSourceMcpContributor {
    fn id(&self) -> &'static str {
        "session_source_mcp_test"
    }

    fn contribute<'a>(
        &'a self,
        context: McpServerContributionContext<'a, Config>,
    ) -> ExtensionFuture<'a, Vec<McpServerContribution>> {
        Box::pin(async move {
            self.observed_sources
                .lock()
                .expect("observed sources lock should not be poisoned")
                .push(
                    context
                        .session_source()
                        .expect("thread-scoped MCP resolution should identify its session source")
                        .clone(),
                );
            Vec::new()
        })
    }
}

fn format_labeled_requests_snapshot(
    scenario: &str,
    sections: &[(&str, &ResponsesRequest)],
) -> String {
    context_snapshot::format_labeled_requests_snapshot(
        scenario,
        sections,
        &ContextSnapshotOptions::default().rewrite_known_segments(),
    )
}

fn enable_deferred_tool_world_state_without_agents(config: &mut Config) {
    config.update_plan_enabled = true;
    config.agents_enabled = false;
    config
        .features
        .enable(Feature::DeferredToolWorldState)
        .expect("test config should allow feature update");
}

fn tools_state_sections(request: &ResponsesRequest) -> Vec<String> {
    request
        .message_input_texts("developer")
        .into_iter()
        .filter(|text| text.starts_with("<tools>"))
        .collect()
}

fn completed_response_sequence(count: usize) -> Vec<String> {
    (1..=count)
        .map(|index| {
            sse(vec![
                ev_response_created(&format!("resp-{index}")),
                ev_assistant_message(&format!("msg-{index}"), "done"),
                ev_completed(&format!("resp-{index}")),
            ])
        })
        .collect()
}

impl ThreadLifecycleContributor<Config> for McpResourceClientCapture {
    fn on_thread_start<'a>(
        &'a self,
        input: ThreadStartInput<'a, Config>,
    ) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            let client = input
                .mcp_resource_client
                .as_ref()
                .expect("host should supply an MCP resource client");
            *self
                .client
                .lock()
                .expect("capture lock should not be poisoned") = Some(client.as_ref().clone());
        })
    }
}

fn config_with_mcp_marker(base: &Config, marker: &str) -> Config {
    let mut config = base.clone();
    let server = serde_json::from_value(json!({
        "url": "http://127.0.0.1:1/mcp",
        "enabled": false,
    }))
    .expect("test MCP server config");
    config
        .mcp_servers
        .set(HashMap::from([(marker.to_string(), server)]))
        .expect("test config should allow MCP servers");
    config
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn root_and_spawned_subagent_receive_distinct_mcp_session_sources() -> Result<()> {
    skip_if_no_network!(Ok(()));

    const PARENT_PROMPT: &str = "spawn an agent to verify its MCP session source";
    const CHILD_PROMPT: &str = "child: report that MCP configuration completed";
    const SPAWN_CALL_ID: &str = "mcp-session-source-spawn";

    let server = responses::start_mock_server().await;
    let spawn_args = serde_json::to_string(&json!({ "message": CHILD_PROMPT }))?;
    mount_sse_once_match(
        &server,
        |request: &Request| {
            std::str::from_utf8(&request.body).is_ok_and(|body| body.contains(PARENT_PROMPT))
                && request.body_json()["client_metadata"]["subagent_kind"].is_null()
        },
        sse(vec![
            ev_response_created("resp-parent-spawn"),
            ev_function_call_with_namespace(
                SPAWN_CALL_ID,
                "multi_agent_v1",
                "spawn_agent",
                &spawn_args,
            ),
            ev_completed("resp-parent-spawn"),
        ]),
    )
    .await;
    let child_response = mount_sse_once_match(
        &server,
        |request: &Request| {
            std::str::from_utf8(&request.body).is_ok_and(|body| body.contains(CHILD_PROMPT))
                && request.body_json()["client_metadata"]["subagent_kind"] == "collab_spawn"
        },
        sse(vec![
            ev_response_created("resp-child"),
            ev_assistant_message("msg-child", "child done"),
            ev_completed("resp-child"),
        ]),
    )
    .await;
    mount_sse_once_match(
        &server,
        |request: &Request| {
            std::str::from_utf8(&request.body).is_ok_and(|body| body.contains(SPAWN_CALL_ID))
                && request.body_json()["client_metadata"]["subagent_kind"].is_null()
        },
        sse(vec![
            ev_response_created("resp-parent-complete"),
            ev_assistant_message("msg-parent", "parent done"),
            ev_completed("resp-parent-complete"),
        ]),
    )
    .await;

    let observed_sources = Arc::new(Mutex::new(Vec::new()));
    let mut extensions = ExtensionRegistryBuilder::<Config>::new();
    extensions.mcp_server_contributor(Arc::new(SessionSourceMcpContributor {
        observed_sources: observed_sources.clone(),
    }));
    let test = core_test_support::test_codex::test_codex()
        .with_extensions(Arc::new(extensions.build()))
        .build(&server)
        .await?;

    test.submit_turn(PARENT_PROMPT).await?;
    tokio::time::timeout(Duration::from_secs(/*secs*/ 10), async {
        while child_response.requests().is_empty() {
            tokio::time::sleep(Duration::from_millis(/*millis*/ 10)).await;
        }
    })
    .await?;

    let observed_sources = observed_sources
        .lock()
        .expect("observed sources lock should not be poisoned");
    assert!(observed_sources.contains(&SessionSource::Exec));
    assert!(observed_sources.iter().any(|source| matches!(
        source,
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id,
            ..
        }) if *parent_thread_id == test.session_configured.thread_id
    )));

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rapid_mcp_refreshes_coalesce_to_the_latest_config() -> Result<()> {
    let server = responses::start_mock_server().await;
    let response = responses::mount_sse_once(
        &server,
        sse(vec![
            ev_response_created("resp-1"),
            ev_assistant_message("msg-1", "done"),
            ev_completed("resp-1"),
        ]),
    )
    .await;
    let contributor = Arc::new(CoalescingMcpContributor::new());
    let mut extensions = ExtensionRegistryBuilder::<Config>::new();
    extensions.mcp_server_contributor(contributor.clone());
    let test = core_test_support::test_codex::test_codex()
        .with_extensions(Arc::new(extensions.build()))
        .build(&server)
        .await?;

    contributor.block_next.store(true, Ordering::SeqCst);
    test.codex
        .refresh_runtime_config(config_with_mcp_marker(&test.config, "config-a"))
        .await;
    tokio::time::timeout(Duration::from_secs(5), contributor.entered.acquire())
        .await
        .expect("configuration A should enter MCP projection")
        .expect("entered semaphore should remain open")
        .forget();

    test.codex
        .refresh_runtime_config(config_with_mcp_marker(&test.config, "config-b"))
        .await;
    test.codex
        .refresh_runtime_config(config_with_mcp_marker(&test.config, "config-c"))
        .await;
    contributor.release.add_permits(1);

    tokio::time::timeout(Duration::from_secs(5), contributor.entered.acquire())
        .await
        .expect("the coalesced refresh should project the latest configuration")
        .expect("entered semaphore should remain open")
        .forget();
    let observed = contributor
        .observed_markers
        .lock()
        .expect("observed markers lock should not be poisoned")
        .iter()
        .filter(|marker| marker.as_str() != "initial")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        vec!["config-a".to_string(), "config-c".to_string()]
    );

    test.submit_turn("bind the latest MCP state").await?;
    assert!(
        !contributor
            .observed_markers
            .lock()
            .expect("observed markers lock should not be poisoned")
            .iter()
            .any(|marker| marker == "config-b")
    );
    response.single_request();
    Ok(())
}

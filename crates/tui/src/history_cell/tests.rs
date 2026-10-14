//! Coverage for history-cell rendering, wrapping, and transcript behavior.

use super::*;
use crate::exec_cell::CommandOutput;
use crate::exec_cell::ExecCall;
use crate::exec_cell::ExecCell;
use crate::legacy_core::config::Config;
use crate::legacy_core::config::ConfigBuilder;
use crate::line_truncation::line_width;
use crate::render::highlight::MAX_HIGHLIGHT_LINE_BYTES;
use crate::session_state::ThreadSessionState;
use crate::wrapping::word_wrap_lines;
use genex_app_server_protocol::AskForApproval;
use genex_app_server_protocol::McpAuthStatus;
use genex_config::types::McpServerConfig;
use genex_otel::RuntimeMetricTotals;
use genex_otel::RuntimeMetricsSummary;
use genex_protocol::ThreadId;
use genex_protocol::account::PlanType;
use genex_protocol::error::UnexpectedResponseError;
use genex_protocol::parse_command::ParsedCommand;
use genex_utils_absolute_path::AbsolutePathBuf;
use dirs::home_dir;
use http::StatusCode;
use pretty_assertions::assert_eq;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

use genex_app_server_protocol::CommandExecutionSource as ExecCommandSource;
use genex_protocol::mcp::CallToolResult;
use genex_protocol::mcp::Tool;
use rmcp::model::ContentBlock;

const SMALL_PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==";



async fn test_config() -> Config {
    let genex_home = std::env::temp_dir();
    ConfigBuilder::default()
        .genex_home(genex_home.clone())
        .build()
        .await
        .expect("config")
}

fn test_cwd() -> PathBuf {
    // These tests only need a stable absolute cwd; using temp_dir() avoids baking Unix- or
    // Windows-specific root semantics into the fixtures.
    std::env::temp_dir()
}


fn stdio_server_config(
    command: &str,
    args: Vec<&str>,
    env: Option<HashMap<String, String>>,
    env_vars: Vec<&str>,
) -> McpServerConfig {
    let mut table = toml::Table::new();
    table.insert(
        "command".to_string(),
        toml::Value::String(command.to_string()),
    );
    if !args.is_empty() {
        table.insert(
            "args".to_string(),
            toml::Value::Array(
                args.into_iter()
                    .map(|arg| toml::Value::String(arg.to_string()))
                    .collect(),
            ),
        );
    }
    if let Some(env) = env {
        table.insert("env".to_string(), string_map_to_toml_value(env));
    }
    if !env_vars.is_empty() {
        table.insert(
            "env_vars".to_string(),
            toml::Value::Array(
                env_vars
                    .into_iter()
                    .map(|name| toml::Value::String(name.to_string()))
                    .collect(),
            ),
        );
    }

    toml::Value::Table(table)
        .try_into()
        .expect("test stdio MCP config should deserialize")
}

fn streamable_http_server_config(
    url: &str,
    bearer_token_env_var: Option<&str>,
    http_headers: Option<HashMap<String, String>>,
    env_http_headers: Option<HashMap<String, String>>,
) -> McpServerConfig {
    let mut table = toml::Table::new();
    table.insert("url".to_string(), toml::Value::String(url.to_string()));
    if let Some(bearer_token_env_var) = bearer_token_env_var {
        table.insert(
            "bearer_token_env_var".to_string(),
            toml::Value::String(bearer_token_env_var.to_string()),
        );
    }
    if let Some(http_headers) = http_headers {
        table.insert(
            "http_headers".to_string(),
            string_map_to_toml_value(http_headers),
        );
    }
    if let Some(env_http_headers) = env_http_headers {
        table.insert(
            "env_http_headers".to_string(),
            string_map_to_toml_value(env_http_headers),
        );
    }

    toml::Value::Table(table)
        .try_into()
        .expect("test streamable_http MCP config should deserialize")
}

fn string_map_to_toml_value(entries: HashMap<String, String>) -> toml::Value {
    toml::Value::Table(
        entries
            .into_iter()
            .map(|(key, value)| (key, toml::Value::String(value)))
            .collect(),
    )
}

fn render_lines(lines: &[Line<'static>]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect()
}

fn render_transcript(cell: &dyn HistoryCell) -> Vec<String> {
    render_lines(&cell.transcript_lines(u16::MAX))
}

fn assert_unstyled_lines(lines: &[Line<'static>]) {
    for line in lines {
        assert_eq!(line.style, Style::default());
        for span in &line.spans {
            assert_eq!(span.style, Style::default());
        }
    }
}

fn image_block(data: &str) -> serde_json::Value {
    serde_json::to_value(ContentBlock::image(data.to_string(), "image/png"))
        .expect("image content should serialize")
}

fn text_block(text: &str) -> serde_json::Value {
    serde_json::to_value(ContentBlock::text(text)).expect("text content should serialize")
}

fn resource_link_block(
    uri: &str,
    name: &str,
    title: Option<&str>,
    description: Option<&str>,
) -> serde_json::Value {
    let mut resource = rmcp::model::Resource::new(uri, name);
    resource.title = title.map(str::to_string);
    resource.description = description.map(str::to_string);
    serde_json::to_value(ContentBlock::resource_link(resource))
        .expect("resource link content should serialize")
}

#[test]
fn raw_lines_from_source_preserves_explicit_blank_lines() {
    let lines = raw_lines_from_source("alpha\n\nbeta\n");

    assert_eq!(
        render_lines(&lines),
        vec!["alpha".to_string(), String::new(), "beta".to_string()]
    );
    assert_unstyled_lines(&lines);
}

#[test]
fn raw_lines_from_source_preserves_trailing_blank_but_not_trailing_newline() {
    assert_eq!(
        render_lines(&raw_lines_from_source("alpha\n\n")),
        vec!["alpha".to_string(), String::new()]
    );
    assert_eq!(raw_lines_from_source(""), Vec::<Line<'static>>::new());
}

#[test]
fn source_backed_cells_render_raw_source_without_prefix_or_style() {
    let user = new_user_prompt(
        "hello\n\nworld\n".to_string(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let assistant = AgentMarkdownCell::new(
        "- item\n\n| A | B |\n| - | - |\n| x | y |\n".to_string(),
        &test_cwd(),
    );
    let reasoning = ReasoningSummaryCell::new(
        "thinking".to_string(),
        "first thought\n\nsecond thought".to_string(),
        &test_cwd(),
        /*transcript_only*/ false,
    );
    let plan = new_proposed_plan(
        "1. Inspect\n\n```sh\ncargo test\n```".to_string(),
        &test_cwd(),
    );

    let user_lines = user.raw_lines();
    assert_eq!(
        render_lines(&user_lines),
        vec!["hello".to_string(), String::new(), "world".to_string()]
    );
    assert_unstyled_lines(&user_lines);

    let assistant_lines = assistant.raw_lines();
    assert_eq!(
        render_lines(&assistant_lines),
        vec![
            "- item".to_string(),
            String::new(),
            "| A | B |".to_string(),
            "| - | - |".to_string(),
            "| x | y |".to_string(),
        ]
    );
    assert_unstyled_lines(&assistant_lines);

    let reasoning_lines = reasoning.raw_lines();
    assert_eq!(
        render_lines(&reasoning_lines),
        vec![
            "first thought".to_string(),
            String::new(),
            "second thought".to_string(),
        ]
    );
    assert_unstyled_lines(&reasoning_lines);

    let plan_lines = plan.raw_lines();
    assert_eq!(
        render_lines(&plan_lines),
        vec![
            "1. Inspect".to_string(),
            String::new(),
            "```sh".to_string(),
            "cargo test".to_string(),
            "```".to_string(),
        ]
    );
    assert_unstyled_lines(&plan_lines);
}

#[test]
fn proposed_plan_cell_renders_markdown_table() {
    let plan = new_proposed_plan(
        "## Plan\n\n| Step | Owner |\n| --- | --- |\n| Verify | Genex |\n".to_string(),
        &test_cwd(),
    );

    let rendered = render_lines(&plan.display_lines(/*width*/ 80));

    assert!(
        rendered.iter().any(|line| line.contains('━')),
        "expected separated table in proposed plan output: {rendered:?}"
    );
    assert!(
        !rendered
            .iter()
            .any(|line| line.trim() == "| Step | Owner |"),
        "did not expect raw table header in rich proposed plan output: {rendered:?}"
    );

    let raw = render_lines(&plan.raw_lines());
    assert!(
        raw.iter().any(|line| line == "| Step | Owner |"),
        "expected raw mode to preserve table markdown source: {raw:?}"
    );
}

#[test]
fn proposed_plan_cell_preserves_wrapped_table_web_links() {
    let destination = "https://example.com/a/very/long/path/to/a/table/artifact";
    let plan = new_proposed_plan(
        format!("| Step | URL |\n| --- | --- |\n| Verify | {destination} |\n"),
        &test_cwd(),
    );

    let lines = plan.display_hyperlink_lines(/*width*/ 32);
    let linked_rows = lines
        .iter()
        .filter(|line| !line.hyperlinks.is_empty())
        .collect::<Vec<_>>();

    assert!(linked_rows.len() > 1);
    assert!(linked_rows.iter().all(|line| {
        line.hyperlinks
            .iter()
            .all(|link| link.destination == destination)
    }));
}

#[test]
fn composite_cell_preserves_child_web_links() {
    let destination = "https://example.com/usage";
    let cell = CompositeHistoryCell::new(vec![
        Box::new(PlainHistoryCell::new(vec![Line::from("/status")])),
        Box::new(WebHyperlinkHistoryCell::new(vec![Line::from(destination)])),
    ]);

    let lines = cell.display_hyperlink_lines(/*width*/ 80);

    assert_eq!(
        lines[2].hyperlinks,
        vec![crate::terminal_hyperlinks::TerminalHyperlink::web(
            /*columns*/ 0..destination.len(),
            destination.to_string(),
        )]
    );
}


#[test]
fn proposed_plan_cell_unwraps_markdown_fenced_table() {
    let plan = new_proposed_plan(
        "## Plan\n\n```markdown\n| Step | Owner |\n| --- | --- |\n| Verify | Genex |\n```\n"
            .to_string(),
        &test_cwd(),
    );

    let rendered = render_lines(&plan.display_lines(/*width*/ 80));

    assert!(
        rendered.iter().any(|line| line.contains('━')),
        "expected separated table for markdown-fenced proposed plan output: {rendered:?}"
    );
    assert!(
        !rendered.iter().any(|line| line.trim() == "```markdown"),
        "did not expect markdown fence to render as code block: {rendered:?}"
    );
}

#[test]
fn structured_tool_cell_renders_raw_plain_text_without_prefix_or_style() {
    let invocation = McpInvocation {
        server: "search".into(),
        tool: "find_docs".into(),
        arguments: Some(json!({"query": "raw mode"})),
    };
    let result = CallToolResult {
        content: vec![text_block("alpha\nbeta")],
        is_error: None,
        structured_content: None,
        meta: None,
    };
    let mut cell = new_active_mcp_tool_call(
        "call-raw".to_string(),
        invocation,
        /*animations_enabled*/ false,
    );
    cell.complete(Duration::from_millis(1), Ok(result));

    let lines = cell.raw_lines();
    let rendered = render_lines(&lines);
    assert!(rendered[0].starts_with("Called search.find_docs("));
    assert_eq!(rendered[1..], ["alpha".to_string(), "beta".to_string()]);
    assert_unstyled_lines(&lines);
}


#[test]
fn image_generation_call_renders_saved_path() {
    let saved_path = test_path_buf("/tmp/generated-image.png").abs();
    let expected_saved_path = format!(
        "  └ Saved to: {}",
        Url::from_file_path(saved_path.as_path()).expect("test path should convert to file URL")
    );
    let cell = new_image_generation_call(
        "call-image-generation".to_string(),
        "completed",
        Some("A tiny blue square".to_string()),
        Some(saved_path),
    );

    assert_eq!(
        render_lines(&cell.display_lines(/*width*/ 80)),
        vec![
            "• Generated Image:".to_string(),
            "  └ A tiny blue square".to_string(),
            expected_saved_path,
        ],
    );
}

fn session_configured_event(model: &str) -> ThreadSessionState {
    ThreadSessionState {
        windows_sandbox_host: crate::app::WindowsSandboxHost::Local,
        thread_id: ThreadId::new(),
        forked_from_id: None,
        fork_parent_title: None,
        thread_name: None,
        model: model.to_string(),
        service_tier: None,
        approval_policy: AskForApproval::Never,
        approvals_reviewer: genex_protocol::config_types::ApprovalsReviewer::User,
        permission_profile: PermissionProfile::read_only(),
        active_permission_profile: None,
        cwd: test_path_buf("/tmp/project").abs(),
        runtime_workspace_roots: Vec::new(),
        instruction_source_paths: Vec::new(),
        reasoning_effort: None,
        collaboration_mode: None,
        message_history: None,
        network_proxy: None,
        rollout_path: Some(PathBuf::new()),
    }
}


#[test]
fn unified_exec_interaction_cell_renders_wait() {
    let cell = new_unified_exec_interaction(/*command_display*/ None, String::new());
    let lines = render_transcript(&cell);
    assert_eq!(lines, vec!["• Waited for background terminal"]);
}

#[test]
fn final_message_separator_preserves_runtime_metrics_for_short_turns() {
    let summary = RuntimeMetricsSummary {
        tool_calls: RuntimeMetricTotals {
            count: 3,
            duration_ms: 2_450,
        },
        api_calls: RuntimeMetricTotals {
            count: 2,
            duration_ms: 1_200,
        },
        streaming_events: RuntimeMetricTotals {
            count: 6,
            duration_ms: 900,
        },
        websocket_calls: RuntimeMetricTotals {
            count: 1,
            duration_ms: 700,
        },
        websocket_events: RuntimeMetricTotals {
            count: 4,
            duration_ms: 1_200,
        },
        responses_api_overhead_ms: 650,
        responses_api_inference_time_ms: 1_940,
        responses_api_engine_iapi_ttft_ms: 410,
        responses_api_engine_service_ttft_ms: 460,
        responses_api_engine_iapi_tbt_ms: 1_180.0,
        responses_api_engine_service_tbt_ms: 1_240.0,
        turn_ttft_ms: 0,
        turn_ttfm_ms: 0,
    };
    let cell = FinalMessageSeparator::new(Some(12), Some(summary));
    let rendered = render_lines(&cell.display_lines(/*width*/ 600));

    assert_eq!(rendered.len(), 1);
    assert!(rendered[0].starts_with("  Local tools:"));
    assert!(rendered[0].contains("Local tools: 3 calls (2.5s)"));
    assert!(rendered[0].contains("Inference: 2 calls (1.2s)"));
    assert!(rendered[0].contains("WebSocket: 1 events send (700ms)"));
    assert!(rendered[0].contains("Streams: 6 events (900ms)"));
    assert!(rendered[0].contains("4 events received (1.2s)"));
    assert!(rendered[0].contains("Responses API overhead: 650ms"));
    assert!(rendered[0].contains("Responses API inference: 1.9s"));
    assert!(rendered[0].contains("TTFT: 410ms (iapi) 460ms (service)"));
    assert!(rendered[0].contains("TBT: 1.2s (iapi) 1.2s (service)"));
}


#[test]
fn final_message_separator_includes_worked_label_after_one_minute() {
    let cell = FinalMessageSeparator::new(Some(61), /*runtime_metrics*/ None);
    let rendered = render_lines(&cell.display_lines(/*width*/ 200));

    assert_eq!(rendered.len(), 1);
    assert!(rendered[0].contains("Worked for"));
}


#[tokio::test]
async fn session_info_uses_availability_nux_tooltip_override() {
    let config = test_config().await;
    let cell = new_session_info(
        &config,
        &crate::local_settings::LocalSettings::from(&config),
        "example-model",
        &session_configured_event("example-model"),
        /*is_first_event*/ false,
        Some("Model just became available".to_string()),
        Some(PlanType::Free),
        "Provider",
    );

    let rendered = render_transcript(&cell).join("\n");
    assert!(rendered.contains("Model just became available"));
}

#[tokio::test]
#[cfg_attr(
    target_os = "windows",
    ignore = "snapshot path rendering differs on Windows"
)]


#[tokio::test]
async fn session_info_first_event_suppresses_tooltips_and_nux() {
    let config = test_config().await;
    let cell = new_session_info(
        &config,
        &crate::local_settings::LocalSettings::from(&config),
        "example-model",
        &session_configured_event("example-model"),
        /*is_first_event*/ true,
        Some("Model just became available".to_string()),
        Some(PlanType::Free),
        "Provider",
    );

    let rendered = render_transcript(&cell).join("\n");
    assert!(!rendered.contains("Model just became available"));
    assert!(rendered.contains("To get started"));
}

#[tokio::test]
async fn session_info_hides_tooltips_when_disabled() {
    let mut config = test_config().await;
    config.show_tooltips = false;
    let cell = new_session_info(
        &config,
        &crate::local_settings::LocalSettings::from(&config),
        "example-model",
        &session_configured_event("example-model"),
        /*is_first_event*/ false,
        Some("Model just became available".to_string()),
        Some(PlanType::Free),
        "Provider",
    );

    let rendered = render_transcript(&cell).join("\n");
    assert!(!rendered.contains("Model just became available"));
}

















#[test]
fn empty_agent_message_cell_transcript() {
    let cell = AgentMessageCell::new(vec![Line::default()], /*is_first_line*/ false);
    assert_eq!(cell.transcript_lines(/*width*/ 80), vec![Line::from("  ")]);
    assert_eq!(cell.desired_transcript_height(/*width*/ 80), 1);
}

#[test]
fn prefixed_wrapped_history_cell_indents_wrapped_lines() {
    let summary = Line::from(vec![
        "You ".into(),
        "approved".bold(),
        " genex to run ".into(),
        "echo something really long to ensure wrapping happens".dim(),
        " this time".bold(),
    ]);
    let cell = PrefixedWrappedHistoryCell::new(summary, "✔ ".green(), "  ");
    let rendered = render_lines(&cell.display_lines(/*width*/ 24));
    assert_eq!(
        rendered,
        vec![
            "✔ You approved genex to".to_string(),
            "  run echo something".to_string(),
            "  really long to ensure".to_string(),
            "  wrapping happens this".to_string(),
            "  time".to_string(),
        ]
    );
}

#[test]
fn prefixed_wrapped_history_cell_does_not_split_url_like_token() {
    let url_like = "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890";
    let cell = PrefixedWrappedHistoryCell::new(Line::from(url_like), "✔ ".green(), "  ");
    let rendered = render_lines(&cell.display_lines(/*width*/ 24));

    assert_eq!(
        rendered
            .iter()
            .filter(|line| line.contains(url_like))
            .count(),
        1,
        "expected full URL-like token in one rendered line, got: {rendered:?}"
    );
}

#[test]
fn unified_exec_interaction_cell_does_not_split_url_like_stdin_token() {
    let url_like = "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890";
    let cell = UnifiedExecInteractionCell::new(Some("true".to_string()), url_like.to_string());
    let rendered = render_lines(&cell.display_lines(/*width*/ 24));

    assert_eq!(
        rendered
            .iter()
            .filter(|line| line.contains(url_like))
            .count(),
        1,
        "expected full URL-like token in one rendered line, got: {rendered:?}"
    );
}

#[test]
fn prefixed_wrapped_history_cell_height_matches_wrapped_rendering() {
    let url_like = "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/reports/performance/summary/detail/with/a/very/long/path";
    let cell: Box<dyn HistoryCell> = Box::new(PrefixedWrappedHistoryCell::new(
        Line::from(url_like),
        "✔ ".green(),
        "  ",
    ));

    let width: u16 = 24;
    let logical_height = cell.display_lines(width).len() as u16;
    let wrapped_height = cell.desired_height(width);
    assert!(
        wrapped_height > logical_height,
        "expected wrapped height to exceed logical line count ({logical_height}), got {wrapped_height}"
    );

    let area = Rect::new(0, 0, width, wrapped_height);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    cell.render(area, &mut buf);

    let first_row = (0..area.width)
        .map(|x| {
            let symbol = buf[(x, 0)].symbol();
            if symbol.is_empty() {
                ' '
            } else {
                symbol.chars().next().unwrap_or(' ')
            }
        })
        .collect::<String>();
    assert!(
        first_row.contains("✔"),
        "expected first rendered row to keep the prefix visible, got: {first_row:?}"
    );
}

#[test]
fn unified_exec_interaction_cell_height_matches_wrapped_rendering() {
    let url_like = "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/reports/performance/summary/detail/with/a/very/long/path";
    let cell: Box<dyn HistoryCell> = Box::new(UnifiedExecInteractionCell::new(
        Some("true".to_string()),
        url_like.to_string(),
    ));

    let width: u16 = 24;
    let logical_height = cell.display_lines(width).len() as u16;
    let wrapped_height = cell.desired_height(width);
    assert!(
        wrapped_height > logical_height,
        "expected wrapped height to exceed logical line count ({logical_height}), got {wrapped_height}"
    );

    let area = Rect::new(0, 0, width, wrapped_height);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    cell.render(area, &mut buf);

    let first_row = (0..area.width)
        .map(|x| {
            let symbol = buf[(x, 0)].symbol();
            if symbol.is_empty() {
                ' '
            } else {
                symbol.chars().next().unwrap_or(' ')
            }
        })
        .collect::<String>();
    assert!(
        first_row.contains("Interacted with"),
        "expected first rendered row to keep the header visible, got: {first_row:?}"
    );
}







#[test]
fn web_search_history_cell_truncates() {
    let query = "example search query with several generic words to exercise wrapping".to_string();
    let cell = new_web_search_call(
        "call-1".to_string(),
        query.clone(),
        WebSearchAction::Search {
            query: Some(query),
            queries: None,
        },
    );
    let rendered = render_lines(&cell.display_lines(/*width*/ 64));

    assert_eq!(
        rendered,
        vec!["• Searched the web for example search query with several generi…".to_string(),]
    );
}

#[test]
fn web_search_history_cell_short_query_does_not_wrap() {
    let query = "short query".to_string();
    let cell = new_web_search_call(
        "call-1".to_string(),
        query.clone(),
        WebSearchAction::Search {
            query: Some(query),
            queries: None,
        },
    );
    let rendered = render_lines(&cell.display_lines(/*width*/ 64));

    assert_eq!(
        rendered,
        vec!["• Searched the web for short query".to_string()]
    );
}






#[test]
fn mcp_inventory_loading_without_animations_is_stable() {
    let cell = new_mcp_inventory_loading(/*animations_enabled*/ false);
    let first = render_lines(&cell.display_lines(/*width*/ 80));
    let second = render_lines(&cell.display_lines(/*width*/ 80));

    assert_eq!(first, second);
    assert_eq!(first, vec!["• Loading MCP inventory…".to_string()]);
}










#[test]
fn session_header_includes_reasoning_level_when_present() {
    let cell = SessionHeaderHistoryCell::new(
        "legacy-model-4o".to_string(),
        Some(ReasoningEffortConfig::High),
        std::env::temp_dir(),
        "test",
    );

    let lines = render_lines(&cell.display_lines(/*width*/ 80));
    let model_line = lines
        .iter()
        .find(|line| line.contains("model:"))
        .expect("model line");

    assert!(model_line.contains("legacy-model-4o high"));
    assert!(model_line.contains("/model to change"));
}

#[test]
fn session_header_uses_injected_provider_name() {
    let cell = SessionHeaderHistoryCell::new(
        "example-model".to_string(),
        None,
        std::env::temp_dir(),
        "test",
    )
    .with_provider_name("Injected Provider".to_string());

    let rendered = render_lines(&cell.display_lines(/*width*/ 80)).join("\n");
    assert!(rendered.contains("provider Injected Provider"));
    assert!(!rendered.contains("provider Genex"));
}


#[test]
#[cfg_attr(
    target_os = "windows",
    ignore = "snapshot path rendering differs on Windows"
)]




#[test]
fn yolo_mode_includes_managed_full_access_profiles() {
    let permission_profile: PermissionProfile = PermissionProfile::Managed {
        network: NetworkSandboxPolicy::Enabled,
        file_system: ManagedFileSystemPermissions::Unrestricted,
    };

    assert!(has_yolo_permissions(
        AskForApproval::Never,
        &permission_profile
    ));
}

#[test]
fn yolo_mode_excludes_external_sandbox_profiles() {
    let permission_profile: PermissionProfile = PermissionProfile::External {
        network: NetworkSandboxPolicy::Enabled,
    };

    assert!(!has_yolo_permissions(
        AskForApproval::Never,
        &permission_profile
    ));
}

#[test]
fn session_header_directory_center_truncates() {
    let mut dir = home_dir().expect("home directory");
    for part in ["hello", "the", "fox", "is", "very", "fast"] {
        dir.push(part);
    }

    let formatted = SessionHeaderHistoryCell::format_directory_inner(&dir, Some(24));
    let sep = std::path::MAIN_SEPARATOR;
    let expected = format!("~{sep}hello{sep}the{sep}…{sep}very{sep}fast");
    assert_eq!(formatted, expected);
}

#[test]
fn session_header_directory_front_truncates_long_segment() {
    let mut dir = home_dir().expect("home directory");
    dir.push("supercalifragilisticexpialidocious");

    let formatted = SessionHeaderHistoryCell::format_directory_inner(&dir, Some(18));
    let sep = std::path::MAIN_SEPARATOR;
    let expected = format!("~{sep}…cexpialidocious");
    assert_eq!(formatted, expected);
}

















#[test]
fn user_history_cell_height_matches_rendered_lines_with_remote_images() {
    let cell = UserHistoryCell {
        spoken: false,
        message: "line one\nline two".to_string(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: vec![
            "https://example.com/one.png".to_string(),
            "https://example.com/two.png".to_string(),
        ],
    };

    let width = 80;
    let rendered_len: u16 = cell
        .display_lines(width)
        .len()
        .try_into()
        .unwrap_or(u16::MAX);
    assert_eq!(cell.desired_height(width), rendered_len);
    assert_eq!(cell.desired_transcript_height(width), rendered_len);
}

#[test]
fn user_history_cell_trims_trailing_blank_message_lines() {
    let cell = UserHistoryCell {
        spoken: false,
        message: "line one\n\n   \n\t \n".to_string(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: vec!["https://example.com/one.png".to_string()],
    };

    let rendered = render_lines(&cell.display_lines(/*width*/ 80));
    let trailing_blank_count = rendered
        .iter()
        .rev()
        .take_while(|line| line.trim().is_empty())
        .count();
    assert_eq!(trailing_blank_count, 1);
    assert!(rendered.iter().any(|line| line.contains("line one")));
}

#[test]
fn user_history_cell_trims_trailing_blank_message_lines_with_text_elements() {
    let message = "tokenized\n\n\n".to_string();
    let cell = UserHistoryCell {
        spoken: false,
        message,
        text_elements: vec![TextElement::new(
            (0..8).into(),
            Some("tokenized".to_string()),
        )],
        local_image_paths: Vec::new(),
        remote_image_urls: vec!["https://example.com/one.png".to_string()],
    };

    let rendered = render_lines(&cell.display_lines(/*width*/ 80));
    let trailing_blank_count = rendered
        .iter()
        .rev()
        .take_while(|line| line.trim().is_empty())
        .count();
    assert_eq!(trailing_blank_count, 1);
    assert!(rendered.iter().any(|line| line.contains("tokenized")));
}

#[test]
fn render_uses_wrapping_for_long_url_like_line() {
    let url = "https://example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/reports/performance/summary/detail/with/a/very/long/path/that/keeps/going/for/testing/purposes-only-and-does/not/need/to/resolve/index.html?session_id=abc123def456ghi789jkl012mno345pqr678stu901vwx234yz";
    let cell: Box<dyn HistoryCell> = Box::new(UserHistoryCell {
        spoken: false,
        message: url.to_string(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: Vec::new(),
    });

    let width: u16 = 52;
    let height = cell.desired_height(width);
    assert!(
        height > 1,
        "expected wrapped height for long URL, got {height}"
    );

    let area = Rect::new(0, 0, width, height);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    cell.render(area, &mut buf);

    let rendered = (0..area.height)
        .map(|y| {
            (0..area.width)
                .map(|x| {
                    let symbol = crate::terminal_hyperlinks::strip_osc8(buf[(x, y)].symbol());
                    if symbol.is_empty() {
                        ' '
                    } else {
                        symbol.chars().next().unwrap_or(' ')
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    let rendered_blob = rendered.join("\n");
    let rendered_url = rendered
        .iter()
        .filter(|row| !row.trim().is_empty())
        .enumerate()
        .map(|(index, row)| {
            if index == 0 {
                row.strip_prefix("› ").unwrap().trim()
            } else {
                row.trim()
            }
        })
        .collect::<String>();

    assert_eq!(
        rendered_url, url,
        "wrapped URL must preserve every character"
    );

    let non_empty_rows = rendered.iter().filter(|row| !row.trim().is_empty()).count() as u16;
    assert!(
        non_empty_rows > 3,
        "expected long URL to span multiple visible rows, got:\n{rendered_blob}"
    );
}



#[test]
fn plan_update_does_not_split_url_like_tokens_in_note_or_step() {
    let note_url = "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890";
    let step_url = "example.test/api/v1/projects/beta-team/releases/2026-02-17/builds/0987654321/artifacts/reports/performance";
    let update = UpdatePlanArgs {
        explanation: Some(format!(
            "Investigate failures under {note_url} immediately."
        )),
        plan: vec![PlanItemArg {
            step: format!("Validate callbacks under {step_url} before rollout."),
            status: StepStatus::InProgress,
        }],
    };

    let cell = new_plan_update(update);
    let rendered = render_lines(&cell.display_lines(/*width*/ 30));

    assert_eq!(
        rendered
            .iter()
            .filter(|line| line.contains(note_url))
            .count(),
        1,
        "expected full note URL-like token in one rendered line, got: {rendered:?}"
    );
    assert_eq!(
        rendered
            .iter()
            .filter(|line| line.contains(step_url))
            .count(),
        1,
        "expected full step URL-like token in one rendered line, got: {rendered:?}"
    );
}

#[test]
fn reasoning_summary_block() {
    let cell = new_reasoning_summary_block(
        vec!["**High level reasoning**\n\nDetailed reasoning goes here.".to_string()],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(rendered_transcript, vec!["• Detailed reasoning goes here."]);
}

#[test]
fn reasoning_summary_height_matches_wrapped_rendering_for_url_like_content() {
    let summary = "example.test/api/v1/projects/alpha-team/releases/2026-02-17/builds/1234567890/artifacts/reports/performance/summary/detail/with/a/very/long/path/that/keeps/going";
    let cell: Box<dyn HistoryCell> = Box::new(ReasoningSummaryCell::new(
        "High level reasoning".to_string(),
        summary.to_string(),
        &test_cwd(),
        /*transcript_only*/ false,
    ));
    let width: u16 = 24;

    let logical_height = cell.display_lines(width).len() as u16;
    let wrapped_height = cell.desired_height(width);
    let expected_wrapped_height = Paragraph::new(Text::from(cell.display_lines(width)))
        .wrap(Wrap { trim: false })
        .line_count(width) as u16;
    assert_eq!(wrapped_height, expected_wrapped_height);
    assert!(
        wrapped_height >= logical_height,
        "expected wrapped height to be at least logical line count ({logical_height}), got {wrapped_height}"
    );

    let wrapped_transcript_height = cell.desired_transcript_height(width);
    assert_eq!(wrapped_transcript_height, wrapped_height);

    let area = Rect::new(0, 0, width, wrapped_height);
    let mut buf = ratatui::buffer::Buffer::empty(area);
    cell.render(area, &mut buf);

    let first_row = (0..area.width)
        .map(|x| {
            let symbol = buf[(x, 0)].symbol();
            if symbol.is_empty() {
                ' '
            } else {
                symbol.chars().next().unwrap_or(' ')
            }
        })
        .collect::<String>();
    assert!(
        first_row.contains("•"),
        "expected first rendered row to keep summary bullet visible, got: {first_row:?}"
    );
}

#[test]
fn reasoning_summary_block_returns_reasoning_cell_when_feature_disabled() {
    let cell = new_reasoning_summary_block(
        vec!["Detailed reasoning goes here.".to_string()],
        &test_cwd(),
    );

    let rendered = render_transcript(cell.as_ref());
    assert_eq!(rendered, vec!["• Detailed reasoning goes here."]);
}

#[tokio::test]
async fn reasoning_summary_block_respects_config_overrides() {
    let mut config = test_config().await;
    config.model = Some("legacy-model-3".to_string());
    let cell = new_reasoning_summary_block(
        vec!["**High level reasoning**\n\nDetailed reasoning goes here.".to_string()],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());
}

#[test]
fn reasoning_summary_block_falls_back_when_header_is_missing() {
    let cell = new_reasoning_summary_block(
        vec!["**High level reasoning without closing".to_string()],
        &test_cwd(),
    );

    let rendered = render_transcript(cell.as_ref());
    assert_eq!(rendered, vec!["• **High level reasoning without closing"]);
}

#[test]
fn reasoning_summary_block_falls_back_when_summary_is_missing() {
    let cell = new_reasoning_summary_block(
        vec!["**High level reasoning without closing**".to_string()],
        &test_cwd(),
    );

    let rendered = render_transcript(cell.as_ref());
    assert_eq!(rendered, vec!["• High level reasoning without closing"]);

    let cell = new_reasoning_summary_block(
        vec!["**High level reasoning without closing**\n\n  ".to_string()],
        &test_cwd(),
    );

    let rendered = render_transcript(cell.as_ref());
    assert_eq!(rendered, vec!["• High level reasoning without closing"]);
}

#[test]
fn reasoning_summary_block_keeps_title_only_summary_in_expanded_transcript() {
    let cell = new_reasoning_summary_block(
        vec!["**Confirming backend JSONL source**".to_string()],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(
        rendered_transcript,
        vec!["• Confirming backend JSONL source"]
    );
}

#[test]
fn reasoning_summary_block_splits_header_and_summary_when_present() {
    let cell = new_reasoning_summary_block(
        vec!["**High level plan**\n\nWe should fix the bug next.".to_string()],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(rendered_transcript, vec!["• We should fix the bug next."]);
}

#[test]
fn reasoning_summary_block_hides_empty_html_comment_parts() {
    let cell = new_reasoning_summary_block(
        vec![
            "**Checking the first thing**\n\n<!-- -->".to_string(),
            "**Checking the second thing**\n\n<!-- -->".to_string(),
        ],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(rendered_transcript, Vec::<String>::new());
}

#[test]
fn reasoning_summary_block_preserves_bold_content_after_empty_html_comment_part() {
    let cell = new_reasoning_summary_block(
        vec![
            "**Status**\n\n<!-- -->".to_string(),
            "**Important conclusion**".to_string(),
            "<!-- -->".to_string(),
        ],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(rendered_transcript, vec!["• Important conclusion"]);

    let cell = new_reasoning_summary_block(
        vec![
            "**Status**\n\n<!-- -->".to_string(),
            "**Result:** keep **this**".to_string(),
        ],
        &test_cwd(),
    );

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(rendered_transcript, vec!["• Result: keep this"]);
}

#[test]
fn reasoning_summary_block_strips_header_after_leading_empty_part() {
    let cell = new_reasoning_summary_block(
        vec![
            "**Status**\n\n<!-- -->".to_string(),
            "**Checking tests**\n\nTests passed".to_string(),
        ],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(rendered_transcript, vec!["• Tests passed"]);
}

#[test]
fn reasoning_summary_block_drops_empty_part_after_real_content() {
    let cell = new_reasoning_summary_block(
        vec![
            "**Plan**\n\ndone".to_string(),
            "**Checking tests**\n\n<!-- -->".to_string(),
        ],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(rendered_transcript, vec!["• done"]);
}

#[test]
fn reasoning_summary_block_preserves_literal_html_comment() {
    let cell = new_reasoning_summary_block(
        vec!["**Plan**\n\nUse `<!-- -->` in JSX.".to_string()],
        &test_cwd(),
    );

    let rendered_display = render_lines(&cell.display_lines(/*width*/ 80));
    assert_eq!(rendered_display, Vec::<String>::new());

    let rendered_transcript = render_transcript(cell.as_ref());
    assert_eq!(rendered_transcript, vec!["• Use <!-- --> in JSX."]);
}

#[test]
fn deprecation_notice_renders_summary_with_details() {
    let cell = new_deprecation_notice(
        "Feature flag `foo`".to_string(),
        Some("Use flag `bar` instead.".to_string()),
    );
    let lines = cell.display_lines(/*width*/ 80);
    let rendered = render_lines(&lines);
    assert_eq!(
        rendered,
        vec![
            "⚠ Feature flag `foo`".to_string(),
            "Use flag `bar` instead.".to_string(),
        ]
    );
}

#[test]
fn agent_markdown_cell_renders_source_at_different_widths() {
    let source =
        "A long agent message that should wrap differently when the terminal width changes.\n";
    let cell = AgentMarkdownCell::new(source.to_string(), &test_cwd());

    let lines_80 = render_lines(&cell.display_lines(/*width*/ 80));
    assert!(
        lines_80.first().is_some_and(|line| line.starts_with("• ")),
        "first line should start with bullet prefix: {:?}",
        lines_80[0]
    );

    let lines_32 = render_lines(&cell.display_lines(/*width*/ 32));
    assert!(
        lines_32.len() > lines_80.len(),
        "narrower width should produce more wrapped lines: {lines_32:?}",
    );
}

#[test]
fn agent_markdown_cell_does_not_split_words_after_inline_markdown() {
    let source = "This paragraph is intentionally long so you can inspect soft wrapping behavior while also checking inline formatting like **bold text**, *italic text*, ***bold italic text***, `inline code`, ~~strikethrough~~, a [link to example.com](https://example.com), and a literal path like [README.md](/Users/felipe.coury/code/genex.fcoury-worktrees/README.md) without introducing manual line breaks.\n";
    let cell = AgentMarkdownCell::new(source.to_string(), &test_cwd());

    let lines = render_lines(&cell.display_lines(/*width*/ 190));
    assert!(
        lines[0].ends_with("inline code,"),
        "expected wrapping to stop before 'strikethrough': {lines:?}",
    );
    assert!(
        lines[1].starts_with("  strikethrough,"),
        "expected the next line to resume with the full word: {lines:?}",
    );
}


#[test]
fn agent_markdown_cell_narrow_width_shows_prefix_only() {
    let source = "narrow width coverage\n";
    let cell = AgentMarkdownCell::new(source.to_string(), &test_cwd());

    let lines = render_lines(&cell.display_lines(/*width*/ 2));
    assert_eq!(lines, vec!["• ".to_string()]);
}

#[test]
fn wrapped_and_prefixed_cells_handle_tiny_widths() {
    let user_cell = UserHistoryCell {
        spoken: false,
        message: "tiny width coverage for wrapped user history".to_string(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: Vec::new(),
    };
    let agent_message_cell = AgentMessageCell::new(
        vec!["tiny width agent line".into()],
        /*is_first_line*/ true,
    );
    let reasoning_cell = ReasoningSummaryCell::new(
        "Plan".to_string(),
        "Reasoning summary content for tiny widths.".to_string(),
        &test_cwd(),
        /*transcript_only*/ false,
    );
    let agent_markdown_cell =
        AgentMarkdownCell::new("tiny width agent markdown line\n".to_string(), &test_cwd());

    for width in 1..=4 {
        assert!(
            !user_cell.display_lines(width).is_empty(),
            "user cell should render at width {width}",
        );
        assert!(
            !agent_message_cell.display_lines(width).is_empty(),
            "agent message cell should render at width {width}",
        );
        assert!(
            !reasoning_cell.display_lines(width).is_empty(),
            "reasoning cell should render at width {width}",
        );
        assert!(
            !agent_markdown_cell.display_lines(width).is_empty(),
            "agent markdown cell should render at width {width}",
        );
    }
}

#[test]
fn render_clears_area_when_cell_content_shrinks() {
    let area = Rect::new(0, 0, 40, 6);
    let mut buf = Buffer::empty(area);

    let first: Box<dyn HistoryCell> = Box::new(PlainHistoryCell::new(vec![
        Line::from("STALE ROW 1"),
        Line::from("STALE ROW 2"),
        Line::from("STALE ROW 3"),
        Line::from("STALE ROW 4"),
    ]));
    first.render(area, &mut buf);

    let second: Box<dyn HistoryCell> = Box::new(PlainHistoryCell::new(vec![Line::from("fresh")]));
    second.render(area, &mut buf);

    let mut rendered_rows: Vec<String> = Vec::new();
    for y in 0..area.height {
        let mut row = String::new();
        for x in 0..area.width {
            row.push_str(buf.cell((x, y)).expect("cell should exist").symbol());
        }
        rendered_rows.push(row);
    }

    assert!(
        rendered_rows.iter().all(|row| !row.contains("STALE")),
        "rendered buffer should not retain stale glyphs: {rendered_rows:?}",
    );
    assert!(
        rendered_rows
            .first()
            .is_some_and(|row| row.contains("fresh")),
        "expected fresh content in first row: {rendered_rows:?}",
    );
}

#[test]
fn agent_markdown_cell_survives_insert_history_rewrap() {
    let source = "\
  Canary rollout remained at limited traffic longer than planned because p95
  latency briefly regressed during cold-cache periods.
  Regional expansion succeeded with stable error rates, though internal
  analytics lagged temporarily.
  ";
    let cell = AgentMarkdownCell::new(source.to_string(), &test_cwd());
    let width: u16 = 80;
    let lines = cell.display_lines(width);

    // Simulate what insert_history_lines does: word_wrap_lines with
    // the terminal width and no indent.
    let rewrapped = word_wrap_lines(&lines, width as usize);
    let before = render_lines(&lines);
    let after = render_lines(&rewrapped);
    assert_eq!(
        before, after,
        "word_wrap_lines should not alter lines that already fit within width"
    );
}

/// Simulate the consolidation backward-walk logic from `App::handle_event`
/// to verify it correctly identifies and replaces `AgentMessageCell` runs.
#[test]
fn consolidation_walker_replaces_agent_message_cells() {
    use std::sync::Arc;

    // Build a transcript with: [UserCell, AgentMsg(head), AgentMsg(cont), AgentMsg(cont)]
    let user = Arc::new(UserHistoryCell {
        spoken: false,
        message: "hello".to_string(),
        text_elements: Vec::new(),
        local_image_paths: Vec::new(),
        remote_image_urls: Vec::new(),
    }) as Arc<dyn HistoryCell>;
    let head = Arc::new(AgentMessageCell::new(
        vec![Line::from("line 1")],
        /*is_first_line*/ true,
    )) as Arc<dyn HistoryCell>;
    let cont1 = Arc::new(AgentMessageCell::new(
        vec![Line::from("line 2")],
        /*is_first_line*/ false,
    )) as Arc<dyn HistoryCell>;
    let cont2 = Arc::new(AgentMessageCell::new(
        vec![Line::from("line 3")],
        /*is_first_line*/ false,
    )) as Arc<dyn HistoryCell>;

    let mut transcript_cells: Vec<Arc<dyn HistoryCell>> = vec![user.clone(), head, cont1, cont2];

    // Run the same consolidation logic as the handler.
    let source = "line 1\nline 2\nline 3\n".to_string();
    let end = transcript_cells.len();
    let mut start = end;
    while start > 0
        && transcript_cells[start - 1].is_stream_continuation()
        && transcript_cells[start - 1]
            .as_any()
            .is::<AgentMessageCell>()
    {
        start -= 1;
    }
    if start > 0
        && transcript_cells[start - 1]
            .as_any()
            .is::<AgentMessageCell>()
        && !transcript_cells[start - 1].is_stream_continuation()
    {
        start -= 1;
    }

    assert_eq!(
        start, 1,
        "should find all 3 agent cells starting at index 1"
    );
    assert_eq!(end, 4);

    // Splice.
    let consolidated: Arc<dyn HistoryCell> = Arc::new(AgentMarkdownCell::new(source, &test_cwd()));
    transcript_cells.splice(start..end, std::iter::once(consolidated));

    assert_eq!(transcript_cells.len(), 2, "should be [user, consolidated]");

    // Verify first cell is still the user cell.
    assert!(
        transcript_cells[0].as_any().is::<UserHistoryCell>(),
        "first cell should be UserHistoryCell"
    );

    // Verify second cell is AgentMarkdownCell.
    assert!(
        transcript_cells[1].as_any().is::<AgentMarkdownCell>(),
        "second cell should be AgentMarkdownCell"
    );
}

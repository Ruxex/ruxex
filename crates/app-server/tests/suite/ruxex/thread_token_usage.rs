use anyhow::Result;
use app_test_support::MockResponsesConfig;
use app_test_support::TestAppServer;
use app_test_support::create_fake_rollout;
use app_test_support::create_fake_rollout_with_token_usage_and_amount;
use app_test_support::create_mock_responses_server_repeating_assistant;
use app_test_support::rollout_path;
use pretty_assertions::assert_eq;
use ruxex_app_server_protocol::ClientRequest;
use ruxex_app_server_protocol::RequestId;
use ruxex_app_server_protocol::ThreadResumeParams;
use ruxex_app_server_protocol::ThreadResumeResponse;
use ruxex_app_server_protocol::ThreadTokenUsageReadParams;
use ruxex_app_server_protocol::ThreadTokenUsageReadResponse;
use serde_json::json;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use tempfile::TempDir;
use tokio::time::timeout;

const DEFAULT_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::test]
async fn token_usage_read_returns_persisted_snapshot() -> Result<()> {
    let responses = create_mock_responses_server_repeating_assistant("Done").await;
    let ruxex_home = TempDir::new()?;
    MockResponsesConfig::new(&responses.uri()).write(ruxex_home.path())?;
    let thread_id = create_fake_rollout_with_token_usage_and_amount(
        ruxex_home.path(),
        "2025-01-01T00-00-00",
        "2025-01-01T00:00:00Z",
        "old chat",
    )?;
    let mut server = TestAppServer::builder()
        .with_ruxex_home(ruxex_home.path())
        .build_initialized()
        .await?;

    let request_id = server
        .send_thread_token_usage_read_request(ThreadTokenUsageReadParams {
            thread_id,
        })
        .await?;
    let response: ThreadTokenUsageReadResponse =
        timeout(DEFAULT_READ_TIMEOUT, server.read_response(request_id)).await??;
    let token_usage = response.token_usage.expect("usage should be restored");

    assert_eq!(token_usage.total.total_tokens, 150);
    assert_eq!(token_usage.total.input_tokens, 120);
    assert_eq!(token_usage.total.cached_input_tokens, 20);
    assert_eq!(token_usage.total.output_tokens, 30);
    assert_eq!(token_usage.total.reasoning_output_tokens, 10);
    assert_eq!(token_usage.last.total_tokens, 90);
    assert_eq!(token_usage.total_amount, Some(0.56));
    assert_eq!(token_usage.last_amount, Some(0.03));
    assert_eq!(token_usage.model_context_window, Some(200_000));

    Ok(())
}

#[tokio::test]
async fn token_usage_read_returns_latest_record_before_and_after_resume() -> Result<()> {
    let responses = create_mock_responses_server_repeating_assistant("Done").await;
    let ruxex_home = TempDir::new()?;
    MockResponsesConfig::new(&responses.uri()).write(ruxex_home.path())?;
    let filename_ts = "2025-01-01T00-00-00";
    let thread_id = create_fake_rollout_with_token_usage_and_amount(
        ruxex_home.path(),
        filename_ts,
        "2025-01-01T00:00:00Z",
        "old chat",
    )?;
    let rollout = rollout_path(ruxex_home.path(), filename_ts, &thread_id);
    let newer_record = json!({
        "timestamp": "2025-01-01T00:00:01Z",
        "type": "token_usage_record",
        "payload": {
            "thread_id": thread_id,
            "turn_id": "turn-new",
            "session_id": thread_id,
            "root_turn_id": "turn-new",
            "response_id": "response-new",
            "usage": token_usage_json(100),
            "amount": 0.05,
            "turn_token_usage": token_usage_json(100),
            "thread_token_usage": token_usage_json(250),
            "turn_amount": 0.05,
            "thread_amount": 0.88
        }
    });
    let mut file = OpenOptions::new().append(true).open(&rollout)?;
    writeln!(file, "{newer_record}")?;
    normalize_rollout_ordinals(&rollout)?;

    let mut server = TestAppServer::builder()
        .with_ruxex_home(ruxex_home.path())
        .build_initialized()
        .await?;
    assert_latest_record(&mut server, &thread_id).await?;

    let _: ThreadResumeResponse = server
        .request(|request_id| ClientRequest::ThreadResume {
            request_id,
            params: ThreadResumeParams {
                thread_id: thread_id.clone(),
                exclude_turns: true,
                model: Some("test-model".to_string()),
                ..Default::default()
            },
        })
        .await?;
    assert_latest_record(&mut server, &thread_id).await?;

    Ok(())
}

async fn assert_latest_record(server: &mut TestAppServer, thread_id: &str) -> Result<()> {
    let request_id = server
        .send_thread_token_usage_read_request(ThreadTokenUsageReadParams {
            thread_id: thread_id.to_string(),
        })
        .await?;
    let response: ThreadTokenUsageReadResponse =
        timeout(DEFAULT_READ_TIMEOUT, server.read_response(request_id)).await??;
    let token_usage = response.token_usage.expect("usage should be restored");
    assert_eq!(token_usage.total.total_tokens, 250);
    assert_eq!(token_usage.last.total_tokens, 100);
    assert_eq!(token_usage.total_amount, Some(0.88));
    assert_eq!(token_usage.last_amount, Some(0.05));
    assert_eq!(token_usage.model_context_window, None);
    Ok(())
}

fn token_usage_json(total_tokens: u64) -> serde_json::Value {
    json!({
        "input_tokens": total_tokens,
        "cached_input_tokens": 0,
        "cache_write_input_tokens": 0,
        "output_tokens": 0,
        "reasoning_output_tokens": 0,
        "total_tokens": total_tokens
    })
}

fn normalize_rollout_ordinals(path: &std::path::Path) -> Result<()> {
    let lines = fs::read_to_string(path)?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let contents = lines
        .into_iter()
        .enumerate()
        .map(|(ordinal, mut line)| {
            line["ordinal"] = json!(ordinal);
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{contents}\n"))?;
    Ok(())
}

#[tokio::test]
async fn token_usage_read_returns_null_when_thread_has_no_usage() -> Result<()> {
    let responses = create_mock_responses_server_repeating_assistant("Done").await;
    let ruxex_home = TempDir::new()?;
    MockResponsesConfig::new(&responses.uri()).write(ruxex_home.path())?;
    let thread_id = create_fake_rollout(
        ruxex_home.path(),
        "2025-01-01T00-00-00",
        "2025-01-01T00:00:00Z",
        "chat without usage",
        None,
    )?;
    let mut server = TestAppServer::builder()
        .with_ruxex_home(ruxex_home.path())
        .build_initialized()
        .await?;

    let request_id = server
        .send_thread_token_usage_read_request(ThreadTokenUsageReadParams {
            thread_id,
        })
        .await?;
    let response: ThreadTokenUsageReadResponse =
        timeout(DEFAULT_READ_TIMEOUT, server.read_response(request_id)).await??;

    assert_eq!(response.token_usage, None);
    Ok(())
}

#[tokio::test]
async fn token_usage_read_missing_thread_uses_thread_read_error() -> Result<()> {
    let responses = create_mock_responses_server_repeating_assistant("Done").await;
    let ruxex_home = TempDir::new()?;
    MockResponsesConfig::new(&responses.uri()).write(ruxex_home.path())?;
    let mut server = TestAppServer::builder()
        .with_ruxex_home(ruxex_home.path())
        .build_initialized()
        .await?;

    let request_id = server
        .send_thread_token_usage_read_request(ThreadTokenUsageReadParams {
            thread_id: "00000000-0000-7000-8000-000000000001".to_string(),
        })
        .await?;
    let error = timeout(
        DEFAULT_READ_TIMEOUT,
        server.read_stream_until_error_message(RequestId::Integer(request_id)),
    )
    .await??;

    assert_eq!(error.error.code, -32600);
    assert!(error.error.message.contains("thread not loaded"));
    Ok(())
}

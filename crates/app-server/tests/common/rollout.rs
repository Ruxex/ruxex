use anyhow::Result;
use core_test_support::test_path_buf;
use ruxex_protocol::SessionId;
use ruxex_protocol::ThreadId;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::GitInfo;
use ruxex_protocol::protocol::SessionMeta;
use ruxex_protocol::protocol::SessionMetaLine;
use ruxex_protocol::protocol::SessionSource;
use ruxex_protocol::protocol::ThreadHistoryMode;
use ruxex_protocol::protocol::ThreadSource;
use ruxex_protocol::protocol::TokenCountEvent;
use ruxex_protocol::protocol::TokenUsage;
use ruxex_protocol::protocol::TokenUsageInfo;
use serde_json::json;
use std::fs;
use std::fs::FileTimes;
use std::path::Path;
use std::path::PathBuf;
use uuid::Uuid;

pub fn rollout_path(ruxex_home: &Path, filename_ts: &str, thread_id: &str) -> PathBuf {
    let year = &filename_ts[0..4];
    let month = &filename_ts[5..7];
    let day = &filename_ts[8..10];
    ruxex_home
        .join("sessions")
        .join(year)
        .join(month)
        .join(day)
        .join(format!("rollout-{filename_ts}-{thread_id}.jsonl"))
}

/// Create a minimal rollout file under `RUXEX_HOME/sessions/YYYY/MM/DD/`.
///
/// - `filename_ts` is the filename timestamp component in `YYYY-MM-DDThh-mm-ss` format.
/// - `meta_rfc3339` is the envelope timestamp used in JSON lines.
/// - `preview` is the user message preview text.
///
/// Returns the generated conversation/session UUID as a string.
pub fn create_fake_rollout(
    ruxex_home: &Path,
    filename_ts: &str,
    meta_rfc3339: &str,
    preview: &str,
    git_info: Option<GitInfo>,
) -> Result<String> {
    create_fake_rollout_with_source(
        ruxex_home,
        filename_ts,
        meta_rfc3339,
        preview,
        git_info,
        SessionSource::Cli,
    )
}

/// Creates a minimal paginated rollout with ordinalized JSONL records.
pub fn create_fake_paginated_rollout(
    ruxex_home: &Path,
    filename_ts: &str,
    meta_rfc3339: &str,
    preview: &str,
    git_info: Option<GitInfo>,
) -> Result<String> {
    let thread_id = create_fake_rollout(ruxex_home, filename_ts, meta_rfc3339, preview, git_info)?;
    let path = rollout_path(ruxex_home, filename_ts, &thread_id);
    let mut lines = fs::read_to_string(path.as_path())?
        .lines()
        .map(serde_json::from_str::<serde_json::Value>)
        .collect::<Result<Vec<_>, _>>()?;
    lines[0]["payload"]["history_mode"] = serde_json::to_value(ThreadHistoryMode::Paginated)?;
    for (ordinal, line) in lines.iter_mut().enumerate() {
        line["ordinal"] = serde_json::to_value(ordinal)?;
    }
    let contents = lines
        .into_iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(path, format!("{contents}\n"))?;
    Ok(thread_id)
}

/// Creates a minimal rollout whose history includes a persisted token usage event.
///
/// Resume and fork tests use this fixture to verify lifecycle replay of restored
/// usage without starting a model turn. The exact token values are intentionally
/// non-zero and asymmetric so assertions catch swapped total/last fields and
/// dropped cached or reasoning counters.
pub fn create_fake_rollout_with_token_usage(
    ruxex_home: &Path,
    filename_ts: &str,
    meta_rfc3339: &str,
    preview: &str,
) -> Result<String> {
    let thread_id = create_fake_rollout(
        ruxex_home,
        filename_ts,
        meta_rfc3339,
        preview,
        /*git_info*/ None,
    )?;
    let payload = serde_json::to_value(EventMsg::TokenCount(TokenCountEvent {
        info: Some(TokenUsageInfo {
            total_token_usage: TokenUsage {
                input_tokens: 120,
                cached_input_tokens: 20,
                cache_write_input_tokens: 0,
                output_tokens: 30,
                reasoning_output_tokens: 10,
                total_tokens: 150,
                ruxex_rollout_budget_units: None,
            },
            last_token_usage: TokenUsage {
                input_tokens: 70,
                cached_input_tokens: 10,
                cache_write_input_tokens: 0,
                output_tokens: 20,
                reasoning_output_tokens: 5,
                total_tokens: 90,
                ruxex_rollout_budget_units: None,
            },
            model_context_window: Some(200_000),
        }),
        rate_limits: None,
    }))?;
    let file_path = rollout_path(ruxex_home, filename_ts, &thread_id);
    let line = json!({
        "timestamp": meta_rfc3339,
        "type": "event_msg",
        "payload": payload
    })
    .to_string();
    fs::write(
        &file_path,
        format!("{}{}\n", fs::read_to_string(&file_path)?, line),
    )?;
    Ok(thread_id)
}

/// Create a minimal rollout file with an explicit session source.
pub fn create_fake_rollout_with_source(
    ruxex_home: &Path,
    filename_ts: &str,
    meta_rfc3339: &str,
    preview: &str,
    git_info: Option<GitInfo>,
    source: SessionSource,
) -> Result<String> {
    create_fake_rollout_with_session_and_thread_source(
        ruxex_home,
        filename_ts,
        meta_rfc3339,
        preview,
        git_info,
        source,
        /*thread_source*/ None,
    )
}

/// Create a minimal rollout file with explicit session and thread sources.
#[allow(clippy::too_many_arguments)]
pub fn create_fake_rollout_with_session_and_thread_source(
    ruxex_home: &Path,
    filename_ts: &str,
    meta_rfc3339: &str,
    preview: &str,
    git_info: Option<GitInfo>,
    source: SessionSource,
    thread_source: Option<ThreadSource>,
) -> Result<String> {
    create_fake_rollout_with_source_and_parent_thread_id(
        ruxex_home,
        filename_ts,
        meta_rfc3339,
        preview,
        git_info,
        source,
        thread_source,
        /*session_id*/ None,
        /*parent_thread_id*/ None,
    )
}

/// Create a minimal rollout file with an explicit root session and control parent.
#[allow(clippy::too_many_arguments)]
pub fn create_fake_parented_rollout_with_source(
    ruxex_home: &Path,
    filename_ts: &str,
    meta_rfc3339: &str,
    preview: &str,
    git_info: Option<GitInfo>,
    source: SessionSource,
    session_id: SessionId,
    parent_thread_id: ThreadId,
) -> Result<String> {
    create_fake_rollout_with_source_and_parent_thread_id(
        ruxex_home,
        filename_ts,
        meta_rfc3339,
        preview,
        git_info,
        source,
        /*thread_source*/ None,
        Some(session_id),
        Some(parent_thread_id),
    )
}

#[allow(clippy::too_many_arguments)]
fn create_fake_rollout_with_source_and_parent_thread_id(
    ruxex_home: &Path,
    filename_ts: &str,
    meta_rfc3339: &str,
    preview: &str,
    git_info: Option<GitInfo>,
    source: SessionSource,
    thread_source: Option<ThreadSource>,
    session_id: Option<SessionId>,
    parent_thread_id: Option<ThreadId>,
) -> Result<String> {
    let uuid = Uuid::new_v4();
    let uuid_str = uuid.to_string();
    let conversation_id = ThreadId::from_string(&uuid_str)?;
    let session_id = session_id.unwrap_or_else(|| conversation_id.into());

    let file_path = rollout_path(ruxex_home, filename_ts, &uuid_str);
    let dir = file_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("missing rollout parent directory"))?;
    fs::create_dir_all(dir)?;

    // Build JSONL lines
    let meta = SessionMeta {
        session_id,
        id: conversation_id,
        forked_from_id: None,
        forked_from_ordinal_exclusive: None,
        parent_thread_id,
        timestamp: meta_rfc3339.to_string(),
        cwd: test_path_buf("/"),
        runtime_workspace_roots: None,
        originator: "ruxex".to_string(),
        cli_version: "0.0.0".to_string(),
        source,
        thread_source,
        agent_path: None,
        agent_nickname: None,
        agent_role: None,
        base_instructions: None,
        dynamic_tools: None,
        selected_capability_roots: Vec::new(),
        history_mode: Default::default(),
        history_base: None,
        subagent_history_start_ordinal: None,
        multi_agent_version: None,
        context_window: None,
    };
    let payload = serde_json::to_value(SessionMetaLine {
        meta,
        git: git_info,
    })?;

    let lines = [
        json!({
            "timestamp": meta_rfc3339,
            "type": "session_meta",
            "payload": payload
        })
        .to_string(),
        json!({
            "timestamp": meta_rfc3339,
            "type":"response_item",
            "payload": {
                "type":"message",
                "role":"user",
                "content":[{"type":"input_text","text": preview}]
            }
        })
        .to_string(),
        json!({
            "timestamp": meta_rfc3339,
            "type":"event_msg",
            "payload": {
                "type":"user_message",
                "message": preview,
                "kind": "plain"
            }
        })
        .to_string(),
    ];

    fs::write(&file_path, lines.join("\n") + "\n")?;
    let parsed = chrono::DateTime::parse_from_rfc3339(meta_rfc3339)?.with_timezone(&chrono::Utc);
    let times = FileTimes::new().set_modified(parsed.into());
    std::fs::OpenOptions::new()
        .append(true)
        .open(&file_path)?
        .set_times(times)?;
    Ok(uuid_str)
}

pub fn create_fake_rollout_with_text_elements(
    ruxex_home: &Path,
    filename_ts: &str,
    meta_rfc3339: &str,
    preview: &str,
    text_elements: Vec<serde_json::Value>,
    git_info: Option<GitInfo>,
) -> Result<String> {
    let uuid = Uuid::new_v4();
    let uuid_str = uuid.to_string();
    let conversation_id = ThreadId::from_string(&uuid_str)?;

    // sessions/YYYY/MM/DD derived from filename_ts (YYYY-MM-DDThh-mm-ss)
    let year = &filename_ts[0..4];
    let month = &filename_ts[5..7];
    let day = &filename_ts[8..10];
    let dir = ruxex_home.join("sessions").join(year).join(month).join(day);
    fs::create_dir_all(&dir)?;

    let file_path = dir.join(format!("rollout-{filename_ts}-{uuid}.jsonl"));

    // Build JSONL lines
    let meta = SessionMeta {
        session_id: conversation_id.into(),
        id: conversation_id,
        forked_from_id: None,
        forked_from_ordinal_exclusive: None,
        parent_thread_id: None,
        timestamp: meta_rfc3339.to_string(),
        cwd: test_path_buf("/"),
        runtime_workspace_roots: None,
        originator: "ruxex".to_string(),
        cli_version: "0.0.0".to_string(),
        source: SessionSource::Cli,
        thread_source: None,
        agent_path: None,
        agent_nickname: None,
        agent_role: None,
        base_instructions: None,
        dynamic_tools: None,
        selected_capability_roots: Vec::new(),
        history_mode: Default::default(),
        history_base: None,
        subagent_history_start_ordinal: None,
        multi_agent_version: None,
        context_window: None,
    };
    let payload = serde_json::to_value(SessionMetaLine {
        meta,
        git: git_info,
    })?;

    let lines = [
        json!( {
            "timestamp": meta_rfc3339,
            "type": "session_meta",
            "payload": payload
        })
        .to_string(),
        json!( {
            "timestamp": meta_rfc3339,
            "type":"response_item",
            "payload": {
                "type":"message",
                "role":"user",
                "content":[{"type":"input_text","text": preview}]
            }
        })
        .to_string(),
        json!( {
            "timestamp": meta_rfc3339,
            "type":"event_msg",
            "payload": {
                "type":"user_message",
                "message": preview,
                "text_elements": text_elements,
                "local_images": []
            }
        })
        .to_string(),
    ];

    fs::write(file_path, lines.join("\n") + "\n")?;
    Ok(uuid_str)
}

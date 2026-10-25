//! Exercises cold shared rollout compression through fork, checkpoint resume, and model input.

use std::fs::FileTimes;
use std::fs::OpenOptions;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use ruxex_core::RuxexThread;
use ruxex_core::TurnInputRequest;
use ruxex_features::Feature;
use ruxex_history::InitialHistory;
use ruxex_history::ResumedHistory;
use ruxex_protocol::mcp::ClientMcpExtensions;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::Op;
use ruxex_protocol::protocol::ThreadHistoryMode;
use ruxex_protocol::user_input::UserInput;
use ruxex_thread_store::ForkBoundary;
use ruxex_thread_store::LoadThreadHistoryParams;
use ruxex_thread_store::PrepareForkParams;
use core_test_support::responses::ResponseMock;
use core_test_support::responses::ev_assistant_message;
use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::sse;
use core_test_support::responses::start_mock_server;
use core_test_support::skip_if_no_network;
use core_test_support::test_ruxex::test_ruxex;
use core_test_support::wait_for_event;
use pretty_assertions::assert_eq;
use wiremock::MockServer;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn compressed_shared_fork_resume_preserves_checkpoint_and_frozen_history() -> Result<()> {
    skip_if_no_network!(Ok(()));

    let server = start_mock_server().await;
    let test = test_ruxex()
        .with_history_mode(ThreadHistoryMode::Paginated)
        .with_config(|config| {
            config.model_provider.name = "Local compaction test provider".to_string();
            config
                .features
                .disable(Feature::LocalThreadStoreCompression)
                .expect("disable compression while building the lineage");
        })
        .build_with_auto_env(&server)
        .await?;
    turn(
        &server,
        &test.ruxex,
        "Create a checkpoint",
        "OBSOLETE_PRE_CHECKPOINT_REPLY",
    )
    .await?;
    mount_sse_once(
        &server,
        sse(vec![
            ev_assistant_message("summary", "PERSISTED_COMPRESSION_CHECKPOINT"),
            ev_completed("compact"),
        ]),
    )
    .await;
    test.ruxex.submit(Op::Compact).await?;
    wait_for_event(&test.ruxex, |event| {
        matches!(event, EventMsg::TurnComplete(_))
    })
    .await;
    turn(
        &server,
        &test.ruxex,
        "shared-compression: inherited parent turn",
        "INHERITED_COMPRESSION_REPLY",
    )
    .await?;

    let prepared = test
        .thread_store
        .prepare_fork(PrepareForkParams {
            thread_id: test.session_configured.thread_id,
            boundary: ForkBoundary::Latest,
        })
        .await?;
    let child = test
        .thread_manager
        .fork_prepared_thread(
            ruxex_core::StartThreadOptions::new(test.config.clone()),
            prepared,
        )
        .await?;
    turn(
        &server,
        &test.ruxex,
        "shared-compression: parent AFTER fork cutoff",
        "POST_FORK_COMPRESSION_REPLY",
    )
    .await?;
    turn(
        &server,
        &child.thread,
        "shared-compression: child before shutdown",
        "CHILD_COMPRESSION_REPLY",
    )
    .await?;

    let parent_path = test.ruxex.rollout_path().context("parent rollout")?;
    let child_path = child.thread.rollout_path().context("child rollout")?;
    test.ruxex.shutdown_and_wait().await?;
    child.thread.shutdown_and_wait().await?;
    assert_eq!(
        ruxex_rollout::read_session_meta_line(&child_path)
            .await?
            .meta
            .history_base
            .map(|base| base.thread_id),
        Some(test.session_configured.thread_id),
    );

    let paths = [&parent_path, &child_path];
    let original_bytes = paths
        .iter()
        .map(std::fs::read)
        .collect::<std::io::Result<Vec<_>>>()?;
    let old = SystemTime::now() - Duration::from_secs(8 * 24 * 60 * 60);
    for path in paths {
        OpenOptions::new()
            .write(true)
            .open(path)?
            .set_times(FileTimes::new().set_modified(old))?;
    }
    let mut config = test.config.clone();
    config
        .features
        .enable(Feature::LocalThreadStoreCompression)?;
    // Use production feature wiring, rather than manually writing zstd files or calling the worker.
    let store =
        ruxex_core::thread_store_from_config(&config, ruxex_core::init_state_db(&config).await);
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if paths
                .iter()
                .all(|path| !path.exists() && path.with_extension("jsonl.zst").exists())
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .context("shared rollout compression should finish")?;
    for (path, original) in paths.into_iter().zip(original_bytes) {
        let compressed = std::fs::File::open(path.with_extension("jsonl.zst"))?;
        assert_eq!(zstd::stream::decode_all(compressed)?, original);
    }

    // Follow the paginated resume path: load the latest checkpoint from disk, then resume core.
    let context = store
        .load_latest_model_context(LoadThreadHistoryParams {
            thread_id: child.thread_id,
            include_archived: false,
        })
        .await?;
    let resumed = test
        .thread_manager
        .resume_thread_with_history(
            config,
            InitialHistory::Resumed(ResumedHistory {
                history_revision: context.revision,
                conversation_id: context.thread_id,
                history: Arc::new(context.items),
                rollout_path: Some(child_path),
            }),
            ruxex_core::test_support::credential_manager_from_auth(ruxex_login::RuxexAuth::from_api_key(
                "dummy",
            )),
            /*parent_trace*/ None,
            ClientMcpExtensions::default(),
        )
        .await?;
    let followup = turn(
        &server,
        &resumed.thread,
        "shared-compression: resumed followup",
        "resumed reply",
    )
    .await?;
    let request = followup.single_request();
    assert_eq!(
        request
            .message_input_texts("user")
            .into_iter()
            .filter(|text| text.starts_with("shared-compression:"))
            .collect::<Vec<_>>(),
        vec![
            "shared-compression: inherited parent turn",
            "shared-compression: child before shutdown",
            "shared-compression: resumed followup",
        ],
    );
    let input = serde_json::to_string(&request.input())?;
    assert!(input.contains("PERSISTED_COMPRESSION_CHECKPOINT"));
    assert!(input.contains("INHERITED_COMPRESSION_REPLY"));
    assert!(input.contains("CHILD_COMPRESSION_REPLY"));
    assert!(!input.contains("POST_FORK_COMPRESSION_REPLY"));
    assert!(!input.contains("OBSOLETE_PRE_CHECKPOINT_REPLY"));
    assert!(
        !parent_path.exists(),
        "reading the ancestor must not materialize it"
    );
    assert!(parent_path.with_extension("jsonl.zst").exists());
    resumed.thread.shutdown_and_wait().await?;
    Ok(())
}

async fn turn(
    server: &MockServer,
    thread: &Arc<RuxexThread>,
    prompt: &str,
    reply: &str,
) -> Result<ResponseMock> {
    let mock = mount_sse_once(
        server,
        sse(vec![
            ev_response_created(prompt),
            ev_assistant_message("reply", reply),
            ev_completed(prompt),
        ]),
    )
    .await;
    thread
        .start_or_steer_turn(TurnInputRequest::user_input(vec![UserInput::Text {
            text: prompt.to_string(),
            text_elements: Vec::new(),
        }]))
        .await?;
    wait_for_event(thread, |event| matches!(event, EventMsg::TurnComplete(_))).await;
    Ok(mock)
}

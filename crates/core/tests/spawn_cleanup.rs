//! Verifies that a cancelled subagent spawn does not leave a provisional child behind.

use std::any::Any;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use core_test_support::responses;
use core_test_support::test_ruxex::test_ruxex;
use core_test_support::wait_for_event;
use ruxex_features::Feature;
use ruxex_protocol::ThreadId;
use ruxex_protocol::protocol::EventMsg;
use ruxex_protocol::protocol::Op;
use ruxex_protocol::protocol::ThreadHistoryMode;
use ruxex_protocol::user_input::UserInput;
use ruxex_rollout::RolloutItem;
use ruxex_state::DirectionalThreadSpawnEdgeStatus;
use ruxex_thread_store::AppendThreadItemsParams;
use ruxex_thread_store::ArchiveThreadParams;
use ruxex_thread_store::CreateThreadParams;
use ruxex_thread_store::DeleteThreadParams;
use ruxex_thread_store::InMemoryThreadStore;
use ruxex_thread_store::ListThreadsParams;
use ruxex_thread_store::LoadThreadHistoryParams;
use ruxex_thread_store::PersistContext;
use ruxex_thread_store::ReadThreadByRolloutPathParams;
use ruxex_thread_store::ReadThreadParams;
use ruxex_thread_store::ResumeThreadParams;
use ruxex_thread_store::StoredModelContext;
use ruxex_thread_store::StoredThread;
use ruxex_thread_store::StoredThreadHistory;
use ruxex_thread_store::ThreadPage;
use ruxex_thread_store::ThreadStore;
use ruxex_thread_store::ThreadStoreFuture;
use ruxex_thread_store::UpdateThreadMetadataParams;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::time::timeout;

#[derive(Debug)]
struct PendingCheckpoint {
    thread_id: ThreadId,
    complete: oneshot::Sender<()>,
}

struct GatedCheckpointStore {
    inner: InMemoryThreadStore,
    armed: AtomicBool,
    contexts: Arc<Mutex<Vec<(ThreadId, PersistContext)>>>,
    checkpoints: mpsc::UnboundedSender<PendingCheckpoint>,
}

macro_rules! delegate_store_methods {
    ($(fn $name:ident($param:ident: $params:ty) -> $result:ty;)*) => {
        $(fn $name(&self, $param: $params) -> ThreadStoreFuture<'_, $result> {
            ThreadStore::$name(&self.inner, $param)
        })*
    };
}

impl ThreadStore for GatedCheckpointStore {
    fn as_any(&self) -> &dyn Any {
        self
    }

    delegate_store_methods! {
        fn create_thread(params: CreateThreadParams) -> ();
        fn resume_thread(params: ResumeThreadParams) -> Arc<Vec<RolloutItem>>;
        fn append_items(params: AppendThreadItemsParams) -> ();
        fn discard_thread(thread_id: ThreadId) -> ();
        fn load_history(params: LoadThreadHistoryParams) -> StoredThreadHistory;
        fn load_latest_model_context(params: LoadThreadHistoryParams) -> StoredModelContext;
        fn read_thread(params: ReadThreadParams) -> StoredThread;
        fn read_thread_by_rollout_path(params: ReadThreadByRolloutPathParams) -> StoredThread;
        fn list_threads(params: ListThreadsParams) -> ThreadPage;
        fn update_thread_metadata(params: UpdateThreadMetadataParams) -> Option<StoredThread>;
        fn archive_thread(params: ArchiveThreadParams) -> ();
        fn unarchive_thread(params: ArchiveThreadParams) -> StoredThread;
        fn delete_thread(params: DeleteThreadParams) -> ();
        fn flush_thread(thread_id: ThreadId) -> ();
        fn shutdown_thread(thread_id: ThreadId) -> ();
    }

    fn persist_thread(
        &self,
        thread_id: ThreadId,
        context: PersistContext,
    ) -> ThreadStoreFuture<'_, ()> {
        Box::pin(async move {
            self.contexts
                .lock()
                .expect("context log should not be poisoned")
                .push((thread_id, context));
            if context == PersistContext::SubagentSpawn {
                self.armed.store(true, Ordering::SeqCst);
                return self.inner.persist_thread(thread_id, context).await;
            }
            if context == PersistContext::Standard && self.armed.swap(false, Ordering::SeqCst) {
                let (complete, completed) = oneshot::channel();
                self.checkpoints
                    .send(PendingCheckpoint {
                        thread_id,
                        complete,
                    })
                    .expect("checkpoint receiver should stay alive");
                completed.await.expect("test should complete checkpoint");
            }
            self.inner.persist_thread(thread_id, context).await
        })
    }
}

#[test]
fn cancelled_spawn_discards_provisional_child() -> Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .thread_stack_size(4 * 1024 * 1024)
        .enable_all()
        .build()
        .context("build test runtime")?
        .block_on(cancelled_spawn_discards_provisional_child_impl())
}

async fn cancelled_spawn_discards_provisional_child_impl() -> Result<()> {
    let server = responses::start_mock_server().await;
    let (checkpoints, mut checkpoint_requests) = mpsc::unbounded_channel();
    let contexts = Arc::new(Mutex::new(Vec::new()));
    let store = Arc::new(GatedCheckpointStore {
        inner: InMemoryThreadStore::default(),
        armed: AtomicBool::new(false),
        contexts: Arc::clone(&contexts),
        checkpoints,
    });
    let mut builder = test_ruxex()
        .with_thread_store(store.clone())
        .with_history_mode(ThreadHistoryMode::Paginated)
        .with_config(|config| {
            config.agent_max_depth = 1;
            config
                .features
                .enable(Feature::Collab)
                .expect("enable collaboration");
        });
    let test = builder.build_with_auto_env(&server).await?;
    let _responses = responses::mount_sse_sequence(
        &server,
        vec![responses::sse(vec![
            responses::ev_response_created("parent-spawn-response"),
            responses::ev_function_call_with_namespace(
                "spawn-checkpoint",
                "multi_agent_v1",
                "spawn_agent",
                &serde_json::json!({
                    "message": "child startup task",
                    "task_name": "worker",
                    "fork_context": true,
                })
                .to_string(),
            ),
            responses::ev_completed("parent-spawn"),
        ])],
    )
    .await;

    test.ruxex
        .start_or_steer_turn(ruxex_core::TurnInputRequest::user_input(vec![
            UserInput::Text {
                text: "parent context to inherit".to_string(),
                text_elements: Vec::new(),
            },
        ]))
        .await?;
    let checkpoint = timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                checkpoint = checkpoint_requests.recv() => {
                    break Ok::<_, anyhow::Error>(checkpoint.expect("child history should reach its durability barrier"));
                }
                event = test.ruxex.next_event() => {
                    let event = event.expect("event stream should stay open while spawning");
                    if let EventMsg::Error(error) = event.msg {
                        anyhow::bail!("spawn emitted an error before its durability checkpoint: {}; contexts: {:?}", error.message, contexts.lock().expect("context log should not be poisoned"));
                    }
                }
            }
        }
    })
    .await
    .context("waiting for child durability checkpoint")??;
    let child = test.thread_manager.get_thread(checkpoint.thread_id).await?;
    let state_db = ruxex_core::init_state_db(&test.config)
        .await
        .expect("state db should be enabled");

    test.ruxex.submit(Op::Interrupt).await?;
    wait_for_event(&test.ruxex, |event| {
        matches!(event, EventMsg::TurnAborted(_))
    })
    .await;
    let _ = checkpoint.complete.send(());
    timeout(Duration::from_secs(10), child.wait_until_terminated())
        .await
        .context("waiting for provisional child termination")?;
    timeout(Duration::from_secs(10), async {
        while test
            .thread_manager
            .get_thread(checkpoint.thread_id)
            .await
            .is_ok()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .context("waiting for provisional child removal")?;
    timeout(Duration::from_secs(10), async {
        loop {
            let open_children = state_db
                .list_thread_spawn_children_with_status(
                    test.session_configured.thread_id,
                    DirectionalThreadSpawnEdgeStatus::Open,
                )
                .await?;
            let closed_children = state_db
                .list_thread_spawn_children_with_status(
                    test.session_configured.thread_id,
                    DirectionalThreadSpawnEdgeStatus::Closed,
                )
                .await?;
            if open_children.is_empty() && closed_children == vec![checkpoint.thread_id] {
                return Ok::<(), anyhow::Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .context("waiting for closed spawn edge")??;

    test.ruxex.shutdown_and_wait().await?;
    Ok(())
}
